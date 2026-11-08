use super::*;
use std::time::Duration;

fn sample(ctx: &Context, time: f64, target: f32) -> (f32, Duration) {
    let mut current = 0.0;
    let mut output = ctx.run_ui(
        egui::RawInput {
            time: Some(time),
            ..Default::default()
        },
        |ui| {
            current = value(ui.ctx(), Id::new("test-motion"), target);
        },
    );
    output.textures_delta.clear();
    (
        current,
        output.viewport_output[&egui::ViewportId::ROOT].repaint_delay,
    )
}

#[test]
fn ease_out_in_both_directions_and_no_repaints_after_settling() {
    let ctx = Context::default();
    set_enabled(&ctx, true);
    assert_eq!(sample(&ctx, 0.0, 10.0).0, 10.0);
    assert_eq!(sample(&ctx, 1.0, 20.0).0, 10.0);
    let middle = sample(&ctx, 1.08, 20.0).0;
    assert!(middle > 18.0 && middle < 20.0);
    assert_eq!(sample(&ctx, 1.2, 20.0).0, 20.0);
    assert_eq!(sample(&ctx, 2.0, 10.0).0, 20.0);
    let middle = sample(&ctx, 2.08, 10.0).0;
    assert!(middle < 12.0 && middle > 10.0);
    for frame in 0..120 {
        let (current, repaint) = sample(&ctx, 3.0 + frame as f64 / 60.0, 10.0);
        assert_eq!(current, 10.0);
        if frame > 2 {
            assert_eq!(
                repaint,
                Duration::MAX,
                "settled motion must not wake the app"
            );
        }
    }
}

#[test]
fn disabling_mid_transition_settles_every_target_and_stops_repaints() {
    let ctx = Context::default();
    set_enabled(&ctx, true);
    sample(&ctx, 0.0, 0.0);
    sample(&ctx, 1.0, 1.0);
    assert!(sample(&ctx, 1.04, 1.0).0 < 1.0);
    set_enabled(&ctx, false);
    for frame in 0..10 {
        let target = if frame % 2 == 0 { 0.0 } else { 1.0 };
        let (current, repaint) = sample(&ctx, 2.0 + frame as f64 / 60.0, target);
        assert_eq!(current, target);
        if frame > 2 {
            assert_eq!(repaint, Duration::MAX);
        }
    }
}

#[test]
fn entrance_starts_after_idle_and_finishes_in_the_shared_duration() {
    let ctx = Context::default();
    set_enabled(&ctx, true);
    let mut opacity = 1.0;
    let mut output = ctx.run_ui(
        egui::RawInput {
            time: Some(30.0),
            ..Default::default()
        },
        |ui| {
            opacity = enter(ui.ctx(), Id::new("entrance"));
        },
    );
    output.textures_delta.clear();
    assert_eq!(opacity, 0.0);
    let mut output = ctx.run_ui(
        egui::RawInput {
            time: Some(30.2),
            ..Default::default()
        },
        |ui| {
            opacity = enter(ui.ctx(), Id::new("entrance"));
        },
    );
    output.textures_delta.clear();
    assert_eq!(opacity, 1.0);
}
