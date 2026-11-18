//! What a command that picks its objects by a point or a stroke acts on (C8-vector-5 item 2):
//! `path.knife`, `path.eraseRegion`, `path.joinScrub`, `path.blob`, `shaper.select`,
//! `shaper.scribble` and `symbol.adjust` work on the selection, else on every object under the
//! point or the stroke. [`picked`] answers those objects in a document and selection given (the
//! command's own query, before the parts of it that need the session's paint), so the agent's
//! premise guard can ask which objects a place named in a document as it was.

use kition_vector_doc::{Document, NodeId, NodeKind, Selection};
use kition_vector_geom::Point;
use serde_json::Value;

use super::{brushsym, distortcmds, draw2, shaper};

/// The commands [`picked`] answers.
pub const PICKING: &[&str] =
    &["path.knife", "path.eraseRegion", "path.joinScrub", "path.blob", "shaper.select", "shaper.scribble", "symbol.adjust", "object.liquify"];

/// The objects `command` with `params` would act on in `doc` with `selection`: the candidates its
/// own query takes (a superset of those it changes: a knife's closed paths whose bounds the cut
/// crosses, a blob's closed paths its stroke reaches, whatever their paint). `None` for another
/// command or parameters it refuses.
pub fn picked(doc: &Document, selection: &Selection, command: &str, params: &Value) -> Option<Vec<NodeId>> {
    let closed = |id: &NodeId| draw2::path_of(doc, *id).is_ok_and(|(pd, _)| pd.is_closed());
    match command {
        "path.knife" => {
            let pts = draw2::points_param(params, "points", command).ok().filter(|pts| pts.len() >= 2)?;
            Some(draw2::target_paths(doc, selection, draw2::poly_bounds(&pts, 1.0)).into_iter().filter(closed).collect())
        }
        "path.eraseRegion" => {
            let pts = draw2::points_param(params, "points", command).ok().filter(|pts| !pts.is_empty())?;
            let size = params.get("size").and_then(Value::as_f64).unwrap_or(10.0);
            (size.is_finite() && size > 0.0).then(|| draw2::target_paths(doc, selection, draw2::poly_bounds(&pts, size / 2.0)))
        }
        "path.joinScrub" => {
            let pts = draw2::points_param(params, "points", command).ok().filter(|pts| !pts.is_empty())?;
            let tol = params.get("tolerance").and_then(Value::as_f64).unwrap_or(12.0).max(0.0);
            let near = |id: &NodeId| {
                draw2::path_of(doc, *id).is_ok_and(|(pd, _)| {
                    pd.subpaths
                        .iter()
                        .filter(|sp| !sp.closed && sp.anchors.len() >= 2)
                        .any(|sp| [sp.anchors.first(), sp.anchors.last()].into_iter().flatten().any(|a| draw2::dist_to_polyline(&pts, a.p) <= tol))
                })
            };
            Some(draw2::target_paths(doc, selection, draw2::poly_bounds(&pts, tol)).into_iter().filter(near).collect())
        }
        "path.blob" => {
            if params.get("merge").and_then(Value::as_bool) == Some(false) {
                return Some(Vec::new());
            }
            let pts = draw2::points_param(params, "points", command).ok().filter(|pts| !pts.is_empty())?;
            let size = params.get("size").and_then(Value::as_f64).unwrap_or(10.0);
            if !(size.is_finite() && size > 0.0) {
                return None;
            }
            // The blob merges with paths anywhere (not only the selection's).
            Some(draw2::target_paths(doc, &Selection::default(), draw2::poly_bounds(&pts, size)).into_iter().filter(closed).collect())
        }
        "shaper.select" => {
            let point = params.get("point").and_then(point_of).filter(|p| shaper::finite(*p))?;
            let source = params.get("source").and_then(Value::as_bool).unwrap_or(false);
            Some(shaper::group_at(doc, point, source).into_iter().collect())
        }
        "shaper.scribble" => {
            let pts: Vec<Point> = params.get("points")?.as_array()?.iter().map(point_of).collect::<Option<Vec<_>>>()?;
            if pts.is_empty() || pts.len() > shaper::MAX_POINTS || !pts.iter().all(|p| shaper::finite(*p)) {
                return None;
            }
            let tolerance = params.get("tolerance").and_then(Value::as_f64).unwrap_or(4.0);
            if !tolerance.is_finite() || !(0.01..=1000.0).contains(&tolerance) {
                return None;
            }
            let step = (tolerance / 2.0).max(0.5);
            let samples = kition_vector_tools::builder::sample_polyline(&pts, step);
            shaper::candidates_in(doc, selection, params, &samples, tolerance).ok()
        }
        "object.liquify" => {
            // Its `ids`, else the selection, else every visible unlocked path under the brush.
            let given: Option<Vec<NodeId>> = match (params.get("ids"), params.get("id")) {
                (Some(Value::Array(ids)), _) => Some(ids.iter().filter_map(Value::as_u64).map(NodeId).collect()),
                (_, Some(id)) => id.as_u64().map(|id| vec![NodeId(id)]),
                _ => None,
            };
            let roots = given.unwrap_or_else(|| selection.objects.clone());
            distortcmds::liquify_reached(doc, &roots, params)
        }
        "symbol.adjust" => {
            let pts = brushsym::points(params, command).ok()?;
            let radius = params.get("radius").and_then(Value::as_f64).unwrap_or(40.0).clamp(1.0, 2000.0);
            let mut candidates = Vec::new();
            if selection.is_empty() {
                candidates = brushsym::instances_of(doc, None);
            } else {
                for id in &selection.objects {
                    if let Some(n) = doc.node(*id) {
                        n.walk(&mut |c| {
                            if matches!(c.kind, NodeKind::SymbolInstance { .. }) && !candidates.contains(&c.id) {
                                candidates.push(c.id);
                            }
                        });
                    }
                }
            }
            candidates.retain(|id| {
                doc.is_editable(*id)
                    && doc.node(*id).and_then(brushsym::instance_center).is_some_and(|c| pts.iter().any(|q| c.distance(*q) <= radius))
            });
            Some(candidates)
        }
        _ => None,
    }
}

fn point_of(v: &Value) -> Option<Point> {
    Some(Point::new(v.get(0)?.as_f64()?, v.get(1)?.as_f64()?))
}
