//! Every component on one page, laid out as `prototype/component-styles/index.html` is, so a
//! render can be held against it. Snapshots cover the built-in themes and 150% scale; the
//! geometry checks cover row centring, column alignment, equal tiles and bounds.

use egui::{Align, Color32, CornerRadius, Layout, Rect, RichText, Ui, UiBuilder, pos2, vec2};
use egui_kittest::{
    Harness, SnapshotResults,
    kittest::{NodeT, Queryable},
};
use j_ui::dialog;
use j_ui::empty_state::EmptyState;
use j_ui::panel::Panel;
use j_ui::theme::{self, icon};
use j_ui::tokens::{color, radius, size, space};
use j_ui::widgets::{
    Autocomplete, Badge, Banner, Breadcrumb, Button, ButtonKind, Card, Checkbox, Column, Dropdown,
    MenuRow, NavRow, Pills, Progress, RadioGroup, SectionHeader, Segmented, Slider, State, Table,
    Tabs, TextInput, Timecode, Toast, Toggle, Tone, menu_separator,
};
use j_ui::{test_support, themes};

const W: f32 = 1280.0;
const H: f32 = 2980.0;
const RAIL: f32 = 232.0;
const MARGIN: f32 = 32.0;
const FONTS: [&str; 8] = [
    "Inter",
    "Inter Display",
    "Onest",
    "Geist",
    "IBM Plex Sans",
    "JetBrains Mono",
    "Noto Sans",
    "Source Serif",
];
const SECTIONS: [&str; 16] = [
    "Recent Projects",
    "Typography",
    "Spacing",
    "Buttons",
    "Text Input",
    "Autocomplete",
    "Dropdown and Menu",
    "Selection",
    "Radio Group",
    "Slider and Progress",
    "Navigation",
    "Badge and Banner",
    "Toast",
    "Table",
    "Dialog",
    "Empty State",
];

struct Demo {
    name: String,
    search: String,
    width: String,
    taken: String,
    font: String,
    fps: usize,
    view: usize,
    kind: usize,
    quality: usize,
    volume: f64,
    tab: usize,
    open: bool,
    row: usize,
    looped: bool,
    mute: bool,
    auto_save: bool,
    theme: usize,
    autocomplete: Rect,
    cards: Vec<Rect>,
}

impl Demo {
    fn new() -> Self {
        Self {
            name: "Interview Cut".into(),
            search: String::new(),
            width: "1920".into(),
            taken: "Untitled".into(),
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
            theme: 0,
            autocomplete: Rect::NOTHING,
            cards: Vec::new(),
        }
    }
}

fn muted(text: &str) -> RichText {
    RichText::new(text).color(color::ink_3())
}

/// A row as tall as its text, not as a control.
fn text_row(ui: &mut Ui, contents: impl FnOnce(&mut Ui)) {
    ui.scope(|ui| {
        ui.spacing_mut().interact_size.y = 0.0;
        ui.horizontal(contents);
    });
}

/// A label at the row's left and a muted one at its right.
fn between(ui: &mut Ui, left: RichText, right: &str) {
    text_row(ui, |ui| {
        ui.label(left);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(muted(right));
        });
    });
}

/// A titled panel in `rect`, with `extra` laid out from the header's right.
fn section(
    ui: &mut Ui,
    rect: Rect,
    title: &str,
    extra: impl FnOnce(&mut Ui),
    body: impl FnOnce(&mut Ui),
) {
    ui.scope_builder(UiBuilder::new().max_rect(rect), |ui| {
        Panel::titled(title).show(ui, body);
    });
    let header = Rect::from_min_size(rect.min, vec2(rect.width(), size::HEADER));
    let header = header.shrink2(vec2(space::INSET, 0.0));
    let layout = Layout::right_to_left(Align::Center);
    ui.scope_builder(UiBuilder::new().max_rect(header).layout(layout), extra);
}

fn new_badge(ui: &mut Ui) {
    ui.add(Badge::new("New").fit());
}

fn card(ui: &mut Ui, width: f32, thumb: Color32, title: &str, text: &str, edited: &str) -> Rect {
    let inset = space::INSET as i8;
    let response = Card::new().show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        ui.set_width(width);
        let (art, _) = ui.allocate_exact_size(vec2(width, 96.0), egui::Sense::hover());
        let top = CornerRadius {
            nw: radius::PANEL,
            ne: radius::PANEL,
            sw: 0,
            se: 0,
        };
        ui.painter().rect_filled(art, top, thumb);
        let head = egui::Margin {
            left: inset,
            right: inset,
            top: inset,
            bottom: 0,
        };
        egui::Frame::new().inner_margin(head).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(title).font(theme::title()));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.add(Button::icon(icon::ELLIPSIS, "More"));
                });
            });
        });
        let text_margin = egui::Margin {
            left: inset,
            right: inset,
            top: 4,
            bottom: inset,
        };
        egui::Frame::new().inner_margin(text_margin).show(ui, |ui| {
            ui.label(muted(text));
        });
        let foot = egui::Margin::symmetric(inset, 8);
        egui::Frame::new().inner_margin(foot).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(muted(edited).font(theme::small()));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.add(Button::secondary("Open"));
                    ui.add(Button::ghost("Share"));
                });
            });
        });
    });
    response.response.rect
}

fn recent_projects(ui: &mut Ui, d: &mut Demo) {
    let width = (ui.available_width() - space::S16) / 2.0;
    let cards = [
        (
            color::CLIP_VIOLET,
            "Interview Cut",
            "Two cameras, synced audio and a rough assembly of the first act.",
            "Edited 2 hours ago",
        ),
        (
            color::CLIP_PLUM,
            "Harbour Shoot",
            "Raw footage from the morning session, sorted by location.",
            "Edited yesterday",
        ),
        (
            color::CLIP_GREEN,
            "Title Sequence",
            "Animated title card with three colour variations to review.",
            "Edited 12 May",
        ),
    ];
    d.cards.clear();
    ui.spacing_mut().item_spacing = egui::Vec2::splat(space::S16);
    for row in cards.chunks(2) {
        ui.horizontal_top(|ui| {
            for (thumb, title, text, edited) in row {
                d.cards.push(card(ui, width, *thumb, title, text, edited));
            }
        });
    }
}

fn typography(ui: &mut Ui) {
    let semibold = |text: &str, font| RichText::new(text).font(font);
    between(ui, semibold("Display", theme::display()), "SemiBold");
    between(ui, semibold("Headline", theme::headline()), "SemiBold");
    between(ui, semibold("Title", theme::title()), "SemiBold");
    let strong = RichText::new("Body Strong for buttons and tabs").font(theme::body_strong());
    between(ui, strong, "Medium");
    let body = RichText::new("Body for labels, values and descriptions");
    between(ui, body, "Regular");
    let small = muted("Small for captions and column headers").font(theme::small());
    between(ui, small, "Regular");
    text_row(ui, |ui| {
        ui.add(Timecode::new(30 * 84 + 8, 30));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(muted("Medium"));
        });
    });
}

fn spacing(ui: &mut Ui) {
    let steps = [
        ("space::S4", space::S4),
        ("space::S8", space::S8),
        ("space::S12", space::S12),
        ("space::S16", space::S16),
        ("space::S24", space::S24),
        ("space::S32", space::S32),
    ];
    for (name, width) in steps {
        let (row, _) = ui.allocate_exact_size(vec2(240.0, 16.0), egui::Sense::hover());
        let font = theme::body();
        let at = row.left_center();
        ui.painter()
            .text(at, egui::Align2::LEFT_CENTER, name, font, color::ink());
        let bar = Rect::from_min_size(
            pos2(row.left() + 124.0, row.center().y - 4.0),
            vec2(width, 8.0),
        );
        ui.painter().rect_filled(bar, 2.0, color::blue());
    }
    ui.label(muted(
        "4 inside a control group, 8 between controls, 16 panel inset, 16 between sections, 24 \
         and 32 for page margins.",
    ));
}

fn buttons(ui: &mut Ui) {
    ui.horizontal(|ui| {
        ui.add(Button::primary("Save"));
        ui.add(Button::secondary("Cancel"));
        ui.add(Button::ghost("Reset"));
        ui.add(Button::destructive("Delete"));
    });
    ui.horizontal(|ui| {
        ui.add(Button::icon(icon::PLUS, "Add").kind(ButtonKind::Secondary));
        ui.add(Button::secondary("New Folder").with_icon(icon::FOLDER));
        ui.add(Button::primary("Saving").loading(true));
        ui.add(Button::secondary("Export").state(State::Disabled));
    });
}

fn text_input(ui: &mut Ui, d: &mut Demo) {
    let width = ui.available_width();
    ui.add(TextInput::new(&mut d.name).width(width));
    ui.add(
        TextInput::search(&mut d.search)
            .hint("Search clips")
            .width(width),
    );
    ui.add(
        TextInput::new(&mut d.width)
            .prefix("W")
            .suffix("px")
            .width(width),
    );
    let error = Some("That name is already in use.");
    ui.add(TextInput::new(&mut d.taken).error(error).width(width));
}

fn dropdown_and_menu(ui: &mut Ui, d: &mut Demo) {
    let rates = ["24 fps", "30 fps", "60 fps", "120 fps"];
    ui.add(Dropdown::new(&mut d.fps, &rates).width(ui.available_width()));
    egui::Frame::popup(ui.style()).show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        ui.add(MenuRow::new("Snap to Grid", true).shortcut("Ctrl+G"));
        ui.add(
            MenuRow::new("Duplicate", false)
                .shortcut("Ctrl+D")
                .lit(true),
        );
        ui.add(
            MenuRow::new("Paste", false)
                .shortcut("Ctrl+V")
                .enabled(false),
        );
        menu_separator(ui);
        ui.add(MenuRow::new("Delete", false).shortcut("Del"));
    });
}

fn selection(ui: &mut Ui, d: &mut Demo) {
    ui.horizontal(|ui| {
        ui.add(Segmented::new(&mut d.view, &["Grid", "List", "Detail"]));
        ui.add(Pills::new(&mut d.kind, &["All", "Video", "Audio"]));
    });
    ui.horizontal(|ui| {
        ui.add(Checkbox::new(&mut d.looped, "Loop Playback"));
        ui.add(Checkbox::new(&mut d.mute, "Mute"));
        ui.spacing_mut().item_spacing.x = space::S8;
        ui.add(Toggle::new(&mut d.auto_save, "Auto Save"));
        ui.label("Auto Save");
    });
}

fn slider_and_progress(ui: &mut Ui, d: &mut Demo) {
    between(ui, RichText::new("Volume"), &format!("{}", d.volume));
    ui.add(Slider::new(&mut d.volume, 0.0..=100.0, "Volume"));
    between(ui, RichText::new("Exporting"), "38%");
    ui.add(Progress::new(0.38, "Exporting"));
}

fn navigation(ui: &mut Ui, d: &mut Demo) {
    ui.add(Breadcrumb::new("Projects", "Interview Cut"));
    ui.add(Tabs::new(&["Inspector", "Effects", "History"], &mut d.tab));
    ui.add(SectionHeader::new("Transform", &mut d.open));
    if d.open {
        ui.label(muted("Position, scale and rotation live here."));
    }
}

fn badge_and_banner(ui: &mut Ui) {
    text_row(ui, |ui| {
        ui.add(Badge::new("Draft"));
        ui.add(Badge::new("Synced").tone(Tone::Info));
        ui.add(Badge::new("Ready").tone(Tone::Success));
        ui.add(Badge::new("Offline").tone(Tone::Warning));
        ui.add(Badge::new("Failed").tone(Tone::Danger));
    });
    let _ = Banner::new(Tone::Info, "An update is ready to install.")
        .action("Restart")
        .show(ui);
    let _ = Banner::new(Tone::Warning, "Two clips are offline.")
        .action("Relink")
        .show(ui);
    let _ = Banner::new(Tone::Danger, "The export could not be saved.")
        .action("Retry")
        .show(ui);
}

fn table(ui: &mut Ui, d: &mut Demo) {
    let columns = [
        Column::fill("Name"),
        Column::fixed("Status", 72.0),
        Column::fixed("Size", 80.0).right(),
    ];
    let rows = [
        ("Interview Cut", "Ready", Tone::Success, "1.24 GB"),
        ("B-Roll Harbour", "Synced", Tone::Info, "812 MB"),
        ("Voiceover Take 3", "Offline", Tone::Warning, "46 MB"),
        ("Title Card", "Draft", Tone::Neutral, "3 MB"),
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

fn dialog(ui: &mut Ui) {
    dialog::frame(ui.style()).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(RichText::new("Delete Clip").font(theme::title()));
        ui.label(muted(
            "The clip is removed from the timeline. The source file stays on disk.",
        ));
        ui.add_space(space::S8);
        let buttons = [Button::secondary("Cancel"), Button::destructive("Delete")];
        dialog::action_footer(ui, buttons);
    });
}

fn chrome(ui: &mut Ui, d: &mut Demo) {
    let screen = ui.ctx().content_rect();
    ui.painter().rect_filled(screen, 0.0, color::ground());
    let rail = Rect::from_min_max(pos2(12.0, 0.0), pos2(RAIL - 12.0, screen.bottom()));
    ui.scope_builder(UiBuilder::new().max_rect(rail), |ui| {
        ui.spacing_mut().item_spacing.y = 2.0;
        let (brand, _) =
            ui.allocate_exact_size(vec2(rail.width(), size::APP_BAR), egui::Sense::hover());
        let at = brand.left_center() + vec2(10.0, 0.0);
        let left = egui::Align2::LEFT_CENTER;
        ui.painter()
            .text(at, left, "Studio", theme::title(), color::ink());
        let (head, _) =
            ui.allocate_exact_size(vec2(rail.width(), size::CONTROL), egui::Sense::hover());
        let at = head.left_center() + vec2(10.0, 0.0);
        ui.painter()
            .text(at, left, "Components", theme::small(), color::ink_3());
        for (i, name) in SECTIONS.into_iter().enumerate() {
            ui.add(NavRow::new(name, i == 0));
        }
    });
    let bar = Rect::from_min_max(pos2(RAIL + MARGIN, 0.0), pos2(W - MARGIN, size::APP_BAR));
    let layout = Layout::right_to_left(Align::Center);
    ui.scope_builder(UiBuilder::new().max_rect(bar).layout(layout), |ui| {
        ui.add(Segmented::new(&mut d.theme, &["Dark", "Light"]));
        ui.add(Segmented::new(&mut 2, &["Current", "Line", "Soft"]));
        ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
            ui.label(RichText::new("Component Styles").font(theme::headline()));
        });
    });
}

fn page(ui: &mut Ui, d: &mut Demo) {
    chrome(ui, d);
    let left = RAIL + MARGIN;
    let gap = space::S16;
    let column = (W - MARGIN - left - gap) / 2.0;
    let mut top = size::APP_BAR + 28.0;
    let mut row = |height: f32| {
        let y = top;
        top += height + gap;
        let a = Rect::from_min_size(pos2(left, y), vec2(column, height));
        (a, a.translate(vec2(column + gap, 0.0)))
    };
    let none = |_: &mut Ui| {};

    let (a, b) = row(584.0);
    let add = |ui: &mut Ui| {
        ui.add(Button::primary("New Project").with_icon(icon::PLUS));
    };
    section(ui, a.union(b), "Recent Projects", add, |ui| {
        recent_projects(ui, d)
    });

    let (a, b) = row(316.0);
    section(ui, a, "Typography", none, typography);
    section(ui, b, "Spacing", none, spacing);

    let (a, b) = row(292.0);
    section(ui, a, "Buttons", none, buttons);
    section(ui, b, "Text Input", none, |ui| text_input(ui, d));

    let (a, b) = row(322.0);
    section(ui, a, "Autocomplete", new_badge, |ui| {
        let field = Autocomplete::new(&mut d.font, &FONTS)
            .hint("Search fonts")
            .empty("No matching fonts")
            .width(ui.available_width());
        d.autocomplete = ui.add(field).rect;
    });
    section(ui, b, "Dropdown and Menu", none, |ui| {
        dropdown_and_menu(ui, d)
    });

    let (a, b) = row(166.0);
    section(ui, a, "Selection", none, |ui| selection(ui, d));
    section(ui, b, "Radio Group", new_badge, |ui| {
        ui.add(RadioGroup::new(
            &mut d.quality,
            &["Draft", "Standard", "High"],
        ));
    });

    let (a, b) = row(242.0);
    section(ui, a, "Slider and Progress", new_badge, |ui| {
        slider_and_progress(ui, d);
    });
    section(ui, b, "Navigation", none, |ui| navigation(ui, d));

    let (a, b) = row(286.0);
    section(ui, a, "Badge and Banner", new_badge, badge_and_banner);
    section(ui, b, "Toast", new_badge, |ui| {
        let _ = Toast::new("Export finished.", "Dismiss")
            .action("Open")
            .show(ui);
        ui.add(Button::secondary("Show Toast"));
    });

    let (a, b) = row(282.0);
    section(ui, a, "Table", new_badge, |ui| table(ui, d));
    section(ui, b, "Dialog", none, dialog);

    let (a, _) = row(227.0);
    section(ui, a, "Empty State", none, |ui| {
        let help = "Import media to start a project.";
        let _ = EmptyState::new(icon::FOLDER, "No Clips Yet", help)
            .action("Import Media")
            .show(ui);
    });
}

fn harness(palette: themes::Palette, ppp: f32, gpu: bool) -> Harness<'static, Demo> {
    let mut fresh = true;
    let builder = Harness::builder()
        .with_size([W, H])
        .with_pixels_per_point(ppp);
    let builder = if gpu { builder.wgpu() } else { builder };
    let mut h = builder.build_ui_state(
        move |ui, demo: &mut Demo| {
            if test_support::install_theme(ui, &mut fresh) {
                themes::apply(ui.ctx(), &palette);
                return;
            }
            page(ui, demo);
        },
        Demo::new(),
    );
    // The spinner never settles, so step a fixed number of frames.
    h.run_steps(3);
    // The prototype shows the suggestions open: click into the field.
    let pos = h.state().autocomplete.center();
    h.hover_at(pos);
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
    }
    h.run_steps(4);
    // Off the page, so nothing is drawn hovered.
    h.event(egui::Event::PointerGone);
    h.run_steps(2);
    h
}

#[test]
fn components_snapshots() {
    if !test_support::gpu() {
        return;
    }
    let mut results = SnapshotResults::new();
    for (name, palette) in themes::BUILT_IN {
        let mut h = harness(palette, 1.0, true);
        results.add(h.try_snapshot(format!("components_{name}")));
    }
    let mut h = harness(themes::DARK, 1.5, true);
    results.add(h.try_snapshot("components_dark_1.5x"));
    results.unwrap();
}

/// The lowest node named `label`: cards, buttons and chips repeat names further up the page.
fn rect_of<T>(h: &Harness<'_, T>, label: &str) -> Rect {
    let all = h.query_all_by_label(label).map(|n| n.rect());
    all.max_by(|a, b| a.top().total_cmp(&b.top()))
        .unwrap_or_else(|| panic!("no {label}"))
}

#[test]
fn rows_are_centred_columns_aligned_and_everything_stays_inside() {
    for ppp in [1.0, 1.5] {
        let h = harness(themes::DARK, ppp, false);
        // A toast's and a banner's message and buttons share one centre line.
        let rows = [
            vec!["Export finished.", "Open", "Dismiss"],
            vec!["An update is ready to install.", "Restart"],
            vec!["Two clips are offline.", "Relink"],
            vec!["The export could not be saved.", "Retry"],
        ];
        for row in rows {
            let first = rect_of(&h, row[0]).center().y;
            for label in row {
                let off = (rect_of(&h, label).center().y - first).abs();
                assert!(off <= 1.0, "{ppp}x {label} is {off}pt off its row's centre");
            }
        }
        // The table's status chips are one width and share a left edge.
        let chips: Vec<Rect> = ["Ready", "Synced", "Offline", "Draft"]
            .into_iter()
            .map(|label| rect_of(&h, label))
            .collect();
        for chip in &chips {
            assert_eq!(
                chip.width(),
                chips[0].width(),
                "{ppp}x chip widths {chips:?}"
            );
            assert_eq!(chip.left(), chips[0].left(), "{ppp}x chip edges {chips:?}");
            assert_eq!(chip.height(), size::BADGE);
        }
        // Cards are one height whatever their text.
        let cards = &h.state().cards;
        assert_eq!(cards.len(), 3);
        for card in cards {
            assert_eq!(card.height(), cards[0].height(), "{ppp}x cards {cards:?}");
        }
        // Controls keep the control height, and nothing leaves the page.
        for label in [
            "Save",
            "Cancel",
            "Reset",
            "Delete Clip",
            "New Folder",
            "Show Toast",
        ] {
            let rect = rect_of(&h, label);
            let page = Rect::from_min_size(pos2(0.0, 0.0), vec2(W, H));
            assert!(page.contains_rect(rect), "{ppp}x {label} at {rect:?}");
        }
        for label in [
            "Save",
            "Cancel",
            "Reset",
            "New Folder",
            "Show Toast",
            "Saving",
        ] {
            let height = rect_of(&h, label).height();
            assert_eq!(height, size::CONTROL, "{ppp}x {label}");
        }
        // The suggestions open under the field with the first match lit.
        assert!(h.query_by_label("Inter Display").is_some(), "{ppp}x menu");
        assert!(h.query_by_label("Onest").is_none());
    }
}

#[test]
fn autocomplete_filters_moves_and_picks_from_the_keyboard() {
    let mut fresh = true;
    let mut h = Harness::builder().with_size([320.0, 320.0]).build_ui_state(
        move |ui, text: &mut String| {
            if test_support::install_theme(ui, &mut fresh) {
                return;
            }
            ui.add(Autocomplete::new(text, &FONTS).empty("No matching fonts"));
            ui.add(Button::secondary("Next"));
        },
        String::from("in"),
    );
    h.key_press(egui::Key::Tab);
    h.run();
    assert!(h.query_by_label("JetBrains Mono").is_some());
    h.key_press(egui::Key::ArrowDown);
    h.run();
    h.key_press(egui::Key::Enter);
    h.run();
    assert_eq!(h.state(), "Inter Display");
    assert!(
        h.query_by_label("JetBrains Mono").is_none(),
        "picking closes"
    );
    let field = h.get_by_role(egui::accesskit::Role::TextInput);
    assert!(
        field.accesskit_node().is_focused(),
        "focus stays in the field"
    );

    // A click into the field opens the menu and a click on a row takes it.
    *h.state_mut() = String::from("pl");
    h.key_press(egui::Key::Tab);
    h.run();
    h.get_by_role(egui::accesskit::Role::TextInput).click();
    h.run();
    assert!(h.query_by_label("IBM Plex Sans").is_some(), "a click opens");
    h.key_press(egui::Key::Tab);
    h.run();
    assert!(h.query_by_label("IBM Plex Sans").is_none(), "Tab closes");
    h.get_by_role(egui::accesskit::Role::TextInput).click();
    h.run();
    h.get_by_label("IBM Plex Sans").click();
    h.run();
    assert_eq!(h.state(), "IBM Plex Sans");
    assert!(h.query_by_label("Geist").is_none(), "picking closes");
}

#[test]
fn radio_tabs_table_and_toast_respond_to_clicks() {
    #[derive(Default)]
    struct Picked {
        quality: usize,
        tab: usize,
        row: usize,
        dismissed: bool,
        changed: [bool; 3],
    }
    let mut fresh = true;
    let mut h = Harness::builder().with_size([480.0, 480.0]).build_ui_state(
        move |ui, p: &mut Picked| {
            if test_support::install_theme(ui, &mut fresh) {
                return;
            }
            let radio = RadioGroup::new(&mut p.quality, &["Draft", "Standard"]);
            p.changed[0] |= ui.add(radio).changed();
            let tabs = Tabs::new(&["Inspector", "Effects"], &mut p.tab);
            p.changed[1] |= ui.add(tabs).changed();
            let columns = [Column::fill("Name"), Column::fixed("Size", 80.0).right()];
            let table = Table::new(&columns, &mut p.row);
            let r = table.show(ui, &["Interview Cut", "Title Card"], |ui, row, column| {
                ui.label(["Interview", "Title", "1 GB", "3 MB"][row + 2 * column]);
            });
            p.changed[2] |= r.changed();
            let toast = Toast::new("Export finished.", "Dismiss").show(ui);
            p.dismissed |= toast.dismissed;
        },
        Picked::default(),
    );
    h.run();
    for label in ["Standard", "Effects", "Title Card", "Dismiss"] {
        h.get_by_label(label).click();
        h.run();
    }
    let p = h.state();
    assert_eq!((p.quality, p.tab, p.row), (1, 1, 1));
    assert!(p.dismissed);
    assert_eq!(p.changed, [true; 3]);
}
