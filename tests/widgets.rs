use egui::{Key, PointerButton, Pos2, vec2};
use egui_kittest::{
    Harness, SnapshotResults,
    kittest::{NodeT, Queryable},
};
use j_ui::{
    test_support, themes,
    tokens::{color, size},
    widgets::{Button, MenuRow, Slider, State},
};

fn click_at(h: &Harness<'_, f64>, pos: Pos2) {
    h.hover_at(pos);
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
    }
}

#[test]
fn slider_orientation_pointer_keyboard_steps_and_disabled() {
    for vertical in [false, true] {
        let mut fresh = true;
        let mut h = Harness::builder().with_size([240.0, 220.0]).build_ui_state(
            move |ui, value| {
                if test_support::install_theme(ui, &mut fresh) {
                    return;
                }
                let slider = Slider::new(value, 1.0..=33.0, "Size").step(2.0);
                if vertical {
                    ui.add_sized([size::CONTROL, 160.0], slider.vertical());
                } else {
                    ui.add_sized([160.0, size::CONTROL], slider);
                }
                ui.add(Button::secondary("Next"));
            },
            1.0,
        );
        let rect = h.get_by_label("Size").rect();
        assert_eq!(
            h.get_by_label("Size").accesskit_node().orientation(),
            Some(if vertical {
                egui::accesskit::Orientation::Vertical
            } else {
                egui::accesskit::Orientation::Horizontal
            })
        );
        assert_eq!(
            rect.size(),
            if vertical {
                vec2(24.0, 160.0)
            } else {
                vec2(160.0, 24.0)
            }
        );
        h.get_by_label("Size").click();
        h.run();
        assert_eq!(*h.state(), 17.0);
        assert!(h.get_by_label("Size").accesskit_node().is_focused());

        let (increase, decrease) = if vertical {
            (Key::ArrowUp, Key::ArrowDown)
        } else {
            (Key::ArrowRight, Key::ArrowLeft)
        };
        h.key_press(increase);
        h.run();
        assert_eq!(*h.state(), 19.0);
        h.key_press(decrease);
        h.run();
        assert_eq!(*h.state(), 17.0);

        let maximum = if vertical {
            rect.center_top()
        } else {
            rect.right_center()
        };
        let minimum = if vertical {
            rect.center_bottom()
        } else {
            rect.left_center()
        };
        click_at(
            &h,
            maximum
                + if vertical {
                    vec2(0.0, 1.0)
                } else {
                    vec2(-1.0, 0.0)
                },
        );
        h.run();
        assert_eq!(*h.state(), 33.0);
        h.key_press(increase);
        h.run();
        assert_eq!(*h.state(), 33.0);

        h.hover_at(rect.center());
        h.drag_at(rect.center());
        h.run();
        h.hover_at(minimum);
        h.run();
        h.drop_at(minimum);
        h.run();
        assert_eq!(*h.state(), 1.0);
    }

    let mut fresh = true;
    let mut h = Harness::builder().build_ui_state(
        move |ui, value| {
            if test_support::install_theme(ui, &mut fresh) {
                return;
            }
            ui.add_sized(
                [24.0, 120.0],
                Slider::new(value, 0.0..=1.0, "Disabled Size")
                    .vertical()
                    .state(State::Disabled),
            );
        },
        0.25,
    );
    assert!(
        h.get_by_label("Disabled Size")
            .accesskit_node()
            .is_disabled()
    );
    h.get_by_label("Disabled Size").click();
    h.key_press(Key::ArrowUp);
    h.run();
    assert_eq!(*h.state(), 0.25);
}

#[test]
fn menu_rows_keep_checkmarks_and_disable_mouse_and_keyboard() {
    let mut fresh = true;
    let mut h = Harness::builder().with_size([360.0, 180.0]).build_ui_state(
        move |ui, clicks: &mut [usize; 3]| {
            if test_support::install_theme(ui, &mut fresh) {
                return;
            }
            if ui
                .add(MenuRow::new("Save", false).shortcut("Ctrl+S"))
                .clicked()
            {
                clicks[0] += 1;
            }
            if ui
                .add(
                    MenuRow::new("Link Views", true)
                        .shortcut("L")
                        .enabled(false),
                )
                .clicked()
            {
                clicks[1] += 1;
            }
            if ui
                .add_enabled(false, MenuRow::new("Unavailable", false))
                .clicked()
            {
                clicks[2] += 1;
            }
        },
        [0; 3],
    );
    assert_eq!(
        h.get_by_label("Link Views").accesskit_node().toggled(),
        Some(egui::accesskit::Toggled::True)
    );
    for label in ["Link Views", "Unavailable"] {
        assert!(h.get_by_label(label).accesskit_node().is_disabled());
        h.get_by_label(label).click();
        h.run();
    }
    h.key_press(Key::Tab);
    h.run();
    h.key_press(Key::Enter);
    h.run();
    assert_eq!(*h.state(), [1, 0, 0]);
    h.get_by_label("Save").click();
    h.run();
    assert_eq!(*h.state(), [2, 0, 0]);
}

#[test]
fn additions_snapshots_dark_and_light() {
    if !test_support::gpu() {
        return;
    }
    let mut results = SnapshotResults::new();
    for (name, palette) in [("dark", themes::DARK), ("light", themes::LIGHT)] {
        let mut fresh = true;
        let mut h = Harness::builder().with_size([480.0, 420.0]).wgpu().build_ui(move |ui| {
            if test_support::install_theme(ui, &mut fresh) {
                themes::apply(ui.ctx(), &palette);
                return;
            }
            ui.strong("Palette variants");
            ui.horizontal(|ui| {
                for (label, fill) in [("Blue", color::blue_soft()), ("Red", color::red_soft()),
                    ("Amber", color::amber_soft()), ("Green", color::green_soft())] {
                    egui::Frame::new().fill(fill).inner_margin(8.0).show(ui, |ui| {
                        ui.label(label);
                    });
                }
                ui.add(Button::primary("Hover").state(State::Hover));
            });
            ui.separator();
            ui.strong("Sliders");
            ui.horizontal(|ui| {
                ui.add_sized([24.0, 128.0], Slider::new(&mut 9.0, 1.0..=33.0, "Brush Size").vertical().step(4.0));
                ui.add_sized([24.0, 128.0], Slider::new(&mut -50.0, -100.0..=100.0, "Centred Size").vertical().centred(true).state(State::Focus));
                ui.add_sized([24.0, 128.0], Slider::new(&mut 0.7, 0.0..=1.0, "Disabled Size").vertical().state(State::Disabled));
                ui.vertical(|ui| {
                    ui.add(Slider::new(&mut 0.7, 0.0..=1.0, "Horizontal"));
                    ui.add(Slider::new(&mut -25.0, -100.0..=100.0, "Centred").centred(true));
                });
            });
            ui.separator();
            ui.strong("Menu rows");
            ui.add(MenuRow::new("Save Project", false).shortcut("Ctrl+S"));
            ui.add(MenuRow::new("Link Views", true).shortcut("L").lit(true));
            ui.add(MenuRow::new("Flipbook", false).shortcut("Shift+F").enabled(false));
            ui.add(MenuRow::new("Auto-Group", true));
            ui.add(MenuRow::new("Long translated command label that must leave room for shortcut and checkmark", true).shortcut("Ctrl+Shift+S"));
        });
        h.run();
        results.add(h.try_snapshot(format!("additions_{name}")));
    }
    results.unwrap();
}
