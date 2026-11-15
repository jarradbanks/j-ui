use super::*;
use egui::plugin::Plugin;

fn frame(ctx: &Context, show: bool) -> FullOutput {
    let input = RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 800.0),
        )),
        ..Default::default()
    };
    let mut output = ctx.run_ui(input, |ui| {
        ui.push_id("stable", |ui| {
            if show {
                let response = ui.button("Apply");
                crate::target(ui, &response, "demo/apply", "Apply");
            }
            let mut enabled = false;
            let enabled_id = ui.make_persistent_id("enabled");
            ui.scope_builder(egui::UiBuilder::new().id(enabled_id), |ui| {
                ui.checkbox(&mut enabled, "Enabled")
            });
            ui.text_edit_singleline(&mut String::new());
        });
    });
    output.textures_delta.clear();
    output
}

#[test]
fn plugin_is_idempotent_discovers_nodes_and_removes_dynamic_targets() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("connection.json");
    let ctx = Context::default();
    install(&ctx, "test", &path).unwrap();
    install(&ctx, "test", &path).unwrap();
    assert!(install(&ctx, "test", &directory.path().join("another.json")).is_err());
    frame(&ctx, true);
    frame(&ctx, true);
    let targets = ctx
        .with_plugin::<Bridge, _>(|bridge| bridge.targets.clone())
        .unwrap();
    let apply = targets
        .iter()
        .find(|target| target.path == "demo/apply")
        .unwrap();
    assert_eq!(apply.role, "Button");
    assert!(apply.source.as_ref().unwrap().contains("bridge_tests.rs:"));
    let checkbox = targets
        .iter()
        .find(|target| target.label == "Enabled")
        .unwrap();
    assert_eq!(checkbox.role, "CheckBox");
    assert!(checkbox.path.starts_with("egui/") && checkbox.source.is_none());
    assert!(
        targets
            .iter()
            .any(|target| target.role == "TextInput" && target.label.is_empty())
    );
    frame(&ctx, false);
    let targets = ctx
        .with_plugin::<Bridge, _>(|bridge| bridge.targets.clone())
        .unwrap();
    assert!(!targets.iter().any(|target| target.path == "demo/apply"));
    assert_eq!(
        targets
            .iter()
            .find(|target| target.label == "Enabled")
            .unwrap()
            .id,
        checkbox.id
    );
    drop(ctx);
    assert!(
        !path.exists(),
        "dropping the context must stop the server and remove its token file"
    );
}

#[test]
fn explicit_targets_keep_caller_clipping_and_parent_targets_stay_below_children() {
    let directory = tempfile::tempdir().unwrap();
    let ctx = Context::default();
    install(&ctx, "clipping", &directory.path().join("connection.json")).unwrap();
    let mut output = ctx.run_ui(
        RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(120.0, 100.0),
            )),
            ..Default::default()
        },
        |ui| {
            let scope = ui.scope(|ui| {
                ui.set_clip_rect(egui::Rect::from_min_max(
                    egui::pos2(30.0, 10.0),
                    egui::pos2(100.0, 80.0),
                ));
                let mut response = ui.add_sized([180.0, 90.0], egui::Button::new("Clip"));
                response.rect =
                    egui::Rect::from_min_max(egui::pos2(20.0, 5.0), egui::pos2(130.0, 75.0));
                response.interact_rect =
                    egui::Rect::from_min_max(egui::pos2(15.0, 20.0), egui::pos2(110.0, 90.0));
                crate::target(ui, &response, "timeline/clip", "Clip");
                let mut hidden = ui.button("Hidden");
                hidden.rect = egui::Rect::NOTHING;
                crate::target(ui, &hidden, "timeline/hidden", "Hidden");
            });
            crate::target(ui, &scope.response, "timeline/lane", "Lane");
        },
    );
    output.textures_delta.clear();
    let targets = ctx
        .with_plugin::<Bridge, _>(|bridge| bridge.targets.clone())
        .unwrap();
    let clip = targets
        .iter()
        .find(|target| target.path == "timeline/clip")
        .unwrap();
    for (actual, expected) in clip
        .rect
        .into_iter()
        .zip([30.0 / 120.0, 0.2, 70.0 / 120.0, 0.55])
    {
        assert!(
            (actual - expected).abs() < 0.0001,
            "caller clipping must override canonical response geometry"
        );
    }
    assert_eq!(clip.role, "Button");
    assert!(
        !targets
            .iter()
            .any(|target| target.path == "timeline/hidden" || target.label == "Hidden")
    );
    let lane = targets
        .iter()
        .find(|target| target.path == "timeline/lane")
        .unwrap();
    assert!(
        lane.z_order < clip.z_order,
        "a registered parent must not cover its semantic children"
    );
}

#[test]
fn screenshots_use_metadata_from_the_final_layout_pass() {
    let directory = tempfile::tempdir().unwrap();
    let ctx = Context::default();
    install(&ctx, "passes", &directory.path().join("connection.json")).unwrap();
    let mut expected = None;
    let mut output = ctx.run_ui(
        RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1280.0, 800.0),
            )),
            ..Default::default()
        },
        |ui| {
            let first_pass = ui.ctx().current_pass_index() == 0;
            if !first_pass {
                ui.add_space(80.0);
            }
            let response = ui.button("Apply");
            crate::target(ui, &response, "passes/apply", "Apply");
            if first_pass {
                ui.ctx().request_discard("measure layout");
            } else {
                expected = Some(response.rect.top() / 800.0);
            }
        },
    );
    output.textures_delta.clear();
    ctx.with_plugin::<Bridge, _>(|bridge| {
        let pending = bridge.pending.as_ref().unwrap();
        let target = pending
            .targets
            .iter()
            .find(|target| target.path == "passes/apply")
            .unwrap();
        assert_eq!(
            target.rect[1],
            expected.unwrap(),
            "discarded layout metadata must not be paired with the final painted frame"
        );
    });
}

#[test]
fn screenshots_keep_matching_metadata_and_leave_app_screenshots_alone() {
    let directory = tempfile::tempdir().unwrap();
    let ctx = Context::default();
    install(&ctx, "test", &directory.path().join("connection.json")).unwrap();
    let output = frame(&ctx, true);
    let token = output.viewport_output[&ViewportId::ROOT]
        .commands
        .iter()
        .find_map(|command| {
            let egui::ViewportCommand::Screenshot(token) = command else {
                return None;
            };
            Some(token.clone())
        })
        .unwrap();
    let image = Arc::new(egui::ColorImage::filled([1280, 800], egui::Color32::BLACK));
    let mut input = RawInput::default();
    input.events.push(Event::Screenshot {
        viewport_id: ViewportId::ROOT,
        user_data: egui::UserData::new("app screenshot"),
        image: image.clone(),
    });
    ctx.with_plugin::<Bridge, _>(|bridge| {
        bridge.receive_capture(&input);
        assert!(bridge.pending.is_some());
        input.events.push(Event::Screenshot {
            viewport_id: ViewportId::ROOT,
            user_data: token,
            image,
        });
        bridge.receive_capture(&input);
        assert!(bridge.pending.is_none());
        assert_eq!(input.events.len(), 2);
    });
}

#[test]
fn queued_input_is_fenced_after_layout_changes_delays_and_disconnect() {
    let directory = tempfile::tempdir().unwrap();
    let ctx = Context::default();
    install(&ctx, "test", &directory.path().join("connection.json")).unwrap();
    frame(&ctx, true);
    ctx.with_plugin::<Bridge, _>(|bridge| {
        let (send, receive) = mpsc::channel();
        bridge.incoming = Mutex::new(receive);
        let mut input = RawInput {
            screen_rect: Some(bridge.bounds),
            ..Default::default()
        };
        let button = || {
            serde_json::from_str(r#"{"kind":"button","x":0.5,"y":0.5,"button":0,"pressed":true}"#)
                .unwrap()
        };
        bridge.held.event(button(), bridge.bounds, &mut input);
        bridge.held.focused = true;
        input.events.clear();
        send.send(Batch {
            generation: bridge.generation - 1,
            accepted: Instant::now(),
            width: 1280.0,
            height: 800.0,
            events: vec![button()],
        })
        .unwrap();
        bridge.input_hook(&ctx, &mut input);
        assert!(
            input
                .events
                .iter()
                .any(|event| matches!(event, Event::PointerButton { pressed: false, .. }))
        );
        assert!(
            !input
                .events
                .iter()
                .any(|event| matches!(event, Event::PointerButton { pressed: true, .. }))
        );
        input.events.clear();
        send.send(Batch {
            generation: bridge.generation,
            accepted: Instant::now() - Duration::from_secs(1),
            width: 1280.0,
            height: 800.0,
            events: vec![button()],
        })
        .unwrap();
        bridge.input_hook(&ctx, &mut input);
        assert!(
            !input
                .events
                .iter()
                .any(|event| matches!(event, Event::PointerButton { pressed: true, .. }))
        );
        bridge.held.event(button(), bridge.bounds, &mut input);
        bridge.held.focused = true;
        bridge.shared.lock().unwrap().last_input = Instant::now() - Duration::from_secs(3);
        input.events.clear();
        bridge.input_hook(&ctx, &mut input);
        assert!(!bridge.held.focused);
        assert!(
            input
                .events
                .iter()
                .any(|event| matches!(event, Event::PointerButton { pressed: false, .. }))
        );
    });
}

#[test]
fn rendered_nodes_match_clipped_geometry_across_themes_sizes_and_dpi() {
    if !j_ui::test_support::gpu() {
        return;
    }
    for palette in [
        j_ui::themes::DARK,
        j_ui::themes::LIGHT,
        j_ui::themes::HIGH_CONTRAST,
    ] {
        for size in [[1280.0, 800.0], [1920.0, 1080.0]] {
            for ppp in [1.0, 1.5] {
                let directory = tempfile::tempdir().unwrap();
                let path = directory.path().join("connection.json");
                let mut context = None;
                let mut fresh = true;
                let mut harness = egui_kittest::Harness::builder()
                    .with_size(size)
                    .with_pixels_per_point(ppp)
                    .wgpu()
                    .build_ui(|ui| {
                        context = Some(ui.ctx().clone());
                        if std::mem::take(&mut fresh) {
                            j_ui::theme::install(ui.ctx());
                            j_ui::themes::apply(ui.ctx(), &palette);
                            install(ui.ctx(), "geometry", &path).unwrap();
                            return;
                        }
                        let clip = egui::Rect::from_min_size(
                            egui::pos2(12.0, 12.0),
                            egui::vec2(100.0, 32.0),
                        );
                        ui.scope_builder(egui::UiBuilder::new().max_rect(clip), |ui| {
                            ui.set_clip_rect(clip);
                            let response = ui.add_sized([180.0, 24.0], egui::Button::new("Apply"));
                            crate::target(ui, &response, "geometry/apply", "Apply");
                        });
                    });
                harness.run_steps(3);
                let image = harness.render().expect("native geometry render");
                assert_eq!(image.width(), (size[0] * ppp) as u32);
                assert_eq!(image.height(), (size[1] * ppp) as u32);
                assert!(image.pixels().any(|pixel| pixel.0[0] > 100));
                drop(harness);
                let ctx = context.unwrap();
                let targets = ctx
                    .with_plugin::<Bridge, _>(|bridge| bridge.targets.clone())
                    .unwrap();
                let target = targets
                    .iter()
                    .find(|target| target.path == "geometry/apply")
                    .unwrap();
                assert!((target.rect[0] * size[0] - 12.0).abs() < 1.0);
                assert!((target.rect[2] * size[0] - 100.0).abs() < 1.0);
                assert!(target.rect[3] * size[1] <= 32.0);
            }
        }
    }
    eprintln!(
        "PASSED native bridge geometry: three themes, 1280x800/1920x1080, DPI 1.0/1.5, clipped semantic targets and rendered pixels"
    );
}
