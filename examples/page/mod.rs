//! Document grid, sticky navigation and toolbar for the gallery.

use super::*;
use j_ui::panel::Panel;
use j_ui::widgets::Badge;

pub const RAIL: f32 = 232.0;
pub const MARGIN: f32 = 32.0;
pub const COUNT: usize = 17;

pub fn titles() -> [&'static str; COUNT] {
    [
        tr!("page.recent_projects"),
        tr!("page.typography"),
        tr!("page.spacing"),
        tr!("page.buttons"),
        tr!("page.text_input"),
        tr!("page.autocomplete"),
        tr!("page.dropdown_and_menu"),
        tr!("row.selection"),
        tr!("page.radio_group"),
        tr!("page.slider_and_progress"),
        tr!("page.navigation"),
        tr!("page.badge_and_banner"),
        tr!("page.toast"),
        tr!("page.table"),
        tr!("page.dialog"),
        tr!("page.empty_state"),
        tr!("page.token_map"),
    ]
}

pub fn line_height(size: f32) -> f32 {
    (size * 1.45 * 64.0).floor() / 64.0
}

pub fn paragraph_height(ui: &Ui, text: &str, width: f32) -> f32 {
    let text = ui
        .painter()
        .layout(text.into(), theme::body(), color::ink(), width);
    line_height(theme::body().size) * text.rows.len() as f32
}

fn body_height(ui: &Ui, index: usize, width: f32, demo: &components::Demo) -> f32 {
    let line = line_height(theme::body().size);
    match index {
        0 => components::projects_height(ui, width),
        1 => components::typography_height(demo.inter),
        2 => {
            6.0 * 16.0 + 6.0 * space::gap() + paragraph_height(ui, tr!("page.spacing_help"), width)
        }
        3 | 7 => 2.0 * size::control() + space::gap(),
        4 => 4.0 * (size::control() + space::gap()) + line_height(theme::small().size),
        5 => 6.0 * size::control() + 24.0,
        6 => 5.0 * size::control() + space::gap() + 19.0,
        8 => size::control(),
        9 => 2.0 * line + size::thumb() + 4.0 + size::slider_track() + 3.0 * space::gap(),
        10 => {
            line + size::header()
                + size::control()
                + 2.0 * space::gap()
                + if demo.open {
                    space::gap()
                        + paragraph_height(
                            ui,
                            tr!("page.position_scale_and_rotation_live_here"),
                            width,
                        )
                } else {
                    0.0
                }
        }
        11 => size::badge() + 3.0 * (space::gap() + size::control() + 12.0),
        12 => {
            size::control()
                + if demo.toast {
                    space::gap() + size::control() + 18.0
                } else {
                    0.0
                }
        }
        13 => 5.0 * (size::control() + 4.0),
        14 => {
            2.0 + 2.0 * space::dialog_pad()
                + (theme::title().size * 1.3 * 64.0).floor() / 64.0
                + 2.0 * space::gap()
                + 8.0
                + size::control()
                + paragraph_height(
                    ui,
                    tr!("page.the_clip_is_removed_from_the_timeline_the_source_file_stays_on_disk"),
                    width - 2.0 * space::dialog_pad() - 2.0,
                )
        }
        15 => {
            32.0 + (theme::title().size * 1.3 * 64.0).floor() / 64.0
                + line
                + size::control()
                + 3.0 * space::gap()
        }
        16 => (27.0 / ((width + 24.0) / 524.0).floor().max(1.0)).ceil() * size::control(),
        _ => unreachable!(),
    }
}

fn grid(ui: &Ui, width: f32, demo: &components::Demo) -> Vec<Rect> {
    let columns = ((width + space::section_gap()) / (400.0 + space::section_gap()))
        .floor()
        .max(1.0) as usize;
    let column = (width - space::section_gap() * (columns - 1) as f32) / columns as f32;
    let inset = space::panel_inset();
    let chrome = 2.0 + size::panel_header() + inset.max(12.0) + inset;
    let mut top = size::app_bar() + 28.0;
    let mut sections = Vec::with_capacity(COUNT);
    let mut row = |indices: std::ops::Range<usize>, width: f32| {
        let height = indices
            .clone()
            .map(|index| body_height(ui, index, width - 2.0 * inset - 2.0, demo) + chrome)
            .fold(0.0, f32::max);
        for (column, _) in indices.enumerate() {
            let left = RAIL + MARGIN + column as f32 * (width + space::section_gap());
            sections.push(Rect::from_min_size(pos2(left, top), vec2(width, height)));
        }
        top += height + space::section_gap();
    };
    row(0..1, width);
    for start in (1..16).step_by(columns) {
        row(start..(start + columns).min(16), column);
    }
    row(16..17, width);
    sections
}

fn section(ui: &mut Ui, rect: Rect, index: usize, demo: &mut components::Demo) {
    let mut section = ui.new_child(
        UiBuilder::new()
            .id_salt(index)
            .max_rect(rect)
            .layout(Layout::top_down(Align::Min)),
    );
    section.spacing_mut().item_spacing = vec2(space::gap(), space::gap());
    let viewport_clip = ui.clip_rect();
    Panel::titled(titles()[index]).show(&mut section, |ui| {
        // Example overlays can cast a shadow through the panel padding.
        ui.set_clip_rect(viewport_clip);
        components::show(ui, index, demo)
    });
    let header = Rect::from_min_size(
        rect.min + vec2(space::panel_inset() + 1.0, 1.0),
        vec2(
            rect.width() - 2.0 * (space::panel_inset() + 1.0),
            size::panel_header() - 1.0,
        ),
    );
    let header = Rect::from_center_size(
        header.center(),
        vec2(header.width(), header.height().max(size::control())),
    );
    let mut header = ui.new_child(
        UiBuilder::new()
            .id_salt((index, "actions"))
            .max_rect(header)
            .layout(Layout::right_to_left(Align::Center)),
    );
    if index == 0 {
        header.add(Button::primary(tr!("page.new_project")).with_icon(icon::PLUS));
    } else if [5, 8, 9, 11, 12, 13].contains(&index) {
        header.add(Badge::new(tr!("page.new")).fit());
    }
}

fn rail(ui: &mut Ui, gallery: &mut Gallery, screen: Rect) {
    let rect = Rect::from_min_max(
        screen.min + vec2(12.0, 0.0),
        pos2(screen.left() + RAIL - 13.0, screen.bottom()),
    );
    let mut rail = ui.new_child(
        UiBuilder::new()
            .id_salt("rail")
            .max_rect(rect)
            .layout(Layout::top_down(Align::Min)),
    );
    egui::ScrollArea::vertical()
        .id_salt("rail_scroll")
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
        .show(&mut rail, |ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            for (text, font, height, ink) in [
                (
                    tr!("page.studio"),
                    theme::title(),
                    size::app_bar(),
                    color::ink(),
                ),
                (
                    tr!("panel.components"),
                    theme::small(),
                    size::control(),
                    color::ink_3(),
                ),
            ] {
                let (rect, _) = ui
                    .allocate_exact_size(vec2(ui.available_width(), height), egui::Sense::hover());
                label(ui, rect.shrink2(vec2(10.0, 0.0)), text, font, ink);
            }
            for (index, title) in titles().into_iter().enumerate() {
                if ui
                    .add(NavRow::new(title, gallery.selected == index))
                    .clicked()
                {
                    gallery.selected = index;
                    gallery.scroll_to = Some(index);
                }
            }
        });
}

fn toolbar(ui: &mut Ui, gallery: &mut Gallery, screen: Rect) {
    let bar = Rect::from_min_max(
        pos2(screen.left() + RAIL, screen.top()),
        pos2(screen.right(), screen.top() + size::app_bar()),
    );
    ui.painter().rect_filled(bar, 0.0, color::ground());
    if gallery.style != 2 {
        ui.painter().hline(
            bar.x_range(),
            bar.bottom() - 0.5,
            theme::hairline(ui.ctx(), color::border()),
        );
    }
    let mut bar = ui.new_child(
        UiBuilder::new()
            .id_salt("toolbar")
            .max_rect(bar.shrink2(vec2(MARGIN, 0.0)))
            .layout(Layout::right_to_left(Align::Center)),
    );
    bar.spacing_mut().item_spacing.x = 12.0;
    bar.add(Segmented::new(
        &mut gallery.theme,
        &[tr!("page.dark"), tr!("page.light")],
    ));
    bar.add(
        Dropdown::new(&mut gallery.font, &[verbatim("Figtree"), verbatim("Inter")])
            .width(160.0)
            .fill(color::panel()),
    );
    bar.add(Segmented::new(
        &mut gallery.style,
        &[tr!("page.current"), tr!("page.line"), tr!("page.soft")],
    ));
    let rect = bar.available_rect_before_wrap();
    label(
        &mut bar,
        rect,
        tr!("page.component_styles"),
        theme::headline(),
        color::ink(),
    );
}

pub fn label(ui: &mut Ui, rect: Rect, text: &str, font: FontId, color: Color32) {
    let mut job = egui::text::LayoutJob::simple_singleline(text.into(), font.clone(), color);
    if font.family == theme::semibold() {
        job.sections[0].format.extra_letter_spacing = -0.01 * font.size;
    }
    let galley = ui.fonts_mut(|fonts| fonts.layout_job(job));
    let pos = pos2(rect.left(), rect.center().y - galley.mesh_bounds.center().y);
    ui.painter().galley(pos, galley, color);
    let response = ui.interact(rect, ui.id().with(("label", text)), egui::Sense::hover());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, true, text));
}

pub fn show(ui: &mut Ui, gallery: &mut Gallery) {
    let screen = ui.ctx().content_rect();
    ui.painter().rect_filled(screen, 0.0, color::ground());
    gallery.demo.inter = gallery.font == 1;
    if gallery.style == 0 {
        let rail_rect = Rect::from_min_size(screen.min, vec2(RAIL, screen.height()));
        ui.painter().rect_filled(rail_rect, 0.0, color::panel());
    }
    if gallery.style != 2 {
        ui.painter().vline(
            screen.left() + RAIL - 0.5,
            screen.y_range(),
            theme::hairline(ui.ctx(), color::border()),
        );
    }
    rail(ui, gallery, screen);
    gallery.sections = grid(
        ui,
        (screen.width() - RAIL - 2.0 * MARGIN).max(400.0),
        &gallery.demo,
    );
    let content = Rect::from_min_max(
        pos2(screen.left() + RAIL, screen.top() + size::app_bar()),
        screen.max,
    );
    let mut body = ui.new_child(
        UiBuilder::new()
            .id_salt("document")
            .max_rect(content)
            .layout(Layout::top_down(Align::Min)),
    );
    let height = gallery.sections.last().unwrap().bottom() + 48.0 - size::app_bar();
    let mut scroll = egui::ScrollArea::vertical()
        .id_salt("page_scroll")
        .auto_shrink(false)
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden);
    if let Some(index) = gallery.scroll_to.take() {
        scroll = scroll.vertical_scroll_offset(
            (gallery.sections[index].top() - size::app_bar() - 16.0).max(0.0),
        );
    }
    let output = scroll.show(&mut body, |ui| {
        let offset = ui.cursor().top() - size::app_bar();
        for (index, rect) in gallery.sections.iter().enumerate() {
            section(
                ui,
                rect.translate(vec2(screen.left(), offset)),
                index,
                &mut gallery.demo,
            );
        }
        ui.allocate_space(vec2(content.width(), height));
    });
    gallery.scroll = output.state.offset.y;
    toolbar(ui, gallery, screen);
}
