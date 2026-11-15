use std::io;
use std::panic::Location;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant};

use egui::{Context, Event, FullOutput, RawInput, Ui, ViewportId};

use crate::input::{Batch, HeldInput};
use crate::nodes::{self, DomainTarget, Target};
use crate::server::{self, Server, Shared};

pub(crate) struct Capture {
    pub image: Arc<egui::ColorImage>,
    pub ppp: f32,
    pub sequence: u64,
    pub generation: u64,
    pub targets: Vec<Target>,
}

struct CaptureId(u64);

struct Pending {
    sequence: u64,
    generation: u64,
    started: Instant,
    ppp: f32,
    size: egui::Vec2,
    targets: Vec<Target>,
}

pub(crate) struct Bridge {
    path: PathBuf,
    shared: Arc<Mutex<Shared>>,
    incoming: Mutex<mpsc::Receiver<Batch>>,
    images: mpsc::SyncSender<Capture>,
    _server: Server,
    pending: Option<Pending>,
    custom: Vec<DomainTarget>,
    targets: Vec<Target>,
    ppp: f32,
    bounds: egui::Rect,
    generation: u64,
    release_pending: bool,
    sequence: u64,
    last_capture: Instant,
    held: HeldInput,
}

pub(crate) fn install(ctx: &Context, app: &str, path: &Path) -> io::Result<()> {
    if let Some(existing) = ctx.with_plugin::<Bridge, _>(|bridge| bridge.path.clone()) {
        if existing == path {
            return Ok(());
        }
        return Err(io::Error::other(
            "debug bridge already installed with another path",
        ));
    }
    let (send_input, incoming) = mpsc::sync_channel(64);
    let (images, receive_images) = mpsc::sync_channel(1);
    let shared = Arc::new(Mutex::new(Shared::default()));
    let server = server::start(path, app, shared.clone(), send_input, receive_images)?;
    ctx.enable_accesskit();
    ctx.add_plugin(Bridge {
        path: path.into(),
        shared,
        incoming: Mutex::new(incoming),
        images,
        _server: server,
        pending: None,
        custom: Vec::new(),
        targets: Vec::new(),
        ppp: 0.0,
        bounds: egui::Rect::NOTHING,
        generation: 0,
        release_pending: false,
        sequence: 0,
        last_capture: Instant::now() - Duration::from_secs(1),
        held: HeldInput::default(),
    });
    Ok(())
}

impl Bridge {
    pub fn target(
        &mut self,
        _ui: &Ui,
        response: &egui::Response,
        path: &str,
        label: &str,
        location: &Location<'_>,
    ) {
        if self.custom.len() < 2048 {
            self.custom
                .push(DomainTarget::new(response, path, label, location));
        }
    }

    fn receive_capture(&mut self, input: &RawInput) {
        if self
            .pending
            .as_ref()
            .is_some_and(|pending| pending.started.elapsed() > Duration::from_secs(2))
        {
            self.pending = None;
            return;
        }
        let Some(pending) = &self.pending else {
            return;
        };
        for event in &input.events {
            let Event::Screenshot {
                viewport_id: ViewportId::ROOT,
                user_data,
                image,
            } = event
            else {
                continue;
            };
            let Some(token) = user_data
                .data
                .as_ref()
                .and_then(|data| data.downcast_ref::<CaptureId>())
            else {
                continue;
            };
            if token.0 != pending.sequence {
                continue;
            }
            let expected = pending.size * pending.ppp;
            let aligned = (image.width() as f32 - expected.x).abs() <= 1.0
                && (image.height() as f32 - expected.y).abs() <= 1.0;
            if aligned {
                let _ = self.images.try_send(Capture {
                    image: image.clone(),
                    ppp: pending.ppp,
                    sequence: token.0,
                    generation: pending.generation,
                    targets: pending.targets.clone(),
                });
            }
            self.pending = None;
            self.last_capture = Instant::now();
            break;
        }
    }
}

impl egui::plugin::Plugin for Bridge {
    fn debug_name(&self) -> &'static str {
        "j-ui-debug"
    }

    fn on_begin_pass(&mut self, ui: &mut Ui) {
        if ui.ctx().viewport_id() == ViewportId::ROOT {
            self.custom.clear();
        }
    }

    fn input_hook(&mut self, ctx: &Context, input: &mut RawInput) {
        if input.viewport_id != ViewportId::ROOT {
            return;
        }
        self.receive_capture(input);
        if self.release_pending && self.held.focused {
            self.held.release(input);
        }
        self.release_pending = false;
        let bounds = input.screen_rect.unwrap_or_else(|| ctx.viewport_rect());
        let lost_focus = input
            .events
            .iter()
            .any(|event| matches!(event, Event::WindowFocused(false)));
        if self.held.focused && lost_focus {
            self.held.release(input);
        }
        while let Ok(batch) = self
            .incoming
            .get_mut()
            .expect("input receiver poisoned")
            .try_recv()
        {
            let same_size = (bounds.width() - batch.width).abs() < 1.0
                && (bounds.height() - batch.height).abs() < 1.0;
            if batch.generation != self.generation
                || batch.accepted.elapsed() > Duration::from_millis(300)
                || !same_size
            {
                self.held.release(input);
                continue;
            }
            for event in batch.events {
                self.held.event(event, bounds, input);
            }
        }
        let disconnected = self.shared.lock().map_or(true, |shared| {
            shared.last_input.elapsed() > Duration::from_secs(2)
        });
        if self.held.focused && disconnected {
            self.held.release(input);
        }
        if self.held.focused {
            input.focused = true;
        }
    }

    fn output_hook(&mut self, ctx: &Context, output: &mut FullOutput) {
        if ctx.viewport_id() != ViewportId::ROOT {
            return;
        }
        let targets = nodes::collect(
            ctx,
            output.platform_output.accesskit_update.as_ref(),
            &self.custom,
        );
        let bounds = ctx.viewport_rect();
        let ppp = output.pixels_per_point;
        let same_layout = targets.len() == self.targets.len()
            && targets.iter().zip(&self.targets).all(|(target, previous)| {
                target.id == previous.id
                    && target.path == previous.path
                    && target.rect == previous.rect
                    && target.z_order == previous.z_order
                    && target.role == previous.role
                    && target.enabled == previous.enabled
            });
        if !same_layout || bounds != self.bounds || ppp != self.ppp {
            self.generation += 1;
            self.release_pending = true;
            self.bounds = bounds;
            self.ppp = ppp;
            if let Ok(mut shared) = self.shared.lock() {
                shared.generation = self.generation;
            }
        }
        self.targets = targets;
        if self.pending.is_none() && self.last_capture.elapsed() >= Duration::from_millis(100) {
            self.sequence += 1;
            self.pending = Some(Pending {
                sequence: self.sequence,
                generation: self.generation,
                started: Instant::now(),
                ppp,
                size: bounds.size(),
                targets: self.targets.clone(),
            });
            if let Some(viewport) = output.viewport_output.get_mut(&ViewportId::ROOT) {
                viewport
                    .commands
                    .push(egui::ViewportCommand::Screenshot(egui::UserData::new(
                        CaptureId(self.sequence),
                    )));
            }
        }
        ctx.request_repaint_after(Duration::from_millis(50));
    }
}

#[cfg(test)]
#[path = "bridge_tests.rs"]
mod tests;
