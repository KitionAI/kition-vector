//! C8-vector-engine: a parameter given is used or refused, never quietly replaced by a default
//! that makes the command act on something other than what the call named
//! (issue-c8-engine-param-defaults).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

use kition_vector_doc::{Document, NodeId};
use kition_vector_engine::{Origin, Session};
use serde_json::{Value, json};

fn session() -> Session {
    let mut s = Session::new();
    s.execute("file.new", &json!({"width": 800, "height": 600})).unwrap();
    s
}

fn id(v: &Value) -> NodeId {
    NodeId(v["id"].as_u64().unwrap())
}

fn rect(s: &mut Session, x: f64) -> NodeId {
    id(&s.execute("shape.rectangle", &json!({"x": x, "y": 10, "width": 20, "height": 20})).unwrap())
}

fn doc(s: &Session) -> std::sync::Arc<Document> {
    s.doc().unwrap().doc.clone()
}

/// `command` with `params` is refused and leaves the document as it was.
fn refused(s: &mut Session, command: &str, params: Value) -> String {
    let before = doc(s);
    let error = s.execute(command, &params).map(|v| format!("accepted: {v}")).unwrap_or_else(|e| e.to_string());
    assert!(!error.starts_with("accepted"), "{command} {params}: {error}");
    assert!(doc(s) == before, "{command} {params} changed the document");
    error
}

#[test]
fn node_move_needs_its_index() {
    let mut s = session();
    let a = rect(&mut s, 10.0);
    rect(&mut s, 50.0);
    let layer = s.doc().unwrap().doc.layers[0].id;
    let error = refused(&mut s, "node.move", json!({"id": a.0, "parent": layer.0}));
    assert!(error.contains("index"), "{error}");
}

#[test]
fn an_agents_brush_that_names_nothing_is_refused() {
    let mut s = session();
    let stroke = json!({"points": [[0, 0], [40, 40], [80, 0]], "brush": "Nope"});
    let before = doc(&s);
    let error = s.execute_as("brush.freehand", &stroke, &Origin::agent("agent:test")).unwrap_err();
    assert!(error.to_string().contains("Nope"), "{error}");
    assert!(doc(&s) == before, "an agent's stroke with no such brush draws nothing");
    // The Paintbrush tool's stroke, with a current brush since deleted, still draws.
    s.execute_as("brush.freehand", &stroke, &Origin::person("user:test")).unwrap();
}

#[test]
fn smart_punctuation_of_the_selection_needs_one() {
    let mut s = session();
    s.execute("text.add", &json!({"x": 10, "y": 40, "text": "a -- b..."})).unwrap();
    s.execute("edit.deselectAll", &json!({})).unwrap();
    refused(&mut s, "text.smartPunctuation", json!({"scope": "selection"}));
    s.execute("text.smartPunctuation", &json!({})).unwrap();
}

#[test]
fn release_from_perspective_refuses_an_object_that_is_not_there() {
    let mut s = session();
    refused(&mut s, "perspective.release", json!({"ids": [9999]}));
}
