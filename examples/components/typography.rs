//! Type specimens and the spacing scale.

use super::super::page;
use super::*;

pub fn typography_height(inter: bool) -> f32 {
    row_heights(inter).into_iter().sum::<f32>() + 6.0 * space::gap()
}

// Baseline alignment includes each font’s normal line box beside the 1.45-leading weight label.
fn row_heights(inter: bool) -> [f32; 7] {
    let line = page::line_height(theme::body().size);
    if theme::body().size == 12.0 {
        let heights = [
            if inter { 26.0 } else { 27.0 },
            line + 3.0,
            line + 1.0,
            line,
            line,
            line,
            line + 3.0,
        ];
        return heights;
    }
    [
        34.0,
        line + if inter { 4.0 } else { 5.0 },
        line + 1.0,
        line,
        line,
        line,
        line + 1.0,
    ]
}

pub fn typography(ui: &mut Ui, inter: bool) {
    let specimens = [
        (
            tr!("page.display"),
            theme::display(),
            tr!("page.semibold"),
            0.0,
        ),
        (
            tr!("page.headline"),
            theme::headline(),
            tr!("page.semibold"),
            0.0,
        ),
        (tr!("page.title"), theme::title(), tr!("page.semibold"), 0.0),
        (
            tr!("page.body_strong"),
            theme::body_strong(),
            tr!("page.medium"),
            1.0,
        ),
        (
            tr!("page.body_regular"),
            theme::body(),
            tr!("page.regular"),
            1.0,
        ),
        (
            tr!("page.small_caption"),
            theme::small(),
            tr!("page.regular"),
            3.0,
        ),
        (
            verbatim("00:01:24:08"),
            theme::timecode(),
            tr!("page.medium"),
            0.0,
        ),
    ];
    let mut top = ui.cursor().top();
    let heights = row_heights(inter);
    for (index, (text, font, weight, inset)) in specimens.into_iter().enumerate() {
        let height = heights[index];
        let row = Rect::from_min_size(
            pos2(ui.max_rect().left(), top),
            vec2(ui.available_width(), height),
        );
        let ink = if index == 5 {
            color::ink_3()
        } else {
            color::ink()
        };
        let mut child = ui.new_child(
            UiBuilder::new()
                .id_salt(index)
                .max_rect(row.translate(vec2(0.0, inset)))
                .layout(Layout::left_to_right(Align::Min)),
        );
        let text = theme::text(text, font).color(ink);
        child.add(egui::Label::new(text).selectable(false));
        let mut right = ui.new_child(
            UiBuilder::new()
                .id_salt((index, "weight"))
                .max_rect(row)
                .layout(Layout::right_to_left(Align::Max)),
        );
        right.label(muted(weight));
        top += height + space::gap();
    }
}

pub fn spacing(ui: &mut Ui) {
    for (name, width) in [
        (verbatim("space::S4"), 4.0),
        (verbatim("space::S8"), 8.0),
        (verbatim("space::S12"), 12.0),
        (verbatim("space::S16"), 16.0),
        (verbatim("space::S24"), 24.0),
        (verbatim("space::S32"), 32.0),
    ] {
        let (row, _) =
            ui.allocate_exact_size(vec2(ui.available_width(), 16.0), egui::Sense::hover());
        page::label(ui, row, name, theme::body(), color::ink());
        let bar = Rect::from_min_size(
            pos2(row.left() + 124.0, row.center().y - 4.0),
            vec2(width, 8.0),
        );
        ui.painter().rect_filled(bar, 2.0, color::blue());
    }
    ui.label(muted(tr!("page.spacing_help")));
}
