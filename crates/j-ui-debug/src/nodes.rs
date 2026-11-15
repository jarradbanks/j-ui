use std::collections::HashMap;
use std::panic::Location;

use egui::{Context, Rect, Response, accesskit};

/// Rectangles use normalized root-viewport coordinates after clipping and layer transforms.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub(crate) struct Target {
    pub id: String,
    pub path: String,
    pub label: String,
    pub role: String,
    pub source: Option<String>,
    pub rect: [f32; 4],
    pub z_order: usize,
    pub enabled: bool,
}

pub(crate) struct DomainTarget {
    geometry: Geometry,
    path: String,
    label: String,
    source: String,
}

impl DomainTarget {
    pub fn new(
        ui: &egui::Ui,
        response: &Response,
        path: &str,
        label: &str,
        location: &Location<'_>,
    ) -> Self {
        Self {
            geometry: Geometry::new(response, ui.clip_rect()),
            path: path.chars().take(512).collect(),
            label: label.chars().take(512).collect(),
            source: format!("{}:{}", location.file(), location.line()),
        }
    }
}

// A Response owns its Context; retain local geometry without creating a plugin/context cycle.
struct Geometry {
    id: egui::Id,
    layer_id: egui::LayerId,
    rect: Rect,
    enabled: bool,
}

impl Geometry {
    fn new(response: &Response, clip: Rect) -> Self {
        Self {
            id: response.id,
            layer_id: response.layer_id,
            rect: response
                .rect
                .intersect(response.interact_rect)
                .intersect(clip),
            enabled: response.enabled(),
        }
    }
}

pub(crate) fn collect(
    ctx: &Context,
    update: Option<&accesskit::TreeUpdate>,
    custom: &[DomainTarget],
) -> Vec<Target> {
    let Some(update) = update else {
        return Vec::new();
    };
    let nodes: HashMap<_, _> = update.nodes.iter().map(|(id, node)| (*id, node)).collect();
    let layers = ctx.memory(|memory| memory.layer_ids().collect::<Vec<_>>());
    let mut ordered = Vec::new();
    if let Some(tree) = &update.tree {
        visit(tree.root, &nodes, &mut ordered, 0);
    }
    let mut targets = Vec::new();
    for (order, id) in ordered.iter().copied().enumerate() {
        let node = nodes[&id];
        if id.0 == 0
            || node.is_hidden()
            || matches!(
                node.role(),
                accesskit::Role::TextRun
                    | accesskit::Role::Unknown
                    | accesskit::Role::GenericContainer
                    | accesskit::Role::Window
            )
        {
            continue;
        }
        // SAFETY: AccessKit IDs in this update came directly from egui's high-entropy Ids.
        #[allow(unsafe_code)]
        let egui_id = unsafe { egui::Id::from_high_entropy_bits(id.0) };
        let Some(response) = ctx.read_response(egui_id) else {
            continue;
        };
        let label = label(node, &nodes, 0);
        let domain = custom.iter().find(|target| target.geometry.id == egui_id);
        let geometry = Geometry::new(&response, Rect::EVERYTHING);
        let geometry = domain.map_or(&geometry, |domain| &domain.geometry);
        let Some(mut target) = from_geometry(ctx, geometry, &label) else {
            continue;
        };
        target.role = format!("{:?}", node.role());
        if let Some(domain) = domain {
            target.id = format!("domain/{}", domain.path);
            target.path.clone_from(&domain.path);
            target.label.clone_from(&domain.label);
            target.source = Some(domain.source.clone());
        }
        let layer = layers
            .iter()
            .position(|layer| *layer == geometry.layer_id)
            .unwrap_or(0);
        targets.push((layer, order, target));
        if targets.len() >= 2048 {
            break;
        }
    }
    for domain in custom {
        if targets
            .iter()
            .any(|(_, _, target)| target.path == domain.path)
        {
            continue;
        }
        let Some(mut target) = from_geometry(ctx, &domain.geometry, &domain.label) else {
            continue;
        };
        target.id = format!("domain/{}", domain.path);
        target.path.clone_from(&domain.path);
        target.source = Some(domain.source.clone());
        let layer = layers
            .iter()
            .position(|layer| *layer == domain.geometry.layer_id)
            .unwrap_or(0);
        let order = ordered
            .iter()
            .position(|id| id.0 == domain.geometry.id.value())
            .unwrap_or(ordered.len());
        targets.push((layer, order, target));
    }
    targets.sort_by_key(|(layer, order, _)| (*layer, *order));
    targets
        .into_iter()
        .enumerate()
        .map(|(order, (_, _, mut target))| {
            target.z_order = order;
            target
        })
        .collect()
}

fn visit(
    id: accesskit::NodeId,
    nodes: &HashMap<accesskit::NodeId, &accesskit::Node>,
    ordered: &mut Vec<accesskit::NodeId>,
    depth: usize,
) {
    if depth > 64 || ordered.len() >= 8192 || ordered.contains(&id) {
        return;
    }
    let Some(node) = nodes.get(&id) else {
        return;
    };
    ordered.push(id);
    for child in node.children() {
        visit(*child, nodes, ordered, depth + 1);
    }
}

fn label(
    node: &accesskit::Node,
    nodes: &HashMap<accesskit::NodeId, &accesskit::Node>,
    depth: usize,
) -> String {
    if depth > 8 {
        return String::new();
    }
    if let Some(label) = node.label().or_else(|| node.value()) {
        return label.chars().take(512).collect();
    }
    let labels: Vec<_> = node
        .labelled_by()
        .iter()
        .chain(node.children())
        .filter_map(|id| nodes.get(id).map(|node| label(node, nodes, depth + 1)))
        .filter(|label| !label.is_empty())
        .collect();
    labels.join(" ").chars().take(512).collect()
}

fn from_geometry(ctx: &Context, geometry: &Geometry, label: &str) -> Option<Target> {
    let bounds = ctx.viewport_rect();
    let transform = ctx
        .layer_transform_to_global(geometry.layer_id)
        .unwrap_or_default();
    let rect = (transform * geometry.rect).intersect(bounds);
    if !rect.is_positive() || !rect.is_finite() || !bounds.is_positive() {
        return None;
    }
    let id = format!("{:016x}", geometry.id.value());
    Some(Target {
        path: format!("egui/{id}"),
        id,
        label: label.into(),
        role: "Custom".into(),
        source: None,
        rect: normalized(rect, bounds),
        z_order: 0,
        enabled: geometry.enabled
            && ctx.memory(|memory| memory.allows_interaction(geometry.layer_id)),
    })
}

fn normalized(rect: Rect, bounds: Rect) -> [f32; 4] {
    [
        (rect.left() - bounds.left()) / bounds.width(),
        (rect.top() - bounds.top()) / bounds.height(),
        rect.width() / bounds.width(),
        rect.height() / bounds.height(),
    ]
}
