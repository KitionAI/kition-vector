//! A clean session reads its file again, saved meanwhile by another program, without losing its
//! undo histories (kition-sdk `KitDocument::reconcile_reread`): only the file's difference is
//! written, as an import that is no undo step, and an editor state naming what the file no longer
//! has is refused by name.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use kition_kit::KitDocument;
use kition_kit::kdm::Origin;
use kition_vector_kit::VectorDocument;
use serde_json::{Value, json};

const FILE: &str = "Reread.kvector";

/// A drawing with two named rectangles, and their ids.
fn drawing() -> (Vec<u8>, u64, u64) {
    let mut doc = VectorDocument::create(FILE, 400, 300).unwrap();
    let mut ids = Vec::new();
    for (n, name) in ["Mine", "Theirs"].iter().enumerate() {
        let id = doc.execute("shape.rectangle", json!({"x": 20.0 + 40.0 * n as f64, "y": 20, "width": 20, "height": 20})).unwrap()["id"].clone();
        doc.execute("object.setProps", json!({"ids": [id], "name": name})).unwrap();
        ids.push(id.as_u64().unwrap());
    }
    doc.execute("select.none", json!({})).unwrap();
    (doc.save(None).unwrap(), ids[0], ids[1])
}

fn name(doc: &mut VectorDocument, id: u64) -> Value {
    doc.execute("document.node", json!({"id": id})).unwrap()["name"].clone()
}

/// Another program's session: opens `bytes`, runs `steps`, answers what it saves.
fn elsewhere(bytes: &[u8], steps: &[(&str, Value)]) -> Vec<u8> {
    let mut doc = VectorDocument::open(FILE, bytes).unwrap();
    for (command, params) in steps {
        doc.execute(command, params.clone()).unwrap();
    }
    doc.save(None).unwrap()
}

#[test]
fn a_reread_keeps_the_agents_undo_and_the_files_edit() {
    let (bytes, mine, theirs) = drawing();
    let agent = Origin::agent("agent-me");
    let mut live = VectorDocument::open(FILE, &bytes).unwrap();
    live.execute_as("object.setProps", json!({"ids": [mine], "name": "Mine edited"}), &agent).unwrap();
    let saved = live.save(None).unwrap();
    live.mark_saved();

    let mut fresh =
        VectorDocument::open(FILE, &elsewhere(&saved, &[("object.setProps", json!({"ids": [theirs], "name": "Theirs edited"}))])).unwrap();
    fresh.restore_editor_state(&live.editor_state()).unwrap();
    assert_eq!(live.reconcile_reread(&fresh), Ok(true));
    assert!(!live.dirty(), "the session holds what the file holds");
    assert_eq!((name(&mut live, mine), name(&mut live, theirs)), (json!("Mine edited"), json!("Theirs edited")));

    live.execute_as("edit.undo", json!({}), &agent).unwrap();
    assert_eq!(name(&mut live, mine), "Mine", "the agent's own step is still undoable");
    assert_eq!(name(&mut live, theirs), "Theirs edited", "undo keeps the file's edit");
}

#[test]
fn an_unchanged_reread_writes_nothing_and_keeps_the_history() {
    let (bytes, mine, _) = drawing();
    let mut live = VectorDocument::open(FILE, &bytes).unwrap();
    live.execute("object.setProps", json!({"ids": [mine], "name": "Mine edited"})).unwrap();
    let saved = live.save(None).unwrap();
    live.mark_saved();
    let version = live.version().unwrap();

    let fresh = VectorDocument::open(FILE, &saved).unwrap();
    assert_eq!(live.reconcile_reread(&fresh), Ok(true));
    assert_eq!(live.version().unwrap(), version, "a value the file did not change is not written again");
    assert!(!live.dirty());
    live.execute("edit.undo", json!({})).unwrap();
    assert_eq!(name(&mut live, mine), "Mine");
}

#[test]
fn a_reread_refuses_a_selection_the_file_deleted() {
    let (bytes, mine, theirs) = drawing();
    let mut live = VectorDocument::open(FILE, &bytes).unwrap();
    live.execute("select.set", json!({"ids": [mine]})).unwrap();
    let state = live.editor_state();

    let mut fresh = VectorDocument::open(FILE, &elsewhere(&bytes, &[("edit.clear", json!({"ids": [mine]}))])).unwrap();
    let refused = fresh.restore_editor_state(&state).unwrap_err();
    assert!(refused.contains(&format!("deleted active selection {mine}")), "{refused}");

    // A file that keeps the selected object restores it.
    let mut fresh = VectorDocument::open(FILE, &elsewhere(&bytes, &[("edit.clear", json!({"ids": [theirs]}))])).unwrap();
    fresh.restore_editor_state(&state).unwrap();
    assert_eq!(fresh.editor_state()["selection"]["objects"], json!([mine]));
}
