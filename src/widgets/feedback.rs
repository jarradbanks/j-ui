//! Status: badges, banners, toasts and progress.

use super::*;
use egui::Label;

/// What a status means, which picks its soft fill and its ink.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tone {
    #[default]
    Neutral,
    Info,
    Success,
    Warning,
    Danger,
}

impl Tone {
    /// The tone's (fill, ink).
    fn colors(self) -> (Color32, Color32) {
        match self {
            Self::Neutral => (color::neutral_fill(), color::ink_2()),
            Self::Info => (color::blue_soft(), color::light_blue()),
            Self::Success => (color::green_soft(), color::green()),
            Self::Warning => (color::amber_soft(), color::amber()),
            Self::Danger => (color::red_soft(), color::red()),
        }
    }
}

/// A 24px status chip. Chips are at least 72px wide so a column of them lines up; `fit`
/// hugs the label, for a lone tag.
#[must_use = "add it with `ui.add(..)`"]
pub struct Badge<'a> {
    label: &'a str,
    tone: Tone,
    min_width: f32,
}

impl<'a> Badge<'a> {
    pub fn new(label: &'a str) -> Self {
        Self {
            label,
            tone: Tone::Neutral,
            min_width: 72.0,
        }
    }

    pub fn tone(mut self, tone: Tone) -> Self {
        self.tone = tone;
        self
    }

    pub fn fit(mut self) -> Self {
        self.min_width = 0.0;
        self
    }
}

impl Widget for Badge<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let (fill, ink) = self.tone.colors();
        let font = theme::small_strong();
        let width = (text_width(ui, self.label, &font) + 20.0).max(self.min_width);
        let size = vec2(width.ceil(), size::badge());
        let (rect, response) = allocate(ui, size, State::Live, Sense::hover());
        response.widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, self.label));
        ui.painter()
            .rect_filled(rect, CornerRadius::same(radius::small()), fill);
        paint_text(
            ui,
            rect.center(),
            Align2::CENTER_CENTER,
            self.label,
            font,
            ink,
        );
        response
    }
}

/// A slider-weight track showing how much of a task is done.
#[must_use = "add it with `ui.add(..)`"]
pub struct Progress<'a> {
    fraction: f32,
    name: &'a str,
}

impl<'a> Progress<'a> {
    /// `fraction` runs 0 to 1; `name` is for screen readers.
    pub fn new(fraction: f32, name: &'a str) -> Self {
        Self {
            fraction: fraction.clamp(0.0, 1.0),
            name,
        }
    }
}

impl Widget for Progress<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let size = vec2(ui.available_width(), size::slider_track());
        let (rect, response) = allocate(ui, size, State::Live, Sense::hover());
        response.widget_info(|| {
            let mut info = WidgetInfo::labeled(WidgetType::ProgressIndicator, true, self.name);
            info.value = Some(f64::from(self.fraction * 100.0).floor());
            info
        });
        let round = size::slider_track() / 2.0;
        ui.painter()
            .rect_filled(rect, round, color::border_strong());
        let done = rect.with_max_x(rect.left() + rect.width() * self.fraction);
        ui.painter().rect_filled(done, round, color::blue());
        response
    }
}

/// The frame and row a [`Banner`] and a [`Toast`] share: icon, wrapping message, then an
/// optional ghost action and dismiss button at the right.
struct Notice<'a> {
    fill: Color32,
    corner: u8,
    /// A border and the overlay shadow, for a notice that floats.
    floating: bool,
    /// Padding: left, right, top and bottom.
    pad: [f32; 3],
    icon: &'static str,
    icon_ink: Color32,
    message: &'a str,
    action: Option<&'a str>,
    dismiss: Option<&'a str>,
}

impl Notice<'_> {
    /// The action's and the dismiss button's responses.
    fn show(self, ui: &mut Ui) -> (Option<Response>, Option<Response>) {
        let [mut left, mut right, mut y] = self.pad;
        if self.floating {
            left += 1.0;
            right += 1.0;
            y += 1.0;
        }
        let action = self.action.map(Button::ghost);
        let action_width = action.as_ref().map_or(0.0, |b| b.desired_size(ui).x);
        let mut buttons = action_width;
        if self.action.is_some() {
            buttons += space::S8;
        }
        if self.dismiss.is_some() {
            buttons += size::control() + space::S8;
        }
        let width = ui.available_width();
        let room = width - left - right - size::icon() - space::S8 - buttons;
        let mut job = LayoutJob::simple(
            self.message.to_owned(),
            theme::body(),
            color::ink(),
            room.max(1.0),
        );
        job.sections[0].format.line_height = Some(crate::tokens::font::body() * 1.45);
        let message = ui.fonts_mut(|fonts| fonts.layout_job(job));
        let height = (size::control() + 2.0 * y).max(message.size().y + 2.0 * y);
        let (rect, _) = allocate(ui, vec2(width, height), State::Live, Sense::hover());
        let corner = CornerRadius::same(self.corner);
        if self.floating {
            let shadow = ui.visuals().popup_shadow;
            ui.painter().add(shadow.as_shape(rect, corner));
        }
        ui.painter().rect_filled(rect, corner, self.fill);
        if self.floating {
            ui.painter()
                .add(border(ui, rect, self.corner, color::border()));
        }
        let mid = rect.center().y;
        let icon = pos2(rect.left() + left + size::icon() / 2.0, mid);
        paint_icon(ui.painter(), icon, self.icon, self.icon_ink);
        let text = Rect::from_min_size(
            pos2(
                icon.x + size::icon() / 2.0 + space::S8,
                mid - message.size().y / 2.0,
            ),
            message.size(),
        );
        // A label exposes the wrapped text to accessibility, unlike painter-only copy.
        place(ui, text, Label::new(message).selectable(false));
        let mut x = rect.right() - right;
        let dismissed = self.dismiss.map(|name| {
            x -= size::control();
            let slot = Rect::from_min_size(
                pos2(x, mid - size::control() / 2.0),
                Vec2::splat(size::control()),
            );
            x -= space::S8;
            let button = Button::icon(icon::X, name).ink(self.icon_ink);
            place(ui, slot, button).on_hover_text(name)
        });
        let acted = action.map(|button| {
            let slot = Rect::from_min_size(
                pos2(x - action_width, mid - size::control() / 2.0),
                vec2(action_width, size::control()),
            );
            place(ui, slot, button)
        });
        (acted, dismissed)
    }
}

/// An inline message across its container, tinted by its tone, with an optional action.
#[must_use = "call `.show(ui)`"]
pub struct Banner<'a> {
    tone: Tone,
    message: &'a str,
    action: Option<&'a str>,
}

impl<'a> Banner<'a> {
    pub fn new(tone: Tone, message: &'a str) -> Self {
        Self {
            tone,
            message,
            action: None,
        }
    }

    /// A ghost button at the banner's right.
    pub fn action(mut self, label: &'a str) -> Self {
        self.action = Some(label);
        self
    }

    /// The action's response, when the banner has one.
    pub fn show(self, ui: &mut Ui) -> Option<Response> {
        let (fill, ink) = self.tone.colors();
        let notice = Notice {
            fill,
            corner: radius::control(),
            floating: false,
            pad: [space::inset(), space::inset(), 6.0],
            icon: icon::CIRCLE_ALERT,
            icon_ink: ink,
            message: self.message,
            action: self.action,
            dismiss: None,
        };
        notice.show(ui).0
    }
}

/// What happened in a toast this frame.
pub struct ToastResponse {
    /// The action's response, when the toast has one.
    pub action: Option<Response>,
    pub dismissed: bool,
}

/// A raised confirmation that something finished, with an optional action and a dismiss
/// button. The caller places it and decides how long it stays.
#[must_use = "call `.show(ui)`"]
pub struct Toast<'a> {
    message: &'a str,
    action: Option<&'a str>,
    dismiss: &'a str,
}

impl<'a> Toast<'a> {
    /// `dismiss` names the dismiss button.
    pub fn new(message: &'a str, dismiss: &'a str) -> Self {
        Self {
            message,
            action: None,
            dismiss,
        }
    }

    /// A ghost button before the dismiss button.
    pub fn action(mut self, label: &'a str) -> Self {
        self.action = Some(label);
        self
    }

    pub fn show(self, ui: &mut Ui) -> ToastResponse {
        let notice = Notice {
            fill: color::raised(),
            corner: radius::panel(),
            floating: true,
            pad: [space::inset(), space::S8, space::S8],
            icon: icon::CIRCLE_CHECK,
            icon_ink: color::green(),
            message: self.message,
            action: self.action,
            dismiss: Some(self.dismiss),
        };
        let (action, dismiss) = notice.show(ui);
        let dismissed = dismiss.is_some_and(|r| r.clicked());
        ToastResponse { action, dismissed }
    }
}
