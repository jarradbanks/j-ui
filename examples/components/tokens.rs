//! The live correspondence between CSS variables and shared component tokens.

use super::*;

pub fn token_map(ui: &mut Ui) {
    let px = |value: f32| format!("{value}px");
    let hex = |color: Color32| {
        if color.a() == 0 {
            return verbatim("transparent").to_owned();
        }
        format!("#{:02X}{:02X}{:02X}", color.r(), color.g(), color.b())
    };
    let rows = [
        (
            verbatim("radius::SMALL"),
            "--r-small",
            px(radius::small() as f32),
        ),
        (
            verbatim("radius::CONTROL"),
            "--r-control",
            px(radius::control() as f32),
        ),
        (
            verbatim("radius::PANEL"),
            "--r-panel",
            px(radius::panel() as f32),
        ),
        (
            verbatim("radius::DIALOG"),
            "--r-dialog",
            px(radius::dialog() as f32),
        ),
        (verbatim("size::CONTROL"), "--control", px(size::control())),
        (verbatim("size::HEADER"), "--header", px(size::header())),
        (verbatim("size::APP_BAR"), "--bar", px(size::app_bar())),
        (verbatim("size::ICON"), "--icon", px(size::icon())),
        (
            verbatim("SELECTION_DIAMETER"),
            "--check",
            px(size::selection()),
        ),
        (verbatim("space::INSET"), "--inset", px(space::inset())),
        (verbatim("font::BODY"), "--body", px(theme::body().size)),
        (verbatim("font::SMALL"), "--small", px(theme::small().size)),
        (verbatim("font::TITLE"), "--title", px(theme::title().size)),
        (
            verbatim("font::HEADLINE"),
            "--headline",
            px(theme::headline().size),
        ),
        (
            verbatim("font::DISPLAY"),
            "--display",
            px(theme::display().size),
        ),
        (
            tr!("page.panel_fill"),
            "--panel-fill",
            hex(color::panel_fill()),
        ),
        (
            tr!("page.panel_stroke"),
            "--panel-border",
            hex(color::panel_outline()),
        ),
        (
            tr!("page.input_fill"),
            "--field-fill",
            hex(color::field_fill()),
        ),
        (
            tr!("page.input_stroke"),
            "--field-border",
            hex(color::control_outline()),
        ),
        (
            tr!("page.button_fill"),
            "--btn-fill",
            hex(color::button_fill()),
        ),
        (
            tr!("page.button_stroke"),
            "--btn-border",
            hex(color::control_outline()),
        ),
        (
            tr!("page.segmented_track"),
            "--seg-track",
            hex(color::segment_track()),
        ),
        (
            tr!("page.segmented_fill"),
            "--seg-on",
            hex(color::segment_fill()),
        ),
        (
            tr!("page.card_fill"),
            "--card-fill",
            hex(color::card_fill()),
        ),
        (
            tr!("page.selected_fill"),
            "--sel",
            hex(color::selection_fill()),
        ),
        (
            tr!("page.region_hairline"),
            "--edge",
            hex(color::control_outline()),
        ),
        (
            verbatim("Visuals::popup_shadow"),
            "--shadow",
            verbatim("0 8px 24px rgba(0,0,0,.4)").to_owned(),
        ),
    ];
    let columns = ((ui.available_width() + 24.0) / 524.0).floor().max(1.0) as usize;
    let width = (ui.available_width() - 24.0 * (columns - 1) as f32) / columns as f32;
    let origin = ui.cursor().min;
    for (index, (name, css, value)) in rows.into_iter().enumerate() {
        let top = origin
            + vec2(
                (index % columns) as f32 * (width + 24.0),
                (index / columns) as f32 * size::control(),
            );
        let row = Rect::from_min_size(top, vec2(width, size::control()));
        ui.painter().hline(
            row.x_range(),
            row.bottom() - 0.5,
            theme::hairline(ui.ctx(), color::border()),
        );
        for (column, (x, text, ink)) in [
            (0.0, name, color::ink()),
            (208.0, css, color::ink()),
            (346.0, &value, color::ink_3()),
        ]
        .into_iter()
        .enumerate()
        {
            let cell = Rect::from_min_max(row.min + vec2(x, 0.0), row.max);
            let mut cell_ui =
                ui.new_child(UiBuilder::new().id_salt((index, column)).max_rect(cell));
            super::super::page::label(&mut cell_ui, cell, text, theme::body(), ink);
        }
    }
}
