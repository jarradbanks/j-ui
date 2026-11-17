//! Soft geometry and state colors measured against the committed HTML reference.

use egui::{Color32, Key, Rect, RichText, Sense, Ui, UiBuilder, pos2, vec2};
use egui_kittest::{
    Harness, SnapshotResults,
    kittest::{NodeT, Queryable},
};
use j_ui::{
    test_support, theme, themes,
    tokens::{color, size},
    widgets::*,
};

const STATES: [State; 5] = [
    State::Default,
    State::Hover,
    State::Focus,
    State::Pressed,
    State::Disabled,
];

fn matrix(ui: &mut Ui, rects: &mut Vec<Vec<Rect>>) {
    rects.clear();
    ui.painter()
        .rect_filled(ui.ctx().content_rect(), 0.0, color::panel());
    for (column, state) in STATES.into_iter().enumerate() {
        let rect =
            Rect::from_min_size(pos2(24.0 + column as f32 * 246.0, 16.0), vec2(222.0, 760.0));
        ui.scope_builder(UiBuilder::new().max_rect(rect), |ui| {
            ui.label(format!("{state:?}"));
            let mut rows = Vec::new();
            for kind in [
                ButtonKind::Primary,
                ButtonKind::Secondary,
                ButtonKind::Ghost,
                ButtonKind::Destructive,
            ] {
                rows.push(ui.add(Button::new(kind, "Save").state(state)).rect);
            }
            rows.push(
                ui.add(
                    TextInput::new(&mut "Value".into())
                        .width(222.0)
                        .state(state),
                )
                .rect,
            );
            rows.push(ui.add(Select::new("30 fps").width(222.0).state(state)).rect);
            rows.push(
                ui.add(Checkbox::new(&mut false, "Loop Playback").state(state))
                    .rect,
            );
            rows.push(
                ui.add(Checkbox::new(&mut true, "Loop Playback").state(state))
                    .rect,
            );
            rows.push(
                ui.add(
                    Toggle::new(&mut false, "Auto Save")
                        .show_label()
                        .state(state),
                )
                .rect,
            );
            rows.push(
                ui.add(
                    Toggle::new(&mut true, "Auto Save")
                        .show_label()
                        .state(state),
                )
                .rect,
            );
            rows.push(
                ui.add(RadioGroup::new(&mut 1, &["Draft", "Standard"]).state(state))
                    .rect,
            );
            rows.push(
                ui.add(Segmented::new(&mut 0, &["Grid", "List", "Detail"]).state(state))
                    .rect,
            );
            rows.push(
                ui.add(Pills::new(&mut 0, &["All", "Video", "Audio"]).state(state))
                    .rect,
            );
            rows.push(
                ui.add(Slider::new(&mut 64.0, 0.0..=100.0, "Volume").state(state))
                    .rect,
            );
            rows.push(
                ui.add(SectionHeader::new("Transform", &mut true).state(state))
                    .rect,
            );
            rects.push(rows);
        });
    }
}

#[test]
fn soft_states_geometry_pixels_and_snapshots() {
    if !test_support::gpu() {
        return;
    }
    let mut results = SnapshotResults::new();
    for (name, palette) in themes::BUILT_IN {
        for ppp in [1.0, 1.5] {
            let mut fresh = true;
            let mut h = Harness::builder()
                .with_size([1280.0, 800.0])
                .with_pixels_per_point(ppp)
                .wgpu()
                .build_ui_state(
                    move |ui, rects| {
                        if test_support::install_theme(ui, &mut fresh) {
                            themes::apply(ui.ctx(), &palette);
                            return;
                        }
                        j_ui::motion::set_enabled(ui.ctx(), false);
                        matrix(ui, rects);
                    },
                    Vec::new(),
                );
            h.run();
            let image = h.render().unwrap();
            let sample = |x: f32, y: f32| {
                let pixel = image.get_pixel((x * ppp).round() as u32, (y * ppp).round() as u32);
                Color32::from_rgb(pixel[0], pixel[1], pixel[2])
            };
            for (column, rows) in h.state().iter().enumerate() {
                for (index, row) in rows.iter().enumerate() {
                    let height = if index == 13 {
                        size::THUMB + 4.0
                    } else {
                        size::CONTROL
                    };
                    assert!(
                        (row.height() - height).abs() < 1.0,
                        "{name} {ppp}x {index}: {row:?}"
                    );
                    assert!(
                        Rect::from_min_max(pos2(0.0, 0.0), pos2(1280.0, 800.0)).contains_rect(*row)
                    );
                }
                // Chromium's Figtree Medium Save is 31px wide, plus 16px padding and 1px border per side.
                assert!(
                    (rows[0].width() - 65.0).abs() <= 1.0,
                    "{name} {ppp}x Save: {:?}",
                    rows[0]
                );
                let primary = [
                    palette.blue,
                    palette.blue_hover,
                    palette.blue_hover,
                    color::BLUE_PRESSED,
                    palette.selection_fill(),
                ][column];
                assert_eq!(sample(rows[0].left() + 8.0, rows[0].center().y), primary);
                let secondary = [
                    palette.selection_fill(),
                    palette.raised,
                    palette.selection_fill(),
                    palette.raised,
                    palette.selection_fill(),
                ][column];
                assert_eq!(sample(rows[1].left() + 8.0, rows[1].center().y), secondary);
                let ghost = [
                    palette.panel,
                    palette.raised,
                    palette.selection_fill(),
                    palette.raised,
                    palette.selection_fill(),
                ][column];
                assert_eq!(sample(rows[2].left() + 8.0, rows[2].center().y), ghost);
                // A button never gets an outside focus ring; fields do.
                assert_eq!(
                    sample(rows[0].left() - 2.0, rows[0].center().y),
                    palette.panel
                );
                let field_edge = if column == 2 {
                    palette.blue_soft
                } else {
                    palette.panel
                };
                assert_eq!(sample(rows[4].left() - 2.0, rows[4].center().y), field_edge);
            }
            results.add(h.try_snapshot(format!("soft_states_{name}_{ppp}x")));
        }
    }
    results.unwrap();
    println!(
        "PASSED Soft states: 15 control rows, default/hover/focus/pressed/disabled; Dark/Light/High Contrast, 1280x800, DPI 1/1.5"
    );
}

#[test]
fn dropdown_keyboard_selection_escape_outside_and_disabled() {
    for viewport in [[1280.0, 800.0], [1920.0, 1080.0]] {
        for ppp in [1.0, 1.5] {
            let mut fresh = true;
            let mut h = Harness::builder()
                .with_size(viewport)
                .with_pixels_per_point(ppp)
                .build_ui_state(
                    move |ui, selected| {
                        if test_support::install_theme(ui, &mut fresh) {
                            return;
                        }
                        ui.add(Dropdown::new(
                            selected,
                            &["24 fps", "30 fps", "60 fps", "120 fps"],
                        ));
                        ui.add(Button::secondary("Next"));
                        ui.add(Dropdown::new(&mut 0, &["Disabled"]).state(State::Disabled));
                    },
                    1usize,
                );
            h.run();
            h.key_press(Key::Tab);
            h.run();
            h.key_press(Key::ArrowDown);
            h.run();
            assert!(h.query_by_label("120 fps").is_some());
            h.key_press(Key::ArrowDown);
            h.run();
            h.key_press(Key::Enter);
            h.run();
            assert_eq!(*h.state(), 2);
            assert!(h.query_by_label("120 fps").is_none());
            assert!(h.get_by_label("60 fps").accesskit_node().is_focused());
            h.key_press(Key::ArrowUp);
            h.run();
            h.key_press(Key::ArrowUp);
            h.run();
            h.key_press(Key::Escape);
            h.run();
            assert_eq!(*h.state(), 2);
            assert!(h.query_by_label("120 fps").is_none());
            h.get_by_label("60 fps").click();
            h.run();
            h.get_by_label("Next").click();
            h.run();
            assert!(h.query_by_label("120 fps").is_none());
            h.get_by_label("Disabled").click();
            h.run();
            assert_eq!(h.query_all_by_label("Disabled").count(), 1);
        }
    }
    println!(
        "PASSED dropdown: arrows/Enter/Escape/outside/disabled, 1280x800/1920x1080, DPI 1/1.5"
    );
}

#[test]
fn typography_preserves_weights_tracking_and_tabular_numbers() {
    let mut fresh = true;
    let mut h = Harness::builder().build_ui(move |ui| {
        if test_support::install_theme(ui, &mut fresh) {
            return;
        }
        for font in [theme::body(), theme::body_strong(), theme::title()] {
            let widths: Vec<_> = (0..10)
                .map(|n| {
                    ui.painter()
                        .layout_no_wrap(n.to_string(), font.clone(), Color32::WHITE)
                        .size()
                        .x
                })
                .collect();
            assert!(
                widths.iter().all(|width| (width - widths[0]).abs() < 0.01),
                "{widths:?}"
            );
        }
        let title = fit_text(ui, "Display", theme::display(), Color32::WHITE, 400.0);
        assert_eq!(title.job.sections[0].format.extra_letter_spacing, -0.28);
        ui.add(egui::Label::new(RichText::new("Ready")));
        ui.allocate_exact_size(vec2(1.0, 1.0), Sense::hover());
    });
    h.run();
}

#[test]
fn autocomplete_empty_escape_reopen_and_short_viewport() {
    let fonts = [
        "Inter",
        "Inter Display",
        "JetBrains Mono",
        "Onest",
        "Geist",
        "Noto Sans",
    ];
    for ppp in [1.0, 1.5] {
        let mut fresh = true;
        let mut h = Harness::builder()
            .with_size([320.0, 240.0])
            .with_pixels_per_point(ppp)
            .build_ui_state(
                move |ui, text| {
                    if test_support::install_theme(ui, &mut fresh) {
                        return;
                    }
                    ui.add_space(110.0);
                    ui.add(Autocomplete::new(text, &fonts).empty("No matching fonts"));
                    ui.add(Button::secondary("Next"));
                },
                String::new(),
            );
        h.run();
        h.key_press(Key::Tab);
        h.run();
        for (_, popup) in test_support::area_rects(&h.ctx) {
            assert!(
                h.ctx.content_rect().contains_rect(popup),
                "{ppp}x {popup:?}"
            );
        }
        h.event(egui::Event::Text("zzz".into()));
        h.run();
        assert!(h.query_by_label("No matching fonts").is_some());
        h.key_press(Key::Enter);
        h.run();
        assert_eq!(h.state(), "zzz");
        h.get_by_role(egui::accesskit::Role::EditableComboBox)
            .focus();
        h.run();
        h.key_press_modifiers(egui::Modifiers::COMMAND, Key::A);
        h.event(egui::Event::Text("in".into()));
        h.run();
        assert_eq!(h.state(), "in");
        h.key_press(Key::Escape);
        h.run();
        assert!(
            h.get_by_role(egui::accesskit::Role::EditableComboBox)
                .accesskit_node()
                .is_focused(),
            "Escape keeps focus"
        );
        assert!(h.query_by_label("Inter Display").is_none());
        h.key_press(Key::ArrowDown);
        h.run();
        assert!(h.query_by_label("Inter Display").is_some());
        h.get_by_label("Next").click();
        h.run();
        assert!(h.query_by_label("Inter Display").is_none());
    }
}
