//! Renders the gallery example (the PR screenshot) and checks the radius scale.

#[allow(dead_code)]
#[path = "../examples/gallery.rs"]
mod gallery;

use j_ui::test_support as common;

#[test]
fn gallery_snapshot() {
    if !common::gpu() {
        return;
    }
    let mut state = gallery::Gallery::new();
    let mut fresh = true;
    let mut harness = egui_kittest::Harness::builder()
        .with_size([1280.0, 860.0])
        .wgpu()
        .build_ui(|ui| {
            if !common::install_theme(ui, &mut fresh) {
                gallery::gallery(ui, &mut state);
            }
        });
    // The loading spinners never settle, so step a fixed number of frames.
    harness.run_steps(4);
    harness.snapshot("gallery");
}

#[test]
fn gallery_catalogs_match() {
    assert_eq!(gallery::CATALOG.mismatches(), Vec::<String>::new());
}

#[test]
fn corners_come_from_the_scale() {
    let sources = [
        include_str!("../src/panel.rs"),
        include_str!("../src/theme.rs"),
        include_str!("../src/widgets.rs"),
    ];
    for source in sources {
        for call in source.split("CornerRadius::same(").skip(1) {
            assert!(
                call.starts_with("radius::") || call.starts_with("corner"),
                "corner not from the scale: {}",
                &call[..30]
            );
        }
    }
}

#[test]
fn component_gallery_is_interactive_at_both_window_sizes_and_scales() {
    use egui_kittest::kittest::{NodeT, Queryable};
    if !common::gpu() {
        return;
    }
    let mut results = egui_kittest::SnapshotResults::new();
    for viewport in [[1280.0, 800.0], [1920.0, 1080.0]] {
        for ppp in [1.0, 1.5] {
            let mut state = gallery::Gallery::new();
            let mut fresh = true;
            let mut harness = egui_kittest::Harness::builder()
                .with_size(viewport)
                .with_pixels_per_point(ppp)
                .wgpu()
                .build_ui(|ui| {
                    if !common::install_theme(ui, &mut fresh) {
                        gallery::gallery(ui, &mut state);
                    }
                });
            harness.run_steps(4);
            let plus = harness.get_by_label("+1s").rect();
            for shape in &harness.output().shapes {
                let egui::Shape::Text(text) = &shape.shape else {
                    continue;
                };
                if text.galley.text() == "+1s" {
                    assert!(
                        shape.clip_rect.contains_rect(plus),
                        "{viewport:?} {ppp}x Inspector action clipped: {plus:?}"
                    );
                }
            }
            harness.get_by_label("+1s").click();
            harness.run_steps(2);
            assert!(harness.query_all_by_label("00:01:13:08").next().is_some());
            harness.get_by_label("Components").click();
            harness.run();
            harness
                .query_all_by_label("Export")
                .find(|node| node.accesskit_node().role() == egui::accesskit::Role::Button)
                .unwrap()
                .click();
            harness.run();
            assert!(harness.get_by_label("Dismiss").rect().bottom() < viewport[1]);
            results.add(harness.try_snapshot(format!("gallery_components_{}_{ppp}x", viewport[0])));
            harness.get_by_label("Dismiss").click();
            harness.run();
            assert!(harness.query_by_label("Dismiss").is_none());
        }
    }
    results.unwrap();
    println!("PASSED gallery: live components and toast, 1280x800/1920x1080, DPI 1/1.5");
}

#[test]
fn gallery_disabled_and_loading_columns_are_reachable_without_sideways_clipping() {
    use egui_kittest::kittest::Queryable;
    if !common::gpu() {
        return;
    }
    let mut results = egui_kittest::SnapshotResults::new();
    for viewport in [[1280.0, 860.0], [1920.0, 1080.0]] {
        for ppp in [1.0, 1.5] {
            let mut state = gallery::Gallery::new();
            let mut fresh = true;
            let mut h = egui_kittest::Harness::builder()
                .with_size(viewport)
                .with_pixels_per_point(ppp)
                .wgpu()
                .build_ui(|ui| {
                    if !common::install_theme(ui, &mut fresh) {
                        gallery::gallery(ui, &mut state);
                    }
                });
            h.run_steps(4);
            h.get_by_label("Loading / Error").scroll_to_me();
            h.run_steps(12);
            for label in ["Disabled", "Loading / Error"] {
                let rect = h.get_by_label(label).rect();
                assert!(
                    rect.top() >= 80.0 && rect.bottom() < viewport[1] - 32.0,
                    "{label}: {rect:?}"
                );
            }
            for clipped in &h.output().shapes {
                let egui::Shape::Text(text) = &clipped.shape else {
                    continue;
                };
                if text.galley.text().is_empty() {
                    continue;
                }
                let ink = text.visual_bounding_rect();
                if ink.top() < clipped.clip_rect.top() || ink.bottom() > clipped.clip_rect.bottom()
                {
                    continue;
                }
                assert!(
                    ink.left() >= clipped.clip_rect.left() - 1.0
                        && ink.right() <= clipped.clip_rect.right() + 1.0,
                    "horizontal clipping: {} at {ink:?}",
                    text.galley.text()
                );
            }
            results.add(h.try_snapshot(format!("gallery_states_{}_{}x", viewport[0], ppp)));
        }
    }
    results.unwrap();
    println!(
        "PASSED gallery: Disabled and Loading / Error remain reachable with no horizontal ink clipping; 1280x860/1920x1080 at 100%/150%"
    );
}
