use egui::vec2;
use egui_kittest::{
    Harness,
    kittest::{NodeT, Queryable},
};
use j_ui::widgets::SelectionCheckbox;

#[test]
fn circle_selection_mouse_space_mixed_focus_and_fractional_zoom() {
    for zoom in [1.0, 1.25, 1.5] {
        let mut h = Harness::builder().with_size([360.0, 160.0]).build_ui_state(
            |ui, state: &mut (bool, bool)| {
                let response =
                    ui.add(SelectionCheckbox::new(&mut state.0, "Select Items").mixed(state.1));
                if response.changed() {
                    state.1 = false;
                }
            },
            (false, true),
        );
        h.ctx.set_zoom_factor(zoom);
        h.run();
        let node = h.get_by_role_and_label(egui::accesskit::Role::CheckBox, "Select Items");
        assert_eq!(
            node.accesskit_node().toggled(),
            Some(egui::accesskit::Toggled::Mixed)
        );
        assert_eq!(node.rect().size(), vec2(20.0, 20.0));
        node.click();
        h.run();
        assert_eq!(*h.state(), (true, false));
        let node = h.get_by_label("Select Items");
        assert!(node.accesskit_node().is_focused());
        h.key_press(egui::Key::Space);
        h.run();
        assert!(!h.state().0);
        assert!(h.get_by_label("Select Items").accesskit_node().is_focused());
        h.key_press(egui::Key::Space);
        h.run();
        assert!(h.state().0);
    }
}

#[test]
fn switch_mouse_keyboard_and_disabled_input() {
    use j_ui::widgets::Toggle;
    let mut h = Harness::builder().build_ui_state(
        |ui, on| {
            ui.add(Toggle::new(on, "Live"));
            ui.add_enabled(false, Toggle::new(&mut false, "Unavailable"));
        },
        false,
    );
    h.key_press(egui::Key::Tab);
    h.run();
    h.key_press(egui::Key::Space);
    h.run();
    assert!(*h.state());
    h.get_by_role_and_label(egui::accesskit::Role::Switch, "Live")
        .click();
    h.run();
    assert!(!*h.state());
    assert!(h.get_by_label("Unavailable").accesskit_node().is_disabled());
}

#[test]
fn tall_search_clear_reports_change_and_returns_focus() {
    use j_ui::widgets::TextInput;
    let mut fresh = true;
    let mut h = Harness::builder().build_ui_state(
        move |ui, state: &mut (String, bool)| {
            if j_ui::test_support::install_theme(ui, &mut fresh) {
                return;
            }
            let response = ui.add(
                TextInput::search(&mut state.0)
                    .height(36.0)
                    .clearable("Clear Search"),
            );
            assert_eq!(response.rect.height(), 36.0);
            state.1 |= response.changed();
        },
        (String::from("query"), false),
    );
    h.get_by_label("Clear Search").click();
    h.run();
    assert!(h.state().0.is_empty());
    assert!(h.state().1);
    assert!(h.query_by_label("Clear Search").is_none());
    assert!(
        h.get_by_role(egui::accesskit::Role::TextInput)
            .accesskit_node()
            .is_focused()
    );
}
