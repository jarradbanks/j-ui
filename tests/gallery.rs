//! The running gallery page, including scrolling, reference geometry and live controls.

#[allow(dead_code)]
#[path = "../examples/gallery.rs"]
mod gallery;

use egui_kittest::{Harness, SnapshotResults, kittest::Queryable};
use j_ui::test_support as common;

fn harness(
    width: f32,
    height: f32,
    scale: f32,
    theme: usize,
    gpu: bool,
) -> Harness<'static, gallery::Gallery> {
    let mut fresh = true;
    let mut state = gallery::Gallery::new();
    state.theme = theme;
    let builder = Harness::builder()
        .with_size([width, height])
        .with_pixels_per_point(scale);
    let builder = if gpu { builder.wgpu() } else { builder };
    let mut harness = builder.build_ui_state(
        move |ui, state| {
            if !common::install_theme(ui, &mut fresh) {
                gallery::gallery(ui, state);
            }
        },
        state,
    );
    harness.run_steps(6);
    harness
}

#[test]
fn gallery_snapshots() {
    if !common::gpu() {
        return;
    }
    let mut results = SnapshotResults::new();
    for (theme, name) in [(0, "dark"), (1, "light")] {
        for scale in [1.0, 1.5] {
            for (width, height) in [(1280.0, 4100.0), (1920.0, 2400.0)] {
                let mut h = harness(width, height, scale, theme, true);
                let name = format!("gallery_{name}_{width}_{scale}x");
                results.add(h.try_snapshot(&name));
                if let Ok(dir) = std::env::var("GALLERY_EVIDENCE") {
                    std::fs::create_dir_all(&dir).unwrap();
                    h.render()
                        .unwrap()
                        .save(format!("{dir}/{name}.png"))
                        .unwrap();
                }
            }
        }
    }
    results.unwrap();
}

#[test]
fn gallery_grid_tracks_the_reference_at_both_widths_and_scales() {
    for scale in [1.0, 1.5] {
        for width in [1280.0, 1920.0] {
            let h = harness(width, 860.0, scale, 0, false);
            let sections = &h.state().sections;
            assert_eq!(sections.len(), 17);
            let columns = if width == 1280.0 { 2 } else { 3 };
            assert_eq!(sections[0].min, egui::pos2(264.0, 84.0));
            assert_eq!(sections[0].right(), width - 32.0);
            for row in sections[1..16].chunks(columns) {
                for pair in row.windows(2) {
                    assert!((pair[0].height() - pair[1].height()).abs() < 0.1);
                    assert!((pair[1].left() - pair[0].right() - 16.0).abs() < 0.1);
                }
            }
            let expected = if width == 1280.0 {
                [584.59375, 684.59375, 2946.4531]
            } else {
                [345.59375, 445.59375, 1916.9688]
            };
            for (actual, expected) in [sections[0].height(), sections[1].top(), sections[16].top()]
                .into_iter()
                .zip(expected)
            {
                assert!(
                    (actual - expected).abs() < 1.5,
                    "{width}px/{scale}x: {actual} vs {expected}"
                );
            }
            assert!(h.query_by_label("+1s").is_none());
        }
    }
    println!(
        "PASSED gallery polish: 17 reference sections, responsive columns, equal card rows, margins and gutters at 1280/1920 and 1/1.5x"
    );
}

#[test]
fn gallery_controls_and_rail_remain_live_when_scrolled() {
    let mut h = harness(1280.0, 860.0, 1.0, 0, false);
    h.get_by_label("Light").click();
    h.run_steps(4);
    assert_eq!(h.state().theme, 1);
    h.get_by_label("Toast").click();
    h.run_steps(6);
    assert!(h.state().scroll > 1000.0);
    let dismiss = h.get_by_label("Dismiss");
    assert!(dismiss.rect().top() > 56.0 && dismiss.rect().bottom() < 860.0);
    dismiss.click();
    h.run_steps(4);
    assert!(h.query_by_label("Dismiss").is_none());
    h.get_by_label("Show Toast").click();
    h.run_steps(4);
    assert!(h.query_by_label("Dismiss").is_some());
    h.get_by_label("Recent Projects").click();
    h.run_steps(4);
    assert!(h.state().scroll < 30.0);
    h.get_by_label("Current").click();
    h.run_steps(4);
    assert_eq!(h.state().style, 0);
    assert_eq!(h.get_by_label("New Project").rect().height(), 24.0);
    h.get_by_label("Line").click();
    h.run_steps(4);
    assert_eq!(h.state().style, 1);
    assert_eq!(h.get_by_label("New Project").rect().height(), 36.0);
}

#[test]
fn gallery_catalogs_match() {
    assert_eq!(gallery::CATALOG.mismatches(), Vec::<String>::new());
    assert_eq!(gallery::PAGE_CATALOG.mismatches(), Vec::<String>::new());
}

#[test]
fn corners_come_from_the_scale() {
    for source in [
        include_str!("../src/panel.rs"),
        include_str!("../src/theme.rs"),
        include_str!("../src/widgets.rs"),
    ] {
        for call in source.split("CornerRadius::same(").skip(1) {
            assert!(call.starts_with("radius::") || call.starts_with("corner"));
        }
    }
}

#[test]
fn gallery_font_and_style_choices_rebuild_the_page() {
    if !common::gpu() {
        return;
    }
    let mut results = SnapshotResults::new();
    for (style, name, font, theme) in [
        (0, "current", 0, 0),
        (1, "line", 1, 0),
        (1, "line_light", 0, 1),
    ] {
        let mut h = harness(1280.0, 4100.0, 1.0, theme, true);
        h.state_mut().style = style;
        h.state_mut().font = font;
        h.run_steps(8);
        if theme == 1 {
            h.get_by_label("Auto Save").click();
            h.run_steps(12);
        }
        let actual = h.get_by_label("New Project").rect().height();
        assert_eq!(actual, if style == 0 { 24.0 } else { 36.0 });
        assert_eq!(h.state().sections.len(), 17);
        assert_eq!(
            h.state().sections[0].top(),
            if style == 0 { 68.0 } else { 84.0 }
        );
        assert_eq!(j_ui::tokens::size::control(), j_ui::tokens::size::CONTROL);
        results.add(h.try_snapshot(format!("gallery_{name}")));
    }
    results.unwrap();
}

#[test]
fn gallery_ink_is_unclipped_and_project_actions_align() {
    for scale in [1.0, 1.5] {
        for width in [1280.0, 1920.0] {
            let h = harness(width, 4100.0, scale, 0, false);
            for clipped in &h.output().shapes {
                let egui::Shape::Text(text) = &clipped.shape else {
                    continue;
                };
                if text.galley.text().is_empty() {
                    continue;
                }
                let ink = text.visual_bounding_rect();
                assert!(
                    clipped.clip_rect.expand(1.0).contains_rect(ink),
                    "{width}px/{scale}x clipped {}: {ink:?} in {:?}",
                    text.galley.text(),
                    clipped.clip_rect
                );
            }
            let actions: Vec<_> = h
                .query_all_by_label("Open")
                .take(3)
                .map(|node| node.rect())
                .collect();
            assert_eq!(actions.len(), 3);
            assert!((actions[0].top() - actions[1].top()).abs() < 1.0);
            for action in actions {
                assert_eq!(action.height(), 36.0);
            }
        }
    }
    println!(
        "PASSED gallery polish: painted text fits its clip; project footers and control heights align at both widths and scales"
    );
}
