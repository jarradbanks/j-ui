//! The interactive Component Styles page: `cargo run --example gallery`.

use eframe::egui;
use egui::{Align, Color32, FontId, Layout, Rect, Ui, UiBuilder, pos2, vec2};
use j_ui::i18n::{self, Catalog, verbatim};
use j_ui::theme::{self, icon};
use j_ui::themes;
use j_ui::tokens::{color, radius, size, space};
use j_ui::widgets::{Button, Dropdown, NavRow, Segmented};

/// The gallery's demo labels; the widgets' own strings are in j-ui's catalog.
pub static CATALOG: Catalog = Catalog::new([
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

/// New page copy uses Catalog's explicit English fallback until translated.
pub static PAGE_CATALOG: Catalog = Catalog::new([
    include_str!("../i18n/gallery/page/en.toml"),
    include_str!("../i18n/gallery/page/es.toml"),
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
]);

macro_rules! tr {
    ($key:literal) => {
        PAGE_CATALOG.get($key).unwrap_or_else(|| CATALOG.tr($key))
    };
}

mod components;
mod page;

fn main() -> eframe::Result {
    i18n::set_language(i18n::resolve(i18n::SYSTEM));
    #[cfg(all(debug_assertions, feature = "hot-reload"))]
    dioxus_devtools::connect_subsecond();
    eframe::run_native(
        tr!("page.component_styles"),
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_inner_size([1280.0, 860.0]),
            ..Default::default()
        },
        Box::new(|cc| {
            theme::install(&cc.egui_ctx);
            #[cfg(all(debug_assertions, feature = "debug-ui"))]
            j_ui_debug::install_from_env(&cc.egui_ctx, "j-ui-gallery")?;
            Ok(Box::new(Gallery::new()))
        }),
    )
}

pub struct Gallery {
    pub theme: usize,
    pub style: usize,
    pub font: usize,
    pub selected: usize,
    /// Section rectangles in document coordinates, before scrolling.
    pub sections: Vec<Rect>,
    pub scroll: f32,
    pub scroll_to: Option<usize>,
    demo: components::Demo,
    installed_font: (usize, &'static str),
}

impl Gallery {
    pub fn new() -> Self {
        Self {
            theme: 0,
            style: 2,
            font: 0,
            selected: 0,
            sections: Vec::new(),
            scroll: 0.0,
            scroll_to: None,
            demo: components::Demo::new(),
            installed_font: (0, i18n::language()),
        }
    }
}

impl Default for Gallery {
    fn default() -> Self {
        Self::new()
    }
}

impl eframe::App for Gallery {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        #[cfg(all(debug_assertions, feature = "hot-reload"))]
        subsecond::call(|| gallery(ui, self));
        #[cfg(not(all(debug_assertions, feature = "hot-reload")))]
        gallery(ui, self);
    }
}

/// The actual window content, shared with the rendered gallery tests.
pub fn gallery(ui: &mut Ui, gallery: &mut Gallery) {
    let presentation = [
        theme::ComponentStyle::Current,
        theme::ComponentStyle::Line,
        theme::ComponentStyle::Soft,
    ][gallery.style];
    theme::with_component_style(presentation, || {
        let palette = if gallery.theme == 0 {
            themes::DARK
        } else {
            themes::LIGHT
        };
        themes::apply(ui.ctx(), &palette);
        let mut style = theme::style();
        style.visuals = theme::visuals_with(&palette);
        ui.set_style(style);
        page::show(ui, gallery);
    });
    if (gallery.font, i18n::language()) != gallery.installed_font {
        let mut fonts = theme::fonts(i18n::language());
        if gallery.font == 1 {
            for chain in fonts.families.values_mut() {
                chain.sort_by_key(|name| !name.starts_with("inter"));
            }
        }
        ui.ctx().set_fonts(fonts);
        gallery.installed_font = (gallery.font, i18n::language());
    }
}
