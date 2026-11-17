//! Moving between views: underlined tabs and a rail's rows.

use super::*;

/// Lays out underlined tabs along `row` from `left`, each as wide as its label up to `max`
/// and 20px apart. Those that don't fit before `right` move into a chevron menu at the end.
/// True when the active tab changed.
pub(crate) fn tab_strip(
    ui: &mut Ui,
    (left, right): (f32, f32),
    row: Rect,
    labels: &[&str],
    active: &mut usize,
    max: f32,
) -> bool {
    const GAP: f32 = 20.0;
    let font = theme::body_strong();
    let widths: Vec<f32> = labels
        .iter()
        .map(|label| text_width(ui, label, &font).min(max).ceil())
        .collect();
    let needed = |n: usize| widths[..n].iter().sum::<f32>() + GAP * n.saturating_sub(1) as f32;
    let avail = (right - left).max(0.0);
    let menu = size::CONTROL + space::S8;
    let mut shown = labels.len();
    if needed(shown) > avail {
        while shown > 1 && needed(shown) + menu > avail {
            shown -= 1;
        }
    }
    let overflow = shown < labels.len();
    let was = *active;
    let mut x = left;
    for (i, label) in labels.iter().take(shown).enumerate() {
        // The last tab left takes what room there is, however short.
        let reserved = if overflow { menu } else { 0.0 };
        let width = widths[i].min((right - reserved - x).max(0.0));
        let tab = Rect::from_x_y_ranges(x..=x + width, row.y_range());
        x = tab.right() + GAP;
        let r = ui.interact(tab, ui.id().with(("tab", i)), Sense::click());
        let on = i == *active;
        r.widget_info(|| {
            WidgetInfo::selected(WidgetType::SelectableLabel, r.enabled(), on, *label)
        });
        if r.clicked() {
            *active = i;
        }
        // Keyboard focus lifts the label to Ink, never a ring.
        let ink = match () {
            _ if !r.enabled() => color::ink_4(),
            _ if on || r.has_focus() => color::ink(),
            _ => color::ink_3(),
        };
        let ink = motion::color(ui.ctx(), r.id.with("ink"), ink);
        let edge = if on {
            color::ink()
        } else {
            Color32::TRANSPARENT
        };
        let edge = motion::color(ui.ctx(), r.id.with("edge"), edge);
        let galley = fit_text(ui, label, font.clone(), ink, width);
        let _ = hover_full(r, &galley);
        paint_left(ui, tab.left(), row.center().y, galley, ink);
        let underline = tab.with_min_y(row.bottom() - 2.0);
        ui.painter().rect_filled(underline, 0.0, edge);
    }
    if overflow {
        let slot = Rect::from_min_size(
            pos2(x - GAP + space::S8, row.center().y - size::CONTROL / 2.0),
            Vec2::splat(size::CONTROL),
        );
        let name = tr!(CATALOG, "panel.more_tabs");
        let r = place(ui, slot, Button::icon(icon::CHEVRON_DOWN, name));
        egui::Popup::menu(&r).gap(space::S4).show(|ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            for (i, label) in labels.iter().enumerate().skip(shown) {
                if ui.add(MenuRow::option(label, i == *active)).clicked() {
                    *active = i;
                }
            }
        });
    }
    *active != was
}

/// A row of underlined tabs on a 1px Border rule; clicking one sets `active`.
#[must_use = "add it with `ui.add(..)`"]
pub struct Tabs<'a> {
    labels: &'a [&'a str],
    active: &'a mut usize,
}

impl<'a> Tabs<'a> {
    pub fn new(labels: &'a [&'a str], active: &'a mut usize) -> Self {
        Self { labels, active }
    }
}

impl Widget for Tabs<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let size = vec2(ui.available_width(), size::HEADER);
        let (rect, mut response) = allocate(ui, size, State::Live, Sense::hover());
        let rule = hairline(ui.ctx(), color::border());
        ui.painter()
            .hline(rect.x_range(), rect.bottom() - rule.width / 2.0, rule);
        let ends = (rect.left(), rect.right());
        if tab_strip(ui, ends, rect, self.labels, self.active, f32::INFINITY) {
            response.mark_changed();
        }
        response
    }
}

/// One destination in a navigation rail: a full-width 36px row, filled while hovered or
/// `selected`.
#[must_use = "add it with `ui.add(..)`"]
pub struct NavRow<'a> {
    label: &'a str,
    selected: bool,
}

impl<'a> NavRow<'a> {
    pub fn new(label: &'a str, selected: bool) -> Self {
        Self { label, selected }
    }
}

impl Widget for NavRow<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        const PAD: f32 = 10.0;
        let size = vec2(ui.available_width(), size::CONTROL);
        let (rect, response) = allocate(ui, size, State::Live, Sense::click());
        response.widget_info(|| {
            let kind = WidgetType::SelectableLabel;
            WidgetInfo::selected(kind, response.enabled(), self.selected, self.label)
        });
        // Keyboard focus shows the hover fill, never a ring.
        let fill = if self.selected || response.hovered() || response.has_focus() {
            color::selection_fill()
        } else {
            Color32::TRANSPARENT
        };
        let fill = motion::color(ui.ctx(), response.id.with("fill"), fill);
        ui.painter()
            .rect_filled(rect, CornerRadius::same(radius::CONTROL), fill);
        let (font, ink) = match self.selected {
            true => (theme::body_strong(), color::ink()),
            false => (theme::body(), color::ink_2()),
        };
        let ink = if response.enabled() {
            ink
        } else {
            color::ink_4()
        };
        let ink = motion::color(ui.ctx(), response.id.with("ink"), ink);
        let label = fit_text(ui, self.label, font, ink, rect.width() - 2.0 * PAD);
        let response = hover_full(response, &label);
        paint_left(ui, rect.left() + PAD, rect.center().y, label, ink);
        response
    }
}
