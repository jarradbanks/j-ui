//! Short, bounded egui animations. Zero duration also settles egui's own Area fades.

use egui::{Color32, Context, Id, Ui, emath::TSTransform, epaint::ClippedShape, layers::ShapeIdx};

const SECONDS: f32 = 0.16;

#[derive(Clone, Copy)]
struct ColorTransition {
    from: Color32,
    to: Color32,
    started: f64,
}

/// A 120ms CSS-ease color transition; interruptions continue from the painted color.
pub(crate) fn color(ctx: &Context, id: Id, target: Color32) -> Color32 {
    let id = id.with("color-transition");
    if !enabled(ctx) {
        ctx.data_mut(|data| data.remove::<ColorTransition>(id));
        return target;
    }
    let now = ctx.input(|input| input.time);
    let mut state = ctx
        .data_mut(|data| data.get_temp::<ColorTransition>(id))
        .unwrap_or(ColorTransition {
            from: target,
            to: target,
            started: now,
        });
    let elapsed = ((now - state.started) / 0.12).clamp(0.0, 1.0) as f32;
    let current = state.from.lerp_to_gamma(state.to, ease(elapsed));
    if state.to != target {
        state = ColorTransition {
            from: current,
            to: target,
            started: now,
        };
    }
    if state.from != state.to && now - state.started < 0.12 {
        ctx.request_repaint();
    }
    ctx.data_mut(|data| data.insert_temp(id, state));
    current
}

// CSS ease is cubic-bezier(0.25, 0.1, 0.25, 1); solve x before sampling y.
fn ease(progress: f32) -> f32 {
    if progress == 0.0 || progress == 1.0 {
        return progress;
    }
    let (mut low, mut high) = (0.0, 1.0);
    for _ in 0..16 {
        let t = (low + high) / 2.0;
        let x = 0.75 * (1.0 - t) * t + t * t * t;
        if x < progress {
            low = t;
        } else {
            high = t;
        }
    }
    let t = (low + high) / 2.0;
    0.3 * (1.0 - t).powi(2) * t + 3.0 * (1.0 - t) * t * t + t * t * t
}

pub fn set_enabled(ctx: &Context, enabled: bool) {
    ctx.all_styles_mut(|style| style.animation_time = if enabled { SECONDS } else { 0.0 });
}

pub fn enabled(ctx: &Context) -> bool {
    ctx.global_style().animation_time > 0.0
}

#[derive(Clone, Copy)]
struct Value {
    from: f32,
    to: f32,
}

pub fn value(ctx: &Context, id: Id, target: f32) -> f32 {
    let state_id = id.with("motion-value");
    if !enabled(ctx) {
        ctx.data_mut(|d| d.remove::<Value>(state_id));
        ctx.animate_value_with_time(id, 1.0, 0.0);
        return target;
    }
    let mut state = ctx
        .data_mut(|d| d.get_temp::<Value>(state_id))
        .unwrap_or(Value {
            from: target,
            to: target,
        });
    let t = ctx.animate_value_with_time(id, 1.0, SECONDS);
    let current = egui::lerp(state.from..=state.to, egui::emath::easing::cubic_out(t));
    if state.to != target {
        state = Value {
            from: current,
            to: target,
        };
        ctx.animate_value_with_time(id, 0.0, 0.0);
        ctx.animate_value_with_time(id, 1.0, SECONDS);
    }
    ctx.data_mut(|d| d.insert_temp(state_id, state));
    current
}

pub fn toggle(ctx: &Context, id: Id, on: bool) -> f32 {
    value(ctx, id, f32::from(on))
}

pub fn restart(ctx: &Context, id: Id) {
    ctx.data_mut(|d| d.insert_temp(id.with("motion-value"), Value { from: 0.0, to: 1.0 }));
    ctx.animate_value_with_time(id, 0.0, 0.0);
}

pub fn enter(ctx: &Context, id: Id) -> f32 {
    if enabled(ctx) && !ctx.data_mut(|d| d.get_temp::<bool>(id.with("entered")).unwrap_or(false)) {
        restart(ctx, id);
        ctx.data_mut(|d| d.insert_temp(id.with("entered"), true));
    }
    value(ctx, id, 1.0)
}

/// A paint-only copy never retains widget input or selection state.
#[derive(Clone, Default)]
pub struct Paint(pub Vec<ClippedShape>);

impl Paint {
    pub fn start(ui: &Ui) -> ShapeIdx {
        ui.ctx().graphics_mut(|g| g.entry(ui.layer_id()).next_idx())
    }

    pub fn capture(ui: &Ui, start: ShapeIdx) -> Self {
        Self(ui.ctx().graphics(|g| {
            g.get(ui.layer_id())
                .map(|p| p.all_entries().skip(start.0).cloned().collect())
                .unwrap_or_default()
        }))
    }

    pub fn draw(&self, ui: &Ui, opacity: f32, transform: TSTransform) {
        if opacity <= 0.0 {
            return;
        }
        ui.ctx().graphics_mut(|g| {
            let list = g.entry(ui.layer_id());
            for clipped in &self.0 {
                let mut shape = clipped.shape.clone();
                shape.transform(transform);
                fade(&mut shape, opacity);
                list.add(clipped.clip_rect.intersect(ui.clip_rect()), shape);
            }
        });
    }

    pub fn fade_since(ui: &Ui, start: ShapeIdx, opacity: f32) {
        if opacity == 1.0 {
            return;
        }
        ui.ctx().graphics_mut(|g| {
            let list = g.entry(ui.layer_id());
            for index in start.0..list.next_idx().0 {
                list.mutate_shape(ShapeIdx(index), |s| fade(&mut s.shape, opacity));
            }
        });
    }
}

fn fade(shape: &mut egui::Shape, opacity: f32) {
    if opacity != 1.0 {
        egui::epaint::shape_transform::adjust_colors(shape, move |c| {
            if *c != egui::Color32::PLACEHOLDER {
                *c = c.gamma_multiply(opacity);
            }
        });
    }
}

/// Areas already fade in using style.animation_time; add a small scale without changing layout.
pub fn install(ctx: &Context) {
    set_enabled(ctx, true);
    #[cfg(windows)]
    windows::watch(ctx.clone());
    ctx.on_end_pass(
        "popover-motion",
        std::sync::Arc::new(|ctx| {
            if !enabled(ctx) {
                return;
            }
            let layers = ctx.memory(|m| m.areas().visible_layer_ids());
            for layer in layers
                .into_iter()
                .filter(|l| l.order == egui::Order::Foreground)
            {
                let painted = ctx.graphics(|g| g.get(layer).is_some_and(|p| !p.is_empty()));
                if !painted {
                    continue;
                }
                let id = layer.id.with("popover-motion");
                if !ctx.memory(|m| m.areas().visible_last_frame(&layer)) {
                    restart(ctx, id);
                }
                let t = value(ctx, id, 1.0);
                if t < 1.0
                    && let Some(area) = egui::AreaState::load(ctx, layer.id)
                {
                    let center = area.rect().center();
                    let scale = egui::lerp(0.97..=1.0, t);
                    let transform = TSTransform::from_translation(center.to_vec2())
                        * TSTransform::from_scaling(scale)
                        * TSTransform::from_translation(-center.to_vec2());
                    let screen = ctx.content_rect();
                    ctx.graphics_mut(|g| {
                        let list = g.entry(layer);
                        for index in 0..list.next_idx().0 {
                            list.mutate_shape(ShapeIdx(index), |s| {
                                // Keep modal backdrops at the viewport bounds.
                                if !s.shape.visual_bounding_rect().contains_rect(screen) {
                                    s.shape.transform(transform);
                                }
                            });
                        }
                    });
                }
            }
        }),
    );
}

#[cfg(windows)]
#[allow(unsafe_code)]
mod windows;

#[cfg(test)]
mod tests;
