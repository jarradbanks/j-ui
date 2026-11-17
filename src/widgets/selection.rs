//! Switches, selection indicators and detail-page navigation.

use super::*;

/// The corner that makes a [`SELECTION_DIAMETER`] box a circle.
const ROUND: u8 = (SELECTION_DIAMETER / 2.0) as u8;

/// Logical-point geometry shared by selection controls and composite tiles.
pub const SELECTION_TILE: f32 = 20.0;
pub const SELECTION_DIAMETER: f32 = 18.0;

/// Paints a circular selection indicator without adding interaction or a focus ring.
pub fn paint_selection(ui: &Ui, tile: Rect, selected: bool, mixed: bool) {
    let center = tile.center();
    let radius = SELECTION_DIAMETER / 2.0;
    let blue = color::blue();
    ui.painter().circle_filled(
        center,
        radius,
        if selected && !mixed {
            blue
        } else {
            Color32::from_black_alpha(178)
        },
    );
    if mixed {
        let points = (0..=32)
            .map(|i| {
                let angle = std::f32::consts::FRAC_PI_2 + std::f32::consts::PI * i as f32 / 32.0;
                center + vec2(angle.cos(), angle.sin()) * radius
            })
            .collect();
        ui.painter()
            .add(Shape::convex_polygon(points, blue, Stroke::NONE));
    }
    ui.painter().circle_stroke(
        center,
        radius - 0.5,
        Stroke::new(
            1.0,
            if selected || mixed {
                blue
            } else {
                color::ink_3()
            },
        ),
    );
}

/// A circular multi-selection checkbox; activating a mixed selection selects all.
/// Use [`Checkbox`] for ordinary options and approvals.
#[must_use = "add it with `ui.add(..)`"]
pub struct SelectionCheckbox<'a> {
    selected: &'a mut bool,
    label: &'a str,
    mixed: bool,
    id: Option<Id>,
    state: State,
}

impl<'a> SelectionCheckbox<'a> {
    pub fn new(selected: &'a mut bool, label: &'a str) -> Self {
        Self {
            selected,
            label,
            mixed: false,
            id: None,
            state: State::Live,
        }
    }
    pub fn mixed(mut self, mixed: bool) -> Self {
        self.mixed = mixed;
        self
    }
    pub fn id(mut self, id: Id) -> Self {
        self.id = Some(id);
        self
    }
    pub fn state(mut self, state: State) -> Self {
        self.state = state;
        self
    }
}

impl Widget for SelectionCheckbox<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let (rect, allocated) = allocate(
            ui,
            Vec2::splat(SELECTION_TILE),
            self.state,
            if self.id.is_some() {
                Sense::hover()
            } else {
                Sense::click()
            },
        );
        let mut response = match self.id {
            Some(id) => ui.interact(
                rect,
                id,
                if self.state == State::Disabled {
                    Sense::hover()
                } else {
                    Sense::click()
                },
            ),
            None => allocated,
        };
        if response.clicked() {
            *self.selected = self.mixed || !*self.selected;
            response.mark_changed();
            response.request_focus();
        }
        let mixed = self.mixed && !response.changed();
        response.widget_info(|| {
            WidgetInfo::selected(
                WidgetType::Checkbox,
                self.state != State::Disabled && response.enabled(),
                *self.selected,
                self.label,
            )
        });
        ui.ctx().accesskit_node_builder(response.id, |node| {
            node.set_toggled(if mixed {
                egui::accesskit::Toggled::Mixed
            } else if *self.selected {
                egui::accesskit::Toggled::True
            } else {
                egui::accesskit::Toggled::False
            });
        });
        paint_selection(ui, rect, *self.selected, mixed);
        let l = look(&response, self.state);
        if l.hover && !l.disabled {
            ui.painter().circle_stroke(
                rect.center(),
                SELECTION_DIAMETER / 2.0 - 0.5,
                Stroke::new(1.0, color::light_blue()),
            );
        }
        let circle = Rect::from_center_size(rect.center(), Vec2::splat(SELECTION_DIAMETER));
        paint_focus(ui, circle, ROUND, response.id, l.focus);
        response.on_hover_text(self.label)
    }
}

/// A 32 × 18 switch with a 14-point knob and an accessible switch role.
#[must_use = "add it with `ui.add(..)`"]
pub struct Toggle<'a> {
    value: &'a mut bool,
    label: &'a str,
    state: State,
    show_label: bool,
}

impl<'a> Toggle<'a> {
    pub fn new(value: &'a mut bool, label: &'a str) -> Self {
        Self {
            value,
            label,
            state: State::Live,
            show_label: false,
        }
    }
    /// Includes the label in the 36-point click target, 8 points after the switch.
    pub fn show_label(mut self) -> Self {
        self.show_label = true;
        self
    }

    pub fn state(mut self, state: State) -> Self {
        self.state = state;
        self
    }
}

impl Widget for Toggle<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let size = vec2(2.0 * SELECTION_DIAMETER - 4.0, SELECTION_DIAMETER);
        let desired = if self.show_label {
            vec2(
                size.x + space::S8 + text_width(ui, self.label, &theme::body()),
                size::CONTROL,
            )
        } else {
            size
        };
        let (row, mut response) = allocate(ui, desired, self.state, Sense::click());
        let rect = Rect::from_center_size(pos2(row.left() + size.x / 2.0, row.center().y), size);
        if response.clicked() {
            *self.value = !*self.value;
            response.mark_changed();
        }
        response.widget_info(|| {
            WidgetInfo::selected(
                WidgetType::Checkbox,
                self.state != State::Disabled && response.enabled(),
                *self.value,
                self.label,
            )
        });
        ui.ctx().accesskit_node_builder(response.id, |node| {
            node.set_role(egui::accesskit::Role::Switch)
        });
        let l = look(&response, self.state);
        let contrast_outline = color::selection_fill() != color::border();
        let fill = if l.disabled || (contrast_outline && !*self.value) {
            color::selection_fill()
        } else if *self.value {
            color::blue()
        } else {
            color::border_strong()
        };
        let half = rect.height() / 2.0;
        let fill = motion::color(ui.ctx(), response.id.with("fill"), fill);
        ui.painter().rect_filled(rect, half, fill);
        if contrast_outline && !*self.value {
            ui.painter()
                .add(border(ui, rect, ROUND, color::border_strong()));
        }
        let x = if *self.value {
            rect.right() - half
        } else {
            rect.left() + half
        };
        let knob = if l.disabled {
            color::ink_4()
        } else {
            Color32::WHITE
        };
        ui.painter()
            .circle_filled(pos2(x, rect.center().y), half - 2.0, knob);
        if self.show_label {
            let ink = if l.disabled {
                color::ink_4()
            } else {
                color::ink()
            };
            paint_text(
                ui,
                pos2(rect.right() + space::S8, row.center().y),
                Align2::LEFT_CENTER,
                self.label,
                theme::body(),
                ink,
            );
        }
        paint_focus(ui, rect, ROUND, response.id, l.focus);
        response
    }
}

/// A row of mutually exclusive options, each an 18px radio button with a label.
#[must_use = "add it with `ui.add(..)`"]
pub struct RadioGroup<'a> {
    selected: &'a mut usize,
    options: &'a [&'a str],
    state: State,
}

impl<'a> RadioGroup<'a> {
    pub fn new(selected: &'a mut usize, options: &'a [&'a str]) -> Self {
        Self {
            selected,
            options,
            state: State::Live,
        }
    }

    /// Pins the state of the first unselected option.
    pub fn state(mut self, state: State) -> Self {
        self.state = state;
        self
    }
}

impl Widget for RadioGroup<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let first_unselected = (0..self.options.len()).find(|i| i != self.selected);
        let row = ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = space::GAP;
            let mut changed = false;
            for (i, option) in self.options.iter().enumerate() {
                let pinned = match Some(i) == first_unselected || self.state == State::Disabled {
                    true => self.state,
                    false => State::Live,
                };
                let width = SELECTION_DIAMETER + space::S8 + text_width(ui, option, &theme::body());
                let size = vec2(width.ceil(), size::CONTROL);
                let (rect, response) = allocate(ui, size, pinned, Sense::click());
                let on = i == *self.selected;
                response.widget_info(|| {
                    WidgetInfo::selected(
                        WidgetType::RadioButton,
                        pinned != State::Disabled && response.enabled(),
                        on,
                        *option,
                    )
                });
                if response.clicked() && !on {
                    *self.selected = i;
                    changed = true;
                }
                let l = look(&response, pinned);
                let center = pos2(rect.left() + SELECTION_DIAMETER / 2.0, rect.center().y);
                let r = SELECTION_DIAMETER / 2.0;
                let (edge, ink) = match () {
                    _ if l.disabled => (color::border(), color::ink_4()),
                    _ if l.focus => (color::light_blue(), color::ink()),
                    _ => (color::border_strong(), color::ink()),
                };
                let edge = if on && !l.disabled {
                    color::blue()
                } else {
                    edge
                };
                let edge = motion::color(ui.ctx(), response.id.with("edge"), edge);
                if on {
                    let ring = edge;
                    ui.painter().circle_filled(center, r, ring);
                    // The ring is 1/3.5 of the diameter wide; the rest is the white dot.
                    let dot = r - SELECTION_DIAMETER / 3.5;
                    ui.painter().circle_filled(center, dot, color::WHITE);
                } else {
                    let stroke = hairline(ui.ctx(), edge);
                    ui.painter()
                        .circle_stroke(center, r - stroke.width / 2.0, stroke);
                }
                paint_text(
                    ui,
                    pos2(
                        rect.left() + SELECTION_DIAMETER + space::S8,
                        rect.center().y,
                    ),
                    Align2::LEFT_CENTER,
                    option,
                    theme::body(),
                    ink,
                );
                let circle = Rect::from_center_size(center, Vec2::splat(SELECTION_DIAMETER));
                paint_focus(ui, circle, ROUND, response.id, l.focus);
            }
            changed
        });
        let mut response = row.response;
        if row.inner {
            response.mark_changed();
        }
        response
    }
}

/// A detail page's trail: the parent, which returns to it, a chevron, and the elided title.
#[must_use = "add it with `ui.add(..)`"]
pub struct Breadcrumb<'a> {
    parent: &'a str,
    title: &'a str,
    height: f32,
}

impl<'a> Breadcrumb<'a> {
    /// One line of Body text, which also clears the chevron.
    const LINE: f32 = 20.0;

    pub fn new(parent: &'a str, title: &'a str) -> Self {
        Self {
            parent,
            title,
            height: Self::LINE,
        }
    }
    pub fn height(mut self, height: f32) -> Self {
        self.height = height.max(Self::LINE);
        self
    }
}

impl Widget for Breadcrumb<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let (row, _) =
            ui.allocate_exact_size(vec2(ui.available_width(), self.height), Sense::hover());
        let y = row.center().y;
        let width = text_width(ui, self.parent, &theme::body());
        let link = Rect::from_min_size(row.min, vec2(width, row.height()));
        let response = ui.interact(link, ui.id().with("breadcrumb"), Sense::click());
        response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, self.parent));
        // Hover and keyboard focus lift the parent to Ink, never a ring.
        let ink = if response.hovered() || response.has_focus() {
            color::ink()
        } else {
            color::ink_3()
        };
        paint_text(
            ui,
            row.left_center(),
            Align2::LEFT_CENTER,
            self.parent,
            theme::body(),
            ink,
        );
        let chevron = pos2(link.right() + space::S4 + size::ICON / 2.0, y);
        paint_icon(ui.painter(), chevron, icon::CHEVRON_RIGHT, color::ink_3());
        let x = chevron.x + size::ICON / 2.0 + space::S4;
        let room = (row.right() - x).max(0.0);
        let title = fit_text(ui, self.title, theme::body_strong(), color::ink(), room);
        let pos = text_pos(&title, x, y);
        paint_fit(ui, pos, title, color::ink());
        response
    }
}
