//! Two replicas of one drawing through `KitSession<VectorModel>`, taking each other's changes in
//! order (one line of changes): an undo after the other's edit keeps that edit, and live shapes
//! stay live on the other replica.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use common::{
    SharedBlobs, assert_no_dangling, assert_replica_clean, canonical, drawing, edit, first, move_path, named, named_id, peer, recolor, sync,
};
use kition_vector_doc::color::{Color, Paint};
use kition_vector_doc::geom::Affine;
use kition_vector_doc::{Document, NodeKind};

fn pair() -> (SharedBlobs, common::Session, common::Session) {
    let blobs = SharedBlobs::default();
    let a = first(&drawing(), &blobs);
    let b = peer(&a, 2, &blobs);
    (blobs, a, b)
}

fn assert_converged(a: &common::Session, b: &common::Session) {
    assert_eq!(a.state_hash().ok(), b.state_hash().ok(), "same replica state");
    let (x, y) = (canonical(a.model().document()), canonical(b.model().document()));
    assert!(x == y, "same view:\n{:#?}", common::differences(&x, &y));
    assert_no_dangling(a.model().document());
    assert_replica_clean(a.replica());
}

fn path_x(d: &Document, name: &str) -> f64 {
    match &named(d, name).kind {
        NodeKind::Path { path, .. } => path.subpaths.first().and_then(|s| s.anchors.first()).map(|a| a.p.x).unwrap_or(f64::NAN),
        other => panic!("{name} is not a path: {other:?}"),
    }
}

#[test]
fn undo_after_a_remote_edit_keeps_it() {
    let (blobs, mut a, mut b) = pair();
    edit(&mut a, &blobs, |d| {
        let id = named_id(d, "A");
        move_path(d, id, 50.0);
    });
    sync(&mut a, &mut b);
    edit(&mut b, &blobs, |d| {
        let id = named_id(d, "C");
        recolor(d, id, Color::rgb(0.1, 0.2, 0.3));
    });
    sync(&mut a, &mut b);
    let x = path_x(a.model().document(), "A");
    assert!(a.undo().unwrap().0);
    let d = a.model().document();
    assert_eq!(path_x(d, "A"), x - 50.0, "A's own move is undone");
    assert_eq!(named(d, "C").appearance.fill_paint(), Paint::solid(Color::rgb(0.1, 0.2, 0.3)), "B's recolour stays");
    sync(&mut a, &mut b);
    assert_converged(&a, &b);
}

/// SVG basic shapes come in as live shapes, which the replica carries in each node's `data` as it
/// always has (no schema change): a move of the live rectangle on one replica and then a recolour
/// of the live ellipse on the other are both kept, and both stay live.
#[test]
fn live_shapes_from_svg_replicate() {
    use kition_vector_doc::LiveShape;
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="120">
<rect id="R" x="10" y="20" width="50" height="30" rx="5" fill="#e8573f"/>
<circle id="E" cx="150" cy="40" r="25" fill="#33aa55"/>
<line id="L" x1="100" y1="80" x2="190" y2="110" stroke="#000"/></svg>"##;
    let d = kdm_model::decode("shapes.svg", svg.as_bytes()).unwrap();
    let live = |d: &Document, name: &str| match &named(d, name).kind {
        NodeKind::Path { live: Some(live), path, .. } => {
            assert_eq!(path, &live.to_path(), "{name}: the path is the shape's");
            live.clone()
        }
        other => panic!("{name} is not a live shape: {other:?}"),
    };
    assert!(matches!(live(&d, "R"), LiveShape::Rectangle { radii: [5.0, 5.0, 5.0, 5.0], .. }));
    assert!(matches!(live(&d, "E"), LiveShape::Ellipse { .. }));
    assert!(matches!(live(&d, "L"), LiveShape::Line { .. }));
    let blobs = SharedBlobs::default();
    let mut a = first(&d, &blobs);
    let mut b = peer(&a, 2, &blobs);
    edit(&mut a, &blobs, |d| {
        let id = named_id(d, "R");
        if let Some(n) = d.node_mut(id) {
            n.transform(Affine::translate((30.0, 0.0)), false);
        }
    });
    sync(&mut a, &mut b);
    edit(&mut b, &blobs, |d| {
        let id = named_id(d, "E");
        recolor(d, id, Color::rgb(0.0, 0.0, 1.0));
    });
    sync(&mut a, &mut b);
    assert_converged(&a, &b);
    let d = a.model().document();
    let LiveShape::Rectangle { xf, .. } = live(d, "R") else { panic!("R is a rectangle") };
    assert_eq!(xf.as_coeffs()[4], 40.0);
    assert!(matches!(live(d, "E"), LiveShape::Ellipse { .. }));
    assert_eq!(named(d, "E").appearance.fill_paint(), Paint::solid(Color::rgb(0.0, 0.0, 1.0)));
}
