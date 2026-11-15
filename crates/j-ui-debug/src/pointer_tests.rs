use super::*;
use crate::input::RemoteEvent;

struct Scene {
    ctx: Context,
    _directory: tempfile::TempDir,
    send: mpsc::Sender<Batch>,
    buttons: [egui::Rect; 2],
    clicks: Vec<usize>,
    states: [bool; 2],
    drag: egui::Rect,
    drag_moves: Vec<egui::Pos2>,
    drag_started: usize,
    drag_stopped: usize,
    volume: i32,
    slider_id: egui::Id,
    text: String,
    editor_id: egui::Id,
    discard: bool,
}

impl Scene {
    fn new() -> Self {
        let ctx = Context::default();
        let directory = tempfile::tempdir().unwrap();
        install(
            &ctx,
            "pointer-test",
            &directory.path().join("connection.json"),
        )
        .unwrap();
        let (send, receive) = mpsc::channel();
        ctx.with_plugin::<Bridge, _>(|bridge| bridge.incoming = Mutex::new(receive));
        let mut scene = Self {
            ctx,
            _directory: directory,
            send,
            buttons: [egui::Rect::NOTHING; 2],
            clicks: Vec::new(),
            states: [false; 2],
            drag: egui::Rect::NOTHING,
            drag_moves: Vec::new(),
            drag_started: 0,
            drag_stopped: 0,
            volume: 50,
            slider_id: egui::Id::NULL,
            text: String::new(),
            editor_id: egui::Id::NULL,
            discard: false,
        };
        scene.frame();
        scene.frame();
        scene
    }

    fn frame(&mut self) {
        self.frame_with(Vec::new(), egui::vec2(1280.0, 800.0));
    }

    fn frame_with(&mut self, events: Vec<Event>, size: egui::Vec2) {
        let input = RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
            events,
            ..Default::default()
        };
        let mut output = self.ctx.run_ui(input, |ui| {
            for (index, label) in ["Mute", "Phones"].into_iter().enumerate() {
                let response = ui.add_sized([120.0, 24.0], egui::Button::new(label));
                self.buttons[index] = response.rect;
                if response.clicked() {
                    self.clicks.push(index);
                    self.states[index] = !self.states[index];
                }
            }
            let (rect, response) =
                ui.allocate_exact_size(egui::vec2(200.0, 40.0), egui::Sense::drag());
            self.drag = rect;
            self.drag_started += usize::from(response.drag_started());
            self.drag_stopped += usize::from(response.drag_stopped());
            if response.dragged() {
                self.drag_moves
                    .push(response.interact_pointer_pos().unwrap());
            }
            let slider = ui.allocate_response(egui::vec2(24.0, 100.0), egui::Sense::click());
            self.slider_id = slider.id;
            if slider.has_focus() {
                ui.memory_mut(|memory| {
                    memory.set_focus_lock_filter(
                        slider.id,
                        egui::EventFilter {
                            vertical_arrows: true,
                            ..Default::default()
                        },
                    );
                });
                if ui.input(|input| input.key_pressed(egui::Key::ArrowUp)) {
                    self.volume += 1;
                }
            }
            self.editor_id = ui.text_edit_singleline(&mut self.text).id;
            if self.discard && ui.ctx().current_pass_index() == 0 {
                ui.ctx().request_discard("measure controls");
            }
        });
        if self
            .ctx
            .with_plugin::<Bridge, _>(|bridge| bridge.pending_input.is_some())
            == Some(true)
        {
            assert_eq!(
                output.viewport_output[&ViewportId::ROOT].repaint_delay,
                Duration::ZERO
            );
        }
        output.textures_delta.clear();
    }

    fn enqueue(&self, events: Vec<RemoteEvent>) {
        self.enqueue_at(events, Instant::now());
    }

    fn enqueue_at(&self, events: Vec<RemoteEvent>, accepted: Instant) {
        self.ctx.with_plugin::<Bridge, _>(|bridge| {
            self.send
                .send(Batch {
                    generation: bridge.generation,
                    accepted,
                    width: 1280.0,
                    height: 800.0,
                    events,
                })
                .unwrap();
            bridge.shared.lock().unwrap().last_input = Instant::now();
        });
    }

    fn button(&self, index: usize, pressed: bool) -> RemoteEvent {
        Self::pointer(self.buttons[index].center(), pressed)
    }

    fn pointer(point: egui::Pos2, pressed: bool) -> RemoteEvent {
        RemoteEvent::Button {
            x: point.x / 1280.0,
            y: point.y / 800.0,
            button: 0,
            pressed,
            modifiers: Default::default(),
        }
    }
}

#[test]
fn adjacent_clicks_keep_their_native_widgets_across_queued_batches() {
    let mut scene = Scene::new();
    scene.enqueue(vec![scene.button(0, true)]);
    scene.frame();
    assert!(scene.clicks.is_empty());
    let accepted = Instant::now() - Duration::from_millis(150);
    scene.enqueue_at(
        vec![RemoteEvent::KeepAlive {}, scene.button(0, false)],
        accepted,
    );
    scene.enqueue_at(vec![scene.button(1, true)], accepted);
    scene.enqueue_at(vec![scene.button(1, false)], accepted);
    for _ in 0..3 {
        scene.frame();
    }
    assert_eq!(scene.clicks, [0, 1]);
    assert_eq!(scene.states, [true, true]);
}

#[test]
fn rapid_clicks_in_one_batch_each_change_native_state() {
    let mut scene = Scene::new();
    let events = [0, 0, 0, 1, 1, 1]
        .into_iter()
        .flat_map(|index| [scene.button(index, true), scene.button(index, false)])
        .collect();
    scene.enqueue(events);
    for _ in 0..12 {
        scene.frame();
    }
    assert_eq!(scene.clicks, [0, 0, 0, 1, 1, 1]);
    assert_eq!(scene.states, [true, true]);
}

#[test]
fn extra_layout_passes_do_not_advance_the_next_pointer_transition() {
    let mut scene = Scene::new();
    scene.discard = true;
    scene.enqueue(vec![
        scene.button(0, true),
        scene.button(0, false),
        scene.button(1, true),
        scene.button(1, false),
    ]);
    scene.frame();
    assert!(scene.clicks.is_empty());
    assert!(scene.ctx.input(|input| input.pointer.any_down()));
    for _ in 0..3 {
        scene.frame();
    }
    assert_eq!(scene.clicks, [0, 1]);
    assert_eq!(scene.states, [true, true]);
}

#[test]
fn queued_drag_moves_are_observed_before_release() {
    let mut scene = Scene::new();
    let start = scene.drag.left_center() + egui::vec2(10.0, 0.0);
    let end = start + egui::vec2(100.0, 0.0);
    scene.enqueue(vec![
        Scene::pointer(start, true),
        RemoteEvent::Move {
            x: end.x / 1280.0,
            y: end.y / 800.0,
        },
        Scene::pointer(end, false),
    ]);
    for _ in 0..3 {
        scene.frame();
    }
    assert_eq!(scene.drag_started, 1);
    assert!(
        scene
            .drag_moves
            .iter()
            .any(|point| point.distance(end) < 1.0)
    );
    assert_eq!(scene.drag_stopped, 1);
    assert!(!scene.ctx.input(|input| input.pointer.any_down()));
}

#[test]
fn repeated_arrow_presses_and_batched_unicode_text_reach_native_widgets() {
    let mut scene = Scene::new();
    scene
        .ctx
        .memory_mut(|memory| memory.request_focus(scene.slider_id));
    scene.frame();
    scene.frame();
    let events = (0..7)
        .flat_map(|_| {
            [true, false].map(|pressed| RemoteEvent::Key {
                key: "ArrowUp".into(),
                pressed,
                repeat: false,
                modifiers: Default::default(),
            })
        })
        .collect();
    scene.enqueue(events);
    for _ in 0..7 {
        scene.frame();
    }
    assert_eq!(scene.volume, 57);
    scene
        .ctx
        .memory_mut(|memory| memory.request_focus(scene.editor_id));
    scene.enqueue(vec![
        RemoteEvent::Text { text: "a".into() },
        RemoteEvent::Text {
            text: "🙂".into()
        },
        RemoteEvent::Text {
            text: "日本語".into(),
        },
        RemoteEvent::Text {
            text: " pasted".into(),
        },
    ]);
    scene.frame();
    assert_eq!(scene.text, "a🙂日本語 pasted");
}

#[test]
fn native_focus_loss_cancels_deferred_gestures_and_releases_controls() {
    let mut scene = Scene::new();
    scene.enqueue(vec![
        scene.button(0, true),
        scene.button(0, false),
        scene.button(1, true),
        scene.button(1, false),
    ]);
    scene.frame();
    scene.frame_with(vec![Event::WindowFocused(false)], egui::vec2(1280.0, 800.0));
    scene.frame();
    assert!(scene.clicks.is_empty());
    assert!(!scene.ctx.input(|input| input.pointer.any_down()));
}

#[test]
fn reset_between_pointer_gestures_does_not_activate_the_cancelled_button() {
    let mut scene = Scene::new();
    scene.enqueue(vec![
        scene.button(0, true),
        RemoteEvent::Reset {},
        scene.button(1, true),
        scene.button(1, false),
    ]);
    for _ in 0..4 {
        scene.frame();
    }
    assert_eq!(scene.clicks, [1]);
    assert!(!scene.ctx.input(|input| input.pointer.any_down()));
}

#[test]
fn deferred_gestures_keep_age_generation_and_resize_fences() {
    for fence in ["age", "generation", "resize"] {
        let mut scene = Scene::new();
        scene.enqueue(vec![
            scene.button(0, true),
            scene.button(0, false),
            scene.button(1, true),
            scene.button(1, false),
        ]);
        scene.frame();
        scene.ctx.with_plugin::<Bridge, _>(|bridge| match fence {
            "age" => {
                bridge.pending_input.as_mut().unwrap().accepted =
                    Instant::now() - Duration::from_millis(301);
            }
            "generation" => bridge.generation += 1,
            _ => {}
        });
        if fence == "resize" {
            scene.frame_with(Vec::new(), egui::vec2(1000.0, 800.0));
        } else {
            scene.frame();
        }
        scene.frame();
        assert!(
            scene.clicks.is_empty(),
            "cancelled by {fence}: {:?}",
            scene.clicks
        );
        assert!(!scene.ctx.input(|input| input.pointer.any_down()));
        scene.enqueue(vec![scene.button(1, true), scene.button(1, false)]);
        scene.frame();
        scene.frame();
        assert_eq!(scene.clicks, [1], "fresh input after {fence}");
    }
}
