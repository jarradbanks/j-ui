//! Quiet Centre geometry, overflow and accessible interaction independent of GPU snapshots.
use egui::{Key, Rect, Sense, UiBuilder, pos2, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use j_ui::empty_state::EmptyState;
use j_ui::test_support::install_theme;
use j_ui::theme::icon;
use j_ui::tokens::font;

struct State {
    body: Rect,
    content: Rect,
    action: Rect,
    clicks: usize,
    background_clicked: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            body: Rect::NOTHING,
            content: Rect::NOTHING,
            action: Rect::NOTHING,
            clicks: 0,
            background_clicked: false,
        }
    }
}

fn harness(size: [f32; 2], action: &'static str, ppp: f32) -> Harness<'static, State> {
    let mut fresh = true;
    Harness::builder()
        .with_size(size)
        .with_pixels_per_point(ppp)
        .build_ui_state(
            move |ui, state: &mut State| {
                if install_theme(ui, &mut fresh) {
                    return;
                }
                // A real body between a header and a pinned footer.
                let outer = ui.available_rect_before_wrap();
                let body =
                    Rect::from_min_max(outer.min + vec2(0.0, 32.0), outer.max - vec2(0.0, 32.0));
                state.body = body;
                state.background_clicked |= ui
                    .interact(
                        body,
                        ui.id().with("drop_body"),
                        Sense::click_and_drag() - Sense::FOCUSABLE,
                    )
                    .clicked();
                ui.scope_builder(UiBuilder::new().max_rect(body), |ui| {
                    let r = EmptyState::new(
                        icon::FOLDER,
                        "No Media Yet",
                        "Import video, audio, or images to start editing.",
                    )
                    .primary_action(action)
                    .show(ui);
                    state.content = r.content_rect;
                    let r = r.action.unwrap();
                    state.action = r.rect;
                    state.clicks += usize::from(r.clicked());
                });
            },
            State::default(),
        )
}

#[test]
fn centres_the_whole_group_below_header_and_above_footer_at_240px() {
    for (size, ppp) in [[240.0, 360.0], [440.0, 560.0]]
        .into_iter()
        .flat_map(|size| [1.0, 1.5].map(|ppp| (size, ppp)))
    {
        let mut h = harness(size, "Import Media…", ppp);
        h.run();
        let s = h.state();
        assert!(s.body.contains_rect(s.content));
        assert!(
            (s.body.center() - s.content.center()).length() < 1.0,
            "{s:?}",
            s = (s.body, s.content)
        );
        let heading = h.get_by_label("No Media Yet").rect();
        let help = h
            .get_by_label("Import video, audio, or images to start editing.")
            .rect();
        assert!(heading.bottom() < help.top());
        assert!(help.bottom() < s.action.top());
        assert!(s.body.contains_rect(help));
        assert!((heading.center().x - s.body.center().x).abs() < 1.0);
        assert!((help.center().x - s.body.center().x).abs() < 1.0);
        h.get_by_label("Import Media…").click();
        h.run();
        assert_eq!(h.state().clicks, 1);
        assert!(!h.state().background_clicked);
        // The text must not steal click/drag handling from an underlying editor/drop body.
        let at = h.get_by_label("No Media Yet").rect().center();
        h.drag_at(at);
        h.run();
        h.drop_at(at);
        h.run();
        assert!(h.state().background_clicked);
    }
}

#[test]
fn short_body_scrolls_to_a_keyboard_action() {
    let mut h = harness([240.0, 150.0], "Import Media…", 1.0);
    h.run();
    assert!(h.state().content.height() > h.state().body.height());
    h.key_press(Key::Tab);
    h.run();
    h.key_press(Key::Enter);
    h.run();
    assert_eq!(
        h.state().clicks,
        1,
        "action must be reachable when initially below the viewport"
    );
}

#[test]
fn long_localized_action_can_scroll_without_expanding_the_panel() {
    let action = "Ausgewählte Mediendateien in das aktuelle Projekt importieren…";
    let mut h = harness([240.0, 150.0], action, 1.0);
    h.run();
    assert!(h.state().action.width() > h.state().body.width());
    assert!(h.state().body.width() <= 240.0);
    let heading = h.get_by_label("No Media Yet").rect();
    assert!((heading.center().x - h.state().body.center().x).abs() < 1.0);
    h.key_press(Key::Tab);
    h.run();
    h.key_press(Key::Enter);
    h.run();
    assert_eq!(h.state().clicks, 1);
}

#[test]
fn unbounded_parent_uses_the_finite_visible_body() {
    let mut fresh = true;
    let mut h = Harness::builder()
        .with_size([240.0, 180.0])
        .build_ui(move |ui| {
            if install_theme(ui, &mut fresh) {
                return;
            }
            let clip = ui.clip_rect();
            let rect = Rect::from_min_max(clip.min, pos2(f32::INFINITY, f32::INFINITY));
            ui.scope_builder(UiBuilder::new().max_rect(rect), |ui| {
                let r = EmptyState::new(
                    icon::POINTER,
                    "Select a Clip",
                    "Select a clip in the timeline to edit its properties.",
                )
                .show(ui);
                assert!(r.content_rect.is_finite());
                assert!(clip.contains_rect(r.content_rect));
                assert!((r.content_rect.center() - clip.center()).length() < 1.0);
            });
        });
    h.run();
    assert!(h.query_all_by_label("Select a Clip").next().is_some());
}

#[test]
fn readable_theme_inks_and_type_sizes_keep_the_viewer_neutral() {
    use j_ui::themes;
    for palette in [themes::DARK, themes::LIGHT, themes::HIGH_CONTRAST] {
        for viewer in [false, true] {
            let mut fresh = true;
            let mut h = Harness::builder()
                .with_size([240.0, 280.0])
                .build_ui(move |ui| {
                    if install_theme(ui, &mut fresh) {
                        return;
                    }
                    themes::apply(ui.ctx(), &palette);
                    let empty = EmptyState::new(
                        icon::PLAY,
                        "Nothing to Preview",
                        "Add a video clip to the timeline to see it here.",
                    );
                    if viewer { empty.viewer() } else { empty }.show(ui);
                });
            h.run();
            let palette = if viewer { themes::DARK } else { palette };
            for (label, size, ink) in [
                ("Nothing to Preview", font::TITLE, palette.ink),
                (
                    "Add a video clip to the timeline to see it here.",
                    font::BODY,
                    palette.ink_3,
                ),
            ] {
                let text = h
                    .output()
                    .shapes
                    .iter()
                    .find_map(|s| match &s.shape {
                        egui::Shape::Text(t) if t.galley.job.text == label => Some(t),
                        _ => None,
                    })
                    .expect("visible text shape");
                assert!(
                    text.galley
                        .job
                        .sections
                        .iter()
                        .all(|s| s.format.font_id.size == size && s.format.color == ink)
                );
                assert!(
                    h.query_by_label(label).is_some(),
                    "text is exposed to accessibility"
                );
            }
        }
    }
}

#[test]
fn absent_help_has_no_label_gap_or_height() {
    use j_ui::{test_support, themes};
    if !test_support::gpu() {
        return;
    }
    for (name, palette) in [
        ("dark", themes::DARK),
        ("light", themes::LIGHT),
        ("high-contrast", themes::HIGH_CONTRAST),
    ] {
        for ppp in [1.0, 1.5] {
            for action in [false, true] {
                let mut fresh = true;
                let mut h = Harness::builder()
                    .with_size([440.0, 360.0])
                    .with_pixels_per_point(ppp)
                    .wgpu()
                    .build_ui_state(
                        move |ui, state: &mut State| {
                            if install_theme(ui, &mut fresh) {
                                if let Some(directory) = std::env::var_os("EMPTY_STATE_FONT_DIR") {
                                    let mut fonts = j_ui::theme::fonts("en");
                                    for (key, file) in [
                                        ("figtree", "Onest-Regular.ttf"),
                                        ("figtree-medium", "Onest-Medium.ttf"),
                                        ("figtree-semibold", "Onest-SemiBold.ttf"),
                                    ] {
                                        let bytes = std::fs::read(
                                            std::path::Path::new(&directory).join(file),
                                        )
                                        .unwrap();
                                        fonts.font_data.insert(
                                            key.into(),
                                            egui::FontData::from_owned(bytes).into(),
                                        );
                                    }
                                    ui.ctx().set_fonts(fonts);
                                }
                                return;
                            }
                            themes::apply(ui.ctx(), &palette);
                            state.body = ui.available_rect_before_wrap().intersect(ui.clip_rect());
                            let empty = EmptyState::new(icon::FOLDER, "No Media Yet", "");
                            let empty = if action {
                                empty.action("Import Media…")
                            } else {
                                empty
                            };
                            let response = empty.show(ui);
                            state.content = response.content_rect;
                            state.action = response.action.map_or(Rect::NOTHING, |r| r.rect);
                        },
                        State::default(),
                    );
                h.run();
                let title = h.get_by_label("No Media Yet").rect();
                let bottom = if action {
                    h.state().action.bottom()
                } else {
                    title.bottom()
                };
                let expected_height =
                    32.0 + 12.0 + title.height() + if action { 12.0 + 36.0 } else { 0.0 };
                let expected_top = h.state().body.center().y - expected_height / 2.0;
                let image = h.render().unwrap();
                if let Some(output) = std::env::var_os("EMPTY_STATE_EVIDENCE") {
                    image
                        .save(
                            std::path::Path::new(&output)
                                .join(format!("{name}-{ppp}-{action}.png")),
                        )
                        .unwrap();
                }
                let body = h.state().body;
                let background =
                    image.get_pixel((body.left() * ppp) as u32, (body.top() * ppp) as u32);
                let mut raster_top = image.height();
                let mut raster_bottom = 0;
                for (x, y, pixel) in image.enumerate_pixels() {
                    if body
                        .shrink(1.0)
                        .contains(pos2(x as f32 / ppp, y as f32 / ppp))
                        && pixel != background
                    {
                        raster_top = raster_top.min(y);
                        raster_bottom = raster_bottom.max(y + 1);
                    }
                }
                let painted = h
                    .output()
                    .shapes
                    .iter()
                    .filter_map(|shape| {
                        let egui::Shape::Text(text) = &shape.shape else {
                            return None;
                        };
                        Some(text.visual_bounding_rect())
                    })
                    .fold(h.state().action, Rect::union);
                assert!(
                    raster_top < raster_bottom,
                    "render contains visible guidance"
                );
                assert!(
                    (raster_top as f32 - painted.top() * ppp).abs() <= 1.5,
                    "raster {raster_top} vs {painted:?} at {ppp}"
                );
                assert!((raster_bottom as f32 - painted.bottom() * ppp).abs() <= 1.5);
                let icon = h
                    .output()
                    .shapes
                    .iter()
                    .find_map(|shape| {
                        let egui::Shape::Text(text) = &shape.shape else {
                            return None;
                        };
                        (text.galley.text() == icon::FOLDER).then(|| text.visual_bounding_rect())
                    })
                    .unwrap();
                let visible = icon.union(title).union(h.state().action);
                assert!(
                    (visible.center().y - h.state().body.center().y).abs() < 2.0,
                    "{name} {ppp} action={action}: {visible:?} vs {:?}",
                    h.state().body
                );
                assert!((title.top() - (expected_top + 44.0)).abs() <= 1.0);
                assert!((bottom - (expected_top + expected_height)).abs() <= 1.0);
                assert!(
                    h.query_all_by_label("").next().is_none(),
                    "empty help label"
                );
            }
        }
    }
}
