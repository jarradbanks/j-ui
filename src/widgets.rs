//! Base controls. Each paints its own default, hover, focus-visible, pressed and
//! disabled states (plus loading and error where they apply); `.state(..)` pins
//! one of them, which the gallery uses to show every state at once.

use egui::emath::GuiRounding as _;
use std::sync::Arc;

use egui::emath::RectAlign;
use egui::text::{LayoutJob, TextWrapping};
use egui::{
    Align2, Color32, CornerRadius, FontId, Galley, Id, Key, Rect, Response, Sense, Shape, Stroke,
    StrokeKind, TextEdit, Ui, Vec2, Widget, WidgetInfo, WidgetType, pos2, vec2,
};

use crate::i18n::CATALOG;
use crate::theme::{self, hairline, icon};
use crate::tokens::{color, radius, size, space};
use crate::tr;

mod autocomplete;
mod content;
mod feedback;
mod navigation;
mod selection;
pub use autocomplete::Autocomplete;
pub use content::{Card, Column, Table};
pub use feedback::{Badge, Banner, Progress, Toast, ToastResponse, Tone};
pub(crate) use navigation::tab_strip;
pub use navigation::{NavRow, Tabs};
pub use selection::{
    Breadcrumb, RadioGroup, SELECTION_DIAMETER, SELECTION_TILE, SelectionCheckbox, Toggle,
    paint_selection,
};

/// The interaction state a control is drawn in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum State {
    /// Follow the pointer and keyboard.
    #[default]
    Live,
    Default,
    Hover,
    Focus,
    Pressed,
    Disabled,
}

/// What to draw: the resolved state, and whether to add the focus ring.
#[derive(Clone, Copy)]
struct Look {
    hover: bool,
    pressed: bool,
    focus: bool,
    disabled: bool,
}

fn look(response: &Response, pinned: State) -> Look {
    match pinned {
        State::Live => Look {
            hover: response.hovered(),
            pressed: response.is_pointer_button_down_on(),
            focus: response.has_focus(),
            disabled: !response.enabled(),
        },
        s => Look {
            hover: s == State::Hover,
            pressed: s == State::Pressed,
            focus: s == State::Focus,
            disabled: s == State::Disabled,
        },
    }
}

/// Fades a colour towards its hover value over 100ms.
fn hover_fade(ui: &Ui, id: Id, hover: bool, rest: Color32, hovered: Color32) -> Color32 {
    // No fade when the app turns animation off, as Reduce Motion does.
    let time = if ui.style().animation_time == 0.0 {
        0.0
    } else {
        0.1
    };
    let t = ui
        .ctx()
        .animate_bool_with_time(id.with("hover"), hover, time);
    rest.lerp_to_gamma(hovered, t)
}

/// Paints icon `glyph` with its ink, not its line box, centred on `center`: a Lucide glyph's
/// box runs a pixel taller than the icon, and some, such as Folder and Check, sit high in it.
pub fn paint_icon(painter: &egui::Painter, center: egui::Pos2, glyph: &str, color: Color32) {
    let galley = painter.layout_no_wrap(glyph.into(), theme::icon_font(), color);
    let pos = center - galley.mesh_bounds.center().to_vec2();
    painter.galley(pos, galley, color);
}

const RING: f32 = 3.0;

/// The ring a field or check control shows on keyboard focus: 3pt of Blue Soft outside
/// `rect`. Buttons, tabs and rows show their hover look instead.
pub fn focus_ring(rect: Rect, corner: u8) -> Shape {
    Shape::rect_stroke(
        rect,
        CornerRadius::same(corner),
        Stroke::new(RING, color::blue_soft()),
        StrokeKind::Outside,
    )
}

/// Paints [`focus_ring`] up to its width past the parent's clip, so a control at a clipped
/// edge keeps its whole ring.
fn paint_focus(ui: &Ui, rect: Rect, corner: u8) {
    ui.painter()
        .with_clip_rect(ui.clip_rect().expand(RING))
        .add(focus_ring(rect, corner));
}

/// Adds `widget` centred in `rect`, inside a control that has already allocated it. Unlike
/// `ui.put`, this leaves `ui`'s cursor alone, so what follows keeps the layout's spacing.
fn place(ui: &mut Ui, rect: Rect, widget: impl Widget) -> Response {
    let layout = egui::Layout::centered_and_justified(egui::Direction::TopDown);
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect).layout(layout));
    child.add(widget)
}

/// Lucide's loader arc at icon size, turning once a second.
fn paint_spinner(ui: &Ui, center: egui::Pos2, ink: Color32) {
    use std::f32::consts::TAU;
    let turn = ui.input(|i| i.time).fract() as f32 * TAU;
    let points = (0..=24)
        .map(|i| {
            let angle = turn + 0.75 * TAU * i as f32 / 24.0;
            center + 0.375 * size::ICON * vec2(angle.cos(), angle.sin())
        })
        .collect();
    let stroke = Stroke::new(size::ICON / 12.0, ink);
    ui.painter().add(Shape::line(points, stroke));
    ui.ctx().request_repaint();
}

/// A one-pixel border on a control, inside its rect.
fn border(ui: &Ui, rect: Rect, corner: u8, color: Color32) -> Shape {
    Shape::rect_stroke(
        rect,
        CornerRadius::same(corner),
        hairline(ui.ctx(), color),
        StrokeKind::Inside,
    )
}

fn text_width(ui: &Ui, text: &str, font: &FontId) -> f32 {
    ui.painter()
        .layout_no_wrap(text.to_owned(), font.clone(), Color32::WHITE)
        .size()
        .x
}

/// Allocates a control, disabling it first when `pinned` is `Disabled`.
fn allocate(ui: &mut Ui, size: Vec2, pinned: State, sense: Sense) -> (Rect, Response) {
    let sense = if pinned == State::Disabled {
        Sense::hover()
    } else {
        sense
    };
    let (rect, response) = ui.allocate_exact_size(size, sense);
    let rect = rect.round_to_pixels(ui.pixels_per_point());
    (rect, response)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonKind {
    /// One per surface: Export, or a dialog's confirm button.
    Primary,
    #[default]
    Secondary,
    /// No fill until hovered. Also the icon button.
    Ghost,
    /// Red text on Red Soft, for confirmation dialogs.
    Destructive,
}

/// A 36px button with an optional leading icon.
#[must_use = "add it with `ui.add(..)`"]
pub struct Button<'a> {
    label: &'a str,
    kind: ButtonKind,
    icon: Option<&'static str>,
    icon_only: bool,
    loading: bool,
    on: bool,
    state: State,
    min_width: f32,
    min_height: f32,
    wrap_width: f32,
    small: bool,
    ink: Option<Color32>,
}

impl<'a> Button<'a> {
    pub fn new(kind: ButtonKind, label: &'a str) -> Self {
        Self {
            label,
            kind,
            icon: None,
            icon_only: false,
            loading: false,
            on: false,
            state: State::Live,
            min_width: 0.0,
            min_height: 0.0,
            wrap_width: f32::INFINITY,
            small: false,
            ink: None,
        }
    }

    pub fn primary(label: &'a str) -> Self {
        Self::new(ButtonKind::Primary, label)
    }

    pub fn secondary(label: &'a str) -> Self {
        Self::new(ButtonKind::Secondary, label)
    }

    pub fn ghost(label: &'a str) -> Self {
        Self::new(ButtonKind::Ghost, label)
    }

    pub fn destructive(label: &'a str) -> Self {
        Self::new(ButtonKind::Destructive, label)
    }

    /// A square ghost button holding one glyph from [`icon`]; `name` is for screen readers.
    pub fn icon(glyph: &'static str, name: &'a str) -> Self {
        let mut b = Self::ghost(name).with_icon(glyph);
        b.icon_only = true;
        b
    }

    /// Redraws the button as `kind`, such as a filled Secondary icon button.
    pub fn kind(mut self, kind: ButtonKind) -> Self {
        self.kind = kind;
        self
    }

    /// A 28px icon button with 8px corners, for the tool slots of an inspector row.
    pub fn small(mut self) -> Self {
        self.small = true;
        self
    }

    pub fn with_icon(mut self, glyph: &'static str) -> Self {
        self.icon = Some(glyph);
        self
    }

    /// Swaps the icon for a spinner; the label and width stay.
    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// A ghost toggle that's on: Ink on Raised.
    pub fn on(mut self, on: bool) -> Self {
        self.on = on;
        self
    }

    pub fn state(mut self, state: State) -> Self {
        self.state = state;
        self
    }

    /// Widens the button to at least `width`, centring its content.
    pub fn width(mut self, width: f32) -> Self {
        self.min_width = width;
        self
    }

    /// Draws the label and icon in `ink` while enabled, such as Red for an alert.
    pub fn ink(mut self, ink: Color32) -> Self {
        self.ink = Some(ink);
        self
    }

    /// Makes the button at least `height` tall, for a large call to action.
    pub fn height(mut self, height: f32) -> Self {
        self.min_height = height;
        self
    }

    /// Keeps translated footer labels inside the available width.
    pub(crate) fn wrap_at(mut self, width: f32) -> Self {
        self.wrap_width = width;
        self
    }

    fn label_galley(&self, ui: &Ui) -> Arc<Galley> {
        let icon = if self.icon.is_some() || self.loading {
            size::ICON + space::S8
        } else {
            0.0
        };
        ui.painter().layout(
            self.label.to_owned(),
            theme::body_strong(),
            Color32::WHITE,
            (self.wrap_width - 2.0 * space::BUTTON_PAD - icon).max(1.0),
        )
    }

    pub(crate) fn desired_size(&self, ui: &Ui) -> Vec2 {
        let side = if self.small {
            size::SMALL_CONTROL
        } else {
            size::CONTROL
        };
        let label = self.label_galley(ui);
        let icon = if self.icon.is_some() || self.loading {
            size::ICON + space::S8
        } else {
            0.0
        };
        let width = if self.icon_only {
            side
        } else {
            2.0 * space::BUTTON_PAD + icon + label.size().x
        };
        let height = if self.icon_only {
            side
        } else {
            side.max(label.size().y + space::S8)
        };
        vec2(
            width.max(self.min_width).ceil(),
            height.max(self.min_height),
        )
    }
}

impl Widget for Button<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let has_label = !self.icon_only;
        let has_icon = self.icon.is_some() || self.loading;
        let galley = self.label_galley(ui);
        let desired = self.desired_size(ui);
        let content_width = galley.size().x
            + if has_icon {
                size::ICON + space::S8
            } else {
                0.0
            };
        let pad = ((desired.x - 2.0 * space::BUTTON_PAD - content_width).floor() / 2.0).max(0.0);
        let (rect, response) = allocate(ui, desired, self.state, Sense::click());
        response.widget_info(|| {
            WidgetInfo::labeled(
                WidgetType::Button,
                self.state != State::Disabled,
                self.label,
            )
        });
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let l = look(&response, self.state);
        let id = response.id;
        // Keyboard focus shows the hover look, never a ring.
        let (fill, ink) = match self.kind {
            ButtonKind::Ghost if l.disabled => (Color32::TRANSPARENT, color::ink_4()),
            _ if l.disabled => (color::border(), color::ink_4()),
            ButtonKind::Primary if l.pressed => (color::BLUE_PRESSED, color::WHITE),
            ButtonKind::Primary => {
                let lit = l.hover || l.focus;
                let fill = hover_fade(ui, id, lit, color::blue(), color::blue_hover());
                (fill, color::WHITE)
            }
            ButtonKind::Secondary => {
                let lit = l.hover || l.pressed;
                let fill = hover_fade(ui, id, lit, color::border(), color::raised());
                (fill, color::ink())
            }
            ButtonKind::Destructive => (color::red_soft(), color::red()),
            ButtonKind::Ghost => {
                let lit = l.hover || l.pressed || self.on;
                let rest = if l.focus {
                    color::border()
                } else {
                    Color32::TRANSPARENT
                };
                let fill = hover_fade(ui, id, lit, rest, color::raised());
                let ink = if self.on {
                    color::ink()
                } else {
                    color::ink_2()
                };
                (fill, ink)
            }
        };
        let ink = self.ink.filter(|_| !l.disabled).unwrap_or(ink);
        let corner = if self.small {
            radius::SMALL
        } else {
            radius::CONTROL
        };
        ui.painter()
            .rect_filled(rect, CornerRadius::same(corner), fill);
        let mut x = if has_label {
            rect.left() + space::BUTTON_PAD + pad
        } else {
            rect.center().x - size::ICON / 2.0
        };
        if has_icon {
            let center = pos2(x + size::ICON / 2.0, rect.center().y);
            if self.loading {
                paint_spinner(ui, center, ink);
            } else if let Some(glyph) = self.icon {
                paint_icon(ui.painter(), center, glyph, ink);
            }
            x += size::ICON + space::S8;
        }
        if has_label {
            ui.painter().galley_with_override_text_color(
                pos2(x, rect.center().y - galley.size().y / 2.0),
                galley,
                ink,
            );
        }
        response
    }
}

/// Paints a field's frame into `slot`: a borderless Ground fill, a Red border on error, and
/// on focus a Blue border with the focus ring.
fn paint_field(ui: &Ui, slot: egui::layers::ShapeIdx, rect: Rect, l: Look, error: bool) {
    let focus = l.focus && !l.disabled;
    let mut shapes = vec![Shape::rect_filled(
        rect,
        CornerRadius::same(radius::CONTROL),
        color::ground(),
    )];
    let stroke = match () {
        _ if l.disabled => None,
        _ if error => Some(color::red()),
        _ if focus => Some(color::blue()),
        _ => None,
    };
    shapes.extend(stroke.map(|c| border(ui, rect, radius::CONTROL, c)));
    ui.painter().set(slot, Shape::Vec(shapes));
    if focus {
        paint_focus(ui, rect, radius::CONTROL);
    }
}

/// Paints an optional prefix (Ink 3) at the field's left padding; returns where the value starts.
fn paint_prefix(ui: &Ui, rect: Rect, prefix: Option<&str>, glyph: Option<&str>) -> f32 {
    let mut x = rect.left() + space::FIELD_PAD;
    let y = rect.center().y;
    if let Some(glyph) = glyph {
        paint_icon(
            ui.painter(),
            pos2(x + size::ICON / 2.0, y),
            glyph,
            color::ink_3(),
        );
        x += size::ICON + space::S8;
    }
    if let Some(prefix) = prefix {
        let r = ui.painter().text(
            pos2(x, y),
            Align2::LEFT_CENTER,
            prefix,
            theme::body(),
            color::ink_3(),
        );
        x = r.right() + space::S8;
    }
    x
}

/// Shows an error message in Red Small under a field.
fn error_line(ui: &mut Ui, error: Option<&str>) {
    if let Some(message) = error {
        ui.label(
            egui::RichText::new(message)
                .font(theme::small())
                .color(color::red()),
        );
    }
}

/// A text input with an optional inline prefix, suffix, icon and error. One line unless
/// `.rows(..)`.
#[must_use = "add it with `ui.add(..)`"]
pub struct TextInput<'a> {
    text: &'a mut String,
    prefix: Option<&'a str>,
    suffix: Option<&'a str>,
    glyph: Option<&'static str>,
    hint: &'a str,
    error: Option<&'a str>,
    width: f32,
    rows: usize,
    state: State,
    height: f32,
    clear_label: Option<&'a str>,
    id: Option<Id>,
}

impl<'a> TextInput<'a> {
    pub fn new(text: &'a mut String) -> Self {
        Self {
            text,
            prefix: None,
            suffix: None,
            id: None,
            glyph: None,
            hint: "",
            error: None,
            width: 160.0,
            rows: 1,
            state: State::Live,
            height: size::CONTROL,
            clear_label: None,
        }
    }

    /// Multi-line, `rows` lines tall; longer text scrolls.
    pub fn rows(mut self, rows: usize) -> Self {
        self.rows = rows.max(1);
        self
    }

    /// A search field: search icon prefix and an Ink 3 placeholder.
    pub fn search(text: &'a mut String) -> Self {
        let mut s = Self::new(text).hint(tr!(CATALOG, "input.search"));
        s.glyph = Some(icon::SEARCH);
        s
    }

    /// Inline prefix such as X, Y, W or H, in Ink 3.
    pub fn prefix(mut self, prefix: &'a str) -> Self {
        self.prefix = Some(prefix);
        self
    }

    /// Inline unit such as px, in Ink 3 at the field's right.
    pub fn suffix(mut self, suffix: &'a str) -> Self {
        self.suffix = Some(suffix);
        self
    }

    pub fn hint(mut self, hint: &'a str) -> Self {
        self.hint = hint;
        self
    }

    /// The editor's id, for a caller that reads its focus before adding it.
    pub(crate) fn id(mut self, id: Id) -> Self {
        self.id = Some(id);
        self
    }

    /// Red border, with `message` in Red under the row.
    pub fn error(mut self, message: Option<&'a str>) -> Self {
        self.error = message;
        self
    }

    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Minimum single-line height; the text and clear button stay vertically centred.
    pub fn height(mut self, height: f32) -> Self {
        self.height = height.max(size::CONTROL);
        self
    }

    /// Shows a clear button while the field contains text; `label` names the action.
    pub fn clearable(mut self, label: &'a str) -> Self {
        self.clear_label = Some(label);
        self
    }

    pub fn state(mut self, state: State) -> Self {
        self.state = state;
        self
    }
}

impl Widget for TextInput<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        ui.vertical(|ui| {
            let row = ui.fonts_mut(|f| f.row_height(&theme::body()));
            let extra = (self.rows - 1) as f32 * row;
            let (rect, outer) = allocate(
                ui,
                vec2(self.width, self.height + extra),
                self.state,
                Sense::hover(),
            );
            let frame = ui.painter().add(Shape::Noop);
            let x = paint_prefix(ui, rect, self.prefix, self.glyph);
            let clear = self
                .clear_label
                .filter(|_| !self.text.is_empty())
                .map(|label| {
                    let side = size::SMALL_CONTROL;
                    (
                        Rect::from_center_size(
                            rect.right_center() - vec2(side / 2.0 + space::S4, 0.0),
                            Vec2::splat(side),
                        ),
                        label,
                    )
                });
            let mut right = clear.map_or(rect.right() - space::FIELD_PAD, |(r, _)| {
                r.left() - space::S4
            });
            if let Some(suffix) = self.suffix {
                let r = ui.painter().text(
                    pos2(right, rect.center().y),
                    Align2::RIGHT_CENTER,
                    suffix,
                    theme::body(),
                    color::ink_3(),
                );
                right = r.left() - space::S8;
            }
            let inner = Rect::from_x_y_ranges(x..=right, rect.y_range());
            let disabled = self.state == State::Disabled || !ui.is_enabled();
            let ink = if disabled {
                color::ink_4()
            } else {
                color::ink()
            };
            let edit = {
                let edit_rect =
                    Rect::from_center_size(inner.center(), vec2(inner.width(), row + extra));
                let edit = if self.rows > 1 {
                    TextEdit::multiline(self.text).desired_width(f32::INFINITY)
                } else {
                    TextEdit::singleline(self.text)
                }
                .frame(egui::Frame::NONE)
                .margin(0)
                .font(theme::body())
                .text_color(ink)
                .hint_text(self.hint);
                let edit = match self.id {
                    Some(id) => edit.id(id),
                    None => edit,
                };
                if self.rows == 1 {
                    // A stable capital keeps the baseline fixed while the value changes.
                    let metrics = ui.painter().layout_no_wrap('H'.into(), theme::body(), ink);
                    let offset = row / 2.0 - metrics.mesh_bounds.center().y;
                    let edit_rect = edit_rect.translate(vec2(0.0, offset));
                    place(ui, edit_rect, |ui: &mut Ui| ui.add_enabled(!disabled, edit))
                } else {
                    place(ui, edit_rect, |ui: &mut Ui| {
                        ui.add_enabled_ui(!disabled, |ui| {
                            egui::ScrollArea::vertical()
                                // egui's default 64pt viewport overflows short multiline fields.
                                .min_scrolled_height(0.0)
                                .auto_shrink(false)
                                .show(ui, |ui| ui.add(edit))
                                .inner
                        })
                        .inner
                    })
                }
            };
            let mut response = edit.union(outer);
            response.rect = rect;
            ui.ctx().accesskit_node_builder(response.id, |node| {
                node.set_bounds(egui::accesskit::Rect {
                    x0: rect.min.x.into(),
                    y0: rect.min.y.into(),
                    x1: rect.max.x.into(),
                    y1: rect.max.y.into(),
                });
            });
            let mut l = look(&response, self.state);
            l.hover |= self.state == State::Live && ui.rect_contains_pointer(rect);
            paint_field(ui, frame, rect, l, self.error.is_some());
            if let Some((clear, label)) = clear {
                let button = Button::icon(icon::X, label).small();
                let button = |ui: &mut Ui| ui.add_enabled(!disabled, button);
                if place(ui, clear, button).on_hover_text(label).clicked() {
                    self.text.clear();
                    response.mark_changed();
                    response.request_focus();
                }
            }
            error_line(ui, self.error);
            response
        })
        .inner
    }
}

/// A row of colour swatches to pick one of: 20px squares 4px apart, each a radio button
/// named for screen readers and on hover, the selected one ringed in Ink.
#[must_use = "add it with `ui.add(..)`"]
pub struct Swatches<'a> {
    selected: &'a mut usize,
    colors: &'a [Color32],
    names: &'a [&'a str],
    state: State,
}

impl<'a> Swatches<'a> {
    /// `names[i]` names `colors[i]`.
    pub fn new(selected: &'a mut usize, colors: &'a [Color32], names: &'a [&'a str]) -> Self {
        Self {
            selected,
            colors,
            names,
            state: State::Live,
        }
    }

    /// Pins the state of the first unselected swatch.
    pub fn state(mut self, state: State) -> Self {
        self.state = state;
        self
    }
}

impl Widget for Swatches<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let n = self.colors.len() as f32;
        let side = size::SMALL_CONTROL;
        let width = n * side + (n - 1.0).max(0.0) * space::S4;
        let (rect, mut response) = allocate(ui, vec2(width, side), self.state, Sense::hover());
        let disabled = self.state == State::Disabled || !ui.is_enabled();
        let first_unselected = (0..self.colors.len()).find(|i| i != self.selected);
        let corner = CornerRadius::same(radius::SMALL);
        for (i, c) in self.colors.iter().enumerate() {
            let x = rect.left() + i as f32 * (side + space::S4);
            let swatch = Rect::from_min_size(pos2(x, rect.top()), Vec2::splat(side));
            let sense = if disabled {
                Sense::hover()
            } else {
                Sense::click()
            };
            let name = self.names.get(i).copied().unwrap_or_default();
            let on = i == *self.selected;
            let r = ui.interact(swatch, response.id.with(i), sense);
            r.widget_info(|| WidgetInfo::selected(WidgetType::RadioButton, !disabled, on, name));
            let r = r.on_hover_text(name);
            if r.clicked() {
                *self.selected = i;
                response.mark_changed();
            }
            let pinned = if Some(i) == first_unselected {
                self.state
            } else {
                State::Live
            };
            let l = look(&r, pinned);
            let fill = if disabled { c.gamma_multiply(0.4) } else { *c };
            ui.painter().rect_filled(swatch, corner, fill);
            let edge = if l.hover || l.pressed {
                color::ink_3()
            } else {
                color::border_strong()
            };
            ui.painter().add(border(ui, swatch, radius::SMALL, edge));
            if on {
                ui.painter().rect_stroke(
                    swatch,
                    corner,
                    Stroke::new(1.5, color::ink()),
                    StrokeKind::Outside,
                );
            }
            if l.focus {
                paint_focus(ui, swatch, radius::SMALL);
            }
        }
        response
    }
}

/// A numeric field: drag sideways to scrub, double-click or press Enter to type.
#[must_use = "add it with `ui.add(..)`"]
pub struct DragField<'a> {
    value: &'a mut f64,
    prefix: Option<&'a str>,
    suffix: &'a str,
    speed: f64,
    range: std::ops::RangeInclusive<f64>,
    decimals: usize,
    width: f32,
    state: State,
    mixed: bool,
}

impl<'a> DragField<'a> {
    pub fn new(value: &'a mut f64) -> Self {
        Self {
            value,
            prefix: None,
            suffix: "",
            speed: 1.0,
            range: f64::NEG_INFINITY..=f64::INFINITY,
            decimals: 0,
            width: 80.0,
            state: State::Live,
            mixed: false,
        }
    }

    pub fn prefix(mut self, prefix: &'a str) -> Self {
        self.prefix = Some(prefix);
        self
    }

    /// Shows a dash in place of the value, for a selection whose values differ.
    pub fn mixed(mut self, mixed: bool) -> Self {
        self.mixed = mixed;
        self
    }

    pub fn suffix(mut self, suffix: &'a str) -> Self {
        self.suffix = suffix;
        self
    }

    /// Value change per point dragged.
    pub fn speed(mut self, speed: f64) -> Self {
        self.speed = speed;
        self
    }

    pub fn range(mut self, range: std::ops::RangeInclusive<f64>) -> Self {
        self.range = range;
        self
    }

    pub fn decimals(mut self, decimals: usize) -> Self {
        self.decimals = decimals;
        self
    }

    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    pub fn state(mut self, state: State) -> Self {
        self.state = state;
        self
    }
}

impl Widget for DragField<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let (rect, mut response) = allocate(
            ui,
            vec2(self.width, size::CONTROL),
            self.state,
            Sense::click_and_drag(),
        );
        let id = response.id;
        let frame = ui.painter().add(Shape::Noop);
        let x = paint_prefix(ui, rect, self.prefix, None);
        let (lo, hi) = (*self.range.start(), *self.range.end());
        let mut editing: Option<String> = ui.data(|d| d.get_temp(id));
        if editing.is_none()
            && (response.double_clicked()
                || (response.has_focus() && ui.input(|i| i.key_pressed(Key::Enter))))
        {
            editing = Some(format!("{:.*}", self.decimals, self.value));
        }
        let mut focus = response.has_focus();
        if let Some(mut buffer) = editing {
            let row = ui.fonts_mut(|f| f.row_height(&theme::body()));
            let edit_rect = Rect::from_x_y_ranges(
                x..=rect.right() - space::FIELD_PAD,
                rect.center().y - row / 2.0..=rect.center().y + row / 2.0,
            );
            let edit = place(
                ui,
                edit_rect,
                TextEdit::singleline(&mut buffer)
                    .id(id.with("edit"))
                    .frame(egui::Frame::NONE)
                    .margin(0)
                    .font(theme::body())
                    .text_color(color::ink()),
            );
            if !edit.has_focus() && !edit.lost_focus() {
                edit.request_focus();
            }
            focus = true;
            if edit.lost_focus() {
                if !ui.input(|i| i.key_pressed(Key::Escape))
                    && let Ok(v) = buffer
                        .trim()
                        .trim_end_matches(self.suffix)
                        .trim()
                        .parse::<f64>()
                {
                    *self.value = v.clamp(lo, hi);
                    response.mark_changed();
                }
                ui.data_mut(|d| d.remove::<String>(id));
            } else {
                ui.data_mut(|d| d.insert_temp(id, buffer));
            }
        } else {
            if response.dragged() {
                let delta = response.drag_delta().x as f64 * self.speed;
                *self.value = (*self.value + delta).clamp(lo, hi);
                response.mark_changed();
            }
            if response.hovered() || response.dragged() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
            }
            let disabled = self.state == State::Disabled || !response.enabled();
            let ink = if disabled {
                color::ink_4()
            } else {
                color::ink()
            };
            let text = match self.mixed {
                true => "\u{2014}".to_owned(),
                false => format!("{:.*}{}", self.decimals, self.value, self.suffix),
            };
            ui.painter().text(
                pos2(x, rect.center().y),
                Align2::LEFT_CENTER,
                &text,
                theme::body(),
                ink,
            );
        }
        response.widget_info(|| {
            WidgetInfo::labeled(WidgetType::DragValue, true, self.prefix.unwrap_or(""))
        });
        let mut l = look(&response, self.state);
        l.focus |= self.state == State::Live && focus;
        paint_field(ui, frame, rect, l, false);
        response
    }
}

/// One line of `text` cut to `width` with an ellipsis; check `elided` to know it was.
///
/// Text in a fixed-width spot (a tab, a column header, a caption beside its fields) goes
/// through this, or through `egui::Label::truncate`, which cuts the same way and adds the
/// tooltip itself. Translations run long, and the layout copes rather than the translator.
/// Show the full text on hover with [`hover_full`] or [`paint_fit`].
pub fn fit_text(ui: &Ui, text: &str, font: FontId, color: Color32, width: f32) -> Arc<Galley> {
    let mut job = LayoutJob::simple_singleline(text.to_owned(), font, color);
    job.wrap = TextWrapping::truncate_at_width(width.max(0.0));
    ui.fonts_mut(|f| f.layout_job(job))
}

/// `response` with `galley`'s full text on hover when [`fit_text`] cut it.
pub fn hover_full(response: Response, galley: &Galley) -> Response {
    if galley.elided {
        response.on_hover_text(galley.text())
    } else {
        response
    }
}

/// Paints `galley` from [`fit_text`] with its top left at `pos`; cut, it shows the full text on
/// hover. For text with no widget of its own, such as a column header.
pub fn paint_fit(ui: &Ui, pos: egui::Pos2, galley: Arc<Galley>, color: Color32) {
    if galley.elided {
        let id = ui.id().with(("fit", pos.x.to_bits(), pos.y.to_bits()));
        let rect = Rect::from_min_size(pos, galley.size());
        ui.interact(rect, id, Sense::hover())
            .on_hover_text(galley.text());
    }
    ui.painter().galley(pos, galley, color);
}

/// Paints `galley` left-aligned at `x`, centred on `y`.
fn paint_left(ui: &Ui, x: f32, y: f32, galley: Arc<Galley>, color: Color32) {
    let pos = pos2(x, y - galley.size().y / 2.0);
    ui.painter().galley(pos, galley, color);
}

/// Where a dropdown's menu opens, and its tallest scroll height: below the field unless
/// there's more room above, never past `bounds`, and `MARGIN` short of its edge.
pub fn menu_place(field: Rect, bounds: Rect, wanted: f32) -> (RectAlign, f32) {
    // The menu opens `S4` off the field.
    const MARGIN: f32 = space::S8 + space::S4;
    let below = bounds.bottom() - field.bottom() - MARGIN;
    let above = field.top() - bounds.top() - MARGIN;
    // The menu's frame takes its margin off both ends.
    let frame = 2.0 * space::S4;
    if wanted + frame <= below || below >= above {
        (RectAlign::BOTTOM_START, below - frame)
    } else {
        (RectAlign::TOP_START, above - frame)
    }
}

/// A dropdown trigger for callers that own the popup contents.
#[must_use = "add it with `ui.add(..)`"]
pub struct Select<'a> {
    current: &'a str,
    width: f32,
    height: f32,
    glyph: Option<&'static str>,
    state: State,
    placeholder: bool,
}

impl<'a> Select<'a> {
    pub fn new(current: &'a str) -> Self {
        Self {
            current,
            width: 120.0,
            height: size::CONTROL,
            glyph: None,
            state: State::Live,
            placeholder: false,
        }
    }
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }
    pub fn height(mut self, height: f32) -> Self {
        self.height = height.max(size::CONTROL);
        self
    }
    pub fn with_icon(mut self, glyph: &'static str) -> Self {
        self.glyph = Some(glyph);
        self
    }
    pub fn state(mut self, state: State) -> Self {
        self.state = state;
        self
    }

    fn show(self, ui: &mut Ui) -> (Response, bool) {
        let (rect, response) = allocate(
            ui,
            vec2(self.width, self.height),
            self.state,
            Sense::click(),
        );
        response.widget_info(|| {
            WidgetInfo::labeled(WidgetType::ComboBox, response.enabled(), self.current)
        });
        let mut l = look(&response, self.state);
        // An open menu keeps the field's focus look.
        l.focus |= egui::Popup::is_id_open(ui.ctx(), egui::Popup::default_response_id(&response));
        let frame = ui.painter().add(Shape::Noop);
        paint_field(ui, frame, rect, l, false);
        let ink = match () {
            _ if l.disabled => color::ink_4(),
            _ if self.placeholder => color::ink_2(),
            _ => color::ink(),
        };
        let chevron = if l.disabled {
            color::ink_4()
        } else {
            color::ink_3()
        };
        let chevron_x = rect.right() - space::FIELD_PAD - size::ICON / 2.0;
        let left = paint_prefix(ui, rect, None, self.glyph);
        let room = chevron_x - size::ICON / 2.0 - space::S8 - left;
        let text = fit_text(ui, self.current, theme::body(), ink, room);
        let elided = text.elided;
        paint_left(ui, left, rect.center().y, text, ink);
        paint_icon(
            ui.painter(),
            pos2(chevron_x, rect.center().y),
            icon::CHEVRON_DOWN,
            chevron,
        );
        (response, elided)
    }
}

impl Widget for Select<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        self.show(ui).0
    }
}

/// A field with a chevron that opens a menu of options. The menu is as wide as the field,
/// stays inside the window (or [`Dropdown::bounds`]) and scrolls past that; a name too long
/// for the field or a row ends in an ellipsis and shows in full on hover.
#[must_use = "add it with `ui.add(..)`"]
pub struct Dropdown<'a> {
    selected: &'a mut usize,
    options: &'a [&'a str],
    width: f32,
    state: State,
    bounds: Option<Rect>,
    placeholder: &'a str,
    height: f32,
    glyph: Option<&'static str>,
}

impl<'a> Dropdown<'a> {
    pub fn new(selected: &'a mut usize, options: &'a [&'a str]) -> Self {
        Self {
            selected,
            options,
            width: 120.0,
            state: State::Live,
            bounds: None,
            placeholder: "",
            height: size::CONTROL,
            glyph: None,
        }
    }

    /// What the field shows, in Ink 2, while `selected` is past the options, such as "Custom"
    /// for a value no preset matches. It isn't in the menu.
    pub fn placeholder(mut self, text: &'a str) -> Self {
        self.placeholder = text;
        self
    }

    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    pub fn height(mut self, height: f32) -> Self {
        self.height = height.max(size::CONTROL);
        self
    }

    pub fn with_icon(mut self, glyph: &'static str) -> Self {
        self.glyph = Some(glyph);
        self
    }

    pub fn state(mut self, state: State) -> Self {
        self.state = state;
        self
    }

    /// Keeps the open menu inside `rect` as well as the window, such as a dialog's body so it
    /// never covers the dialog's buttons.
    pub fn bounds(mut self, rect: Rect) -> Self {
        self.bounds = Some(rect);
        self
    }
}

impl Widget for Dropdown<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let picked = self.options.get(*self.selected).copied();
        let current = picked.unwrap_or(self.placeholder);
        let (mut response, elided) = Select {
            current,
            width: self.width,
            height: self.height,
            glyph: self.glyph,
            state: self.state,
            placeholder: picked.is_none(),
        }
        .show(ui);
        let rect = response.rect;
        let screen = ui.ctx().content_rect();
        let bounds = self.bounds.map_or(screen, |b| b.intersect(screen));
        let wanted = self.options.len() as f32 * size::CONTROL;
        let (align, max_height) = menu_place(rect, bounds, wanted);
        let popup = egui::Popup::menu(&response)
            .width(rect.width())
            .gap(space::S4)
            .align(align)
            .align_alternatives(&[])
            .show(|ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                egui::ScrollArea::vertical()
                    .max_height(max_height.max(size::CONTROL))
                    .show(ui, |ui| {
                        for (i, option) in self.options.iter().enumerate() {
                            let r = ui.add(MenuRow::option(option, i == *self.selected));
                            if r.clicked() {
                                *self.selected = i;
                                response.mark_changed();
                            }
                        }
                    });
            });
        if elided && popup.is_none() {
            response = response.on_hover_text(current);
        }
        response
    }
}

/// Where a menu row keeps its check.
#[derive(Clone, Copy, PartialEq, Eq)]
enum CheckSlot {
    /// A command menu: an icon-wide slot before every label, so labels line up.
    Leading,
    /// A list of options: after the selected one, which is also Medium.
    Trailing,
    None,
}

/// A 36px menu row: a command with a leading check slot and a right-aligned shortcut, or
/// with [`MenuRow::option`] one choice of a list. Labels elide to leave room for both;
/// disabled rows ignore pointer and keyboard input.
#[must_use = "add it with `ui.add(..)`"]
pub struct MenuRow<'a> {
    label: &'a str,
    checked: bool,
    lit: bool,
    shortcut: &'a str,
    enabled: bool,
    slot: CheckSlot,
    /// The byte range of `label` drawn as a match, in Light Blue Medium.
    hit: std::ops::Range<usize>,
}

impl<'a> MenuRow<'a> {
    /// A command; `checked` puts a check before its label.
    pub fn new(label: &'a str, checked: bool) -> Self {
        Self {
            label,
            checked,
            lit: false,
            shortcut: "",
            enabled: true,
            slot: CheckSlot::Leading,
            hit: 0..0,
        }
    }

    /// One choice of a dropdown's list: Ink 2 until hovered, and Ink Medium with a check at
    /// the row's end once `selected`.
    pub fn option(label: &'a str, selected: bool) -> Self {
        Self {
            slot: CheckSlot::Trailing,
            ..Self::new(label, selected)
        }
    }

    /// A suggestion with no check slot, its `hit` bytes drawn as the matched text.
    pub(crate) fn suggestion(label: &'a str, hit: std::ops::Range<usize>) -> Self {
        Self {
            slot: CheckSlot::None,
            hit,
            ..Self::new(label, false)
        }
    }

    /// Shows the hover fill without the pointer, such as for the row the arrow keys reached.
    pub fn lit(mut self, lit: bool) -> Self {
        self.lit = lit;
        self
    }

    /// Display text only: the application handles the shortcut itself.
    pub fn shortcut(mut self, shortcut: &'a str) -> Self {
        self.shortcut = shortcut;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// The label cut to `room`, with the matched bytes in Light Blue Medium.
    fn label_galley(&self, ui: &Ui, font: FontId, ink: Color32, room: f32) -> Arc<Galley> {
        let mut job = LayoutJob::default();
        let plain = egui::TextFormat::simple(font, ink);
        let matched = egui::TextFormat::simple(theme::body_strong(), color::light_blue());
        let std::ops::Range { start, end } = self.hit.clone();
        job.append(&self.label[..start], 0.0, plain.clone());
        job.append(&self.label[start..end], 0.0, matched);
        job.append(&self.label[end..], 0.0, plain);
        job.wrap = TextWrapping::truncate_at_width(room.max(0.0));
        ui.fonts_mut(|f| f.layout_job(job))
    }
}

impl Widget for MenuRow<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        ui.add_enabled_ui(self.enabled, |ui| {
            let width = ui.available_width();
            let (rect, response) =
                ui.allocate_exact_size(vec2(width, size::CONTROL), Sense::click());
            response.widget_info(|| {
                WidgetInfo::selected(
                    WidgetType::SelectableLabel,
                    response.enabled(),
                    self.checked,
                    self.label,
                )
            });
            let enabled = response.enabled();
            let lit = enabled && (self.lit || response.hovered() || response.has_focus());
            if lit {
                ui.painter()
                    .rect_filled(rect, CornerRadius::same(radius::SMALL), color::border());
            }
            let option = self.slot == CheckSlot::Trailing;
            let ink = match () {
                _ if !enabled => color::ink_4(),
                _ if option && !lit && !self.checked => color::ink_2(),
                _ => color::ink(),
            };
            let secondary = color::ink_3();
            let font = if option && self.checked {
                theme::body_strong()
            } else {
                theme::body()
            };
            let (mut left, mut right) = (
                rect.left() + space::FIELD_PAD,
                rect.right() - space::FIELD_PAD,
            );
            let y = rect.center().y;
            let check_x = match self.slot {
                CheckSlot::Leading => {
                    left += size::ICON + space::S8;
                    left - space::S8 - size::ICON / 2.0
                }
                CheckSlot::Trailing => {
                    right -= size::ICON + space::S8;
                    right + space::S8 + size::ICON / 2.0
                }
                CheckSlot::None => 0.0,
            };
            if self.checked && self.slot != CheckSlot::None {
                paint_icon(ui.painter(), pos2(check_x, y), icon::CHECK, ink);
            }
            let half = ((right - left) / 2.0).max(0.0);
            let shortcut = fit_text(ui, self.shortcut, theme::body(), secondary, half);
            let shortcut_left = right - shortcut.size().x;
            let label_right = if self.shortcut.is_empty() {
                right
            } else {
                shortcut_left - space::S12
            };
            let text = self.label_galley(ui, font, ink, label_right - left);
            let elided = text.elided || shortcut.elided;
            paint_left(ui, left, y, text, ink);
            paint_left(ui, shortcut_left, y, shortcut, secondary);
            if elided {
                let full = if self.shortcut.is_empty() {
                    self.label.to_owned()
                } else {
                    format!("{} ({})", self.label, self.shortcut)
                };
                return response.on_hover_text(full);
            }
            response
        })
        .inner
    }
}

/// A 1px Border rule between groups of menu rows, 4px clear above and below.
pub fn menu_separator(ui: &mut Ui) {
    let size = vec2(ui.available_width(), 2.0 * space::S4 + 1.0);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let stroke = hairline(ui.ctx(), color::border());
    ui.painter().hline(rect.x_range(), rect.center().y, stroke);
}

/// A row of mutually exclusive options on one Ground track, the selected one Raised.
#[must_use = "add it with `ui.add(..)`"]
pub struct Segmented<'a> {
    selected: &'a mut usize,
    options: &'a [&'a str],
    /// Each option's name when `options` are icon glyphs.
    names: Option<&'a [&'a str]>,
    state: State,
    off: &'a [Option<&'a str>],
    tips: &'a [Option<&'a str>],
    height: f32,
    width: Option<f32>,
    pills: bool,
    track: Option<Color32>,
}

impl<'a> Segmented<'a> {
    pub fn new(selected: &'a mut usize, options: &'a [&'a str]) -> Self {
        Self {
            selected,
            options,
            names: None,
            state: State::Live,
            off: &[],
            tips: &[],
            height: size::CONTROL,
            width: None,
            pills: false,
            track: None,
        }
    }

    pub fn height(mut self, height: f32) -> Self {
        self.height = height.max(size::CONTROL);
        self
    }

    /// Equal segments filling a fixed width; long labels elide within their segment.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width.max(0.0));
        self
    }

    pub fn track(mut self, track: Color32) -> Self {
        self.track = Some(track);
        self
    }

    /// Icon segments: `glyphs` drawn in the icon font, each named by `names` in its tooltip.
    pub fn icons(selected: &'a mut usize, glyphs: &'a [&'a str], names: &'a [&'a str]) -> Self {
        Self {
            names: Some(names),
            ..Self::new(selected, glyphs)
        }
    }

    /// Per option: `Some(why)` disables it, with `why` on hover.
    pub fn off(mut self, off: &'a [Option<&'a str>]) -> Self {
        self.off = off;
        self
    }

    /// Per option: hover text while it's enabled.
    pub fn tips(mut self, tips: &'a [Option<&'a str>]) -> Self {
        self.tips = tips;
        self
    }

    /// Pins the state of the first unselected segment.
    pub fn state(mut self, state: State) -> Self {
        self.state = state;
        self
    }
}

impl Widget for Segmented<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let font = match self.names {
            Some(_) => theme::icon_font(),
            None => theme::body_strong(),
        };
        let (pad, gap, side) = if self.pills {
            (0.0, space::S4, space::BUTTON_PAD)
        } else {
            (2.0, 2.0, space::BUTTON_PAD - 2.0)
        };
        let mut widths: Vec<f32> = self
            .options
            .iter()
            .map(|o| match self.names {
                Some(_) => self.height - 2.0 * pad,
                None => (text_width(ui, o, &font) + 2.0 * side).ceil(),
            })
            .collect();
        let chrome = 2.0 * pad + gap * widths.len().saturating_sub(1) as f32;
        let natural = widths.iter().sum::<f32>();
        let total = self.width.unwrap_or(natural + chrome);
        let total = if self.pills || self.width.is_some() {
            total.min(ui.available_width())
        } else {
            total
        };
        if self.width.is_some() {
            widths.fill(((total - chrome) / self.options.len().max(1) as f32).max(0.0));
        } else if total < natural + chrome && natural > 0.0 {
            for w in &mut widths {
                *w *= ((total - chrome) / natural).max(0.0);
            }
        }
        let (rect, mut response) =
            allocate(ui, vec2(total, self.height), self.state, Sense::hover());
        let disabled = self.state == State::Disabled || !ui.is_enabled();
        // Pills stand alone; segments share a track.
        let track = match self.pills {
            true => self.track,
            false => Some(self.track.unwrap_or_else(color::ground)),
        };
        if let Some(track) = track {
            ui.painter()
                .rect_filled(rect, CornerRadius::same(radius::CONTROL), track);
        }
        let (corner, selected_fill) = if self.pills {
            ((self.height / 2.0) as u8, color::border())
        } else {
            (radius::SMALL, color::raised())
        };
        let first_unselected = (0..self.options.len()).find(|i| i != self.selected);
        let mut x = rect.left() + pad;
        for (i, (option, w)) in self.options.iter().zip(&widths).enumerate() {
            let seg = Rect::from_x_y_ranges(x..=x + w, rect.top() + pad..=rect.bottom() - pad);
            x += w + gap;
            let off = self.off.get(i).copied().flatten();
            let pinned = if Some(i) == first_unselected {
                self.state
            } else if disabled || off.is_some() {
                State::Disabled
            } else {
                State::Live
            };
            let seg_response = ui.interact(
                seg,
                response.id.with(i),
                if disabled || off.is_some() {
                    Sense::hover()
                } else {
                    Sense::click()
                },
            );
            let on = i == *self.selected;
            let name = self.names.map_or(*option, |n| n[i]);
            seg_response.widget_info(|| {
                WidgetInfo::selected(
                    WidgetType::RadioButton,
                    !disabled && off.is_none(),
                    on,
                    name,
                )
            });
            let tip = off
                .or(self.tips.get(i).copied().flatten())
                .or(self.names.map(|_| name));
            let seg_response = match tip {
                Some(tip) => seg_response.on_hover_text(tip),
                None => seg_response,
            };
            if seg_response.clicked() {
                if *self.selected != i {
                    response.mark_changed();
                }
                *self.selected = i;
            }
            let l = look(&seg_response, pinned);
            let l = Look {
                disabled: l.disabled || disabled || off.is_some(),
                ..l
            };
            // Keyboard focus shows the selected pill's fill, never a ring.
            let fill = match () {
                _ if l.disabled => Color32::TRANSPARENT,
                _ if on => selected_fill,
                _ if l.focus => color::border(),
                _ => Color32::TRANSPARENT,
            };
            ui.painter()
                .rect_filled(seg, CornerRadius::same(corner), fill);
            let ink = match () {
                _ if l.disabled => color::ink_4(),
                _ if on => color::ink(),
                _ => color::ink_2(),
            };
            if self.pills || self.width.is_some() {
                let text = fit_text(
                    ui,
                    option,
                    font.clone(),
                    ink,
                    (seg.width() - space::S8).max(0.0),
                );
                let at = Align2::CENTER_CENTER
                    .align_size_within_rect(text.size(), seg)
                    .min;
                ui.painter().galley(at, text, ink);
            } else {
                ui.painter().text(
                    seg.center(),
                    Align2::CENTER_CENTER,
                    *option,
                    font.clone(),
                    ink,
                );
            }
        }
        response
    }
}

/// Content-sized pill choices 4px apart, the selected one filled.
#[must_use = "add it with `ui.add(..)`"]
pub struct Pills<'a>(Segmented<'a>);

impl<'a> Pills<'a> {
    pub fn new(selected: &'a mut usize, options: &'a [&'a str]) -> Self {
        Self(Segmented {
            pills: true,
            ..Segmented::new(selected, options)
        })
    }
    pub fn height(mut self, height: f32) -> Self {
        self.0 = self.0.height(height);
        self
    }
    pub fn track(mut self, track: Color32) -> Self {
        self.0 = self.0.track(track);
        self
    }
    pub fn state(mut self, state: State) -> Self {
        self.0 = self.0.state(state);
        self
    }
}

impl Widget for Pills<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        self.0.ui(ui)
    }
}

/// The check drawn in a ticked box, in the middle of `check`.
fn paint_tick(ui: &Ui, check: Rect, ink: Color32) {
    // Lucide's check on a 24 unit grid, drawn 4pt smaller than the box and heavier.
    let side = check.width() - 4.0;
    let at = |x: f32, y: f32| check.center() + (vec2(x, y) - Vec2::splat(12.0)) * side / 24.0;
    let points = vec![at(5.5, 12.5), at(10.0, 17.0), at(18.5, 7.5)];
    ui.painter()
        .add(Shape::line(points, Stroke::new(side / 8.0, ink)));
}

/// An 18px checkbox with a label.
#[must_use = "add it with `ui.add(..)`"]
pub struct Checkbox<'a> {
    checked: &'a mut bool,
    label: &'a str,
    state: State,
}

impl<'a> Checkbox<'a> {
    pub fn new(checked: &'a mut bool, label: &'a str) -> Self {
        Self {
            checked,
            label,
            state: State::Live,
        }
    }

    pub fn state(mut self, state: State) -> Self {
        self.state = state;
        self
    }
}

impl Widget for Checkbox<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let box_size = SELECTION_DIAMETER;
        let width = box_size + space::S8 + text_width(ui, self.label, &theme::body());
        let (rect, mut response) = allocate(
            ui,
            vec2(width.ceil(), size::CONTROL),
            self.state,
            Sense::click(),
        );
        if response.clicked() {
            *self.checked = !*self.checked;
            response.mark_changed();
        }
        response.widget_info(|| {
            WidgetInfo::selected(WidgetType::Checkbox, true, *self.checked, self.label)
        });
        let l = look(&response, self.state);
        let ppp = ui.pixels_per_point();
        let check = Rect::from_min_size(
            pos2(rect.left(), rect.center().y - box_size / 2.0),
            Vec2::splat(box_size),
        )
        .round_to_pixels(ppp);
        let corner = CornerRadius::same(radius::SMALL);
        let (fill, edge, tick) = match () {
            _ if l.disabled => (color::raised(), color::border(), color::ink_4()),
            _ if *self.checked => (color::blue(), color::blue(), color::WHITE),
            _ if l.focus => (Color32::TRANSPARENT, color::light_blue(), color::WHITE),
            _ => (Color32::TRANSPARENT, color::border_strong(), color::WHITE),
        };
        ui.painter().rect_filled(check, corner, fill);
        ui.painter().add(border(ui, check, radius::SMALL, edge));
        if *self.checked {
            paint_tick(ui, check, tick);
        }
        let ink = if l.disabled {
            color::ink_4()
        } else {
            color::ink()
        };
        ui.painter().text(
            pos2(check.right() + space::S8, rect.center().y),
            Align2::LEFT_CENTER,
            self.label,
            theme::body(),
            ink,
        );
        if l.focus {
            paint_focus(ui, check, radius::SMALL);
        }
        response
    }
}

/// Whether a property is animated, and whether it has a keyframe at the playhead.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Keyed {
    #[default]
    Static,
    Animated,
    /// Animated, with a keyframe at the playhead.
    Here,
}

#[derive(Clone, Copy)]
enum KeyGlyph {
    Diamond(Keyed),
    Previous,
    Next,
}

/// A 28px keyframe control: the diamond that animates a property and shows a key at the
/// playhead, or an arrow to the previous or next keyframe.
#[must_use = "add it with `ui.add(..)`"]
pub struct KeyButton<'a> {
    glyph: KeyGlyph,
    name: &'a str,
    state: State,
}

impl<'a> KeyButton<'a> {
    /// An outline at rest, Light Blue once animated, filled when keyed at the playhead.
    pub fn diamond(keyed: Keyed, name: &'a str) -> Self {
        Self::new(KeyGlyph::Diamond(keyed), name)
    }

    pub fn previous(name: &'a str) -> Self {
        Self::new(KeyGlyph::Previous, name)
    }

    pub fn next(name: &'a str) -> Self {
        Self::new(KeyGlyph::Next, name)
    }

    fn new(glyph: KeyGlyph, name: &'a str) -> Self {
        Self {
            glyph,
            name,
            state: State::Live,
        }
    }

    pub fn state(mut self, state: State) -> Self {
        self.state = state;
        self
    }
}

impl Widget for KeyButton<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let (rect, response) = allocate(
            ui,
            Vec2::splat(size::SMALL_CONTROL),
            self.state,
            Sense::click(),
        );
        let on = matches!(self.glyph, KeyGlyph::Diamond(Keyed::Here));
        response.widget_info(|| {
            WidgetInfo::selected(
                WidgetType::Button,
                self.state != State::Disabled,
                on,
                self.name,
            )
        });
        let l = look(&response, self.state);
        // Keyboard focus shows a fill, never a ring.
        let rest = if l.focus {
            color::border()
        } else {
            Color32::TRANSPARENT
        };
        let lit = l.hover || l.pressed;
        let fill = hover_fade(ui, response.id, lit, rest, color::raised());
        ui.painter()
            .rect_filled(rect, CornerRadius::same(radius::SMALL), fill);
        let ink = match () {
            _ if l.disabled => color::ink_4(),
            _ if lit => color::ink(),
            _ => color::ink_3(),
        };
        // The timeline's diamond size, so a row's key matches its lane.
        let (c, h) = (rect.center(), 5.0);
        let shape = match self.glyph {
            KeyGlyph::Diamond(keyed) => {
                let points = vec![
                    c + vec2(0.0, -h),
                    c + vec2(h, 0.0),
                    c + vec2(0.0, h),
                    c + vec2(-h, 0.0),
                ];
                let (fill, line) = match keyed {
                    _ if l.disabled => (Color32::TRANSPARENT, color::ink_4()),
                    Keyed::Static => (Color32::TRANSPARENT, ink),
                    Keyed::Animated => (Color32::TRANSPARENT, color::light_blue()),
                    Keyed::Here => (color::light_blue(), color::light_blue()),
                };
                Shape::convex_polygon(points, fill, Stroke::new(1.0, line))
            }
            KeyGlyph::Previous | KeyGlyph::Next => {
                let dx = if matches!(self.glyph, KeyGlyph::Next) {
                    1.0
                } else {
                    -1.0
                };
                let points = vec![
                    c + vec2(-2.0 * dx, -4.0),
                    c + vec2(2.0 * dx, 0.0),
                    c + vec2(-2.0 * dx, 4.0),
                ];
                Shape::line(points, Stroke::new(1.5, ink))
            }
        };
        ui.painter().add(shape);
        response
    }
}

/// A slider for a ranged value: a 6px track filled Blue from its start, or from zero when
/// `centred`, and a round 16px thumb. Drag or click to set; arrow keys step 1% of the range.
/// Takes the width left in the row, or the height left in the column when vertical.
#[must_use = "add it with `ui.add(..)`"]
pub struct Slider<'a> {
    value: &'a mut f64,
    range: std::ops::RangeInclusive<f64>,
    step: Option<f64>,
    centred: bool,
    vertical: bool,
    name: &'a str,
    state: State,
}

impl<'a> Slider<'a> {
    /// `name` is for screen readers.
    pub fn new(value: &'a mut f64, range: std::ops::RangeInclusive<f64>, name: &'a str) -> Self {
        Self {
            value,
            range,
            step: None,
            centred: false,
            vertical: false,
            name,
            state: State::Live,
        }
    }

    /// Snaps to multiples of `step` from the start, marked with ticks; arrow keys move a step.
    pub fn step(mut self, step: f64) -> Self {
        self.step = Some(step);
        self
    }

    /// Fills from zero, for values that go either way from it.
    pub fn centred(mut self, centred: bool) -> Self {
        self.centred = centred;
        self
    }

    /// Runs bottom to top, with Up/Down stepping the value. Use `ui.add_sized` to set height.
    pub fn vertical(mut self) -> Self {
        self.vertical = true;
        self
    }

    pub fn state(mut self, state: State) -> Self {
        self.state = state;
        self
    }
}

impl Widget for Slider<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        // The thumb plus room for its focus ring.
        let across = size::THUMB + 4.0;
        let desired = if self.vertical {
            vec2(across, ui.available_height().max(40.0))
        } else {
            vec2(ui.available_width().max(40.0), across)
        };
        let (rect, mut response) = allocate(ui, desired, self.state, Sense::click_and_drag());
        let (lo, hi) = (*self.range.start(), *self.range.end());
        // The thumb's centre travels inside the track so it never overhangs the row.
        let inset = size::THUMB / 2.0;
        let track = if self.vertical {
            rect.shrink2(vec2(0.0, inset)).y_range()
        } else {
            rect.shrink2(vec2(inset, 0.0)).x_range()
        };
        let position_of = |v: f64| {
            let t = ((v - lo) / (hi - lo)).clamp(0.0, 1.0) as f32;
            track.min + (if self.vertical { 1.0 - t } else { t }) * track.span()
        };
        let snap = |v: f64| match self.step {
            Some(s) => (lo + ((v - lo) / s).round() * s).clamp(lo, hi),
            None => v,
        };
        if response.clicked() || response.drag_started() {
            response.request_focus();
        }
        if (response.dragged() || response.clicked())
            && let Some(p) = response.interact_pointer_pos()
        {
            let axis = if self.vertical { p.y } else { p.x };
            let t = ((axis - track.min) / track.span()).clamp(0.0, 1.0) as f64;
            let t = if self.vertical { 1.0 - t } else { t };
            let v = snap(lo + t * (hi - lo));
            if v != *self.value {
                *self.value = v;
                response.mark_changed();
            }
        }
        if response.has_focus() && response.enabled() && self.state != State::Disabled {
            ui.memory_mut(|m| {
                m.set_focus_lock_filter(
                    response.id,
                    egui::EventFilter {
                        horizontal_arrows: !self.vertical,
                        vertical_arrows: self.vertical,
                        ..Default::default()
                    },
                );
            });
            let step = ui.input(|i| {
                let (increase, decrease) = if self.vertical {
                    (Key::ArrowUp, Key::ArrowDown)
                } else {
                    (Key::ArrowRight, Key::ArrowLeft)
                };
                i32::from(i.key_pressed(increase)) - i32::from(i.key_pressed(decrease))
            });
            if step != 0 {
                let by = self.step.unwrap_or((hi - lo) / 100.0);
                *self.value = snap(*self.value + f64::from(step) * by).clamp(lo, hi);
                response.mark_changed();
            }
        }
        response.widget_info(|| {
            WidgetInfo::slider(
                response.enabled() && self.state != State::Disabled,
                *self.value,
                self.name,
            )
        });
        ui.ctx().accesskit_node_builder(response.id, |node| {
            node.set_orientation(if self.vertical {
                egui::accesskit::Orientation::Vertical
            } else {
                egui::accesskit::Orientation::Horizontal
            });
        });
        let l = look(&response, self.state);
        let cross = if self.vertical {
            rect.center().x
        } else {
            rect.center().y
        };
        let point = |axis, cross| {
            if self.vertical {
                pos2(cross, axis)
            } else {
                pos2(axis, cross)
            }
        };
        let box_of = |axis: egui::Rangef, cross: egui::Rangef| {
            Rect::from_two_pos(point(axis.min, cross.min), point(axis.max, cross.max))
        };
        let from = if self.centred { 0.0 } else { lo };
        let (a, b) = (position_of(from), position_of(*self.value));
        let painter = ui.painter();
        let half = size::SLIDER_TRACK / 2.0;
        if self.centred {
            let line = 1.0 / ui.pixels_per_point();
            painter.rect_filled(
                box_of(
                    (a - line / 2.0..=a + line / 2.0).into(),
                    (cross - half - 4.0..=cross + half + 4.0).into(),
                ),
                0.0,
                color::border_strong(),
            );
        }
        let bar = |axis: egui::Rangef| box_of(axis, (cross - half..=cross + half).into());
        // The track runs the whole row; the thumb's centre stops its radius short of each end.
        let whole = if self.vertical {
            rect.y_range()
        } else {
            rect.x_range()
        };
        painter.rect_filled(bar(whole), half, color::border_strong());
        if let Some(s) = self.step {
            let line = 1.0 / ui.pixels_per_point();
            for k in 0..=((hi - lo) / s).round() as usize {
                let x = position_of(lo + k as f64 * s);
                let tick = box_of(
                    (x - line / 2.0..=x + line / 2.0).into(),
                    (cross + half + 2.0..=cross + half + 5.0).into(),
                );
                painter.rect_filled(tick, 0.0, color::border_strong());
            }
        }
        if !l.disabled {
            // From the start the fill takes the track's round end; from zero it is square.
            let filled = match (self.centred, self.vertical) {
                (true, _) => egui::Rangef::new(a.min(b), a.max(b)),
                (false, true) => egui::Rangef::new(b, whole.max),
                (false, false) => egui::Rangef::new(whole.min, b),
            };
            painter.rect_filled(bar(filled), half, color::blue());
        }
        let thumb = point(b, cross);
        let ink = if l.disabled {
            color::ink_4()
        } else {
            color::ink()
        };
        if l.focus {
            let ring = Stroke::new(4.0, color::blue_soft());
            let past_clip = painter.with_clip_rect(ui.clip_rect().expand(4.0));
            past_clip.circle_stroke(thumb, size::THUMB / 2.0 + 2.0, ring);
        }
        painter.circle_filled(thumb, size::THUMB / 2.0, ink);
        response
    }
}

/// A colour wheel for grading: a puck in a recessed Field disc with a 1px Border edge and a
/// muted hue rim, BT.709 Cb across and Cr up so it reads like the vectorscope. Drag or click to
/// place the puck, double-click to centre it, arrow keys nudge it 2% of the radius. `value` is
/// the puck in the unit disc, y up.
#[must_use = "add it with `ui.add(..)`"]
pub struct ColorWheel<'a> {
    value: &'a mut [f64; 2],
    diameter: f32,
    name: &'a str,
    state: State,
}

impl<'a> ColorWheel<'a> {
    /// `name` is for screen readers.
    pub fn new(value: &'a mut [f64; 2], diameter: f32, name: &'a str) -> Self {
        Self {
            value,
            diameter,
            name,
            state: State::Live,
        }
    }

    pub fn state(mut self, state: State) -> Self {
        self.state = state;
        self
    }
}

/// Where in the unit disc (y up) a pointer `d` from the centre puts the puck, `travel` the
/// radius it moves in.
fn wheel_value(d: Vec2, travel: f32) -> [f64; 2] {
    let v = vec2(d.x, -d.y) / travel;
    let v = v / v.length().max(1.0);
    [v.x as f64, v.y as f64]
}

/// The rim's hue at `angle` from the Cb axis towards Cr: a zero-luma shift off mid grey.
fn wheel_hue(angle: f32) -> Color32 {
    let (cr, cb) = (0.12 * angle.sin(), 0.12 * angle.cos());
    let c = |v: f32| ((0.5 + v).clamp(0.0, 1.0) * 255.0) as u8;
    Color32::from_rgb(
        c(1.5748 * cr),
        c(-0.1873 * cb - 0.4681 * cr),
        c(1.8556 * cb),
    )
}

impl Widget for ColorWheel<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let (rect, mut response) = allocate(
            ui,
            Vec2::splat(self.diameter),
            self.state,
            Sense::click_and_drag(),
        );
        let c = rect.center();
        let r = rect.width() / 2.0;
        // The puck stays inside the rim.
        let travel = r - 6.0;
        if response.double_clicked() {
            *self.value = [0.0; 2];
            response.mark_changed();
        } else if (response.dragged() || response.clicked())
            && let Some(p) = response.interact_pointer_pos()
        {
            let v = wheel_value(p - c, travel);
            if v != *self.value {
                *self.value = v;
                response.mark_changed();
            }
        }
        if response.has_focus() {
            let (x, y) = ui.input(|i| {
                let k = |a, b| f64::from(i32::from(i.key_pressed(a)) - i32::from(i.key_pressed(b)));
                (
                    k(Key::ArrowRight, Key::ArrowLeft),
                    k(Key::ArrowUp, Key::ArrowDown),
                )
            });
            if x != 0.0 || y != 0.0 {
                let [vx, vy] = *self.value;
                let to = vec2((vx + 0.02 * x) as f32, -(vy + 0.02 * y) as f32);
                *self.value = wheel_value(to, 1.0);
                response.mark_changed();
            }
        }
        response.widget_info(|| {
            WidgetInfo::labeled(WidgetType::Slider, self.state != State::Disabled, self.name)
        });
        let l = look(&response, self.state);
        let painter = ui.painter();
        painter.circle_filled(c, r, color::field());
        // The hue rim, faded when disabled.
        let rim = r - 2.5;
        let n = 48;
        let at = |i: usize| {
            let a = i as f32 / n as f32 * std::f32::consts::TAU;
            (a, c + vec2(a.cos(), -a.sin()) * rim)
        };
        let fade = if l.disabled { 0.3 } else { 0.8 };
        for i in 0..n {
            let ((a, p0), (_, p1)) = (at(i), at(i + 1));
            // Opaque, so the segments' overlaps don't bead.
            let hue =
                color::field().lerp_to_gamma(wheel_hue(a + std::f32::consts::PI / n as f32), fade);
            painter.line_segment([p0, p1], Stroke::new(2.0, hue));
        }
        let edge = hover_fade(
            ui,
            response.id,
            l.hover || response.dragged(),
            color::border(),
            color::border_strong(),
        );
        painter.circle_stroke(c, r - 0.5, hairline(ui.ctx(), edge));
        // Centre tick.
        let tick = hairline(ui.ctx(), color::border_strong());
        painter.hline(c.x - 3.0..=c.x + 3.0, c.y, tick);
        painter.vline(c.x, c.y - 3.0..=c.y + 3.0, tick);
        let [vx, vy] = *self.value;
        let puck = c + vec2(vx as f32, -vy as f32) * travel;
        if *self.value != [0.0; 2] {
            painter.line_segment([c, puck], hairline(ui.ctx(), color::ink_3()));
        }
        let ink = if l.disabled {
            color::ink_4()
        } else {
            let lit = l.hover || l.pressed || response.dragged();
            hover_fade(
                ui,
                response.id.with("puck"),
                lit,
                color::ink_2(),
                color::ink(),
            )
        };
        painter.circle(puck, 4.0, ink, Stroke::new(1.0, color::field()));
        if l.focus {
            painter.circle_stroke(c, r + 2.0, Stroke::new(RING, color::blue_soft()));
        }
        response
    }
}

/// What a curve editor's untouched curve looks like, drawn as its guide line.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CurveNeutral {
    /// Output equals input: the diagonal.
    #[default]
    Diagonal,
    /// 0.5 changes nothing: a flat line across the middle.
    Middle,
}

/// A grading curve on a small graph: a recessed Field box with a 1px Border edge (Border Strong
/// on hover), quarter gridlines, the neutral line, the curve in Ink and its points. Pressing
/// adds a point there, or picks up the nearest within reach; dragging moves it between its
/// neighbours; double-clicking removes it. With focus, the arrow keys nudge the last point
/// touched 1% and Delete removes it. `points` are x ascending, both axes 0..=1, y up; `at`
/// evaluates the curve through them.
#[must_use = "add it with `ui.add(..)`"]
pub struct CurveEditor<'a> {
    points: &'a mut Vec<[f64; 2]>,
    at: &'a dyn Fn(&[[f64; 2]], f64) -> f64,
    size: Vec2,
    name: &'a str,
    neutral: CurveNeutral,
    hue: bool,
    max: usize,
    state: State,
}

impl<'a> CurveEditor<'a> {
    /// `name` is for screen readers.
    pub fn new(
        points: &'a mut Vec<[f64; 2]>,
        at: &'a dyn Fn(&[[f64; 2]], f64) -> f64,
        size: Vec2,
        name: &'a str,
    ) -> Self {
        Self {
            points,
            at,
            size,
            name,
            neutral: CurveNeutral::Diagonal,
            hue: false,
            max: usize::MAX,
            state: State::Live,
        }
    }

    pub fn neutral(mut self, neutral: CurveNeutral) -> Self {
        self.neutral = neutral;
        self
    }

    /// The x axis is hue: a muted spectrum along the bottom edge.
    pub fn hue(mut self, hue: bool) -> Self {
        self.hue = hue;
        self
    }

    /// The most points a press may add up to.
    pub fn max_points(mut self, max: usize) -> Self {
        self.max = max;
        self
    }

    pub fn state(mut self, state: State) -> Self {
        self.state = state;
        self
    }
}

/// Points closer in x than this stay apart while dragging.
const CURVE_GAP: f64 = 0.01;
/// How near, in points, a press picks up an existing point.
const CURVE_REACH: f32 = 8.0;

/// Where point `i` may go in x: between its neighbours, `CURVE_GAP` clear of them.
fn curve_room(points: &[[f64; 2]], i: usize) -> (f64, f64) {
    let lo = i.checked_sub(1).map_or(0.0, |j| points[j][0] + CURVE_GAP);
    let hi = points.get(i + 1).map_or(1.0, |p| p[0] - CURVE_GAP);
    (lo, hi.max(lo))
}

/// Moves point `i` to `to`, kept in the graph and between its neighbours.
fn curve_move(points: &mut [[f64; 2]], i: usize, [x, y]: [f64; 2]) {
    let (lo, hi) = curve_room(points, i);
    points[i] = [x.clamp(lo, hi), y.clamp(0.0, 1.0)];
}

/// Adds a point at `p` in x order, or replaces one already within `CURVE_GAP`; its index.
fn curve_insert(points: &mut Vec<[f64; 2]>, p: [f64; 2]) -> usize {
    let i = points.partition_point(|q| q[0] < p[0]);
    let near = [i, i.wrapping_sub(1)].into_iter().find(|&j| {
        points
            .get(j)
            .is_some_and(|q| (q[0] - p[0]).abs() < CURVE_GAP)
    });
    match near {
        Some(j) => points[j] = p,
        None => points.insert(i, p),
    }
    near.unwrap_or(i)
}

impl Widget for CurveEditor<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let (rect, mut response) = allocate(ui, self.size, self.state, Sense::click_and_drag());
        // The points' travel, inset so a point on the edge stays whole.
        let graph = rect.shrink(4.0);
        // Hue wraps, so its 1 is its 0: keep points short of it.
        let right = if self.hue { 1.0 - CURVE_GAP } else { 1.0 };
        let to_value = |p: egui::Pos2| {
            let v = (p - graph.left_bottom()) / graph.size();
            [
                f64::from(v.x).clamp(0.0, right),
                f64::from(-v.y).clamp(0.0, 1.0),
            ]
        };
        let to_screen = |[x, y]: [f64; 2]| {
            graph.left_bottom() + vec2(x as f32 * graph.width(), -(y as f32) * graph.height())
        };
        let active_id = response.id.with("active");
        let mut active: Option<usize> = ui.data(|d| d.get_temp(active_id)).flatten();
        let points = self.points;
        active = active.filter(|i| *i < points.len());
        let pressed =
            response.is_pointer_button_down_on() && ui.input(|i| i.pointer.primary_pressed());
        if let Some(p) = response.interact_pointer_pos() {
            if pressed {
                let near = points
                    .iter()
                    .map(|q| to_screen(*q).distance(p))
                    .enumerate()
                    .min_by(|a, b| a.1.total_cmp(&b.1))
                    .filter(|(_, d)| *d <= CURVE_REACH);
                active = match near {
                    Some((i, _)) => Some(i),
                    None if points.len() < self.max => {
                        response.mark_changed();
                        Some(curve_insert(points, to_value(p)))
                    }
                    None => None,
                };
            } else if response.dragged()
                && let Some(i) = active
            {
                let before = points[i];
                curve_move(points, i, to_value(p));
                if points[i] != before {
                    response.mark_changed();
                }
            }
        }
        if response.double_clicked()
            && let Some(i) = active.take()
        {
            points.remove(i);
            response.mark_changed();
        }
        if response.has_focus()
            && let Some(i) = active
        {
            let (x, y, del) = ui.input(|i| {
                let k = |a, b| f64::from(i32::from(i.key_pressed(a)) - i32::from(i.key_pressed(b)));
                (
                    k(Key::ArrowRight, Key::ArrowLeft),
                    k(Key::ArrowUp, Key::ArrowDown),
                    i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace),
                )
            });
            if del {
                points.remove(i);
                active = None;
                response.mark_changed();
            } else if x != 0.0 || y != 0.0 {
                let [px, py] = points[i];
                curve_move(points, i, [px + 0.01 * x, py + 0.01 * y]);
                response.mark_changed();
            }
        }
        ui.data_mut(|d| d.insert_temp(active_id, active));
        response.widget_info(|| {
            WidgetInfo::labeled(WidgetType::Slider, self.state != State::Disabled, self.name)
        });

        let l = look(&response, self.state);
        let painter = ui.painter_at(rect.expand(4.0));
        let corner = radius::CONTROL;
        painter.rect_filled(rect, CornerRadius::same(corner), color::field());
        let grid = hairline(ui.ctx(), color::border());
        for q in [0.25, 0.5, 0.75] {
            painter.vline(graph.left() + q * graph.width(), graph.y_range(), grid);
            painter.hline(graph.x_range(), graph.bottom() - q * graph.height(), grid);
        }
        if self.hue {
            // A muted spectrum: hue x at full saturation, faded into the Field.
            let n = 48;
            let fade = if l.disabled { 0.3 } else { 0.6 };
            for i in 0..n {
                let (a, b) = (i as f32 / n as f32, (i + 1) as f32 / n as f32);
                let hue = egui::ecolor::Hsva::new((a + b) / 2.0, 0.8, 0.9, 1.0);
                let c = color::field().lerp_to_gamma(hue.into(), fade);
                let y = graph.bottom() + 1.0;
                let (x0, x1) = (
                    graph.left() + a * graph.width(),
                    graph.left() + b * graph.width(),
                );
                painter.line_segment([pos2(x0, y), pos2(x1, y)], Stroke::new(2.0, c));
            }
        }
        let guide = hairline(ui.ctx(), color::border_strong());
        match self.neutral {
            CurveNeutral::Diagonal => {
                painter.line_segment([graph.left_bottom(), graph.right_top()], guide);
            }
            CurveNeutral::Middle => {
                painter.hline(graph.x_range(), graph.center().y, guide);
            }
        }
        let ink = if l.disabled {
            color::ink_4()
        } else {
            color::ink()
        };
        let n = graph.width().ceil().max(2.0) as usize;
        let line: Vec<_> = (0..=n)
            .map(|i| {
                let x = i as f64 / n as f64;
                to_screen([x, (self.at)(points, x)])
            })
            .collect();
        painter.add(Shape::line(line, Stroke::new(1.5, ink)));
        let edge = hover_fade(
            ui,
            response.id,
            l.hover || response.dragged(),
            color::border(),
            color::border_strong(),
        );
        painter.add(border(ui, rect, corner, edge));
        for (i, p) in points.iter().enumerate() {
            let lit = active == Some(i) && (l.focus || response.dragged());
            let fill = match () {
                _ if l.disabled => color::ink_4(),
                _ if lit => color::light_blue(),
                _ => color::ink_2(),
            };
            painter.circle(to_screen(*p), 3.5, fill, Stroke::new(1.0, color::field()));
        }
        if l.focus {
            paint_focus(ui, rect, corner);
        }
        response
    }
}

/// A colour: a 24px swatch with a Border Strong edge and a checkerboard behind it only when
/// it isn't opaque, then its hex in a field, with the alpha after it when that fits, or None
/// once fully transparent. Clicking either opens egui's picker under it. Takes the width left
/// in the row; `rgba` is straight (unmultiplied) sRGB.
#[must_use = "add it with `ui.add(..)`"]
pub struct ColorField<'a> {
    rgba: &'a mut [u8; 4],
    name: &'a str,
    state: State,
}

impl<'a> ColorField<'a> {
    /// `name` is for screen readers.
    pub fn new(rgba: &'a mut [u8; 4], name: &'a str) -> Self {
        Self {
            rgba,
            name,
            state: State::Live,
        }
    }

    pub fn state(mut self, state: State) -> Self {
        self.state = state;
        self
    }
}

/// The hex of `rgba`, and its alpha as a percentage when under 100%; `None` when fully
/// transparent.
pub fn color_text(rgba: [u8; 4]) -> Option<(String, Option<String>)> {
    let [r, g, b, a] = rgba;
    // Never 100% while not opaque.
    let alpha = (a < 255).then(|| format!("{}%", (f32::from(a) / 2.55).round().min(99.0)));
    (a > 0).then(|| (format!("{r:02X}{g:02X}{b:02X}"), alpha))
}

/// A 3x3 checkerboard filling `swatch`. Its corner cells are the light base, so the dark
/// cells never reach the rounded corners.
fn checker(ui: &Ui, swatch: Rect, corner: CornerRadius) {
    let p = ui.painter();
    p.rect_filled(swatch, corner, Color32::from_gray(0xBD));
    let cell = swatch.width() / 3.0;
    for (i, j) in [(1.0, 0.0), (0.0, 1.0), (2.0, 1.0), (1.0, 2.0)] {
        let min = swatch.min + vec2(i, j) * cell;
        p.rect_filled(
            Rect::from_min_size(min, Vec2::splat(cell)),
            0.0,
            Color32::from_gray(0x8A),
        );
    }
}

impl Widget for ColorField<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let width = ui.available_width().max(2.0 * size::CONTROL);
        let (rect, mut response) =
            allocate(ui, vec2(width, size::CONTROL), self.state, Sense::click());
        let text = color_text(*self.rgba);
        response.widget_info(|| {
            let shown = text
                .as_ref()
                .map_or(tr!(CATALOG, "color.none").to_owned(), |t| {
                    format!("{} {}", t.0, t.1.as_deref().unwrap_or_default())
                });
            WidgetInfo::labeled(
                WidgetType::ColorButton,
                self.state != State::Disabled,
                format!("{} {}", self.name, shown.trim_end()),
            )
        });
        let popup = response.id.with("picker");
        let open = egui::Popup::is_id_open(ui.ctx(), popup);
        let mut l = look(&response, self.state);
        l.focus |= open;
        let corner = CornerRadius::same(radius::CONTROL);
        let swatch = Rect::from_min_size(rect.min, Vec2::splat(size::CONTROL));
        let [r, g, b, a] = *self.rgba;
        if a < 255 {
            checker(ui, swatch, corner);
        }
        ui.painter()
            .rect_filled(swatch, corner, Color32::from_rgba_unmultiplied(r, g, b, a));
        let edge = if l.hover || l.pressed {
            color::ink_3()
        } else {
            color::border_strong()
        };
        ui.painter().add(border(ui, swatch, radius::CONTROL, edge));
        let field =
            Rect::from_x_y_ranges(swatch.right() + space::S8..=rect.right(), rect.y_range());
        let frame = ui.painter().add(Shape::Noop);
        paint_field(ui, frame, field, l, false);
        let (left, right) = (
            field.left() + space::FIELD_PAD,
            field.right() - space::FIELD_PAD,
        );
        let y = field.center().y;
        let ink = if l.disabled {
            color::ink_4()
        } else {
            color::ink()
        };
        match &text {
            None => {
                let none = tr!(CATALOG, "color.none");
                let p = pos2(left, y);
                ui.painter()
                    .text(p, Align2::LEFT_CENTER, none, theme::body(), color::ink_3());
            }
            Some((hex, alpha)) => {
                ui.painter()
                    .text(pos2(left, y), Align2::LEFT_CENTER, hex, theme::body(), ink);
                let room = right - left - text_width(ui, hex, &theme::body()) - space::S8;
                if let Some(alpha) = alpha.as_ref()
                    && text_width(ui, alpha, &theme::body()) <= room
                {
                    let p = pos2(right, y);
                    ui.painter().text(
                        p,
                        Align2::RIGHT_CENTER,
                        alpha,
                        theme::body(),
                        color::ink_3(),
                    );
                }
            }
        }
        let mut changed = false;
        egui::Popup::menu(&response)
            .id(popup)
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .show(|ui| {
                // The picker's own hue, so it survives a pass through grey or black.
                let key = popup.with("hsva");
                let rgba = *self.rgba;
                let mut hsva = ui
                    .data(|d| d.get_temp::<([u8; 4], egui::ecolor::Hsva)>(key))
                    .filter(|(c, _)| *c == rgba)
                    .map_or_else(
                        || egui::ecolor::Hsva::from_srgba_unmultiplied(rgba),
                        |(_, h)| h,
                    );
                let alpha = egui::color_picker::Alpha::OnlyBlend;
                if egui::color_picker::color_picker_hsva_2d(ui, &mut hsva, alpha) {
                    *self.rgba = hsva.to_srgba_unmultiplied();
                    changed = true;
                }
                ui.data_mut(|d| d.insert_temp(key, (*self.rgba, hsva)));
            });
        if changed {
            response.mark_changed();
        }
        response
    }
}

/// A collapsible section head on a 36px row: chevron, then the title in SemiBold Body.
/// Clicking toggles `open`.
#[must_use = "add it with `ui.add(..)`"]
pub struct SectionHeader<'a> {
    title: &'a str,
    open: &'a mut bool,
    state: State,
    reserve: f32,
}

impl<'a> SectionHeader<'a> {
    pub fn new(title: &'a str, open: &'a mut bool) -> Self {
        Self {
            title,
            open,
            state: State::Live,
            reserve: 0.0,
        }
    }

    /// Keeps `width` at the right clear of the title, for tools put over the header.
    pub fn reserve(mut self, width: f32) -> Self {
        self.reserve = width;
        self
    }

    pub fn state(mut self, state: State) -> Self {
        self.state = state;
        self
    }
}

impl Widget for SectionHeader<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let (rect, mut response) = allocate(
            ui,
            vec2(ui.available_width(), size::CONTROL),
            self.state,
            Sense::click(),
        );
        if response.clicked() {
            *self.open = !*self.open;
            response.mark_changed();
        }
        response
            .widget_info(|| WidgetInfo::selected(WidgetType::Button, true, *self.open, self.title));
        let l = look(&response, self.state);
        let ink = if l.disabled {
            color::ink_4()
        } else {
            color::ink()
        };
        let glyph = if *self.open {
            icon::CHEVRON_DOWN
        } else {
            icon::CHEVRON_RIGHT
        };
        let y = rect.center().y;
        let chevron = pos2(rect.left() + size::ICON / 2.0, y);
        paint_icon(ui.painter(), chevron, glyph, ink);
        let x = rect.left() + size::ICON + space::S4;
        let room = rect.right() - self.reserve - x;
        let title = fit_text(ui, self.title, theme::panel_title(), ink, room);
        let response = hover_full(response, &title);
        paint_left(ui, x, y, title, ink);
        response
    }
}

/// Formats a frame count as `HH:MM:SS:FF`.
pub fn timecode(frame: i64, fps: u32) -> String {
    let fps = i64::from(fps.max(1));
    let sign = if frame < 0 { "-" } else { "" };
    let f = frame.unsigned_abs() as i64;
    let (ff, s) = (f % fps, f / fps);
    format!(
        "{sign}{:02}:{:02}:{:02}:{:02}",
        s / 3600,
        s / 60 % 60,
        s % 60,
        ff
    )
}

/// A timecode in Timecode type (Medium 16), or Body Strong when `small`.
#[must_use = "add it with `ui.add(..)`"]
pub struct Timecode {
    frame: i64,
    fps: u32,
    small: bool,
}

impl Timecode {
    pub fn new(frame: i64, fps: u32) -> Self {
        Self {
            frame,
            fps,
            small: false,
        }
    }

    pub fn small(mut self) -> Self {
        self.small = true;
        self
    }
}

impl Widget for Timecode {
    fn ui(self, ui: &mut Ui) -> Response {
        let font = if self.small {
            theme::body_strong()
        } else {
            theme::timecode()
        };
        ui.add(
            egui::Label::new(
                egui::RichText::new(timecode(self.frame, self.fps))
                    .font(font)
                    .color(color::ink()),
            )
            .selectable(false),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{
        RectAlign, curve_insert, curve_move, menu_place, timecode, wheel_hue, wheel_value,
    };
    use egui::{Rect, pos2, vec2};

    #[test]
    fn curve_points_insert_in_order_and_drag_between_their_neighbours() {
        let mut p = vec![[0.0, 0.0], [1.0, 1.0]];
        assert_eq!(curve_insert(&mut p, [0.5, 0.7]), 1);
        assert_eq!(p, [[0.0, 0.0], [0.5, 0.7], [1.0, 1.0]]);
        // A press right by a point replaces it rather than crowding it.
        assert_eq!(curve_insert(&mut p, [0.995, 0.9]), 2);
        assert_eq!(p.len(), 3);
        // Dragging past a neighbour stops short of it; y stays in the graph.
        curve_move(&mut p, 1, [2.0, -1.0]);
        assert!((p[1][0] - 0.985).abs() < 1e-9 && p[1][1] == 0.0, "{p:?}");
        curve_move(&mut p, 0, [0.99, 0.2]);
        assert!((p[0][0] - 0.975).abs() < 1e-9, "{p:?}");
    }

    #[test]
    fn wheel_puck_follows_the_pointer_inside_the_disc_with_y_up() {
        assert_eq!(wheel_value(vec2(10.0, -10.0), 20.0), [0.5, 0.5]);
        // Past the rim it stays on it, in the same direction.
        let [x, y] = wheel_value(vec2(0.0, 80.0), 20.0);
        assert!(x.abs() < 1e-6 && (y + 1.0).abs() < 1e-6);
        // Up is Cr, towards red; right is Cb, towards blue.
        let up = wheel_hue(std::f32::consts::FRAC_PI_2);
        assert!(up.r() > up.b());
        let right = wheel_hue(0.0);
        assert!(right.b() > right.r());
    }

    #[test]
    fn menus_open_toward_the_room_and_stop_at_the_bounds() {
        let bounds = Rect::from_min_max(pos2(0.0, 0.0), pos2(400.0, 500.0));
        let field = |y: f32| Rect::from_min_max(pos2(10.0, y), pos2(200.0, y + 28.0));
        // A short list fits below.
        assert_eq!(
            menu_place(field(100.0), bounds, 100.0).0,
            RectAlign::BOTTOM_START
        );
        // A long one near the bottom flips up and is capped to the room there.
        let (align, h) = menu_place(field(400.0), bounds, 1000.0);
        assert_eq!(align, RectAlign::TOP_START);
        assert!(h < 400.0 - 8.0, "{h}");
        // With more room below it stays below, capped.
        let (align, h) = menu_place(field(50.0), bounds, 1000.0);
        assert_eq!(align, RectAlign::BOTTOM_START);
        assert!(50.0 + 28.0 + h < 500.0, "{h}");
    }

    #[test]
    fn timecode_formats() {
        assert_eq!(timecode(0, 30), "00:00:00:00");
        assert_eq!(timecode(30 * 3661 + 7, 30), "01:01:01:07");
        assert_eq!(timecode(-1, 25), "-00:00:00:01");
    }
}
