//! The panel frame every dock panel uses: a borderless 16px-radius Panel surface (a 1px
//! Blue border when focused) and one 48px header holding a title or tabs.

use egui::{CornerRadius, Rect, Sense, StrokeKind, Ui, UiBuilder, Vec2, pos2, vec2};

use crate::theme::{self, hairline};
use crate::tokens::{color, radius, size, space};
use crate::widgets::{Button, fit_text, paint_fit, tab_strip, text_pos};

/// What a panel's header shows.
pub enum Header<'a> {
    /// Content only; the enclosing dock supplies the title strip.
    None,
    /// A row reserved for panel tools, below the dock's title strip.
    Toolbar,
    /// A single view: its title sits 16px in.
    Title(&'a str),
    /// Several views as underlined tabs; clicking one sets `active`.
    Tabs {
        labels: &'a [&'a str],
        active: &'a mut usize,
    },
}

/// What happened in a panel this frame.
pub struct PanelResponse<R> {
    pub inner: R,
    /// The header action clicked this frame, by index.
    pub action: Option<usize>,
    pub response: egui::Response,
}

/// A dock panel filling the space it is given.
#[must_use = "call `.show(ui, ..)`"]
pub struct Panel<'a> {
    header: Header<'a>,
    actions: &'a [(&'static str, &'a str)],
    on: &'a [bool],
    focused: bool,
    wide: bool,
    embedded: bool,
}

impl<'a> Panel<'a> {
    pub fn body() -> Self {
        Self::new(Header::None)
    }

    pub fn toolbar() -> Self {
        Self::new(Header::Toolbar)
    }

    pub fn titled(title: &'a str) -> Self {
        Self::new(Header::Title(title))
    }

    pub fn tabbed(labels: &'a [&'a str], active: &'a mut usize) -> Self {
        Self::new(Header::Tabs { labels, active })
    }

    pub fn new(header: Header<'a>) -> Self {
        Self {
            header,
            actions: &[],
            on: &[],
            focused: false,
            wide: false,
            embedded: false,
        }
    }

    /// Icon buttons at the header's right: a glyph from [`icon`] and its accessible name.
    pub fn actions(mut self, actions: &'a [(&'static str, &'a str)]) -> Self {
        self.actions = actions;
        self
    }

    /// Which of the actions are toggles that are on, by index.
    pub fn on(mut self, on: &'a [bool]) -> Self {
        self.on = on;
        self
    }

    /// Tab labels run to their full width instead of eliding at 160px each.
    pub fn wide(mut self, wide: bool) -> Self {
        self.wide = wide;
        self
    }

    /// Keyboard focus: draws a Blue border.
    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    /// The dock paints the surrounding surface and focus border.
    pub fn embedded(mut self, embedded: bool) -> Self {
        self.embedded = embedded;
        self
    }

    pub fn show<R>(self, ui: &mut Ui, content: impl FnOnce(&mut Ui) -> R) -> PanelResponse<R> {
        let ppp = ui.pixels_per_point();
        // Round inward, so the border stays inside a clip rect on fractional pixels.
        let r = ui.available_rect_before_wrap();
        let up = |v: f32| (v * ppp).ceil() / ppp;
        let down = |v: f32| (v * ppp).floor() / ppp;
        let rect = Rect::from_min_max(
            pos2(up(r.left()), up(r.top())),
            pos2(down(r.right()), down(r.bottom())),
        );
        let response = ui.allocate_rect(rect, Sense::hover());
        let corner = CornerRadius::same(radius::panel());
        let painter = ui.painter().clone();
        if !self.embedded {
            painter.rect_filled(rect, corner, color::panel_fill());
            painter.rect_stroke(
                rect,
                corner,
                hairline(ui.ctx(), color::panel_outline()),
                StrokeKind::Inside,
            );
        }

        let height = if matches!(self.header, Header::None) {
            0.0
        } else {
            size::panel_header()
        };
        let inside = rect.shrink(1.0);
        let header = Rect::from_min_size(inside.min, vec2(inside.width(), height));

        if theme::component_style() == theme::ComponentStyle::Current {
            painter.hline(
                header.x_range(),
                header.bottom() - 0.5,
                hairline(ui.ctx(), color::border()),
            );
        }
        let mut action = None;
        let mut right = header.right() - space::panel_inset();
        for (i, (glyph, name)) in self.actions.iter().enumerate().rev() {
            let r = Rect::from_min_size(
                pos2(
                    right - size::control(),
                    header.center().y - size::control() / 2.0,
                ),
                Vec2::splat(size::control()),
            );
            let on = self.on.get(i).copied().unwrap_or(false);
            let b = ui.put(r, Button::icon(glyph, name).on(on));
            if b.on_hover_text(*name).clicked() {
                action = Some(i);
            }
            right = r.left() - space::S8;
        }

        match self.header {
            Header::None | Header::Toolbar => {}
            Header::Title(title) => {
                let left = header.left() + space::panel_inset();
                let galley = fit_text(ui, title, theme::panel_title(), color::ink(), right - left);
                let pos = text_pos(&galley, left, header.center().y);
                paint_fit(ui, pos, galley, color::ink());
            }
            Header::Tabs { labels, active } => {
                let max = if self.wide {
                    f32::INFINITY
                } else {
                    size::TAB_MAX
                };
                let ends = (header.left() + space::panel_inset(), right);
                tab_strip(ui, ends, header, labels, active, max);
            }
        }

        let body = Rect::from_x_y_ranges(
            inside.left() + space::panel_inset()..=inside.right() - space::panel_inset(),
            header.bottom() + space::panel_inset().max(12.0)
                ..=inside.bottom() - space::panel_inset(),
        );
        let mut child = ui.new_child(UiBuilder::new().max_rect(body));
        child.set_clip_rect(body.intersect(ui.clip_rect()));
        let inner = content(&mut child);

        if self.focused && !self.embedded {
            let border = hairline(ui.ctx(), color::blue());
            painter.rect_stroke(rect, corner, border, StrokeKind::Inside);
        }
        PanelResponse {
            inner,
            action,
            response,
        }
    }
}
