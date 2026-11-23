//! Every engine id stored as data must be written as an object id and translated back
//! (`kdm_model`'s `refs` module and its explicit cases), or be per-person state the replica never
//! holds. This lists every model field typed with a node id (and the artboard and graphic style
//! references), so a new one fails here until the bridge handles it.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

const SOURCES: &[(&str, &str)] = &[
    ("appearance.rs", include_str!("../../../engine/doc/src/appearance.rs")),
    ("assets.rs", include_str!("../../../engine/doc/src/assets.rs")),
    ("blend.rs", include_str!("../../../engine/doc/src/blend.rs")),
    ("graph.rs", include_str!("../../../engine/doc/src/graph.rs")),
    ("hit.rs", include_str!("../../../engine/doc/src/hit.rs")),
    ("lib.rs", include_str!("../../../engine/doc/src/lib.rs")),
    ("links.rs", include_str!("../../../engine/doc/src/links.rs")),
    ("live.rs", include_str!("../../../engine/doc/src/live.rs")),
    ("node.rs", include_str!("../../../engine/doc/src/node.rs")),
    ("pattern.rs", include_str!("../../../engine/doc/src/pattern.rs")),
    ("perspective.rs", include_str!("../../../engine/doc/src/perspective.rs")),
    ("placed_document.rs", include_str!("../../../engine/doc/src/placed_document.rs")),
    ("pin_warp.rs", include_str!("../../../engine/doc/src/pin_warp.rs")),
    ("selection.rs", include_str!("../../../engine/doc/src/selection.rs")),
    ("setup.rs", include_str!("../../../engine/doc/src/setup.rs")),
    ("shaper.rs", include_str!("../../../engine/doc/src/shaper.rs")),
    ("slices.rs", include_str!("../../../engine/doc/src/slices.rs")),
    ("text.rs", include_str!("../../../engine/doc/src/text.rs")),
    ("variables.rs", include_str!("../../../engine/doc/src/variables.rs")),
];

/// `file: field` for every field whose type names an id, and how the bridge handles it.
const HANDLED: &[&str] = &[
    // Definitions: each is an object's own identity, written in its object's read-only `id` (its
    // tool id) and kept by a read unless another object of it holds it (then minted afresh).
    "node.rs: id: NodeId",
    "slices.rs: id: NodeId",
    // Explicit translations (refs::map_setting, refs::map_asset_nodes, the node `props` register;
    // a guide's `layer` and `artboard` refs and a thread's `frames` list).
    "assets.rs: nodes: Vec<NodeId>",
    "lib.rs: objects: Vec<NodeId>",
    "lib.rs: layer: Option<NodeId>",
    "lib.rs: artboard: Option<u32>",
    "lib.rs: text_threads: Vec<Vec<NodeId>>",
    "variables.rs: bindings: BTreeMap<NodeId, BTreeSet<String>>",
    "node.rs: graphic_style: Option<u32>",
    // Per person or derived, never in the replica: opacity-mask and pattern editing (`MaskEdit`,
    // never serialized; `PatternEdit`, left out by `shared_part`), the open Layers rows (view
    // state), warp pins, the selection, hit tests and laid-out slices.
    "pattern.rs: layer: NodeId",
    "lib.rs: object: NodeId",
    "lib.rs: layer: NodeId",
    "lib.rs: layers_open: Option<Vec<NodeId>>",
    "pin_warp.rs: ids: Vec<NodeId>",
    "selection.rs: objects: Vec<NodeId>",
    "selection.rs: anchors: BTreeMap<NodeId, BTreeSet<AnchorRef>>",
    "selection.rs: key: Option<NodeId>",
    "selection.rs: target: Option<NodeId>",
    "selection.rs: slices: Vec<NodeId>",
    "hit.rs: leaf: NodeId",
    "hit.rs: ancestry: Vec<NodeId>",
    "hit.rs: contents_of: Option<NodeId>",
    "hit.rs: scope: Option<NodeId>",
    "slices.rs: id: Option<NodeId>",
];

#[test]
fn every_id_field_is_handled() {
    let mut found = Vec::new();
    for (file, source) in SOURCES {
        for line in source.lines() {
            let line = line.trim();
            if line.starts_with("//") || line.contains("fn ") || line.contains("=>") || line.contains(" = ") || line.starts_with("let ") {
                continue;
            }
            let line = line.trim_start_matches("pub(crate) ").trim_start_matches("pub ").trim_end_matches(',');
            let Some((name, ty)) = line.split_once(": ") else { continue };
            if !name.chars().all(|c| c.is_ascii_lowercase() || c == '_') {
                continue;
            }
            let names_id = ty.contains("NodeId") || (["artboard", "graphic_style"].contains(&name) && ty.contains("u32"));
            if names_id && !ty.contains("NodeId(") {
                found.push(format!("{file}: {name}: {ty}"));
            }
        }
    }
    found.sort();
    found.dedup();
    let unhandled: Vec<_> = found.iter().filter(|f| !HANDLED.contains(&f.as_str())).collect();
    assert!(unhandled.is_empty(), "model fields holding ids that kdm-model does not translate: {unhandled:#?}");
    let stale: Vec<_> = HANDLED.iter().filter(|h| !found.iter().any(|f| f == *h)).collect();
    assert!(stale.is_empty(), "listed as handled but no longer in the model (update this list): {stale:#?}");
}
