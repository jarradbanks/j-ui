use egui::Ui;

#[derive(Default)]
struct Gallery {
    activations: usize,
    level: f64,
    enabled: bool,
    text: String,
}

fn main() -> eframe::Result {
    #[cfg(all(debug_assertions, feature = "hot-reload"))]
    dioxus_devtools::connect_subsecond();
    eframe::run_native(
        "Native Debug Gallery",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_inner_size([1280.0, 800.0]),
            persistence_path: Some(std::env::temp_dir().join("j-ui-debug-gallery")),
            ..Default::default()
        },
        Box::new(|cc| {
            j_ui::theme::install(&cc.egui_ctx);
            j_ui_debug::install_from_env(&cc.egui_ctx, "j-ui-gallery")?;
            Ok(Box::<Gallery>::default())
        }),
    )
}

impl eframe::App for Gallery {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        #[cfg(all(debug_assertions, feature = "hot-reload"))]
        subsecond::call(|| self.view(ui));
        #[cfg(not(all(debug_assertions, feature = "hot-reload")))]
        self.view(ui);
    }
}

impl Gallery {
    fn view(&mut self, ui: &mut Ui) {
        ui.painter()
            .rect_filled(ui.max_rect(), 0, j_ui::tokens::color::ground());
        egui::Frame::new().inner_margin(24).show(ui, |ui| {
            ui.set_max_width(420.0);
            ui.heading("Native Debug Gallery");
            ui.label("Patch Before");
            let response = ui.add(j_ui::widgets::Slider::new(
                &mut self.level,
                0.0..=1.0,
                "Master Level",
            ));
            j_ui_debug::target(ui, &response, "demo/mixer/master/level", "Master Level");
            ui.checkbox(&mut self.enabled, "Enabled");
            ui.text_edit_singleline(&mut self.text);
            if ui.button("Apply").clicked() {
                self.activations += 1;
            }
            ui.label(format!("Activations: {}", self.activations));
        });
    }
}
