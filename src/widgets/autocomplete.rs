//! A search field that suggests matches from a list as the user types.

use super::*;

/// A search field over `options`: up to five that contain the typed text open in a menu
/// below it, the match drawn in Light Blue. Up and Down move through them, Enter or a click
/// takes one, Escape restores the previous choice and closes the menu.
#[must_use = "add it with `ui.add(..)`"]
pub struct Autocomplete<'a> {
    text: &'a mut String,
    options: &'a [&'a str],
    hint: Option<&'a str>,
    empty: &'a str,
    width: f32,
    height: f32,
    initially_open: bool,
}

#[derive(Clone)]
struct AutocompleteState {
    choice: String,
    text: String,
    active: usize,
    used: bool,
}

impl<'a> Autocomplete<'a> {
    pub fn new(text: &'a mut String, options: &'a [&'a str]) -> Self {
        Self {
            text,
            options,
            hint: None,
            empty: "",
            width: 240.0,
            height: size::control(),
            initially_open: false,
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

    /// Shows suggestions before first focus, for a populated search preview.
    pub fn initially_open(mut self, open: bool) -> Self {
        self.initially_open = open;
        self
    }

    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Minimum field height in logical points, matching `TextInput::height`.
    pub fn height(mut self, height: f32) -> Self {
        self.height = height.max(size::control());
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

impl Autocomplete<'_> {
    /// Returns the original option index only when picked; typing changes the search text.
    pub fn show(self, ui: &mut Ui) -> egui::InnerResponse<Option<usize>> {
        const SHOWN: usize = 5;
        let id = ui.next_auto_id().with("autocomplete");
        let popup = id.with("suggestions");
        let mut state = ui
            .data(|data| data.get_temp::<AutocompleteState>(id))
            .unwrap_or_else(|| AutocompleteState {
                choice: self.text.clone(),
                text: self.text.clone(),
                active: 0,
                used: false,
            });
        if state.text != *self.text {
            state.choice.clone_from(self.text);
            state.active = 0;
        }
        let enabled = ui.is_enabled();
        let was_open = egui::Popup::is_id_open(ui.ctx(), popup);
        if !enabled {
            egui::Popup::close_id(ui.ctx(), popup);
        }
        let suggest = |text: &str| {
            let query = text.trim().to_lowercase();
            self.options
                .iter()
                .enumerate()
                .filter_map(|(index, option)| Some((index, *option, find(option, &query)?)))
                .take(SHOWN)
                .collect::<Vec<_>>()
        };
        let mut matches = suggest(self.text);
        let last = matches.len().saturating_sub(1);
        state.active = state.active.min(last);
        let mut picked = None;
        let mut dismissed = false;
        // Taken before the editor sees them: it would move the caret and give up focus.
        if enabled && (ui.memory(|m| m.has_focus(id)) || was_open) {
            let pressed = |key| ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, key));
            if pressed(Key::ArrowDown) {
                egui::Popup::open_id(ui.ctx(), popup);
                state.active = if !was_open || state.active == last {
                    0
                } else {
                    state.active + 1
                };
            }
            if pressed(Key::ArrowUp) {
                egui::Popup::open_id(ui.ctx(), popup);
                state.active = state.active.checked_sub(1).unwrap_or(last);
            }
            if pressed(Key::Escape) {
                egui::Popup::close_id(ui.ctx(), popup);
                ui.memory_mut(|m| m.request_focus(id));
                dismissed = true;
                state.used = true;
            }
            if egui::Popup::is_id_open(ui.ctx(), popup) && pressed(Key::Enter) {
                picked = matches
                    .get(state.active)
                    .map(|(index, option, _)| (*index, *option));
            }
        }
        if dismissed || !enabled {
            self.text.clone_from(&state.choice);
            matches = suggest(self.text);
        }
        let mut input = TextInput::search(self.text)
            .id(id)
            .width(self.width)
            .height(self.height);
        if let Some(hint) = self.hint {
            input = input.hint(hint);
        }
        let mut response = ui.add(input);
        if response.changed() {
            state.active = 0;
            matches = suggest(self.text);
        }
        if !dismissed && (response.clicked() || response.gained_focus() || response.changed()) {
            egui::Popup::open_id(ui.ctx(), popup);
        }
        if response.gained_focus() || response.changed() {
            state.used = true;
        }
        let preview = enabled && self.initially_open && !state.used;
        let visible = response.rect.intersects(ui.clip_rect());
        let wanted = matches.len().max(1) as f32 * size::control();
        let (align, max_height) = menu_place(response.rect, ui.ctx().content_rect(), wanted);
        let menu = egui::Popup::from_response(&response).id(popup);
        let menu = if preview || !visible {
            menu.open(preview && visible)
        } else {
            menu.open_memory(None)
        };
        let mut listbox = None;
        let mut active_option = None;
        menu
            // The click that focuses the field would count as one outside the menu.
            .close_behavior(egui::PopupCloseBehavior::IgnoreClicks)
            .width(response.rect.width())
            .gap(space::S4)
            .align(align)
            .align_alternatives(&[])
            .show(|ui| {
                listbox = Some(egui::accesskit::NodeId(ui.id().value()));
                ui.ctx().accesskit_node_builder(ui.id(), |node| node.set_role(egui::accesskit::Role::ListBox));
                ui.spacing_mut().item_spacing.y = 0.0;
                egui::ScrollArea::vertical()
                    .max_height(max_height.max(size::control()))
                    .show(ui, |ui| {
                        for (i, (index, option, hit)) in matches.iter().enumerate() {
                            let selected = *option == state.choice;
                            let row = MenuRow::suggestion(option, hit.clone(), selected).lit(i == state.active);
                            let row = ui.add(row);
                            ui.ctx().accesskit_node_builder(row.id, |node| {
                                node.set_role(egui::accesskit::Role::ListBoxOption);
                                node.set_selected(selected);
                            });
                            if i == state.active {
                                active_option = Some(egui::accesskit::NodeId(row.id.value()));
                            }
                            if row.clicked() {
                                picked = Some((*index, *option));
                            }
                        }
                        if matches.is_empty() && !self.empty.is_empty() {
                            let size = vec2(ui.available_width(), size::control());
                            let (row, _) = ui.allocate_exact_size(size, Sense::hover());
                            let empty = fit_text(ui, self.empty, theme::body(), color::ink_3(), row.width() - 2.0 * space::field_pad());
                            let x = row.left() + space::field_pad();
                            let rect = Rect::from_min_size(pos2(x, row.center().y - empty.size().y / 2.0), empty.size());
                            place(ui, rect, egui::Label::new(empty).selectable(false));
                        }
                    });
            });
        if let Some((_, option)) = picked {
            state.used = true;
            state.choice = option.to_owned();
            *self.text = option.to_owned();
            egui::Popup::close_id(ui.ctx(), popup);
            response.mark_changed();
            response.request_focus();
        } else if !response.has_focus() && !ui.input(|i| i.pointer.any_down()) {
            // Focus left by Tab or a click elsewhere. A press on a row also takes focus, so
            // wait for its release, which picks the row above.
            egui::Popup::close_id(ui.ctx(), popup);
            self.text.clone_from(&state.choice);
        }
        let expanded = visible && (preview || egui::Popup::is_id_open(ui.ctx(), popup));
        ui.ctx().accesskit_node_builder(response.id, |node| {
            node.set_role(egui::accesskit::Role::EditableComboBox);
            node.set_expanded(expanded);
            node.set_has_popup(egui::accesskit::HasPopup::Listbox);
            if let Some(listbox) = listbox.filter(|_| expanded) {
                node.set_controls(vec![listbox]);
            }
            if let Some(option) = active_option.filter(|_| expanded) {
                node.set_active_descendant(option);
            }
        });
        state.text.clone_from(self.text);
        ui.data_mut(|data| data.insert_temp(id, state));
        egui::InnerResponse {
            inner: picked.map(|(index, _)| index),
            response,
        }
    }
}

impl Widget for Autocomplete<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        self.show(ui).response
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
