//! Two replicas through `KitSession<VectorModel>`: concurrent edits of different things both
//! survive, a reparent and an edit of the same object both survive, a reference follows a rename,
//! and the repair after a merge clears what the other side's deletion left dangling.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use std::sync::Arc;

use common::{
    SharedBlobs, assert_no_dangling, assert_replica_clean, canonical, drawing, edit, first, move_path, named, named_id, peer, recolor, sync,
};
use kition_vector_doc::color::{Color, Paint};
use kition_vector_doc::geom::Affine;
use kition_vector_doc::{Document, Node, NodeKind};

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

fn parent_name(d: &Document, name: &str) -> Option<String> {
    d.parent_of(named_id(d, name)).and_then(|p| d.node(p)).and_then(|n| n.name.clone())
}

#[test]
fn moving_one_path_and_recolouring_another_are_both_kept() {
    let (blobs, mut a, mut b) = pair();
    let x = path_x(a.model().document(), "B");
    edit(&mut a, &blobs, |d| {
        let id = named_id(d, "B");
        move_path(d, id, 30.0);
    });
    edit(&mut b, &blobs, |d| {
        let id = named_id(d, "C");
        recolor(d, id, Color::rgb(0.0, 1.0, 0.0));
    });
    sync(&mut a, &mut b);
    assert_converged(&a, &b);
    let d = a.model().document();
    assert_eq!(path_x(d, "B"), x + 30.0);
    assert_eq!(named(d, "C").appearance.fill_paint(), Paint::solid(Color::rgb(0.0, 1.0, 0.0)));
}

#[test]
fn moving_and_recolouring_one_path_are_both_kept() {
    let (blobs, mut a, mut b) = pair();
    let x = path_x(a.model().document(), "A");
    edit(&mut a, &blobs, |d| {
        let id = named_id(d, "A");
        move_path(d, id, 12.0);
    });
    edit(&mut b, &blobs, |d| {
        let id = named_id(d, "A");
        recolor(d, id, Color::rgb(1.0, 1.0, 0.0));
    });
    sync(&mut a, &mut b);
    assert_converged(&a, &b);
    let d = a.model().document();
    assert_eq!(path_x(d, "A"), x + 12.0);
    assert_eq!(named(d, "A").appearance.fill_paint(), Paint::solid(Color::rgb(1.0, 1.0, 0.0)));
}

#[test]
fn a_reparent_and_an_edit_of_the_same_object_are_both_kept() {
    let (blobs, mut a, mut b) = pair();
    // A takes B out of the group, onto the Notes layer; B recolours it inside the group.
    edit(&mut a, &blobs, |d| {
        let id = named_id(d, "B");
        let notes = d.layers[1].id;
        d.move_node(id, Some(notes), usize::MAX).unwrap();
    });
    edit(&mut b, &blobs, |d| {
        let id = named_id(d, "B");
        recolor(d, id, Color::rgb(1.0, 0.0, 1.0));
    });
    sync(&mut a, &mut b);
    assert_converged(&a, &b);
    let d = a.model().document();
    assert_eq!(parent_name(d, "B").as_deref(), Some("Notes"));
    assert_eq!(named(d, "B").appearance.fill_paint(), Paint::solid(Color::rgb(1.0, 0.0, 1.0)));
    assert_eq!(named(d, "Group").children().map(Vec::len), Some(1), "the group keeps C");
}

#[test]
fn two_reparents_of_one_object_end_in_one_place() {
    let (blobs, mut a, mut b) = pair();
    edit(&mut a, &blobs, |d| {
        let id = named_id(d, "C");
        let notes = d.layers[1].id;
        d.move_node(id, Some(notes), usize::MAX).unwrap();
    });
    edit(&mut b, &blobs, |d| {
        let id = named_id(d, "C");
        let layer = d.layers[0].id;
        d.move_node(id, Some(layer), 0).unwrap();
    });
    sync(&mut a, &mut b);
    assert_converged(&a, &b);
    let d = a.model().document();
    let mut count = 0;
    d.walk(|n| count += usize::from(n.name.as_deref() == Some("C")));
    assert_eq!(count, 1, "C is in exactly one place");
}

#[test]
fn deleting_a_symbol_while_the_other_places_an_instance_repairs_the_instance() {
    let (blobs, mut a, mut b) = pair();
    // A deletes the symbol and its instance (symbol.delete without expanding).
    edit(&mut a, &blobs, |d| {
        let id = named_id(d, "Instance");
        d.remove(id).unwrap();
        d.symbols.retain(|s| s.name != "Star");
    });
    // B places another instance of it, meanwhile.
    edit(&mut b, &blobs, |d| {
        let layer = d.layers[0].id;
        let id = d.alloc_id();
        let mut n = Node::new(id, NodeKind::SymbolInstance { symbol: "Star".into(), xf: Affine::translate((10.0, 10.0)) });
        n.name = Some("Instance 2".into());
        d.insert(Some(layer), usize::MAX, n).unwrap();
    });
    sync(&mut a, &mut b);
    assert_converged(&a, &b);
    let d = a.model().document();
    assert!(d.symbols.is_empty(), "the symbol stays deleted");
    let mut instances = 0;
    d.walk(|n| instances += usize::from(matches!(n.kind, NodeKind::SymbolInstance { .. })));
    assert_eq!(instances, 0, "the instance of the deleted symbol was repaired away");
}

#[test]
fn a_swatch_link_follows_a_concurrent_rename() {
    let (blobs, mut a, mut b) = pair();
    edit(&mut a, &blobs, |d| {
        let swatch = d.swatches.iter_mut().find(|s| s.name == "Brand").unwrap();
        swatch.name = "Brand Red".into();
        // The engine renames the links it holds too.
        let id = named_id(d, "A");
        if let Some(n) = d.node_mut(id) {
            n.appearance.set_fill(Paint::Solid { color: Color::rgb(0.9, 0.1, 0.1), swatch: Some("Brand Red".into()), tint: 1.0 });
        }
    });
    edit(&mut b, &blobs, |d| {
        let id = named_id(d, "C");
        if let Some(n) = d.node_mut(id) {
            n.appearance.set_fill(Paint::Solid { color: Color::rgb(0.9, 0.1, 0.1), swatch: Some("Brand".into()), tint: 1.0 });
        }
    });
    sync(&mut a, &mut b);
    assert_converged(&a, &b);
    let d = a.model().document();
    let link = |name: &str| match named(d, name).appearance.fill_paint() {
        Paint::Solid { swatch, .. } => swatch,
        other => panic!("{name}: {other:?}"),
    };
    assert_eq!(link("A").as_deref(), Some("Brand Red"));
    assert_eq!(link("C").as_deref(), Some("Brand Red"), "B's new link follows A's rename");
}

#[test]
fn deleting_a_layer_while_the_other_threads_its_objects_clears_the_references() {
    let (blobs, mut a, mut b) = pair();
    edit(&mut a, &blobs, |d| {
        let notes = d.layers[1].id;
        d.remove(notes).unwrap();
        d.guides.retain(|g| g.layer != Some(notes));
    });
    edit(&mut b, &blobs, |d| {
        let note = named_id(d, "Note");
        let a_id = named_id(d, "A");
        d.saved_selections.push(kition_vector_doc::SavedSelection { name: "Note".into(), objects: vec![note, a_id] });
        let asset = d.alloc_id().0;
        d.assets.push(kition_vector_doc::ExportAsset { id: asset, name: "Asset 2".into(), nodes: vec![note] });
    });
    sync(&mut a, &mut b);
    assert_converged(&a, &b);
    let d = a.model().document();
    assert_eq!(d.layers.len(), 1);
    let saved = d.saved_selections.iter().find(|s| s.name == "Note").unwrap();
    assert_eq!(saved.objects, vec![named_id(d, "A")], "the selection lets go of the deleted object");
    assert!(d.assets.iter().all(|asset| asset.name != "Asset 2"), "the asset left with nothing goes");
}

#[test]
fn concurrent_reorders_of_one_layer_converge() {
    let (blobs, mut a, mut b) = pair();
    edit(&mut a, &blobs, |d| {
        let layer = d.layers[0].id;
        let children = d.children_mut(Some(layer)).unwrap();
        let first = children.remove(0);
        children.push(first);
    });
    edit(&mut b, &blobs, |d| {
        let layer = d.layers[0].id;
        let children = d.children_mut(Some(layer)).unwrap();
        let last = children.pop().unwrap();
        children.insert(0, Arc::clone(&last));
    });
    sync(&mut a, &mut b);
    assert_converged(&a, &b);
    assert_eq!(a.model().document().layers[0].children().map(Vec::len), Some(4), "every object is still there, once");
}

#[test]
fn undo_after_a_remote_edit_keeps_it() {
    let (blobs, mut a, mut b) = pair();
    edit(&mut a, &blobs, |d| {
        let id = named_id(d, "A");
        move_path(d, id, 50.0);
    });
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
/// always has (no schema change): a move of the live rectangle and a recolour of the live ellipse
/// made on two replicas are both kept, and both stay live.
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
