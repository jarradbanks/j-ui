//! Update prompt states: `cargo run --example update_gallery`.
use eframe::egui;
use j_ui::{
    i18n::{CATALOG, verbatim},
    theme, tr,
    update::Prompt,
};
use j_update::{
    metadata::Asset,
    network::{Available, Download},
    state::{State, Work},
};

pub static GALLERY_CATALOG: j_ui::i18n::Catalog = j_ui::i18n::Catalog::new([
    include_str!("../i18n/gallery/en.toml"),
    include_str!("../i18n/gallery/es.toml"),
    include_str!("../i18n/gallery/fr.toml"),
    include_str!("../i18n/gallery/de.toml"),
    include_str!("../i18n/gallery/it.toml"),
    include_str!("../i18n/gallery/pt-BR.toml"),
    include_str!("../i18n/gallery/nl.toml"),
    include_str!("../i18n/gallery/pl.toml"),
    include_str!("../i18n/gallery/cs.toml"),
    include_str!("../i18n/gallery/tr.toml"),
    include_str!("../i18n/gallery/ru.toml"),
    include_str!("../i18n/gallery/uk.toml"),
    include_str!("../i18n/gallery/sv.toml"),
    include_str!("../i18n/gallery/da.toml"),
    include_str!("../i18n/gallery/nb.toml"),
    include_str!("../i18n/gallery/ca.toml"),
    include_str!("../i18n/gallery/ja.toml"),
    include_str!("../i18n/gallery/zh-Hans.toml"),
    include_str!("../i18n/gallery/ko.toml"),
]);

pub fn available() -> Available {
    Available {
        version: "1.2.0".into(),
        notes: tr!(GALLERY_CATALOG, "update.notes").into(),
        asset: Asset {
            target: "x86_64-pc-windows-msvc".into(),
            distribution: "windows-installer".into(),
            name: "demo-1.2.0-setup.exe".into(),
            size: 100,
            sha256: "ab".repeat(32),
        },
        asset_id: 1,
        tag: "v1.2.0".into(),
        manifest: vec![],
        signature: vec![],
    }
}
pub fn states() -> Vec<State> {
    vec![
        State::Idle,
        State::Checking,
        State::Current,
        State::Available(available()),
        State::Downloading {
            update: available(),
            received: 40,
        },
        State::Downloading {
            update: available(),
            received: 100,
        },
        State::Ready(Download {
            update: available(),
            dir: tempfile::tempdir().unwrap(),
        }),
        State::Failed(tr!(CATALOG, "updates.busy").into()),
    ]
}
pub fn gallery(ui: &mut egui::Ui, states: &[State]) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        egui::Grid::new("updates_gallery")
            .num_columns(2)
            .spacing([16.0, 16.0])
            .show(ui, |ui| {
                for (i, state) in states.iter().enumerate() {
                    ui.push_id(i, |ui| {
                        egui::Frame::group(ui.style())
                            .inner_margin(16)
                            .show(ui, |ui| {
                                ui.vertical(|ui| {
                                    ui.set_width(500.0);
                                    let mut automatic = true;
                                    Prompt {
                                        state,
                                        current_version: "1.0.0",
                                        installed: true,
                                        work: Work::default(),
                                        save_before_restart: false,
                                        previous: None,
                                    }
                                    .contents(ui, &mut automatic)
                                })
                                .inner
                            });
                    });
                    if i % 2 == 1 {
                        ui.end_row();
                    }
                }
            });
    });
}
struct Gallery(Vec<State>);
impl eframe::App for Gallery {
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        gallery(ui, &self.0);
    }
}
fn main() -> eframe::Result {
    eframe::run_native(
        verbatim("j-ui updates"),
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_inner_size([1120.0, 1250.0]),
            ..Default::default()
        },
        Box::new(|cc| {
            theme::install(&cc.egui_ctx);
            Ok(Box::new(Gallery(states())))
        }),
    )
}
