//! `write_changes` writes only what differs: moving one path is its `data` register, recolouring
//! is its `appearance`, a reparent is one tree move, a setting is one map key.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use std::collections::BTreeSet;
use std::sync::Arc;

use common::{SharedBlobs, drawing, move_path, named_id, new_replica, person, recolor};
use kdm_model::{Ids, write_changes};
use kition_kit::kdm::{ObjectId, Replica};
use kition_vector_doc::Document;
use kition_vector_doc::color::Color;
use serde_json::Value;

const NODE_FIELDS: [&str; 6] = ["name", "visible", "locked", "appearance", "props", "data"];

fn registers(replica: &Replica, object: &ObjectId) -> Vec<Value> {
    NODE_FIELDS.iter().map(|f| replica.get(object, f).unwrap_or(Value::Null)).collect()
}

/// Writes `before` -> `after` and answers the objects whose state changed.
fn write(replica: &mut Replica, ids: &mut Ids, blobs: &SharedBlobs, before: &Document, after: &Document) -> BTreeSet<ObjectId> {
    replica.take_changed();
    let mut sink = blobs.clone();
    let wrote =
        replica.transact(&person("t"), |tx| write_changes(tx, before, after, ids, &mut sink)).unwrap_or_else(|error| panic!("write: {error}"));
    assert!(wrote, "the edit wrote something");
    replica.take_changed()
}

/// Each node's object and its parent's (`None`: a top-level layer), as the replica's tree holds them.
fn parents(replica: &Replica) -> Vec<(ObjectId, Option<ObjectId>)> {
    let root = ObjectId::from_string("drawing");
    let entries = replica.tree(&root, "nodes").unwrap();
    entries
        .iter()
        .map(|e| {
            let parent = e.parent.as_ref().and_then(|p| entries.iter().find(|x| &x.node == p)).map(|x| x.object.clone());
            (e.object.clone(), parent)
        })
        .collect()
}

#[test]
fn moving_one_path_writes_only_its_data() {
    let blobs = SharedBlobs::default();
    let before = drawing();
    let (mut replica, mut ids) = new_replica(&before, &blobs);
    let id = named_id(&before, "B");
    let object = ids.node(id).cloned().unwrap();
    let untouched = registers(&replica, &object);
    let tree = parents(&replica);

    let mut after = before.clone();
    move_path(&mut after, id, 25.0);
    let changed = write(&mut replica, &mut ids, &blobs, &before, &after);
    assert_eq!(changed, BTreeSet::from([object.clone()]), "only the moved path's object changed");
    let now = registers(&replica, &object);
    for (i, field) in NODE_FIELDS.iter().enumerate() {
        if *field == "data" {
            assert_ne!(now[i], untouched[i], "its geometry changed");
        } else {
            assert_eq!(now[i], untouched[i], "its {field} is as it was");
        }
    }
    assert_eq!(parents(&replica), tree, "the tree is untouched");

    // Nothing more to write.
    let mut sink = blobs.clone();
    let wrote = replica.transact(&person("t"), |tx| write_changes(tx, &after, &after, &mut ids, &mut sink)).unwrap();
    assert!(!wrote);
}

#[test]
fn recolouring_writes_only_the_appearance() {
    let blobs = SharedBlobs::default();
    let before = drawing();
    let (mut replica, mut ids) = new_replica(&before, &blobs);
    let id = named_id(&before, "C");
    let object = ids.node(id).cloned().unwrap();
    let untouched = registers(&replica, &object);
    let mut after = before.clone();
    recolor(&mut after, id, Color::rgb(0.0, 1.0, 0.0));
    let changed = write(&mut replica, &mut ids, &blobs, &before, &after);
    assert_eq!(changed, BTreeSet::from([object.clone()]));
    let now = registers(&replica, &object);
    let differing: Vec<&str> = NODE_FIELDS.iter().zip(now.iter().zip(&untouched)).filter(|(_, (a, b))| a != b).map(|(f, _)| *f).collect();
    assert_eq!(differing, vec!["appearance"]);
}

#[test]
fn reparenting_is_one_tree_move() {
    let blobs = SharedBlobs::default();
    let before = drawing();
    let (mut replica, mut ids) = new_replica(&before, &blobs);
    let a = named_id(&before, "A");
    let a_object = ids.node(a).cloned().unwrap();
    let a_registers = registers(&replica, &a_object);
    let notes = before.layers[1].id;
    let notes_object = ids.node(notes).cloned().unwrap();
    let tree = parents(&replica);

    // A goes from Layer 1 to the top of Notes.
    let mut after = before.clone();
    let node = after.remove(a).unwrap();
    after.children_mut(Some(notes)).unwrap().push(node);
    let changed = write(&mut replica, &mut ids, &blobs, &before, &after);
    assert_eq!(changed, BTreeSet::from([ObjectId::from_string("drawing")]), "only the tree changed (it lives on the drawing)");
    assert_eq!(registers(&replica, &a_object), a_registers, "the moved object's registers are untouched");
    let now = parents(&replica);
    let moved: Vec<_> = now.iter().filter(|entry| !tree.contains(entry)).collect();
    assert_eq!(moved, vec![&(a_object.clone(), Some(notes_object.clone()))], "one node changed parent, nothing else moved");
    let siblings: Vec<&ObjectId> = now.iter().filter(|(_, p)| p.as_ref() == Some(&notes_object)).map(|(o, _)| o).collect();
    assert_eq!(siblings.last(), Some(&&a_object), "on top of Notes");
}

#[test]
fn bringing_to_front_moves_one_node() {
    let blobs = SharedBlobs::default();
    let before = drawing();
    let (mut replica, mut ids) = new_replica(&before, &blobs);
    let layer = before.layers[0].id;
    let a = named_id(&before, "A");
    let mut after = before.clone();
    let children = after.children_mut(Some(layer)).unwrap();
    let first = children.remove(0);
    children.push(first);
    let tree = parents(&replica);
    write(&mut replica, &mut ids, &blobs, &before, &after);
    let now = parents(&replica);
    // Same parents for everyone; A is now last under its layer.
    let mut a_sorted = tree.clone();
    a_sorted.sort_by(|x, y| x.0.as_str().cmp(y.0.as_str()));
    let mut n_sorted = now.clone();
    n_sorted.sort_by(|x, y| x.0.as_str().cmp(y.0.as_str()));
    assert_eq!(a_sorted, n_sorted);
    let layer_object = ids.node(layer).cloned().unwrap();
    let order: Vec<&ObjectId> = now.iter().filter(|(_, p)| p.as_ref() == Some(&layer_object)).map(|(o, _)| o).collect();
    assert_eq!(order.last(), Some(&&ids.node(a).cloned().unwrap()));
}

#[test]
fn deleting_a_group_deletes_its_objects() {
    let blobs = SharedBlobs::default();
    let before = drawing();
    let (mut replica, mut ids) = new_replica(&before, &blobs);
    let group = named_id(&before, "Group");
    let objects: Vec<ObjectId> = ["Group", "B", "C"].iter().map(|n| ids.node(named_id(&before, n)).cloned().unwrap()).collect();
    let mut after = before.clone();
    after.remove(group).unwrap();
    after.text_threads.clear();
    let live: Vec<_> = after.saved_selections[0].objects.iter().copied().filter(|id| after.node(*id).is_some()).collect();
    after.saved_selections[0].objects = live;
    write(&mut replica, &mut ids, &blobs, &before, &after);
    for object in &objects {
        assert!(!replica.exists(object), "{object} was deleted");
    }
}

#[test]
fn a_setting_is_one_map_key() {
    let blobs = SharedBlobs::default();
    let before = drawing();
    let (mut replica, mut ids) = new_replica(&before, &blobs);
    let root = ObjectId::from_string("drawing");
    let entries = replica.map_entries(&root, "settings").unwrap();
    let mut after = before.clone();
    after.grid.spacing = 36.0;
    write(&mut replica, &mut ids, &blobs, &before, &after);
    let now = replica.map_entries(&root, "settings").unwrap();
    let changed: Vec<&String> = now.iter().filter(|e| !entries.contains(e)).map(|(k, _)| k).collect();
    assert_eq!(changed, vec!["grid"]);
}

#[test]
fn editing_a_swatch_writes_that_swatch_only() {
    let blobs = SharedBlobs::default();
    let before = drawing();
    let (mut replica, mut ids) = new_replica(&before, &blobs);
    let index = before.swatches.iter().position(|s| s.name == "Brand").unwrap();
    let object = ids.swatch(index).cloned().unwrap();
    let mut after = before.clone();
    after.swatches[index].paint = kition_vector_doc::color::Paint::solid(Color::rgb(0.5, 0.5, 0.5));
    let changed = write(&mut replica, &mut ids, &blobs, &before, &after);
    assert_eq!(changed, BTreeSet::from([object]));
}

#[test]
fn an_unchanged_drawing_writes_nothing_even_through_new_arcs() {
    let blobs = SharedBlobs::default();
    let before = drawing();
    let (mut replica, mut ids) = new_replica(&before, &blobs);
    let mut after = before.clone();
    after.layers = after.layers.iter().map(|l| Arc::new((**l).clone())).collect();
    let mut sink = blobs.clone();
    let wrote = replica.transact(&person("t"), |tx| write_changes(tx, &before, &after, &mut ids, &mut sink)).unwrap();
    assert!(!wrote);
}
