//! A search field that suggests matches from a list as the user types.

use super::*;

/// A search field over `options`: up to five that contain the typed text open in a menu
/// below it, the match drawn in Light Blue. Up and Down move through them, Enter or a click
/// takes one, Escape closes the menu.
#[must_use = "add it with `ui.add(..)`"]
pub struct Autocomplete<'a> {
    text: &'a mut String,
    options: &'a [&'a str],
    hint: Option<&'a str>,
    empty: &'a str,
    width: f32,
}

impl<'a> Autocomplete<'a> {
    pub fn new(text: &'a mut String, options: &'a [&'a str]) -> Self {
        Self {
            text,
            options,
            hint: None,
            empty: "",
            width: 240.0,
        }
    }

    pub fn hint(mut self, hint: &'a str) -> Self {
        self.hint = Some(hint);
        self
    }

    /// What the menu says when nothing matches, such as "No matching fonts".
    pub fn empty(mut self, empty: &'a str) -> Self {
        self.empty = empty;
        self
    }

    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }
}

/// Where `option` contains `query`, ignoring case, as a byte range of `option`.
fn find(option: &str, query: &str) -> Option<std::ops::Range<usize>> {
    let lower = option.to_lowercase();
    let at = lower.find(query)?;
    let hit = at..at + query.len();
    // Lowercasing can change a character's length; then the row shows no match.
    let aligned = lower.len() == option.len()
        && option.is_char_boundary(hit.start)
        && option.is_char_boundary(hit.end);
    Some(if aligned { hit } else { 0..0 })
}

impl Widget for Autocomplete<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        const SHOWN: usize = 5;
        let id = ui.next_auto_id().with("autocomplete");
        let popup = id.with("suggestions");
        let query = self.text.trim().to_lowercase();
        let matches: Vec<(&str, std::ops::Range<usize>)> = self
            .options
            .iter()
            .filter_map(|option| Some((*option, find(option, &query)?)))
            .take(SHOWN)
            .collect();
        let last = matches.len().saturating_sub(1);
        let mut active = ui
            .data(|d| d.get_temp::<usize>(popup))
            .unwrap_or(0)
            .min(last);
        let mut picked = None;
        // Taken before the editor sees them: it would move the caret and give up focus.
        if egui::Popup::is_id_open(ui.ctx(), popup) && ui.memory(|m| m.has_focus(id)) {
            let pressed = |key| ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, key));
            if pressed(Key::ArrowDown) {
                active = if active == last { 0 } else { active + 1 };
            }
            if pressed(Key::ArrowUp) {
                active = active.checked_sub(1).unwrap_or(last);
            }
            if pressed(Key::Escape) {
                egui::Popup::close_id(ui.ctx(), popup);
            }
            if !matches.is_empty() && pressed(Key::Enter) {
                picked = Some(matches[active].0);
            }
        }
        let mut input = TextInput::search(self.text).id(id).width(self.width);
        if let Some(hint) = self.hint {
            input = input.hint(hint);
        }
        let mut response = ui.add(input);
        if response.changed() {
            active = 0;
        }
        if response.gained_focus() || response.changed() {
            egui::Popup::open_id(ui.ctx(), popup);
        }
        egui::Popup::from_response(&response)
            .id(popup)
            .open_memory(None)
            // The click that focuses the field would count as one outside the menu.
            .close_behavior(egui::PopupCloseBehavior::IgnoreClicks)
            .width(response.rect.width())
            .gap(space::S4)
            .show(|ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                for (i, (option, hit)) in matches.iter().enumerate() {
                    let row = MenuRow::suggestion(option, hit.clone()).lit(i == active);
                    if ui.add(row).clicked() {
                        picked = Some(*option);
                    }
                }
                if matches.is_empty() && !self.empty.is_empty() {
                    let size = vec2(ui.available_width(), size::CONTROL);
                    let (row, _) = ui.allocate_exact_size(size, Sense::hover());
                    let empty = fit_text(
                        ui,
                        self.empty,
                        theme::body(),
                        color::ink_3(),
                        row.width() - 2.0 * space::FIELD_PAD,
                    );
                    let x = row.left() + space::FIELD_PAD;
                    paint_left(ui, x, row.center().y, empty, color::ink_3());
                }
            });
        ui.data_mut(|d| d.insert_temp(popup, active));
        if let Some(option) = picked {
            *self.text = option.to_owned();
            egui::Popup::close_id(ui.ctx(), popup);
            response.mark_changed();
            response.request_focus();
        } else if !response.has_focus() && !ui.input(|i| i.pointer.any_down()) {
            // Focus left by Tab or a click elsewhere. A press on a row also takes focus, so
            // wait for its release, which picks the row above.
            egui::Popup::close_id(ui.ctx(), popup);
        }
        response
    }
}

#[cfg(test)]
mod tests {
    use super::find;

    #[test]
    fn matches_ignore_case_and_keep_byte_ranges_valid() {
        assert_eq!(find("Inter Display", "in"), Some(0..2));
        assert_eq!(
            find("JetBrains Mono", "IN".to_lowercase().as_str()),
            Some(6..8)
        );
        assert_eq!(find("Onest", "in"), None);
        assert_eq!(find("Onest", ""), Some(0..0));
        // U+0130 lowercases to three bytes, so its offsets no longer fit the original.
        assert_eq!(find("\u{130}nter", "nt"), Some(0..0));
    }
}
