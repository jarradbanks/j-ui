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
    id: egui::Id,
    path: String,
    label: String,
    source: String,
}

impl DomainTarget {
    pub fn new(response: &Response, path: &str, label: &str, location: &Location<'_>) -> Self {
        Self {
            id: response.id,
            path: path.chars().take(512).collect(),
            label: label.chars().take(512).collect(),
            source: format!("{}:{}", location.file(), location.line()),
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
    for id in ordered {
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
        let domain = custom.iter().find(|target| target.id == egui_id);
        let Some(mut target) = from_response(ctx, &response, &label) else {
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
            .position(|layer| *layer == response.layer_id)
            .unwrap_or(0);
        targets.push((layer, target));
        if targets.len() >= 2048 {
            break;
        }
    }
    for domain in custom {
        if targets.iter().any(|(_, target)| target.path == domain.path) {
            continue;
        }
        let Some(response) = ctx.read_response(domain.id) else {
            continue;
        };
        let Some(mut target) = from_response(ctx, &response, &domain.label) else {
            continue;
        };
        target.id = format!("domain/{}", domain.path);
        target.path.clone_from(&domain.path);
        target.source = Some(domain.source.clone());
        let layer = layers
            .iter()
            .position(|layer| *layer == response.layer_id)
            .unwrap_or(0);
        targets.push((layer, target));
    }
    targets.sort_by_key(|(layer, _)| *layer);
    targets
        .into_iter()
        .enumerate()
        .map(|(order, (_, mut target))| {
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

fn from_response(ctx: &Context, response: &Response, label: &str) -> Option<Target> {
    let bounds = ctx.viewport_rect();
    let transform = ctx
        .layer_transform_to_global(response.layer_id)
        .unwrap_or_default();
    let rect = (transform * response.interact_rect).intersect(bounds);
    if !rect.is_positive() || !rect.is_finite() || !bounds.is_positive() {
        return None;
    }
    let id = format!("{:016x}", response.id.value());
    Some(Target {
        path: format!("egui/{id}"),
        id,
        label: label.into(),
        role: "Custom".into(),
        source: None,
        rect: normalized(rect, bounds),
        z_order: 0,
        enabled: response.enabled()
            && ctx.memory(|memory| memory.allows_interaction(response.layer_id)),
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
