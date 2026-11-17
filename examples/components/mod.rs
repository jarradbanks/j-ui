//! The prototype's interactive examples, composed from the shared widgets.

use super::*;
use egui::RichText;
use j_ui::dialog;
use j_ui::widgets::{
    Badge, Banner, Breadcrumb, ButtonKind, Checkbox, Column, MenuRow, Pills, Progress, RadioGroup,
    SectionHeader, Slider, State, Table, Tabs, TextInput, Toast, Toggle, Tone, menu_separator,
};
mod cards;
mod tokens;
mod typography;
pub use cards::{projects_height, recent_projects};
pub use tokens::token_map;
pub use typography::{spacing, typography, typography_height};

#[derive(Clone)]
pub struct Demo {
    pub inter: bool,
    pub name: String,
    pub search: String,
    pub width: String,
    pub taken: String,
    pub font: String,
    pub fps: usize,
    pub view: usize,
    pub kind: usize,
    pub quality: usize,
    pub volume: f64,
    pub tab: usize,
    pub open: bool,
    pub row: usize,
    pub looped: bool,
    pub mute: bool,
    pub auto_save: bool,
    pub toast: bool,
}
impl Demo {
    pub fn new() -> Self {
        Self {
            inter: false,
            name: tr!("page.interview_cut").into(),
            search: String::new(),
            width: "1920".into(),
            taken: tr!("page.untitled").into(),
            font: "in".into(),
            fps: 1,
            view: 0,
            kind: 0,
            quality: 1,
            volume: 64.0,
            tab: 0,
            open: true,
            row: 0,
            looped: true,
            mute: false,
            auto_save: true,
            toast: true,
        }
    }
}

fn muted(text: &str) -> RichText {
    theme::text(text, theme::body()).color(color::ink_3())
}
fn text_row(ui: &mut Ui, contents: impl FnOnce(&mut Ui)) {
    ui.scope(|ui| {
        ui.spacing_mut().interact_size.y = 0.0;
        ui.horizontal(contents);
    });
}
fn between(ui: &mut Ui, left: RichText, right: &str) {
    text_row(ui, |ui| {
        ui.label(left);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(muted(right));
        });
    });
}
pub fn buttons(ui: &mut Ui) {
    ui.horizontal_wrapped(|ui| {
        ui.add(Button::primary(tr!("page.save")));
        ui.add(Button::secondary(tr!("page.cancel")));
        ui.add(Button::ghost(tr!("demo.reset")));
        ui.add(Button::destructive(tr!("demo.delete")));
    });
    ui.horizontal_wrapped(|ui| {
        ui.add(Button::icon(icon::PLUS, tr!("panel.add")).kind(ButtonKind::Secondary));
        ui.add(Button::secondary(tr!("page.new_folder")).with_icon(icon::FOLDER));
        ui.add(Button::primary(tr!("page.saving")).loading(true));
        ui.add(Button::secondary(tr!("demo.export")).state(State::Disabled));
    });
}

pub fn text_input(ui: &mut Ui, d: &mut Demo) {
    let width = ui.available_width();
    ui.add(TextInput::new(&mut d.name).width(width));
    ui.add(
        TextInput::search(&mut d.search)
            .hint(tr!("page.search_clips"))
            .width(width),
    );
    ui.add(
        TextInput::new(&mut d.width)
            .prefix(verbatim("W"))
            .suffix(verbatim("px"))
            .width(width),
    );
    let error = Some(tr!("page.that_name_is_already_in_use"));
    ui.add(TextInput::new(&mut d.taken).error(error).width(width));
}

pub fn dropdown_and_menu(ui: &mut Ui, d: &mut Demo) {
    let rates = [
        verbatim("24 fps"),
        verbatim("30 fps"),
        verbatim("60 fps"),
        verbatim("120 fps"),
    ];
    ui.add(Dropdown::new(&mut d.fps, &rates).width(ui.available_width()));
    egui::Frame::popup(ui.style()).show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        ui.add(MenuRow::new(tr!("page.snap_to_grid"), true).shortcut(verbatim("Ctrl+G")));
        ui.add(
            MenuRow::new(tr!("page.duplicate"), false)
                .shortcut(verbatim("Ctrl+D"))
                .lit(true),
        );
        ui.add(
            MenuRow::new(tr!("page.paste"), false)
                .shortcut(verbatim("Ctrl+V"))
                .enabled(false),
        );
        menu_separator(ui);
        ui.add(MenuRow::new(tr!("demo.delete"), false).shortcut(verbatim("Del")));
    });
}

pub fn selection(ui: &mut Ui, d: &mut Demo) {
    ui.horizontal_wrapped(|ui| {
        ui.add(Segmented::new(
            &mut d.view,
            &[tr!("page.grid"), tr!("page.list"), tr!("page.detail")],
        ));
        ui.add(Pills::new(
            &mut d.kind,
            &[tr!("page.all"), tr!("clip.video"), tr!("clip.audio")],
        ));
    });
    ui.horizontal_wrapped(|ui| {
        ui.add(Checkbox::new(&mut d.looped, tr!("page.loop_playback")));
        ui.add(Checkbox::new(&mut d.mute, tr!("page.mute")));
        ui.add(Toggle::new(&mut d.auto_save, tr!("page.auto_save")).show_label());
    });
}

pub fn slider_and_progress(ui: &mut Ui, d: &mut Demo) {
    between(
        ui,
        RichText::new(tr!("page.volume")),
        &format!("{}", d.volume),
    );
    ui.add(
        Slider::new(&mut d.volume, 0.0..=100.0, tr!("page.volume"))
            .step(1.0)
            .show_ticks(false),
    );
    between(ui, RichText::new(tr!("page.exporting")), "38%");
    ui.add(Progress::new(0.38, tr!("page.exporting")));
}

pub fn navigation(ui: &mut Ui, d: &mut Demo) {
    ui.add(
        Breadcrumb::new(tr!("page.projects"), tr!("page.interview_cut"))
            .height(super::page::line_height(theme::body().size)),
    );
    ui.add(Tabs::new(
        &[
            tr!("panel.inspector"),
            tr!("panel.effects"),
            tr!("panel.history"),
        ],
        &mut d.tab,
    ));
    ui.add(SectionHeader::new(tr!("demo.transform"), &mut d.open));
    if d.open {
        ui.label(muted(tr!("page.position_scale_and_rotation_live_here")));
    }
}

pub fn badge_and_banner(ui: &mut Ui) {
    text_row(ui, |ui| {
        ui.add(Badge::new(tr!("page.draft")));
        ui.add(Badge::new(tr!("page.synced")).tone(Tone::Info));
        ui.add(Badge::new(tr!("page.ready")).tone(Tone::Success));
        ui.add(Badge::new(tr!("page.offline")).tone(Tone::Warning));
        ui.add(Badge::new(tr!("page.failed")).tone(Tone::Danger));
    });
    let _ = Banner::new(Tone::Info, tr!("page.an_update_is_ready_to_install"))
        .action(tr!("page.restart"))
        .show(ui);
    let _ = Banner::new(Tone::Warning, tr!("page.two_clips_are_offline"))
        .action(tr!("page.relink"))
        .show(ui);
    let _ = Banner::new(Tone::Danger, tr!("page.the_export_could_not_be_saved"))
        .action(tr!("page.retry"))
        .show(ui);
}

pub fn table(ui: &mut Ui, d: &mut Demo) {
    let columns = [
        Column::fill(tr!("page.name")),
        Column::fixed(tr!("page.status"), 72.0),
        Column::fixed(tr!("page.size"), 80.0).right(),
    ];
    let rows = [
        (
            tr!("page.interview_cut"),
            tr!("page.ready"),
            Tone::Success,
            verbatim("1.24 GB"),
        ),
        (
            tr!("page.b_roll_harbour"),
            tr!("page.synced"),
            Tone::Info,
            verbatim("812 MB"),
        ),
        (
            tr!("page.voiceover_take_3"),
            tr!("page.offline"),
            Tone::Warning,
            verbatim("46 MB"),
        ),
        (
            tr!("page.title_card"),
            tr!("page.draft"),
            Tone::Neutral,
            verbatim("3 MB"),
        ),
    ];
    let names = rows.map(|r| r.0);
    Table::new(&columns, &mut d.row).show(ui, &names, |ui, row, column| {
        let (name, status, tone, size) = rows[row];
        match column {
            0 => ui.label(name),
            1 => ui.add(Badge::new(status).tone(tone)),
            _ => ui.label(size),
        };
    });
}

pub fn dialog(ui: &mut Ui) {
    dialog::frame(ui.style()).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(theme::text(tr!("page.delete_clip"), theme::title()));
        ui.label(muted(tr!(
            "page.the_clip_is_removed_from_the_timeline_the_source_file_stays_on_disk"
        )));
        ui.add_space(space::S8);
        let buttons = [
            Button::secondary(tr!("page.cancel")),
            Button::destructive(tr!("demo.delete")),
        ];
        dialog::action_footer(ui, buttons);
    });
}

pub fn show(ui: &mut Ui, index: usize, demo: &mut Demo) {
    match index {
        0 => recent_projects(ui),
        1 => typography(ui, demo.inter),
        2 => spacing(ui),
        3 => buttons(ui),
        4 => text_input(ui, demo),
        5 => autocomplete(ui, demo),
        6 => dropdown_and_menu(ui, demo),
        7 => selection(ui, demo),
        8 => {
            ui.add(RadioGroup::new(
                &mut demo.quality,
                &[tr!("page.draft"), tr!("page.standard"), tr!("page.high")],
            ));
        }
        9 => slider_and_progress(ui, demo),
        10 => navigation(ui, demo),
        11 => badge_and_banner(ui),
        12 => {
            if demo.toast {
                let toast = Toast::new(tr!("page.export_finished"), tr!("demo.dismiss"))
                    .action(tr!("page.open"))
                    .show(ui);
                demo.toast = !toast.dismissed;
            }
            if ui.add(Button::secondary(tr!("page.show_toast"))).clicked() {
                demo.toast = true;
            }
        }
        13 => table(ui, demo),
        14 => dialog(ui),
        15 => empty(ui),
        16 => token_map(ui),
        _ => unreachable!(),
    }
}

fn autocomplete(ui: &mut Ui, demo: &mut Demo) {
    const FONTS: [&str; 8] = [
        verbatim("Inter"),
        verbatim("Inter Display"),
        verbatim("Onest"),
        verbatim("Geist"),
        verbatim("IBM Plex Sans"),
        verbatim("JetBrains Mono"),
        verbatim("Noto Sans"),
        verbatim("Source Serif"),
    ];
    ui.add(
        j_ui::widgets::Autocomplete::new(&mut demo.font, &FONTS)
            .hint(tr!("page.search_fonts"))
            .empty(tr!("page.no_matching_fonts"))
            .initially_open(true)
            .width(ui.available_width()),
    );
}

fn empty(ui: &mut Ui) {
    let height = 32.0
        + (theme::title().size * 1.3 * 64.0).floor() / 64.0
        + super::page::line_height(theme::body().size)
        + size::control()
        + 3.0 * space::gap();
    let rect = Rect::from_min_size(ui.cursor().min, vec2(ui.available_width(), height));
    let mut body = ui.new_child(UiBuilder::new().max_rect(rect));
    let _ = j_ui::empty_state::EmptyState::new(
        icon::FOLDER,
        tr!("page.no_clips_yet"),
        tr!("page.import_help"),
    )
    .action(tr!("page.import_media"))
    .show(&mut body);
}
