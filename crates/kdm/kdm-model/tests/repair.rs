//! The repair after a merge, an undo or a redo (`validate`): the states it clears can no longer
//! come from two people editing at once, but still come from one person's draft merged after
//! the person's own edits (data model 6.6, rules D6 to D8: a reference held in a value is no
//! collision), and from an undo after a file changed outside was read in. A symbol instance of a
//! deleted symbol goes; saved selections and export assets let go of the objects of a deleted
//! layer (an asset left with none goes).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use common::{SharedBlobs, assert_no_dangling, assert_replica_clean, draft, drawing, edit, edit_as, first, merge, named_id};
use kition_kit::kdm::{Origin, OriginKind};
use kition_vector_doc::color::{Color, Paint};
use kition_vector_doc::geom::{Affine, Rect, shapes};
use kition_vector_doc::{Appearance, Document, ExportAsset, Node, NodeKind, SavedSelection, Symbol};

/// The names of the symbol instances in `d`, depth first.
fn instances(d: &Document) -> Vec<String> {
    let mut out = Vec::new();
    d.walk(|n| {
        if matches!(n.kind, NodeKind::SymbolInstance { .. }) {
            out.push(n.name.clone().unwrap_or_default());
        }
    });
    out
}

/// Places an instance of `symbol` named `name` on the first layer.
fn place(d: &mut Document, symbol: &str, name: &str) {
    let layer = d.layers[0].id;
    let id = d.alloc_id();
    let mut n = Node::new(id, NodeKind::SymbolInstance { symbol: symbol.into(), xf: Affine::translate((10.0, 10.0)) });
    n.name = Some(name.into());
    d.insert(Some(layer), usize::MAX, n).unwrap();
}

fn assert_clean(session: &common::Session) {
    assert_no_dangling(session.model().document());
    assert_replica_clean(session.replica());
}

/// The person deletes the symbol "Star" (and its instance, as `symbol.delete` without expanding
/// does) while a draft places another instance of it: the draft merges, and the repair deletes the
/// instance of the deleted symbol.
#[test]
fn a_draft_placing_an_instance_of_a_symbol_the_person_deleted_meanwhile_is_repaired() {
    let blobs = SharedBlobs::default();
    let mut a = first(&drawing(), &blobs);
    let (mut b, base) = draft(&a, 2, &blobs);
    edit(&mut b, &blobs, |d| place(d, "Star", "Instance 2"));
    edit(&mut a, &blobs, |d| {
        let id = named_id(d, "Instance");
        d.remove(id).unwrap();
        d.symbols.retain(|s| s.name != "Star");
    });
    merge(&b, &base, &mut a).unwrap();
    let d = a.model().document();
    assert!(d.symbols.is_empty(), "the symbol stays deleted");
    assert_eq!(instances(d), Vec::<String>::new(), "the instance of the deleted symbol was repaired away");
    assert_clean(&a);
}

/// The person deletes the layer "Notes" while a draft makes a saved selection and an export asset
/// of its object "Note": the draft merges, the selection lets go of the deleted object, and the
/// asset left with nothing goes.
#[test]
fn a_draft_naming_objects_of_a_layer_the_person_deleted_meanwhile_is_repaired() {
    let blobs = SharedBlobs::default();
    let mut a = first(&drawing(), &blobs);
    let (mut b, base) = draft(&a, 2, &blobs);
    edit(&mut b, &blobs, |d| {
        let note = named_id(d, "Note");
        let a_id = named_id(d, "A");
        d.saved_selections.push(SavedSelection { name: "Note".into(), objects: vec![note, a_id] });
        let asset = d.alloc_id().0;
        d.assets.push(ExportAsset { id: asset, name: "Asset 2".into(), nodes: vec![note] });
    });
    edit(&mut a, &blobs, |d| {
        let notes = d.layers[1].id;
        d.remove(notes).unwrap();
        d.guides.retain(|g| g.layer != Some(notes));
    });
    merge(&b, &base, &mut a).unwrap();
    let d = a.model().document();
    assert_eq!(d.layers.len(), 1);
    let saved: Vec<(String, Vec<_>)> = d.saved_selections.iter().map(|s| (s.name.clone(), s.objects.clone())).collect();
    assert_eq!(
        saved,
        vec![("Both".to_owned(), vec![named_id(d, "A"), named_id(d, "B")]), ("Note".to_owned(), vec![named_id(d, "A")])],
        "the selection lets go of the deleted object"
    );
    let assets: Vec<&str> = d.assets.iter().map(|asset| asset.name.as_str()).collect();
    assert_eq!(assets, vec!["Asset 1"], "the asset left with nothing goes");
    assert_clean(&a);
}

/// The person makes the symbol "Moon" with an instance (one undo step); the file, changed outside,
/// is read in with a second instance of it (one `import` change, no undo step); the person's undo
/// deletes the symbol, and the repair deletes the instance the file placed.
#[test]
fn undoing_a_symbol_an_outside_change_placed_an_instance_of_repairs_the_instance() {
    let blobs = SharedBlobs::default();
    let mut a = first(&drawing(), &blobs);
    edit(&mut a, &blobs, |d| {
        let art_id = d.alloc_id();
        let art =
            Node::path(art_id, shapes::ellipse(Rect::new(0.0, 0.0, 20.0, 20.0)), Appearance::basic(Paint::solid(Color::BLACK), Paint::None, 0.0));
        d.symbols.push(Symbol { uid: Default::default(), name: "Moon".into(), art: std::sync::Arc::new(art) });
        place(d, "Moon", "Moon 1");
    });
    edit_as(&mut a, &blobs, &Origin::new(OriginKind::Import, "file reread"), |d| place(d, "Moon", "Moon 2"));
    assert_eq!(instances(a.model().document()), vec!["Instance", "Moon 1", "Moon 2"]);
    assert!(a.undo().unwrap().0);
    let d = a.model().document();
    let symbols: Vec<&str> = d.symbols.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(symbols, vec!["Star"], "the undo deletes the symbol");
    assert_eq!(instances(d), vec!["Instance"], "the instance the outside change placed was repaired away");
    assert_clean(&a);
}
