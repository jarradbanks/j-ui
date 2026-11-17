//! Responsive project cards with a common footer baseline per row.

use super::*;
use j_ui::widgets::Card;

fn projects() -> [(Color32, &'static str, &'static str, &'static str); 3] {
    [
        (
            color::CLIP_VIOLET,
            tr!("page.interview_cut"),
            tr!("page.interview_description"),
            tr!("page.edited_two_hours"),
        ),
        (
            color::CLIP_PLUM,
            tr!("page.harbour_shoot"),
            tr!("page.harbour_description"),
            tr!("page.edited_yesterday"),
        ),
        (
            color::CLIP_GREEN,
            tr!("page.title_sequence"),
            tr!("page.title_description"),
            tr!("page.edited_may"),
        ),
    ]
}

fn grid(width: f32) -> (usize, f32) {
    let columns = ((width + 16.0) / 336.0).floor().max(1.0) as usize;
    (
        columns,
        (width - 16.0 * (columns - 1) as f32) / columns as f32,
    )
}

fn row_height(ui: &Ui, row: &[(Color32, &str, &str, &str)], width: f32) -> f32 {
    let description = row
        .iter()
        .map(|project| {
            super::super::page::paragraph_height(ui, project.2, width - 2.0 * space::inset() - 2.0)
        })
        .fold(0.0, f32::max);
    119.0 + 2.0 * space::inset() + 2.0 * size::control() + description
}

pub fn projects_height(ui: &Ui, width: f32) -> f32 {
    let (columns, width) = grid(width);
    let projects = projects();
    let rows = projects.chunks(columns);
    rows.map(|row| row_height(ui, row, width) + 16.0)
        .sum::<f32>()
        - 16.0
}

pub fn recent_projects(ui: &mut Ui) {
    let (columns, width) = grid(ui.available_width());
    let mut top = ui.cursor().top();
    for (row, projects) in projects().chunks(columns).enumerate() {
        let height = row_height(ui, projects, width);
        for (column, (fill, title, description, edited)) in projects.iter().enumerate() {
            let left = ui.max_rect().left() + column as f32 * (width + 16.0);
            let rect = Rect::from_min_size(pos2(left, top), vec2(width, height));
            let mut tile = ui.new_child(UiBuilder::new().id_salt((row, column)).max_rect(rect));
            Card::new().show(&mut tile, |ui| {
                ui.set_min_size(rect.size() - vec2(2.0, 2.0));
                let inside = rect.shrink(1.0);
                let thumb = Rect::from_min_size(inside.min, vec2(inside.width(), 96.0));
                let corner = egui::CornerRadius {
                    nw: radius::panel(),
                    ne: radius::panel(),
                    sw: 0,
                    se: 0,
                };
                ui.painter().rect_filled(thumb, corner, *fill);
                let head = Rect::from_min_size(
                    thumb.left_bottom() + vec2(space::inset(), space::inset()),
                    vec2(inside.width() - 2.0 * space::inset(), size::control()),
                );
                super::super::page::label(ui, head, title, theme::title(), color::ink());
                let mut buttons = ui.new_child(
                    UiBuilder::new()
                        .id_salt("more")
                        .max_rect(head)
                        .layout(Layout::right_to_left(Align::Center)),
                );
                buttons.add(Button::icon(icon::ELLIPSIS, tr!("panel.more")));
                let desc = Rect::from_min_max(
                    head.left_bottom() + vec2(0.0, 4.0),
                    pos2(
                        head.right(),
                        rect.bottom() - 18.0 - size::control() - space::inset(),
                    ),
                );
                let mut text = ui.new_child(UiBuilder::new().id_salt("description").max_rect(desc));
                text.label(muted(description));
                let foot = Rect::from_min_max(
                    pos2(head.left(), rect.bottom() - 9.0 - size::control()),
                    pos2(head.right(), rect.bottom() - 9.0),
                );
                if color::control_outline() != Color32::TRANSPARENT {
                    ui.painter().hline(
                        inside.x_range(),
                        foot.top() - 8.5,
                        theme::hairline(ui.ctx(), color::border()),
                    );
                }
                super::super::page::label(ui, foot, edited, theme::small(), color::ink_3());
                let mut buttons = ui.new_child(
                    UiBuilder::new()
                        .id_salt("actions")
                        .max_rect(foot)
                        .layout(Layout::right_to_left(Align::Center)),
                );
                buttons.spacing_mut().item_spacing.x = 8.0;
                buttons.add(Button::secondary(tr!("page.open")));
                buttons.add(Button::ghost(tr!("page.share")));
            });
        }
        top += height + 16.0;
    }
}
