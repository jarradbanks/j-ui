//! Rendered regressions for focus clipping and input content geometry.

use egui::{Rect, Shape, UiBuilder, pos2, vec2};
use egui_kittest::{Harness, SnapshotResults};
use j_ui::{
    test_support,
    theme::icon,
    tokens::color,
    widgets::{Button, State, TextInput},
};

const SCALES: [f32; 3] = [1.0, 1.25, 1.5];
const VIEWPORTS: [[f32; 2]; 2] = [[1280.0, 720.0], [1920.0, 1080.0]];
const HEIGHTS: [f32; 4] = [24.0, 32.0, 36.0, 48.0];

fn texts(shape: &Shape, out: &mut Vec<Rect>) {
    match shape {
        Shape::Text(text) if text.galley.job.text == "Search Games" => {
            out.push(text.visual_bounding_rect());
        }
        Shape::Vec(shapes) => {
            for shape in shapes {
                texts(shape, out);
            }
        }
        _ => {}
    }
}

fn evidence(h: &mut Harness<'_>, name: &str) {
    if let Ok(dir) = std::env::var("JUI_FEEDBACK_SCREENSHOTS") {
        std::fs::create_dir_all(&dir).unwrap();
        h.render()
            .unwrap()
            .save(std::path::Path::new(&dir).join(format!("{name}.png")))
            .unwrap();
    }
}

#[test]
fn focus_ring_is_complete_at_all_clipped_container_edges() {
    if !test_support::gpu() {
        return;
    }
    let mut failures = Vec::new();
    let mut snapshots = SnapshotResults::new();
    for ppp in SCALES {
        let mut fresh = true;
        let mut h = Harness::builder()
            .with_size([240.0, 100.0])
            .with_pixels_per_point(ppp)
            .wgpu()
            .build_ui(move |ui| {
                if test_support::install_theme(ui, &mut fresh) {
                    return;
                }
                ui.painter()
                    .rect_filled(ui.ctx().content_rect(), 0.0, color::ground());
                for (x, clipped) in [(24.0, false), (120.0, true)] {
                    let rect = Rect::from_min_size(pos2(x, 24.0), vec2(36.0, 36.0));
                    ui.scope_builder(UiBuilder::new().max_rect(rect), |ui| {
                        if clipped {
                            ui.set_clip_rect(rect);
                        }
                        ui.add(
                            Button::icon(icon::SLIDERS, "Filters")
                                .width(36.0)
                                .height(36.0)
                                .state(State::Focus),
                        );
                    });
                }
            });
        h.run();
        evidence(&mut h, &format!("10-focus-{ppp}x"));
        let image = h.render().unwrap();
        // Compare the entire ring, including the pixels outside the control's bounds.
        let mut missing = 0;
        for y in (20.0 * ppp) as u32..(64.0 * ppp) as u32 {
            for x in (20.0 * ppp) as u32..(64.0 * ppp) as u32 {
                if image.get_pixel(x, y) != image.get_pixel(x + (96.0 * ppp) as u32, y) {
                    missing += 1;
                }
            }
        }
        if missing > 0 {
            failures.push(format!("{ppp}x: {missing} focus ring pixels clipped"));
        }
        snapshots.add(h.try_snapshot(format!("input_focus_{ppp}x")));
    }
    snapshots.unwrap();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    println!("PASSED focus ring: four clipped edges, DPI 1.0/1.25/1.5");
}

#[test]
fn single_line_placeholder_and_value_are_centred_and_padded() {
    if !test_support::gpu() {
        return;
    }
    let mut failures = Vec::new();
    let mut snapshots = SnapshotResults::new();
    for viewport in VIEWPORTS {
        for ppp in SCALES {
            let mut fresh = true;
            let mut fields = Vec::new();
            let mut h = Harness::builder()
                .with_size(viewport)
                .with_pixels_per_point(ppp)
                .wgpu()
                .build_ui(move |ui| {
                    if test_support::install_theme(ui, &mut fresh) {
                        return;
                    }
                    ui.painter()
                        .rect_filled(ui.ctx().content_rect(), 0.0, color::ground());
                    for (row, height) in HEIGHTS.into_iter().enumerate() {
                        for (column, (filled, search)) in
                            [(false, false), (true, false), (false, true), (true, true)]
                                .into_iter()
                                .enumerate()
                        {
                            let rect = Rect::from_min_size(
                                pos2(24.0 + column as f32 * 280.0, 24.0 + row as f32 * 80.0),
                                vec2(240.0, height),
                            );
                            let mut value = String::new();
                            if filled {
                                value = "Search Games".into();
                            }
                            let field = if search {
                                TextInput::search(&mut value)
                            } else {
                                TextInput::new(&mut value)
                            };
                            ui.put(
                                rect,
                                field
                                    .hint("Search Games")
                                    .width(rect.width())
                                    .height(height),
                            );
                        }
                    }
                });
            h.run();
            evidence(&mut h, &format!("18-inputs-{}-{ppp}x", viewport[0]));
            let image = h.render().unwrap();
            for clipped in &h.output().shapes {
                texts(&clipped.shape, &mut fields);
            }
            assert_eq!(fields.len(), 16);
            for (index, text) in fields.into_iter().enumerate() {
                let row = index / 4;
                let rect = Rect::from_min_size(
                    pos2(24.0 + (index % 4) as f32 * 280.0, 24.0 + row as f32 * 80.0),
                    vec2(240.0, HEIGHTS[row]),
                );
                let delta = (text.center().y - rect.center().y).abs() * ppp;
                if delta > 1.0 {
                    failures.push(format!(
                        "{viewport:?} {ppp}x field {index}: centre off {delta}px"
                    ));
                }
                assert!(text.left() - rect.left() >= 8.0, "inner left padding");
                assert!(rect.right() - text.right() >= 8.0, "inner right padding");
                let top = ((rect.top() + 2.0) * ppp).round() as u32;
                let bottom = ((rect.bottom() - 2.0) * ppp).round() as u32;
                let mut ink_top = u32::MAX;
                let mut ink_bottom = 0;
                for y in top..bottom {
                    for x in (text.left() * ppp).floor() as u32..(text.right() * ppp).ceil() as u32
                    {
                        if image.get_pixel(x, y)[0] > 80 {
                            ink_top = ink_top.min(y);
                            ink_bottom = ink_bottom.max(y);
                        }
                    }
                }
                assert_ne!(ink_top, u32::MAX, "rendered input text");
                let ink_centre = (ink_top + ink_bottom + 1) as f32 / 2.0;
                let delta = (ink_centre - rect.center().y * ppp).abs();
                if delta > 1.0 {
                    failures.push(format!(
                        "{viewport:?} {ppp}x field {index}: ink off {delta}px"
                    ));
                }
            }
            if viewport == VIEWPORTS[0] {
                snapshots.add(h.try_snapshot(format!("input_singleline_{ppp}x")));
            }
        }
    }
    snapshots.unwrap();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    println!(
        "PASSED inputs: placeholder/value, heights 24/32/36/48, 1280x720/1920x1080, DPI 1.0/1.25/1.5, <=1 physical pixel"
    );
}

#[test]
fn multiline_keeps_scrolling_content_inside_padding() {
    if !test_support::gpu() {
        return;
    }
    let mut failures = Vec::new();
    let mut snapshots = SnapshotResults::new();
    for ppp in SCALES {
        let mut fresh = true;
        let mut h = Harness::builder()
            .with_size([600.0, 180.0])
            .with_pixels_per_point(ppp)
            .wgpu()
            .build_ui(move |ui| {
                if test_support::install_theme(ui, &mut fresh) {
                    return;
                }
                ui.painter()
                    .rect_filled(ui.ctx().content_rect(), 0.0, color::ground());
                for (column, value) in ["", "Search Games\nSecond line\nThird line\nOverflow"]
                    .into_iter()
                    .enumerate()
                {
                    let mut value = value.to_owned();
                    let rect = Rect::from_min_size(
                        pos2(24.0 + column as f32 * 280.0, 24.0),
                        vec2(240.0, 100.0),
                    );
                    ui.scope_builder(UiBuilder::new().id_salt(column).max_rect(rect), |ui| {
                        ui.add(
                            TextInput::new(&mut value)
                                .hint("Search Games")
                                .width(240.0)
                                .rows(3),
                        );
                    });
                }
            });
        h.run();
        evidence(&mut h, &format!("21d-multiline-{ppp}x"));
        let mut fields = Vec::new();
        for clipped in &h.output().shapes {
            texts(&clipped.shape, &mut fields);
        }
        let placeholder = fields.first().unwrap();
        assert!(
            placeholder.top() >= 28.0,
            "at least 4pt top padding: {placeholder:?}"
        );
        assert!(placeholder.left() >= 32.0, "at least 8pt left padding");
        let image = h.render().unwrap();
        let row = h
            .ctx
            .fonts_mut(|fonts| fonts.row_height(&j_ui::theme::body()));
        let bottom = 48.0 + 2.0 * row;
        let mut overflow = 0;
        for y in ((bottom - 4.0) * ppp).ceil() as u32..((bottom + 40.0) * ppp) as u32 {
            for x in (312.0 * ppp) as u32..(536.0 * ppp) as u32 {
                if image.get_pixel(x, y)[0] > 140 {
                    overflow += 1;
                }
            }
        }
        if overflow > 0 {
            failures.push(format!(
                "{ppp}x: {overflow} text pixels escape bottom padding"
            ));
        }
        snapshots.add(h.try_snapshot(format!("input_multiline_{ppp}x")));
    }
    snapshots.unwrap();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    println!("PASSED multiline: placeholder/overflow, inner padding, DPI 1.0/1.25/1.5");
}
