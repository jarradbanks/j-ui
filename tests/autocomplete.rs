use egui::{Key, Rect, accesskit::Role};
use egui_kittest::{
    Harness,
    kittest::{NodeT, Queryable},
};
use j_ui::{
    test_support,
    widgets::{Autocomplete, Button},
};

const OPTIONS: &[&str] = &[
    "Inter",
    "Inter Display",
    "Onest",
    "Geist",
    "IBM Plex Sans",
    "JetBrains Mono",
];

struct Picker {
    text: String,
    picks: Vec<usize>,
    enabled: bool,
    rect: Rect,
}

fn harness(
    size: [f32; 2],
    scale: f32,
    options: &'static [&'static str],
) -> Harness<'static, Picker> {
    let mut fresh = true;
    let mut harness = Harness::builder()
        .with_size(size)
        .with_pixels_per_point(scale)
        .build_ui_state(
            move |ui, picker: &mut Picker| {
                if test_support::install_theme(ui, &mut fresh) {
                    return;
                }
                let response = ui
                    .add_enabled_ui(picker.enabled, |ui| {
                        Autocomplete::new(&mut picker.text, options)
                            .hint("Search fonts")
                            .empty("No matching fonts")
                            .width(240.0)
                            .height(48.0)
                            .show(ui)
                    })
                    .inner;
                picker.rect = response.response.rect;
                if let Some(index) = response.inner {
                    picker.picks.push(index);
                }
                // Keep the outside-click target below the filtered suggestion overlay.
                ui.add_space(100.0);
                ui.add(Button::secondary("Next"));
            },
            Picker {
                text: "Inter".into(),
                picks: Vec::new(),
                enabled: true,
                rect: Rect::NOTHING,
            },
        );
    harness.run();
    harness
}

fn search(harness: &mut Harness<'_, Picker>, text: &str) {
    harness.key_press_modifiers(egui::Modifiers::COMMAND, Key::A);
    harness.event(egui::Event::Text(text.into()));
    harness.run();
}

#[test]
fn typing_is_separate_from_picking_and_escape_restores_the_choice() {
    let mut h = harness([320.0, 320.0], 1.0, OPTIONS);
    h.get_by_role(Role::EditableComboBox).click();
    h.run();
    assert!(
        h.get_by_role_and_label(Role::ListBoxOption, "Inter")
            .accesskit_node()
            .data()
            .is_selected()
            .unwrap()
    );
    search(&mut h, "mono");
    assert!(h.state().picks.is_empty());
    h.key_press(Key::Enter);
    h.run();
    assert_eq!(h.state().text, "JetBrains Mono");
    assert_eq!(h.state().picks, [5], "the unfiltered option index");
    search(&mut h, "zzz");
    assert!(h.query_by_label("No matching fonts").is_some());
    h.key_press(Key::Escape);
    h.run();
    assert_eq!(h.state().text, "JetBrains Mono");
    assert_eq!(h.state().picks, [5]);
    assert!(
        h.get_by_role(Role::EditableComboBox)
            .accesskit_node()
            .is_focused()
    );
    assert!(h.query_by_label("No matching fonts").is_none());
    h.get_by_role(Role::EditableComboBox).click();
    h.run();
    assert!(
        h.query_by_role(Role::ListBoxOption).is_some(),
        "clicking the focused field reopens"
    );
}

#[test]
fn blur_and_disabling_cancel_the_query_without_committing() {
    let mut h = harness([320.0, 320.0], 1.0, OPTIONS);
    h.key_press(Key::Tab);
    h.run();
    search(&mut h, "mono");
    h.key_press(Key::Tab);
    h.run();
    assert_eq!(h.state().text, "Inter");
    assert!(h.query_by_role(Role::ListBoxOption).is_none());
    h.get_by_role(Role::EditableComboBox).click();
    h.run();
    search(&mut h, "mono");
    h.state_mut().enabled = false;
    h.key_press(Key::Enter);
    h.run();
    assert_eq!(h.state().text, "Inter");
    assert!(h.state().picks.is_empty());
    assert!(h.query_by_role(Role::ListBoxOption).is_none());
    h.state_mut().enabled = true;
    h.state_mut().text = "Onest".into();
    h.run();
    h.get_by_role(Role::EditableComboBox).click();
    h.run();
    search(&mut h, "mono");
    h.get_by_label("Next").click();
    h.run();
    assert_eq!(
        h.state().text,
        "Onest",
        "external choice becomes the cancellation value"
    );
}

#[test]
fn empty_options_ignore_selection_keys() {
    let mut h = harness([320.0, 240.0], 1.0, &[]);
    h.get_by_role(Role::EditableComboBox).click();
    h.run();
    for key in [Key::ArrowDown, Key::ArrowUp, Key::Enter] {
        h.key_press(key);
        h.run();
    }
    assert!(h.state().picks.is_empty());
    assert_eq!(h.state().text, "Inter");
    assert!(h.query_by_label("No matching fonts").is_some());
}

#[test]
fn field_height_and_popup_bounds_follow_size_and_dpi() {
    for size in [[320.0, 240.0], [1280.0, 800.0], [1920.0, 1080.0]] {
        for scale in [1.0, 1.5] {
            let mut h = harness(size, scale, OPTIONS);
            h.get_by_role(Role::EditableComboBox).click();
            h.run();
            assert_eq!(h.state().rect.height(), 48.0);
            let field = h.get_by_role(Role::EditableComboBox);
            assert_eq!(
                field.accesskit_node().data().has_popup(),
                Some(egui::accesskit::HasPopup::Listbox)
            );
            assert_eq!(field.accesskit_node().data().is_expanded(), Some(true));
            let listbox = h.get_by_role(Role::ListBox);
            let controls: Vec<_> = field
                .accesskit_node()
                .controls()
                .map(|node| node.id())
                .collect();
            assert_eq!(controls, [listbox.accesskit_node().id()]);
            let selected = h.get_by_role_and_label(Role::ListBoxOption, "Inter");
            assert_eq!(
                field
                    .accesskit_node()
                    .active_descendant()
                    .map(|node| node.id()),
                Some(selected.accesskit_node().id())
            );
            for (_, popup) in test_support::area_rects(&h.ctx) {
                assert!(
                    h.ctx.content_rect().contains_rect(popup),
                    "{size:?} {scale}x {popup:?}"
                );
            }
        }
    }
}
