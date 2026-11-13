//! Single-line ink alignment with fonts whose ink and line-box centres differ.

use egui::{Rect, Shape, pos2, vec2};
use egui_kittest::{Harness, kittest::Queryable};
use j_ui::{test_support, theme, tokens::color, widgets::TextInput};

const SCALES: [f32; 3] = [1.0, 1.25, 1.5];
const HEIGHTS: [f32; 4] = [24.0, 32.0, 36.0, 48.0];

fn install_font(ui: &egui::Ui, fresh: &mut bool, offset: f32) -> bool {
    if !std::mem::take(fresh) {
        return false;
    }
    theme::install(ui.ctx());
    let mut fonts = theme::fonts("en");
    // Emulate Onest's ink above its line-box centre without another bundled font.
    std::sync::Arc::make_mut(fonts.font_data.get_mut("inter").unwrap())
        .tweak
        .y_offset = offset;
    ui.ctx().set_fonts(fonts);
    ui.ctx()
        .all_styles_mut(|style| style.visuals.text_cursor.blink = false);
    ui.ctx().request_repaint();
    true
}

fn text_shapes(shape: &Shape, out: &mut Vec<egui::epaint::TextShape>) {
    match shape {
        Shape::Text(text) if text.galley.text() == "Search Games" => out.push(text.clone()),
        Shape::Vec(shapes) => {
            for shape in shapes {
                text_shapes(shape, out);
            }
        }
        _ => {}
    }
}

fn field_rect(index: usize) -> Rect {
    let row = index / 4;
    Rect::from_min_size(
        pos2(24.0 + (index % 4) as f32 * 280.0, 24.0 + row as f32 * 80.0),
        vec2(240.0, HEIGHTS[row]),
    )
}

#[test]
fn singleline_centres_font_ink_at_every_height_and_scale() {
    if !test_support::gpu() {
        return;
    }
    let mut failures = Vec::new();
    for offset in [0.0, -2.0, 2.0] {
        for ppp in SCALES {
            for size in [[1280.0, 720.0], [1920.0, 1080.0]] {
                let mut fresh = true;
                let mut h = Harness::builder()
                    .with_size(size)
                    .with_pixels_per_point(ppp)
                    .wgpu()
                    .build_ui(move |ui| {
                        if install_font(ui, &mut fresh, offset) {
                            return;
                        }
                        ui.painter()
                            .rect_filled(ui.ctx().content_rect(), 0.0, color::ground());
                        for index in 0..16 {
                            let rect = field_rect(index);
                            let mut value = String::new();
                            if index % 2 == 1 {
                                value = "Search Games".into();
                            }
                            let field = match index % 4 >= 2 {
                                true => TextInput::search(&mut value),
                                false => TextInput::new(&mut value),
                            };
                            ui.put(
                                rect,
                                field
                                    .hint("Search Games")
                                    .width(rect.width())
                                    .height(rect.height()),
                            );
                        }
                    });
                h.run();
                let image = h.render().unwrap();
                if size[0] == 1280.0
                    && let Ok(dir) = std::env::var("JUI_FEEDBACK_SCREENSHOTS")
                {
                    let name = format!("18-font-{offset}-{ppp}x.png");
                    image.save(std::path::Path::new(&dir).join(name)).unwrap();
                }
                let mut texts = Vec::new();
                for shape in &h.output().shapes {
                    text_shapes(&shape.shape, &mut texts);
                }
                assert_eq!(texts.len(), 16);
                for (index, text) in texts.iter().enumerate() {
                    let rect = field_rect(index);
                    let bounds = text.visual_bounding_rect();
                    let geometry = (bounds.center().y - rect.center().y).abs() * ppp;
                    let mut top = u32::MAX;
                    let mut bottom = 0;
                    for y in ((rect.top() + 2.0) * ppp).round() as u32
                        ..((rect.bottom() - 2.0) * ppp).round() as u32
                    {
                        for x in (bounds.left() * ppp).floor() as u32
                            ..(bounds.right() * ppp).ceil() as u32
                        {
                            if image.get_pixel(x, y)[0] > 80 {
                                top = top.min(y);
                                bottom = bottom.max(y);
                            }
                        }
                    }
                    assert_ne!(top, u32::MAX, "visible glyphs");
                    let raster = ((top + bottom + 1) as f32 / 2.0 - rect.center().y * ppp).abs();
                    let tolerance = if offset == 0.0 { 1.0 } else { 0.5 };
                    println!(
                        "{size:?} {ppp}x offset={offset} field={index}: geometry={geometry:.3}px raster={raster:.3}px"
                    );
                    if geometry > tolerance || raster > tolerance {
                        failures.push(format!("{size:?} {ppp}x offset={offset} field={index}: geometry={geometry:.3}px raster={raster:.3}px"));
                    }
                    assert!(bounds.left() >= rect.left() + 8.0);
                    assert!(bounds.right() <= rect.right() - 8.0);
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    println!(
        "PASSED font ink: placeholder/value/plain/search, heights 24/32/36/48, 1280x720/1920x1080, DPI 1/1.25/1.5; shifted fonts <=0.5px, Inter <=1px"
    );
}

fn visible_text(h: &Harness<'_, String>) -> egui::epaint::TextShape {
    let mut texts = Vec::new();
    for shape in &h.output().shapes {
        text_shapes(&shape.shape, &mut texts);
    }
    assert_eq!(texts.len(), 1);
    texts.pop().unwrap()
}

fn assert_caret(h: &Harness<'_, String>) {
    let text = visible_text(h);
    let field = h.get_by_role(egui::accesskit::Role::TextInput).rect();
    let ink_offset = (text.visual_bounding_rect().center().y - field.center().y).abs();
    assert!(
        ink_offset * h.ctx.pixels_per_point() <= 0.5,
        "focused glyph centre"
    );
    let row = &text.galley.rows[0];
    let ime = h
        .output()
        .platform_output
        .ime
        .as_ref()
        .expect("IME position");
    let origin = text.pos + row.pos.to_vec2();
    let height = h
        .ctx
        .fonts_mut(|fonts| fonts.row_height(&theme::body()))
        .max(row.size.y);
    assert!((ime.cursor_rect.top() - (origin.y - 1.5)).abs() < 0.01);
    assert!(
        (ime.cursor_rect.bottom() - (origin.y + height + 1.5)).abs() < 0.01,
        "ime={ime:?}, origin={origin:?}, row={:?}",
        row.size
    );
    let caret = h
        .output()
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            Shape::LineSegment { points, .. } if points[0].x == points[1].x => Some(points),
            _ => None,
        })
        .expect("painted caret");
    assert!((caret[0].y - ime.cursor_rect.top()).abs() < 0.01);
    assert!((caret[1].y - ime.cursor_rect.bottom()).abs() < 0.01);
    assert!((caret[0].x - ime.cursor_rect.center().x).abs() < 0.01);
}

#[test]
fn caret_selection_and_ime_follow_the_editor_origin() {
    if !test_support::gpu() {
        return;
    }
    for ppp in SCALES {
        for height in HEIGHTS {
            for search in [false, true] {
                let mut fresh = true;
                let rect = Rect::from_min_size(pos2(24.0, 24.0), vec2(240.0, height));
                let mut h = Harness::builder()
                    .with_size([320.0, 160.0])
                    .with_pixels_per_point(ppp)
                    .wgpu()
                    .build_ui_state(
                        move |ui, value| {
                            if install_font(ui, &mut fresh, -2.0) {
                                return;
                            }
                            let field = match search {
                                true => TextInput::search(value),
                                false => TextInput::new(value),
                            };
                            ui.put(
                                rect,
                                field
                                    .hint("Search Games")
                                    .width(rect.width())
                                    .height(height),
                            );
                        },
                        String::new(),
                    );
                h.run();
                h.get_by_role(egui::accesskit::Role::TextInput).focus();
                h.run();
                assert_caret(&h);
                let empty_origin = visible_text(&h).pos.y;
                h.event(egui::Event::Text("Search Games".into()));
                h.run();
                assert_caret(&h);
                assert_eq!(
                    visible_text(&h).pos.y,
                    empty_origin,
                    "stable baseline while typing"
                );
                h.get_by_role(egui::accesskit::Role::TextInput).click();
                h.run();
                assert_caret(&h);
                h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
                h.run();
                assert_caret(&h);
                let text = visible_text(&h);
                let row = &text.galley.rows[0];
                let selection_color = h.ctx.style_of(egui::Theme::Dark).visuals.selection.bg_fill;
                let vertices: Vec<_> = row
                    .visuals
                    .mesh
                    .vertices
                    .iter()
                    .filter(|vertex| vertex.color == selection_color)
                    .map(|vertex| vertex.pos)
                    .collect();
                assert_eq!(vertices.len(), 4, "selection rectangle");
                let selection =
                    Rect::from_points(&vertices).translate(text.pos.to_vec2() + row.pos.to_vec2());
                assert!((selection.top() - text.pos.y).abs() < 0.01);
                assert!((selection.bottom() - (text.pos.y + row.size.y)).abs() < 0.01);
                let image = h.render().unwrap();
                if height == 36.0
                    && search
                    && let Ok(dir) = std::env::var("JUI_FEEDBACK_SCREENSHOTS")
                {
                    image
                        .save(std::path::Path::new(&dir).join(format!("18-selection-{ppp}x.png")))
                        .unwrap();
                }
                h.event(egui::Event::Ime(egui::ImeEvent::Preedit {
                    text: "Search Games".into(),
                    active_range_chars: Some(0..12),
                }));
                h.run();
                let ime = h.output().platform_output.ime.as_ref().unwrap();
                assert!((ime.cursor_rect.top() - (empty_origin - 1.5)).abs() < 0.01);
                h.event(egui::Event::Ime(egui::ImeEvent::Commit(
                    "Search Games".into(),
                )));
                h.run();
                assert_eq!(h.state(), "Search Games");
                assert_caret(&h);
            }
        }
    }
    println!(
        "PASSED caret/selection/IME: empty/typed/selected/composing, plain/search, heights 24/32/36/48, DPI 1/1.25/1.5; one stable editor origin"
    );
}
