//! Shared update presentation. Actions are requests: apps persist preferences, open release
//! links, save/checkpoint, and launch helpers at their own shutdown boundary.
use crate::{
    dialog,
    i18n::CATALOG,
    tokens::space,
    tr,
    widgets::{Button, Checkbox},
};
use egui::{Context, Id, Ui};
use j_update::{
    network::Available,
    state::{State, Work},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Check,
    Download,
    Cancel,
    Install,
    OpenRelease,
}

pub struct Prompt<'a> {
    pub state: &'a State,
    pub current_version: &'a str,
    pub installed: bool,
    pub work: Work,
    /// The app can save unsaved changes before honoring Install. Core preparation still
    /// requires a fresh Work with unsaved=false after that save completes successfully.
    pub save_before_restart: bool,
    pub previous: Option<&'a str>,
}

impl Prompt<'_> {
    /// Closing during a download requests cancellation. Closing Ready preserves the files.
    pub fn show(&self, ctx: &Context, open: &mut bool, automatic: &mut bool) -> Option<Action> {
        if !*open {
            return None;
        }
        let id = Id::new("j_ui_update");
        let previous_rect = ctx.memory(|m| m.area_rect(id));
        let response = dialog::modal(ctx, id, tr!(CATALOG, "updates.title"), 500.0, |ui| {
            self.contents(ui, automatic)
        });
        if previous_rect.is_some_and(|r| r.size() != response.response.rect.size()) {
            ctx.request_repaint();
        }
        let (mut action, close) = response.inner;
        if close || response.should_close() {
            *open = false;
            if matches!(self.state, State::Downloading { .. }) {
                action = Some(Action::Cancel);
            }
        }
        action
    }

    /// Inline form, also used by the gallery. Returns (action, dismiss).
    pub fn contents(&self, ui: &mut Ui, automatic: &mut bool) -> (Option<Action>, bool) {
        if self.state.busy() {
            ui.ctx().request_repaint_after_secs(0.1);
        }
        ui.label(tr!(
            CATALOG,
            "updates.installed",
            version = self.current_version
        ));
        ui.add(Checkbox::new(automatic, tr!(CATALOG, "updates.automatic")));
        if !self.installed {
            ui.label(tr!(CATALOG, "updates.external"));
        }
        ui.separator();
        match self.state {
            State::Idle => {
                ui.label(tr!(CATALOG, "updates.idle"));
            }
            State::Checking => {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(tr!(CATALOG, "updates.checking"));
                });
            }
            State::Current => {
                ui.label(tr!(CATALOG, "updates.current"));
            }
            State::Failed(e) => {
                ui.label(tr!(CATALOG, "updates.failed", error = e));
            }
            State::Available(u) | State::Downloading { update: u, .. } => {
                release(ui, u);
                if let State::Downloading { received, .. } = self.state {
                    ui.label(if *received >= u.asset.size {
                        tr!(CATALOG, "updates.verifying")
                    } else {
                        tr!(CATALOG, "updates.downloading")
                    });
                    let progress = (*received as f32 / u.asset.size.max(1) as f32).clamp(0.0, 1.0);
                    ui.add(egui::ProgressBar::new(progress).show_percentage());
                    ui.label(tr!(
                        CATALOG,
                        "updates.progress",
                        received = received,
                        total = u.asset.size
                    ));
                }
            }
            State::Ready(d) => {
                release(ui, &d.update);
                ui.label(tr!(CATALOG, "updates.verified"));
            }
        }
        let mut allowed = self.work;
        if self.save_before_restart {
            allowed.unsaved = false;
        }
        if !allowed.can_install() {
            ui.label(tr!(CATALOG, "updates.busy"));
        }
        if let Some(previous) = self.previous {
            ui.collapsing(tr!(CATALOG, "updates.previous"), |ui| {
                ui.label(previous);
            });
        }
        ui.add_space(space::S16);
        let disabled = crate::widgets::State::Disabled;
        let live = crate::widgets::State::Live;
        let mut buttons = vec![
            Button::secondary(tr!(CATALOG, "updates.later")),
            Button::secondary(tr!(CATALOG, "updates.check")).state(
                if self.state.busy() || matches!(self.state, State::Ready(_)) {
                    disabled
                } else {
                    live
                },
            ),
        ];
        let primary = match self.state {
            State::Available(_) if self.installed => Some((
                Button::primary(tr!(CATALOG, "updates.download")),
                Action::Download,
            )),
            State::Available(_) => Some((
                Button::primary(tr!(CATALOG, "updates.release_page")),
                Action::OpenRelease,
            )),
            State::Downloading { .. } => Some((
                Button::secondary(tr!(CATALOG, "updates.cancel")),
                Action::Cancel,
            )),
            State::Ready(_) => Some((
                Button::primary(if self.work.unsaved && self.save_before_restart {
                    tr!(CATALOG, "updates.save_restart")
                } else {
                    tr!(CATALOG, "updates.restart")
                })
                .state(if self.installed && allowed.can_install() {
                    live
                } else {
                    disabled
                }),
                Action::Install,
            )),
            _ => None,
        };
        let primary_action = primary.as_ref().map(|(_, a)| *a);
        if let Some((button, _)) = primary {
            buttons.push(button);
        }
        let responses = dialog::action_footer(ui, buttons);
        let action = if responses[1].clicked() {
            Some(Action::Check)
        } else if responses.get(2).is_some_and(|r| r.clicked()) {
            primary_action
        } else {
            None
        };
        (action, responses[0].clicked())
    }
}
fn release(ui: &mut Ui, update: &Available) {
    ui.strong(tr!(CATALOG, "updates.available", version = &update.version));
    egui::ScrollArea::vertical()
        .max_height(180.0)
        .show(ui, |ui| {
            ui.label(&update.notes);
        });
}
