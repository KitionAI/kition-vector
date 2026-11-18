//! What Kition Vector shows of the selection, as engine overlays: path outlines, anchors and the
//! selected anchors' handles, the Selection tool's bounding box with its handles, and the hover
//! highlight, with no UI toolkit, so a host that draws engine overlays shows the selection as
//! Kition Vector does.

use kition_vector_doc::{Node, NodeKind};
use kition_vector_engine::Session;
use kition_vector_geom::{BezPath, Point};
use kition_vector_tools::Overlay;

/// The most anchor points outlined; above it the selection's bounds stand for its outlines (a
/// traced photo selects hundreds of thousands of paths).
const MAX_ANCHORS: usize = 100_000;

fn too_many_anchors<'a>(nodes: impl IntoIterator<Item = &'a Node>) -> bool {
    fn over(n: &Node, left: &mut usize) -> bool {
        if let NodeKind::Path { path, .. } = &n.kind {
            match left.checked_sub(path.anchor_count()) {
                Some(rest) => *left = rest,
                None => return true,
            }
        }
        !matches!(n.kind, NodeKind::Envelope { .. })
            && n.children().into_iter().flatten().skip(usize::from(n.shaper.is_some())).any(|c| over(c, left))
    }
    let mut left = MAX_ANCHORS;
    nodes.into_iter().any(|n| over(n, &mut left))
}

/// `f` over what a selection highlight shows: an envelope's content is left out.
fn walk_drawn<'a>(n: &'a Node, f: &mut impl FnMut(&'a Node)) {
    f(n);
    if matches!(n.kind, NodeKind::Envelope { .. }) {
        return;
    }
    for c in n.children().into_iter().flatten().skip(usize::from(n.shaper.is_some())) {
        walk_drawn(c, f);
    }
}

fn rectangle(r: kition_vector_geom::Rect) -> BezPath {
    kition_vector_geom::shapes::rectangle(r).to_bezpath()
}

fn outline(n: &Node) -> BezPath {
    let mut bp = BezPath::new();
    walk_drawn(n, &mut |c| match &c.kind {
        NodeKind::Path { path, .. } => bp.extend(path.to_bezpath()),
        NodeKind::Text(t) if c.perspective.is_none() && !matches!(t.kind, kition_vector_doc::TextKind::Point) => {
            if let Some(path) = t.area_frame().or_else(|| t.type_path()) {
                bp.extend(path.to_bezpath());
            }
        }
        NodeKind::Text(_) | NodeKind::Image(_) | NodeKind::SymbolInstance { .. } => {
            if let Some(b) = c.geometric_bounds() {
                bp.extend(rectangle(b));
            }
        }
        NodeKind::PlacedDocument(p) => {
            let [a, b, c, d] = p.corners();
            bp.move_to(a);
            [b, c, d].into_iter().for_each(|q| bp.line_to(q));
            bp.close_path();
        }
        NodeKind::Envelope { .. } => {
            if let Some((lines, _)) = kition_vector_doc::live::envelope_overlay(c) {
                bp.extend(lines.to_bezpath());
            }
        }
        _ => {}
    });
    bp
}

fn path(path: BezPath, color: [u8; 3], width: f32) -> Overlay {
    Overlay::Path { path, color, width, dashed: false }
}

/// The selection's overlays.
pub fn overlays(session: &Session) -> Vec<Overlay> {
    let mut out = Vec::new();
    let Some(st) = session.active() else { return out };
    if st.selection.is_empty() {
        return out;
    }
    let tool = session.tool_id();
    let direct = kition_vector_tools::catalog::edits_anchors(tool);
    let anchor = |direct: bool| if direct { 5.0 } else { 4.0 };
    let nodes = || st.selection.objects.iter().filter_map(|id| st.doc.node(*id));
    let big = too_many_anchors(nodes());
    if big && let (Some(bounds), Some(first)) = (st.doc.bounds_of(&st.selection.objects, true), st.selection.objects.first()) {
        out.push(path(rectangle(bounds), st.doc.layer_color(*first), 1.0));
    }
    let (mut handles, mut with_handles) = (Vec::new(), 0);
    for id in st.selection.objects.iter().filter(|_| !big) {
        let Some(n) = st.doc.node(*id) else { continue };
        let color = st.doc.layer_color(*id);
        let partial = st.selection.partial(*id);
        out.push(path(outline(n), color, if st.selection.key == Some(*id) { 2.5 } else { 1.0 }));
        walk_drawn(n, &mut |c| {
            let NodeKind::Path { path, .. } = &c.kind else { return };
            for (si, ai, a) in path.anchors() {
                let selected = match partial {
                    Some(set) => set.contains(&(si, ai)),
                    None => !direct || c.id == *id,
                };
                if selected && (direct || partial.is_some()) {
                    with_handles += 1;
                    handles.extend([a.h_in, a.h_out].into_iter().filter(|h| h.distance(a.p) > 1e-6).map(|h| (a.p, h, color)));
                }
                out.push(Overlay::Anchor { p: a.p, color, filled: selected, size: anchor(partial.is_some() || direct) });
            }
        });
        if n.shows_center()
            && let Some(b) = n.geometric_bounds()
        {
            out.push(Overlay::Anchor { p: b.center(), color, filled: true, size: anchor(false) });
        }
    }
    if session.prefs.show_handles_multiple_anchors || with_handles <= 1 {
        for (a, h, color) in handles {
            out.push(Overlay::Line { a, b: h, color, dashed: false });
            out.push(Overlay::Handle { p: h, color });
        }
    }
    // The Selection tool's bounding box with its eight handles; hidden while a drag transforms.
    let show_box = tool == "selection" && !session.tool_transforming() && st.selection.anchors.is_empty();
    if show_box
        && let Some(b) = session.transform_box(&st.selection.objects)
        && let Some(first) = st.selection.objects.first()
    {
        let color = st.doc.layer_color(*first);
        let corners = b.corners();
        let mut frame = BezPath::new();
        frame.move_to(corners[0]);
        corners[1..].iter().for_each(|q| frame.line_to(*q));
        frame.close_path();
        out.push(path(frame, color, 1.0));
        for h in kition_vector_tools::bbox::Handle::ALL {
            out.push(Overlay::Anchor { p: b.to_doc() * h.pos(b.rect), color, filled: false, size: 6.0 });
        }
    }
    out
}

/// The outline of the object a selection tool would pick at `at`, unless it is selected.
pub fn hover(session: &Session, at: Point, zoom: f64) -> Option<Overlay> {
    let tool = session.tool_id();
    if session.tool_busy() || !matches!(tool, "selection" | "directSelection" | "groupSelection") {
        return None;
    }
    let st = session.active()?;
    let prefs = &session.prefs;
    let options = kition_vector_doc::hit::HitOptions {
        tol: prefs.selection_tolerance / zoom.max(1e-6),
        outline: false,
        path_only: prefs.object_selection_by_path_only,
        type_path_only: prefs.type_selection_by_path_only,
        scope: st.isolation,
    };
    let hit = kition_vector_doc::hit::hit_test(&st.doc, at, options)?;
    let id = if tool == "selection" { hit.top_object(st.isolation) } else { hit.leaf };
    if st.selection.contains(id) {
        return None;
    }
    let n = st.doc.node(id).filter(|n| !too_many_anchors([*n]))?;
    Some(path(outline(n), st.doc.layer_color(id), 1.5))
}
