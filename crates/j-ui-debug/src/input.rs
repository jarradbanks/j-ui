use serde::Deserialize;

#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct Modifiers {
    alt: bool,
    ctrl: bool,
    shift: bool,
    meta: bool,
}

impl Modifiers {
    pub(crate) fn egui(&self) -> egui::Modifiers {
        egui::Modifiers {
            alt: self.alt,
            ctrl: self.ctrl,
            shift: self.shift,
            mac_cmd: self.meta,
            command: self.ctrl || self.meta,
        }
    }
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum RemoteEvent {
    Move {
        x: f32,
        y: f32,
    },
    Button {
        x: f32,
        y: f32,
        button: u8,
        pressed: bool,
        #[serde(default)]
        modifiers: Modifiers,
    },
    Key {
        key: String,
        pressed: bool,
        #[serde(default)]
        repeat: bool,
        #[serde(default)]
        modifiers: Modifiers,
    },
    Text {
        text: String,
    },
    Wheel {
        dx: f32,
        dy: f32,
    },
    Modifiers {
        modifiers: Modifiers,
    },
    Reset {},
    KeepAlive {},
}

impl RemoteEvent {
    fn valid(&self) -> bool {
        match self {
            Self::Move { x, y }
            | Self::Button {
                x,
                y,
                button: 0..=2,
                ..
            } => (0.0..=1.0).contains(x) && (0.0..=1.0).contains(y),
            Self::Button { .. } => false,
            Self::Key { key, .. } => egui::Key::from_name(key).is_some(),
            Self::Text { text } => text.len() <= 8192 && !text.contains('\0'),
            Self::Wheel { dx, dy } => {
                dx.is_finite() && dy.is_finite() && dx.abs() <= 2000.0 && dy.abs() <= 2000.0
            }
            Self::Reset { .. } | Self::KeepAlive { .. } | Self::Modifiers { .. } => true,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Request {
    pub session: String,
    pub generation: u64,
    pub frame: u64,
    pub events: Vec<RemoteEvent>,
}

impl Request {
    pub(crate) fn valid(&self) -> bool {
        self.frame > 0
            && self.generation > 0
            && !self.events.is_empty()
            && self.events.len() <= 64
            && self.events.iter().all(RemoteEvent::valid)
    }
}

pub(crate) struct Batch {
    pub generation: u64,
    pub accepted: std::time::Instant,
    pub width: f32,
    pub height: f32,
    pub events: Vec<RemoteEvent>,
}

#[derive(Default)]
pub(crate) struct HeldInput {
    position: egui::Pos2,
    buttons: Vec<egui::PointerButton>,
    keys: std::collections::HashSet<egui::Key>,
    pub focused: bool,
}

impl HeldInput {
    pub fn event(&mut self, event: RemoteEvent, bounds: egui::Rect, input: &mut egui::RawInput) {
        use egui::{Event, Key, PointerButton};
        if !matches!(&event, RemoteEvent::Reset { .. }) {
            self.focused = true;
        }
        match event {
            RemoteEvent::Move { x, y } => {
                self.position = bounds.min + egui::vec2(x, y) * bounds.size();
                input.events.push(Event::PointerMoved(self.position));
            }
            RemoteEvent::Button {
                x,
                y,
                button,
                pressed,
                modifiers,
            } => {
                self.position = bounds.min + egui::vec2(x, y) * bounds.size();
                let button = match button {
                    1 => PointerButton::Middle,
                    2 => PointerButton::Secondary,
                    _ => PointerButton::Primary,
                };
                self.buttons.retain(|held| *held != button);
                if pressed {
                    self.buttons.push(button);
                }
                input.events.push(Event::PointerMoved(self.position));
                input.events.push(Event::PointerButton {
                    pos: self.position,
                    button,
                    pressed,
                    modifiers: modifiers.egui(),
                });
            }
            RemoteEvent::Key {
                key,
                pressed,
                repeat,
                modifiers,
            } => {
                let Some(key) = Key::from_name(&key) else {
                    return;
                };
                if pressed {
                    self.keys.insert(key);
                } else {
                    self.keys.remove(&key);
                }
                input.events.push(Event::ModifiersChanged(modifiers.egui()));
                input.events.push(Event::Key {
                    key,
                    physical_key: None,
                    pressed,
                    repeat,
                    modifiers: modifiers.egui(),
                });
            }
            RemoteEvent::Text { text } => input.events.push(Event::Text(text)),
            RemoteEvent::Modifiers { modifiers } => {
                input.events.push(Event::ModifiersChanged(modifiers.egui()))
            }
            RemoteEvent::Wheel { dx, dy } => input.events.push(Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: egui::vec2(-dx, -dy),
                modifiers: Default::default(),
                phase: egui::TouchPhase::Move,
            }),
            RemoteEvent::Reset { .. } => self.release(input),
            RemoteEvent::KeepAlive { .. } => {}
        }
    }

    pub fn release(&mut self, input: &mut egui::RawInput) {
        use egui::Event;
        for button in self.buttons.drain(..) {
            input.events.push(Event::PointerButton {
                pos: self.position,
                button,
                pressed: false,
                modifiers: Default::default(),
            });
        }
        for key in self.keys.drain() {
            input.events.push(Event::Key {
                key,
                physical_key: None,
                pressed: false,
                repeat: false,
                modifiers: Default::default(),
            });
        }
        input.events.push(Event::PointerGone);
        input
            .events
            .push(Event::ModifiersChanged(Default::default()));
        self.focused = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_input_and_releases_every_held_control() {
        let request: Request = serde_json::from_value(serde_json::json!({
            "session":"session", "generation":1, "frame":1,
            "events":[{"kind":"button", "x":0.25, "y":0.5, "button":0, "pressed":true},
                {"kind":"key", "key":"A", "pressed":true, "modifiers":{"ctrl":true}}]
        }))
        .unwrap();
        assert!(request.valid());
        let mut held = HeldInput::default();
        let mut input = egui::RawInput::default();
        let bounds = egui::Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(100.0, 80.0));
        for event in request.events {
            held.event(event, bounds, &mut input);
        }
        assert_eq!(held.position, egui::pos2(35.0, 60.0));
        input.events.clear();
        held.release(&mut input);
        assert!(held.buttons.is_empty() && held.keys.is_empty() && !held.focused);
        assert!(
            input
                .events
                .iter()
                .any(|event| matches!(event, egui::Event::PointerButton { pressed: false, .. }))
        );
        assert!(
            input
                .events
                .iter()
                .any(|event| matches!(event, egui::Event::Key { pressed: false, .. }))
        );
        assert!(input.events.iter().any(
            |event| matches!(event, egui::Event::ModifiersChanged(modifiers) if modifiers.is_none())
        ));
        let invalid: RemoteEvent =
            serde_json::from_str(r#"{"kind":"move","x":-0.1,"y":0.5}"#).unwrap();
        assert!(!invalid.valid());
        held.event(RemoteEvent::Reset {}, bounds, &mut input);
        held.event(RemoteEvent::KeepAlive {}, bounds, &mut input);
        assert!(
            held.focused,
            "events following reset must still be covered by disconnect cleanup"
        );
        assert!(serde_json::from_str::<RemoteEvent>(r#"{"kind":"reset","extra":true}"#).is_err());
    }
}
