use egui_kittest::{
    Harness,
    kittest::{NodeT, Queryable},
};
use j_ui::{
    test_support as common,
    update::{Action, Prompt},
};
use j_update::{
    network::Download,
    state::{State, Work},
};
#[allow(dead_code)]
#[path = "../examples/update_gallery.rs"]
mod gallery;
struct Model {
    state: State,
    work: Work,
    open: bool,
    automatic: bool,
    installed: bool,
    save: bool,
    action: Option<Action>,
}
fn harness(state: State) -> Harness<'static, Model> {
    let mut fresh = true;
    Harness::builder().with_size([820.0, 640.0]).build_ui_state(
        move |ui, m: &mut Model| {
            if common::install_theme(ui, &mut fresh) {
                return;
            }
            if let Some(a) = (Prompt {
                state: &m.state,
                current_version: "1.0.0",
                installed: m.installed,
                work: m.work,
                save_before_restart: m.save,
                previous: None,
            })
            .show(ui.ctx(), &mut m.open, &mut m.automatic)
            {
                m.action = Some(a);
            }
        },
        Model {
            state,
            work: Work::default(),
            open: true,
            automatic: true,
            installed: true,
            save: false,
            action: None,
        },
    )
}
#[test]
fn automatic_checks_are_optional_and_close_has_a_visible_effect() {
    let mut h = harness(State::Current);
    h.run();
    h.get_by_label("Automatically check for updates at startup")
        .click();
    h.run();
    assert!(!h.state().automatic);
    h.get_by_label("Later").click();
    h.run();
    assert!(!h.state().open);
}
#[test]
fn installation_is_disabled_during_work_and_in_source_builds() {
    for work in [
        Work {
            recording: true,
            ..Default::default()
        },
        Work {
            exporting: true,
            ..Default::default()
        },
        Work {
            other_job: true,
            ..Default::default()
        },
        Work {
            unsaved: true,
            ..Default::default()
        },
    ] {
        let mut h = harness(State::Ready(Download {
            update: gallery::available(),
            dir: tempfile::tempdir().unwrap(),
        }));
        h.state_mut().work = work;
        h.run();
        assert!(
            h.get_by_label("Install and restart")
                .accesskit_node()
                .is_disabled()
        );
    }
    let mut h = harness(State::Ready(Download {
        update: gallery::available(),
        dir: tempfile::tempdir().unwrap(),
    }));
    h.state_mut().installed = false;
    h.run();
    assert!(
        h.get_by_label("Install and restart")
            .accesskit_node()
            .is_disabled()
    );
}
#[test]
fn save_and_restart_requests_a_checkpoint_and_retains_ready_until_app_honors_it() {
    let d = tempfile::tempdir().unwrap();
    let stage = d.path().to_owned();
    let mut h = harness(State::Ready(Download {
        update: gallery::available(),
        dir: d,
    }));
    h.state_mut().work.unsaved = true;
    h.state_mut().save = true;
    h.run();
    h.get_by_label("Save, install and restart").click();
    h.run();
    assert_eq!(h.state().action, Some(Action::Install));
    assert!(matches!(h.state().state, State::Ready(_)) && stage.exists());
    assert!(
        h.get_by_label("Check for Updates")
            .accesskit_node()
            .is_disabled()
    );
    h.get_by_label("Later").click();
    h.run();
    assert!(stage.exists());
    drop(h);
    assert!(!stage.exists());
}
#[test]
fn downloading_verifying_cancel_and_close_never_offer_install() {
    for received in [40, 100] {
        let mut h = harness(State::Downloading {
            update: gallery::available(),
            received,
        });
        h.run_steps(4);
        assert!(h.query_by_label("Install and restart").is_none());
        h.get_by_label(if received == 100 {
            "Verifying download…"
        } else {
            "Downloading update…"
        });
        h.get_by_label("Cancel Download").click();
        h.run_steps(2);
        assert_eq!(h.state().action, Some(Action::Cancel));
        h.get_by_label("Later").click();
        h.run_steps(2);
        assert!(!h.state().open);
        assert_eq!(h.state().action, Some(Action::Cancel));
    }
}
#[test]
fn source_builds_open_releases_and_errors_allow_retry() {
    let mut h = harness(State::Available(gallery::available()));
    h.state_mut().installed = false;
    h.run();
    h.get_by_label("Release Notes and Downloads").click();
    h.run();
    assert_eq!(h.state().action, Some(Action::OpenRelease));
    h.state_mut().state = State::Failed("disk full".into());
    h.run();
    h.get_by_label("Update failed: disk full");
    h.get_by_label("Check for Updates").click();
    h.run();
    assert_eq!(h.state().action, Some(Action::Check));
}
#[test]
fn update_gallery_snapshots() {
    if !common::gpu() {
        return;
    }
    let mut results = egui_kittest::SnapshotResults::new();
    for (name, palette) in j_ui::themes::BUILT_IN {
        let states = gallery::states();
        let mut fresh = true;
        let mut h = Harness::builder()
            .with_size([1120.0, 1300.0])
            .wgpu()
            .build_ui(move |ui| {
                if common::install_theme(ui, &mut fresh) {
                    return;
                }
                j_ui::themes::apply(ui.ctx(), &palette);
                gallery::gallery(ui, &states);
            });
        h.run_steps(5);
        h.snapshot(format!("updates_{name}"));
        results.extend(h.take_snapshot_results());
    }
}
