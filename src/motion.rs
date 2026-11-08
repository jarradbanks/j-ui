//! Short, bounded egui animations. Zero duration also settles egui's own Area fades.

use egui::{Context, Id, Ui, emath::TSTransform, epaint::ClippedShape, layers::ShapeIdx};

const SECONDS: f32 = 0.16;

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
