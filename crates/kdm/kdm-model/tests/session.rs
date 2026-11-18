//! `VectorSession`: the engine bridge's replica: `Send`, record/undo/redo and the exchange with a
//! peer, image blobs shipped through `blobs_since`, and garbage-collected once no reachable undo
//! step needs them.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use std::sync::Arc;

use common::{canonical, drawing, named_id, person};
use kdm_model::VectorSession;
use kition_kit::kdm::{UndoScope, Version};
use kition_kit::session::MemoryBlobs;
use kition_vector_doc::geom::Affine;
use kition_vector_doc::{Document, ImageBlob, ImageObject, Node, NodeKind};

fn send(from: &VectorSession, to: &mut VectorSession) {
    let since = to.version();
    to.add_blobs(&MemoryBlobs(from.blobs_since(&since).into_iter().collect()));
    to.import_changes(&from.export_changes(&since).unwrap()).unwrap();
}

fn edited(s: &VectorSession, f: impl FnOnce(&mut Document)) -> Arc<Document> {
    let mut d = (**s.document()).clone();
    f(&mut d);
    Arc::new(d)
}

fn with_image(d: &mut Document, key: &str, bytes: Vec<u8>) {
    d.images.insert(key.into(), ImageBlob::new("image/png", bytes));
    let layer = d.layers[0].id;
    let id = d.alloc_id();
    let mut n = Node::new(
        id,
        NodeKind::Image(ImageObject { key: key.into(), width: 1, height: 1, xf: Affine::IDENTITY, link: None, placement: Default::default() }),
    );
    n.name = Some(key.into());
    d.insert(Some(layer), usize::MAX, n).unwrap();
}

#[test]
fn a_session_is_send() {
    fn send_able<T: Send>() {}
    send_able::<VectorSession>();
}

#[test]
fn record_undo_redo_and_a_peer_agree() {
    let mut a = VectorSession::create(&Arc::new(drawing())).unwrap();
    let snapshot = a.snapshot().unwrap();
    let mut b = VectorSession::open_snapshot(&snapshot, MemoryBlobs(a.blobs_since(&Version::empty()).into_iter().collect())).unwrap();
    assert_eq!(canonical(a.document()), canonical(b.document()));

    let after = edited(&a, |d| d.title = "Renamed".into());
    assert_eq!(a.record(&person("a"), &after).unwrap(), 1, "one undo step");
    assert_eq!(a.record(&person("a"), &after).unwrap(), 0, "nothing changed, no step");
    send(&a, &mut b);
    assert_eq!(b.document().title, "Renamed");
    assert_eq!(a.state_hash().unwrap(), b.state_hash().unwrap());

    assert!(a.undo_in(UndoScope::Person).unwrap().0);
    assert_ne!(a.document().title, "Renamed");
    assert!(a.redo_in(UndoScope::Person).unwrap().0);
    assert_eq!(a.document().title, "Renamed");
    assert!(!a.redo_in(UndoScope::Person).unwrap().0, "nothing more to redo");
}

#[test]
fn a_view_reload_keeps_unchanged_nodes() {
    let mut a = VectorSession::create(&Arc::new(drawing())).unwrap();
    let mut b = VectorSession::open_snapshot(&a.snapshot().unwrap(), MemoryBlobs(a.blobs_since(&Version::empty()).into_iter().collect())).unwrap();
    let note_before = Arc::clone(&b.document().layers[1]);
    let after = edited(&a, |d| {
        let id = named_id(d, "A");
        common::move_path(d, id, 5.0);
    });
    a.record(&person("a"), &after).unwrap();
    send(&a, &mut b);
    assert!(Arc::ptr_eq(&note_before, &b.document().layers[1]), "the untouched layer is the view's own node");
    assert_eq!(canonical(a.document()), canonical(b.document()));
}

#[test]
fn a_new_image_ships_its_blob_and_only_that() {
    let mut a = VectorSession::create(&Arc::new(drawing())).unwrap();
    let mut b = VectorSession::open_snapshot(&a.snapshot().unwrap(), MemoryBlobs(a.blobs_since(&Version::empty()).into_iter().collect())).unwrap();
    let since = b.version();
    let after = edited(&a, |d| with_image(d, "img-new", vec![9, 9, 9, 9]));
    a.record(&person("a"), &after).unwrap();
    let shipped = a.blobs_since(&since);
    assert_eq!(shipped.len(), 1, "only the new image's bytes");
    assert_eq!(&*shipped[0].1, &[9, 9, 9, 9][..]);
    send(&a, &mut b);
    assert_eq!(b.document().images.get("img-new").map(|i| i.bytes.to_vec()), Some(vec![9, 9, 9, 9]));
}

#[test]
fn an_image_deleted_past_the_history_is_garbage() {
    let mut a = VectorSession::create(&Arc::new(drawing())).unwrap();
    let after = edited(&a, |d| with_image(d, "img-tmp", vec![7, 7, 7]));
    a.record(&person("a"), &after).unwrap();
    let count = a.blobs().0.len();
    let after = edited(&a, |d| {
        let id = named_id(d, "img-tmp");
        d.remove(id).unwrap();
        d.images.remove("img-tmp");
    });
    a.record(&person("a"), &after).unwrap();
    assert_eq!(a.blobs().0.len(), count, "an undo step can still bring the image back");
    a.retain_steps(UndoScope::Person, 0, 0);
    assert_eq!(a.blobs().0.len(), count - 1, "with the history trimmed, its bytes go");
}

#[test]
fn holds_compares_content_id_blind() {
    let d = drawing();
    let a = VectorSession::create(&Arc::new(d.clone())).unwrap();
    assert!(a.holds(&d).unwrap());
    let mut other = d.clone();
    other.title = "Else".into();
    assert!(!a.holds(&other).unwrap());
}

#[test]
fn changes_whose_blobs_arrive_later_import_and_show_the_image_once_they_do() {
    let mut a = VectorSession::create(&Arc::new(drawing())).unwrap();
    let mut b = VectorSession::open_snapshot(&a.snapshot().unwrap(), MemoryBlobs(a.blobs_since(&Version::empty()).into_iter().collect())).unwrap();
    let since = b.version();
    let after = edited(&a, |d| with_image(d, "late", vec![7, 7, 7]));
    a.record(&person("a"), &after).unwrap();
    // The changes first, without their blobs: the import holds, with an empty image for now.
    b.import_changes(&a.export_changes(&since).unwrap()).unwrap();
    assert_eq!(a.state_hash().unwrap(), b.state_hash().unwrap());
    assert!(b.document().images["late"].bytes.is_empty());
    // The bytes arrive: the image shows them.
    assert!(b.add_blobs(&MemoryBlobs(a.blobs_since(&since).into_iter().collect())));
    assert_eq!(*b.document().images["late"].bytes, vec![7, 7, 7]);
    assert_eq!(canonical(a.document()), canonical(b.document()));
}

#[test]
fn blobs_handed_over_ahead_wait_for_changes_imported_several_imports_later() {
    let mut a = VectorSession::create(&Arc::new(drawing())).unwrap();
    let mut b = VectorSession::open_snapshot(&a.snapshot().unwrap(), MemoryBlobs(a.blobs_since(&Version::empty()).into_iter().collect())).unwrap();
    let start = b.version();
    let mut steps = Vec::new();
    for (index, bytes) in [vec![1, 2], vec![3, 4]].into_iter().enumerate() {
        let before = a.version();
        let after = edited(&a, |d| with_image(d, &format!("image{index}"), bytes));
        a.record(&person("a"), &after).unwrap();
        steps.push(a.export_changes(&before).unwrap());
    }
    let after = edited(&a, |d| d.title = "Renamed".into());
    let before = a.version();
    a.record(&person("a"), &after).unwrap();
    steps.push(a.export_changes(&before).unwrap());
    // Every blob first, then the changes last to first: the later ones wait for the earlier.
    b.add_blobs(&MemoryBlobs(a.blobs_since(&start).into_iter().collect()));
    for bytes in steps.iter().rev() {
        b.import_changes(bytes).unwrap();
    }
    assert_eq!(*b.document().images["image0"].bytes, vec![1, 2]);
    assert_eq!(*b.document().images["image1"].bytes, vec![3, 4]);
    assert_eq!(canonical(a.document()), canonical(b.document()));
}

/// Kition platform KitModel 1.1 M12a at the session: the person's undo takes back only the
/// person's step, an agent's only the agent's.
#[test]
fn undo_scopes_keep_the_other_scopes_steps() {
    let mut a = VectorSession::create(&Arc::new(drawing())).unwrap();
    let title = a.document().title.clone();
    let after = edited(&a, |d| d.title = "Person".into());
    assert_eq!(a.record(&person("a"), &after).unwrap(), 1);
    let id = named_id(a.document(), "A");
    let after = edited(&a, |d| common::move_path(d, id, 3.0));
    assert_eq!(a.record(&kition_kit::kdm::Origin::agent("agent:helper"), &after).unwrap(), 1);
    let mut want = (*after).clone();
    want.title = title;
    assert!(a.undo_in(UndoScope::Person).unwrap().0);
    assert_eq!(canonical(a.document()), canonical(&want), "the person's step is undone and the agent's move stays");
    assert!(a.undo_in(UndoScope::Agent).unwrap().0, "the agent's undo takes its own step");
    assert_eq!(canonical(a.document()), canonical(&drawing()));
}
