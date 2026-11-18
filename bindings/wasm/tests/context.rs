//! C8-land-11-vector: an agent's call context moves the editor and the host puts the person's
//! whole editing session back, without an edit (issue-c8-context).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

use kition_kit::{CallContext, KitDocument};
use kition_vector_kit::{EDITOR_STATE, VectorDocument};
use serde_json::{Value, json};

fn drawing() -> (VectorDocument, Value, Value, Value) {
    let mut doc = VectorDocument::create("Context.kvector", 320, 200).unwrap();
    let a = doc.execute("shape.rectangle", json!({"x":10,"y":10,"width":20,"height":20})).unwrap()["id"].clone();
    let layer = doc.execute("layer.new", json!({"name":"Second"})).unwrap()["id"].clone();
    let b = doc.execute("shape.ellipse", json!({"x":60,"y":10,"width":20,"height":20})).unwrap()["id"].clone();
    doc.execute("select.set", json!({"ids":[a]})).unwrap();
    doc.mark_saved();
    (doc, a, b, layer)
}

#[test]
fn a_context_that_moves_the_selection_is_no_edit_and_goes_back() {
    let (mut doc, _, b, _) = drawing();
    kition_kit::testing::check_context(&mut doc, &CallContext { on: vec![b], ..Default::default() }, EDITOR_STATE).unwrap();
}

#[test]
fn the_whole_session_comes_back_after_an_agents_steps() {
    let (mut doc, a, b, layer) = drawing();
    let saved = doc.editor_state();
    assert!(saved.is_object(), "{saved}");
    let bytes = doc.save(None).unwrap();
    doc.set_context(&CallContext { on: vec![b.clone()], ..Default::default() }).unwrap();
    assert_eq!(doc.inspect().unwrap()["selection"], json!([b]));
    // The agent changes what its later steps read: the current layer, the tool, the new-art paint.
    doc.execute("layer.setCurrent", json!({"id": doc.inspect().unwrap()["layers"][1]["id"]})).unwrap();
    doc.execute("select.none", json!({})).unwrap();
    doc.execute("paint.setFill", json!({"color":"#ff0000"})).unwrap();
    assert_ne!(doc.editor_state(), saved);
    doc.restore_editor_state(&saved).unwrap();
    assert_eq!(doc.editor_state(), saved);
    let inspected = doc.inspect().unwrap();
    assert_eq!(inspected["selection"], json!([a]));
    assert_eq!(inspected["currentLayer"], layer);
    assert_eq!(doc.save(None).unwrap(), bytes, "no navigation is saved into the file");
}

#[test]
fn a_context_the_drawing_lacks_is_refused_and_changes_nothing() {
    let (mut doc, _, _, layer) = drawing();
    let before = doc.editor_state();
    for context in [
        CallContext { on: vec![json!(987_654)], ..Default::default() },
        CallContext { on: vec![json!("text")], ..Default::default() },
        CallContext { on: vec![layer], ..Default::default() },
        CallContext { part: Some(json!({"page":1})), ..Default::default() },
        CallContext { time: Some(1.0), ..Default::default() },
    ] {
        let error = doc.set_context(&context).unwrap_err();
        assert!(error.starts_with("invalid-params"), "{error}");
        assert_eq!(doc.editor_state(), before);
    }
}
