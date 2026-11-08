//! Switches, selection indicators and detail-page navigation.

use super::*;

/// Logical-point geometry shared by selection controls and composite tiles.
pub const SELECTION_TILE: f32 = 20.0;
pub const SELECTION_DIAMETER: f32 = 14.0;

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
        if l.focus {
            ui.painter().add(focus_ring(rect, radius::SMALL));
        }
        response.on_hover_text(self.label)
    }
}

/// A 40 × 22 switch with an 18-point knob and an accessible switch role.
#[must_use = "add it with `ui.add(..)`"]
pub struct Toggle<'a> {
    value: &'a mut bool,
    label: &'a str,
    state: State,
}

impl<'a> Toggle<'a> {
    pub fn new(value: &'a mut bool, label: &'a str) -> Self {
        Self {
            value,
            label,
            state: State::Live,
        }
    }
    pub fn state(mut self, state: State) -> Self {
        self.state = state;
        self
    }
}

impl Widget for Toggle<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let (rect, mut response) = allocate(ui, vec2(40.0, 22.0), self.state, Sense::click());
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
        let fill = if *self.value {
            color::blue()
        } else {
            color::border_strong()
        };
        ui.painter().rect_filled(rect, 11.0, fill);
        let x = if *self.value {
            rect.right() - 11.0
        } else {
            rect.left() + 11.0
        };
        ui.painter()
            .circle_filled(pos2(x, rect.center().y), 9.0, Color32::WHITE);
        if look(&response, self.state).focus {
            ui.painter().add(focus_ring(rect, 11));
        }
        response
    }
}

/// A return button followed by a separator and an elided detail-page title.
#[must_use = "add it with `ui.add(..)`"]
pub struct Breadcrumb<'a> {
    parent: &'a str,
    title: &'a str,
    height: f32,
}

impl<'a> Breadcrumb<'a> {
    pub fn new(parent: &'a str, title: &'a str) -> Self {
        Self {
            parent,
            title,
            height: size::CONTROL,
        }
    }
    pub fn height(mut self, height: f32) -> Self {
        self.height = height.max(size::CONTROL);
        self
    }
}

impl Widget for Breadcrumb<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let (row, _) =
            ui.allocate_exact_size(vec2(ui.available_width(), self.height), Sense::hover());
        ui.scope_builder(
            egui::UiBuilder::new()
                .max_rect(row)
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
            |ui| {
                let response = ui.add(
                    Button::secondary(self.parent)
                        .with_icon("")
                        .height(self.height),
                );
                let center = response.rect.left_center() + vec2(20.0, 0.0);
                ui.painter().add(Shape::line(
                    vec![
                        center + vec2(2.0, -4.0),
                        center - vec2(2.0, 0.0),
                        center + vec2(2.0, 4.0),
                    ],
                    Stroke::new(1.0, color::ink()),
                ));
                let separator = ui.painter().text(
                    response.rect.right_center() + vec2(space::S8, 0.0),
                    Align2::LEFT_CENTER,
                    "/",
                    theme::body(),
                    color::ink_4(),
                );
                let x = separator.right() + space::S8;
                let title = fit_text(
                    ui,
                    self.title,
                    theme::body_strong(),
                    color::ink(),
                    (row.right() - x).max(0.0),
                );
                paint_fit(
                    ui,
                    pos2(x, row.center().y - title.size().y / 2.0),
                    title,
                    color::ink(),
                );
                response
            },
        )
        .inner
    }
}
