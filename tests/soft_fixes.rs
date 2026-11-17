//! Ink placement and contrast checks for the Soft validation corrections.

use egui::{Color32, Rect, Shape, UiBuilder, pos2, vec2};
use egui_kittest::Harness;
use j_ui::{
    panel::Panel,
    test_support, theme, themes,
    tokens::{color, contrast, size},
    widgets::*,
};

fn text_shapes(shape: &Shape, out: &mut Vec<egui::epaint::TextShape>) {
    match shape {
        Shape::Text(text) => out.push(text.clone()),
        Shape::Vec(shapes) => shapes.iter().for_each(|shape| text_shapes(shape, out)),
        _ => {}
    }
}

#[test]
fn painted_labels_and_notice_actions_center_their_ink() {
    if !test_support::gpu() {
        return;
    }
    for viewport in [[1280.0, 800.0], [1920.0, 1080.0]] {
        for ppp in [1.0, 1.5] {
            let mut fresh = true;
            let mut h = Harness::builder()
                .with_size(viewport)
                .with_pixels_per_point(ppp)
                .wgpu()
                .build_ui_state(
                    move |ui, expected: &mut Vec<(&str, Rect)>| {
                        if test_support::install_theme(ui, &mut fresh) {
                            return;
                        }
                        expected.clear();
                        ui.set_width(420.0);
                        expected.push(("Save", ui.add(Button::primary("Save")).rect));
                        expected.push((
                            "New Folder",
                            ui.add(Button::secondary("New Folder").with_icon(theme::icon::FOLDER))
                                .rect,
                        ));
                        expected.push(("30 fps", ui.add(Select::new("30 fps")).rect));
                        expected.push((
                            "Grid",
                            ui.add(Segmented::new(&mut 0, &["Grid", "List", "Detail"]))
                                .rect,
                        ));
                        expected.push(("All", ui.add(Pills::new(&mut 0, &["All", "Video"])).rect));
                        expected.push((
                            "Loop Playback",
                            ui.add(Checkbox::new(&mut true, "Loop Playback")).rect,
                        ));
                        expected.push((
                            "Auto Save",
                            ui.add(Toggle::new(&mut true, "Auto Save").show_label())
                                .rect,
                        ));
                        expected.push((
                            "Standard",
                            ui.add(RadioGroup::new(&mut 1, &["Draft", "Standard"])).rect,
                        ));
                        expected.push((
                            "Snap to Grid",
                            ui.add(MenuRow::new("Snap to Grid", true).shortcut("Ctrl+G"))
                                .rect,
                        ));
                        expected.push((
                            "Inspector",
                            ui.add(Tabs::new(&["Inspector", "Effects"], &mut 0)).rect,
                        ));
                        expected.push((
                            "Recent Projects",
                            ui.add(NavRow::new("Recent Projects", true)).rect,
                        ));
                        expected.push((
                            "Ready",
                            ui.add(Badge::new("Ready").tone(Tone::Success)).rect,
                        ));
                        let panel = Rect::from_min_size(pos2(500.0, 24.0), vec2(400.0, 180.0));
                        ui.scope_builder(UiBuilder::new().max_rect(panel), |ui| {
                            Panel::titled("Buttons").show(ui, |_| {});
                        });
                        expected.push((
                            "Buttons",
                            Rect::from_min_size(
                                panel.min + vec2(1.0, 1.0),
                                vec2(398.0, size::HEADER),
                            ),
                        ));
                        let notices = Rect::from_min_size(pos2(500.0, 240.0), vec2(600.0, 220.0));
                        ui.scope_builder(UiBuilder::new().max_rect(notices), |ui| {
                            let toast = Toast::new("Export finished.", "Dismiss")
                                .action("Open")
                                .show(ui);
                            expected.push(("Open", toast.action.unwrap().rect));
                            let action = Banner::new(Tone::Info, "An update is ready to install.")
                                .action("Restart")
                                .show(ui)
                                .unwrap();
                            expected.push(("Restart", action.rect));
                        });
                    },
                    Vec::new(),
                );
            h.run();
            let mut texts = Vec::new();
            for clipped in &h.output().shapes {
                text_shapes(&clipped.shape, &mut texts);
            }
            let image = h.render().unwrap();
            for (label, rect) in h.state() {
                let text = texts
                    .iter()
                    .find(|text| text.galley.text() == *label)
                    .unwrap();
                let bounds = text.visual_bounding_rect();
                let geometry = (bounds.center().y - rect.center().y).abs() * ppp;
                assert!(
                    geometry <= 0.501,
                    "{ppp}x {label}: ink geometry {geometry}px"
                );
                let crop = bounds.expand(2.0);
                let x0 = (crop.left() * ppp).floor().max(0.0) as u32;
                let x1 = (crop.right() * ppp).ceil() as u32;
                let y0 = (crop.top() * ppp).floor().max(0.0) as u32;
                let y1 = (crop.bottom() * ppp).ceil() as u32;
                let background = image.get_pixel(x0, y0).0;
                let mut top = u32::MAX;
                let mut bottom = 0;
                for y in y0..y1 {
                    for x in x0..x1 {
                        let pixel = image.get_pixel(x, y).0;
                        let delta: u16 = pixel[..3]
                            .iter()
                            .zip(&background[..3])
                            .map(|(a, b)| u16::from(a.abs_diff(*b)))
                            .sum();
                        if delta > 180 {
                            top = top.min(y);
                            bottom = bottom.max(y);
                        }
                    }
                }
                assert_ne!(top, u32::MAX, "{label}: visible ink");
                let offset = ((top + bottom + 1) as f32 / 2.0 - rect.center().y * ppp).abs();
                assert!(offset <= 1.0, "{ppp}x {label}: raster offset {offset}px");
            }
            let ink_center = |label: &str| {
                texts
                    .iter()
                    .find(|text| text.galley.text() == label)
                    .unwrap()
                    .visual_bounding_rect()
                    .center()
                    .y
            };
            for (message, action) in [
                ("Export finished.", "Open"),
                ("An update is ready to install.", "Restart"),
            ] {
                let offset = (ink_center(message) - ink_center(action)).abs();
                assert!(offset <= 1.0, "{ppp}x {message}/{action}: {offset}pt");
            }
            assert!(
                (h.state()[3].1.width() - 179.0).abs() <= 1.0,
                "segment width {:?}",
                h.state()[3].1
            );
        }
    }
    println!(
        "PASSED ink centering: 15 labels, geometry <=0.5px, raster <=1px; notice/action rows <=1pt; segment width 179 +/-1pt; 1280x800/1920x1080 at 100%/150%"
    );
}

#[test]
fn high_contrast_surfaces_and_selected_controls_remain_distinct() {
    if !test_support::gpu() {
        return;
    }
    let palette = themes::HIGH_CONTRAST;
    assert!(contrast(palette.selection_fill(), palette.panel) >= 3.0);
    assert!(contrast(palette.selection_fill(), palette.raised) >= 3.0);
    assert!(contrast(palette.ink, palette.selection_fill()) >= 4.5);
    for ppp in [1.0, 1.5] {
        let mut fresh = true;
        let mut h = Harness::builder()
            .with_size([480.0, 400.0])
            .with_pixels_per_point(ppp)
            .wgpu()
            .build_ui_state(
                move |ui, rects: &mut Vec<Rect>| {
                    if test_support::install_theme(ui, &mut fresh) {
                        themes::apply(ui.ctx(), &palette);
                        return;
                    }
                    j_ui::motion::set_enabled(ui.ctx(), false);
                    ui.painter()
                        .rect_filled(ui.ctx().content_rect(), 0.0, color::ground());
                    let panel = Rect::from_min_size(pos2(16.0, 16.0), vec2(448.0, 368.0));
                    rects.clear();
                    ui.scope_builder(UiBuilder::new().max_rect(panel), |ui| {
                        Panel::titled("Contrast").show(ui, |ui| {
                            rects.push(ui.add(Button::secondary("Save")).rect);
                            rects.push(ui.add(Button::secondary("Hover").state(State::Hover)).rect);
                            rects.push(ui.add(Segmented::new(&mut 0, &["Grid", "List"])).rect);
                            rects.push(ui.add(MenuRow::option("Selected", true).lit(true)).rect);
                            rects.push(ui.add(NavRow::new("Recent", true)).rect);
                        });
                    });
                },
                Vec::new(),
            );
        h.run();
        let image = h.render().unwrap();
        let sample = |x: f32, y: f32| {
            let pixel = image.get_pixel((x * ppp).floor() as u32, (y * ppp).floor() as u32);
            Color32::from_rgb(pixel[0], pixel[1], pixel[2])
        };
        assert!(
            contrast(sample(200.0, 16.0), palette.ground) >= 3.0,
            "panel edge"
        );
        for (index, rect) in h.state().iter().enumerate() {
            if index == 1 {
                assert!(
                    contrast(
                        sample((rect.left() * ppp).round() / ppp, rect.center().y),
                        palette.panel
                    ) >= 3.0,
                    "hover edge"
                );
            } else {
                let inset = if index == 2 { 5.0 } else { 2.0 };
                let fill = sample(rect.left() + inset, rect.center().y);
                assert!(
                    contrast(fill, palette.panel) >= 3.0,
                    "{ppp}x selected fill {index}: {fill:?}"
                );
            }
        }
    }
    println!(
        "PASSED High Contrast: panel and hover outlines >=3:1; selected fills >=3:1 against Panel/Raised; label contrast >=4.5:1; 100%/150%"
    );
}

#[test]
fn control_colors_animate_between_states_and_settle_in_120ms() {
    if !test_support::gpu() {
        return;
    }
    let mut fresh = true;
    let mut h = Harness::builder()
        .with_size([800.0, 600.0])
        .with_step_dt(0.02)
        .wgpu()
        .build_ui_state(
            move |ui, state: &mut (bool, Vec<egui::Pos2>)| {
                if test_support::install_theme(ui, &mut fresh) {
                    return;
                }
                j_ui::motion::set_enabled(ui.ctx(), true);
                state.1.clear();
                for index in 0..8 {
                    let rect = Rect::from_min_size(
                        pos2(
                            24.0 + (index % 2) as f32 * 360.0,
                            24.0 + (index / 2) as f32 * 130.0,
                        ),
                        vec2(320.0, 110.0),
                    );
                    let sample = ui
                        .scope_builder(UiBuilder::new().max_rect(rect), |ui| {
                            let mut selected = usize::from(state.0);
                            let mut checked = state.0;
                            match index {
                                0 => {
                                    let pinned = if state.0 {
                                        State::Focus
                                    } else {
                                        State::Default
                                    };
                                    let field = ui
                                        .add(
                                            TextInput::new(&mut "Value".into())
                                                .width(240.0)
                                                .state(pinned),
                                        )
                                        .rect;
                                    field.left_center()
                                }
                                1 => {
                                    let segment = ui
                                        .add(Segmented::new(&mut selected, &["Grid", "List"]))
                                        .rect;
                                    segment.left_center() + vec2(6.0, 0.0)
                                }
                                2 => {
                                    ui.add(MenuRow::option("Option", false).lit(state.0))
                                        .rect
                                        .left_center()
                                        + vec2(4.0, 0.0)
                                }
                                3 => {
                                    ui.add(NavRow::new("Projects", state.0)).rect.left_center()
                                        + vec2(4.0, 0.0)
                                }
                                4 => {
                                    ui.add(Checkbox::new(&mut checked, "Loop Playback"))
                                        .rect
                                        .left_center()
                                        + vec2(3.0, 0.0)
                                }
                                5 => {
                                    ui.add(Toggle::new(&mut checked, "Auto Save"))
                                        .rect
                                        .left_top()
                                        + vec2(16.0, 3.0)
                                }
                                6 => {
                                    let columns = [Column::fill("Name")];
                                    let mut selected = if state.0 { 0 } else { usize::MAX };
                                    let table = Table::new(&columns, &mut selected)
                                        .show(ui, &["Clip"], |ui, _, _| {
                                            ui.label("Clip");
                                        })
                                        .rect;
                                    table.left_top() + vec2(4.0, 60.0)
                                }
                                _ => {
                                    ui.add(Tabs::new(&["Inspector", "Effects"], &mut selected))
                                        .rect
                                        .left_bottom()
                                        + vec2(5.0, -1.0)
                                }
                            }
                        })
                        .inner;
                    state.1.push(sample);
                }
            },
            (false, Vec::new()),
        );
    h.run();
    let colors = |h: &mut Harness<'_, (bool, Vec<egui::Pos2>)>| {
        let image = h.render().unwrap();
        h.state()
            .1
            .iter()
            .map(|pos| {
                image
                    .get_pixel(pos.x.round() as u32, pos.y.round() as u32)
                    .0
            })
            .collect::<Vec<_>>()
    };
    let initial = colors(&mut h);
    h.state_mut().0 = true;
    h.step();
    h.run_steps(3);
    let middle = colors(&mut h);
    h.run_steps(4);
    let settled = colors(&mut h);
    for (index, ((initial, middle), settled)) in
        initial.iter().zip(&middle).zip(&settled).enumerate()
    {
        assert_ne!(initial, settled, "control {index} must change color");
        assert_ne!(initial, middle, "control {index} must start transitioning");
        assert_ne!(
            middle, settled,
            "control {index} must not jump directly to its target"
        );
    }
    println!(
        "PASSED transitions: field, segmented control, menu, navigation row, checkbox, toggle, table and tabs paint intermediate colors and settle after 120ms"
    );
}
