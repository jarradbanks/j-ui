//! Content containers: a table of selectable rows and a card.

use super::*;

/// One column of a [`Table`].
#[derive(Clone, Copy)]
pub struct Column<'a> {
    title: &'a str,
    /// `None` shares the width the fixed columns leave.
    width: Option<f32>,
    right: bool,
}

impl<'a> Column<'a> {
    /// A column sharing the width the fixed columns leave.
    pub fn fill(title: &'a str) -> Self {
        Self {
            title,
            width: None,
            right: false,
        }
    }

    pub fn fixed(title: &'a str, width: f32) -> Self {
        Self {
            width: Some(width),
            ..Self::fill(title)
        }
    }

    /// Right-aligns the title and cells, for numbers.
    pub fn right(mut self) -> Self {
        self.right = true;
        self
    }
}

/// A table of 40px rows under Small Ink 3 column titles, with 1px Border rules between
/// them. Clicking a row sets `selected`; a `selected` past the rows selects none.
#[must_use = "call `.show(ui, ..)`"]
pub struct Table<'a> {
    columns: &'a [Column<'a>],
    selected: &'a mut usize,
}

impl<'a> Table<'a> {
    pub fn new(columns: &'a [Column<'a>], selected: &'a mut usize) -> Self {
        Self { columns, selected }
    }

    /// One row per name in `rows`, which screen readers announce; `cell(ui, row, column)`
    /// fills each cell. The response is `changed` when the selection moved.
    pub fn show(
        self,
        ui: &mut Ui,
        rows: &[&str],
        mut cell: impl FnMut(&mut Ui, usize, usize),
    ) -> Response {
        const HEIGHT: f32 = size::CONTROL + space::S4;
        const PAD: f32 = space::S8;
        const GAP: f32 = space::S12;
        let size = vec2(ui.available_width(), HEIGHT * (rows.len() + 1) as f32);
        let (rect, mut response) = allocate(ui, size, State::Live, Sense::hover());
        let fixed: f32 = self.columns.iter().filter_map(|c| c.width).sum();
        let fills = self.columns.iter().filter(|c| c.width.is_none()).count();
        let gaps = GAP * self.columns.len().saturating_sub(1) as f32;
        let share = ((rect.width() - 2.0 * PAD - gaps - fixed) / fills.max(1) as f32).max(0.0);
        let mut x = rect.left() + PAD;
        let cells: Vec<egui::Rangef> = self
            .columns
            .iter()
            .map(|c| {
                let range = egui::Rangef::new(x, x + c.width.unwrap_or(share));
                x = range.max + GAP;
                range
            })
            .collect();
        let rule = hairline(ui.ctx(), color::border());
        for row in 0..=rows.len() {
            let top = rect.top() + HEIGHT * row as f32;
            let line = Rect::from_min_size(pos2(rect.left(), top), vec2(rect.width(), HEIGHT));
            // Below the cells, so the fill is known once the row has been hovered.
            let fill = ui.painter().add(Shape::Noop);
            if row < rows.len() {
                let y = line.bottom() - rule.width / 2.0;
                ui.painter().hline(line.x_range(), y, rule);
            }
            for (i, (column, span)) in self.columns.iter().zip(&cells).enumerate() {
                let slot = Rect::from_x_y_ranges(*span, line.y_range());
                let layout = match column.right {
                    true => egui::Layout::right_to_left(egui::Align::Center),
                    false => egui::Layout::left_to_right(egui::Align::Center),
                };
                let builder = egui::UiBuilder::new()
                    .id_salt((row, i))
                    .max_rect(slot)
                    .layout(layout);
                // A child, not a scope: a scope would move the cursor into the table.
                let mut ui = ui.new_child(builder);
                ui.set_clip_rect(slot.intersect(ui.clip_rect()));
                if row == 0 {
                    let title = egui::RichText::new(column.title)
                        .font(theme::small())
                        .color(color::ink_3());
                    ui.add(egui::Label::new(title).truncate().selectable(false));
                } else {
                    cell(&mut ui, row - 1, i);
                }
            }
            let Some(index) = row.checked_sub(1) else {
                continue;
            };
            // After the cells, so the row takes clicks on them.
            let r = ui.interact(line, response.id.with(index), Sense::click());
            let on = index == *self.selected;
            r.widget_info(|| {
                WidgetInfo::selected(WidgetType::SelectableLabel, r.enabled(), on, rows[index])
            });
            if r.clicked() && !on {
                *self.selected = index;
                response.mark_changed();
            }
            // Keyboard focus shows the hover fill, never a ring.
            let target = if on || r.hovered() || r.has_focus() {
                color::selection_fill()
            } else {
                Color32::TRANSPARENT
            };
            let target = motion::color(ui.ctx(), r.id.with("fill"), target);
            let corner = CornerRadius::same(radius::SMALL);
            ui.painter()
                .set(fill, Shape::rect_filled(line, corner, target));
        }
        response
    }
}

/// A Raised, 16px-radius tile for one item of a collection. The content runs edge to edge,
/// so it sets its own padding and can open with artwork.
#[must_use = "call `.show(ui, ..)`"]
#[derive(Default)]
pub struct Card;

impl Card {
    pub fn new() -> Self {
        Self
    }

    pub fn show<R>(
        self,
        ui: &mut Ui,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> egui::InnerResponse<R> {
        // Top to bottom whatever row or grid the card sits in.
        let output = egui::Frame::new()
            .fill(color::raised())
            .inner_margin(1.0)
            .corner_radius(radius::PANEL)
            .show(ui, |ui| ui.vertical(add_contents).inner);
        ui.painter().add(border(
            ui,
            output.response.rect,
            radius::PANEL,
            color::surface_outline(),
        ));
        output
    }
}
