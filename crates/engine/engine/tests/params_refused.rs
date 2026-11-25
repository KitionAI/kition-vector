//! C8-vector-engine-2: a parameter is used or refused, never dropped, and a call that would
//! change nothing is refused or leaves no undo step (issue-c8-engine2-params).
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

fn undo_len(s: &Session) -> usize {
    s.doc().unwrap().history.undo.len()
}

/// `command` with `params` is refused and leaves the document and its history as they were.
fn refused(s: &mut Session, command: &str, params: Value) -> String {
    let (before, undo) = (doc(s), undo_len(s));
    let error = s.execute(command, &params).map(|v| format!("accepted: {v}")).unwrap_or_else(|e| e.to_string());
    assert!(!error.starts_with("accepted"), "{command} {params}: {error}");
    assert!(doc(s) == before, "{command} {params} changed the document");
    assert_eq!(undo_len(s), undo, "{command} {params} left an undo step");
    error
}

#[test]
fn nudge_copy_with_anchors_is_refused() {
    let mut s = session();
    let r = rect(&mut s, 10.0);
    s.execute("select.anchors", &json!({"id": r.0, "anchors": [[0, 0]], "mode": "set"})).unwrap();
    assert!(!s.doc().unwrap().selection.anchors.is_empty());
    refused(&mut s, "object.nudge", json!({"dx": 1, "copy": true}));
}

#[test]
fn scribble_refuses_named_objects_it_cannot_take() {
    let mut s = session();
    let r = rect(&mut s, 100.0);
    let t = id(&s.execute("text.add", &json!({"x": 100, "y": 20, "text": "Hi"})).unwrap());
    let error = refused(&mut s, "shaper.scribble", json!({"points": [[90, 20], [130, 20]], "ids": [r.0, t.0]}));
    assert!(error.contains(&t.0.to_string()), "{error}");
}

#[test]
fn set_advanced_refuses_a_brush_the_document_lacks() {
    let mut s = session();
    rect(&mut s, 10.0);
    let error = refused(&mut s, "stroke.setAdvanced", json!({"brush": "Nope"}));
    assert!(error.contains("Nope"), "{error}");
}

#[test]
fn set_item_refuses_objects_without_the_item() {
    let mut s = session();
    let a = rect(&mut s, 10.0);
    let b = rect(&mut s, 60.0);
    s.execute("appearance.addFill", &json!({"ids": [a.0]})).unwrap();
    let items = |s: &Session, i: NodeId| doc(s).node(i).unwrap().appearance.items.len();
    let index = items(&s, a) - 1;
    assert!(index >= items(&s, b), "b has no item {index}");
    let agent = Origin::agent("agent:test");
    let before = doc(&s);
    let call = json!({"ids": [a.0, b.0], "index": index, "color": "#ff0000"});
    assert!(s.execute_as("appearance.setItem", &call, &agent).is_err(), "an agent's call names an item b lacks");
    assert!(doc(&s) == before);
    refused(&mut s, "appearance.setItem", json!({"ids": [b.0], "index": index, "color": "#ff0000"}));
    refused(&mut s, "appearance.setItem", json!({"ids": [a.0], "index": 0, "blend": "sideways"}));
}

#[test]
fn image_trace_refuses_both_id_and_ids() {
    // Two embedded images, opened from an SVG.
    const PNG: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==";
    let svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="100"><image x="0" y="0" width="40" height="40" href="data:image/png;base64,{PNG}"/><image x="100" y="0" width="40" height="40" href="data:image/png;base64,{PNG}"/></svg>"#
    );
    let mut s = Session::new();
    s.execute("file.open", &json!({"name": "two.svg", "dataBase64": kition_vector_format::base64_encode(svg.as_bytes())})).unwrap();
    let mut images = vec![];
    doc(&s).walk(|n| {
        if matches!(n.kind, kition_vector_doc::NodeKind::Image(_)) {
            images.push(n.id);
        }
    });
    assert_eq!(images.len(), 2, "two images");
    refused(&mut s, "imageTrace.make", json!({"id": images[0].0, "ids": [images[1].0]}));
}

#[test]
fn recolouring_refuses_an_id_that_names_nothing() {
    let mut s = session();
    let a = rect(&mut s, 10.0);
    refused(&mut s, "edit.colors.invert", json!({"ids": [a.0, 9999]}));
}

#[test]
fn node_move_refuses_a_locked_container() {
    let mut s = session();
    let a = rect(&mut s, 10.0);
    let b = rect(&mut s, 60.0);
    s.execute("select.set", &json!({"ids": [b.0]})).unwrap();
    let g = id(&s.execute("arrange.group", &json!({})).unwrap());
    s.execute("object.setProps", &json!({"ids": [g.0], "locked": true})).unwrap();
    refused(&mut s, "node.move", json!({"id": a.0, "parent": g.0, "index": 0}));
    let layer = doc(&s).layers[0].id;
    refused(&mut s, "node.move", json!({"id": b.0, "parent": layer.0, "index": 0}));
}

#[test]
fn nothing_to_do_is_refused_without_an_undo_step() {
    let mut s = session();
    let p = id(&s.execute("path.add", &json!({"anchors": [{"x": 0, "y": 0}, {"x": 50, "y": 50}]})).unwrap());
    s.execute("select.set", &json!({"ids": [p.0]})).unwrap();
    refused(&mut s, "object.expandShape", json!({}));
    s.execute("edit.deselectAll", &json!({})).unwrap();
    let text = id(&s.execute("text.add", &json!({"x": 10, "y": 200, "text": "Hi"})).unwrap());
    s.execute("select.set", &json!({"ids": [text.0]})).unwrap();
    refused(&mut s, "path.average", json!({}));
    s.execute("variable.define", &json!({"name": "V", "kind": "text"})).unwrap();
    s.execute("edit.deselectAll", &json!({})).unwrap();
    refused(&mut s, "variable.unbind", json!({}));
    // Unbinding what is not bound succeeds with nothing unbound, and leaves no undo step.
    let undo = undo_len(&s);
    assert_eq!(s.execute("variable.unbind", &json!({"ids": [text.0]})).unwrap()["unbound"], 0);
    assert_eq!(undo_len(&s), undo);
}

#[test]
fn swatch_colors_take_the_documents_colour_model() {
    let mut s = Session::new();
    s.execute("file.new", &json!({"width": 800, "height": 600, "colorMode": "cmyk"})).unwrap();
    s.execute("swatch.new", &json!({"colors": ["#ff0000"], "name": "Mine A"})).unwrap();
    s.execute("swatch.new", &json!({"color": "#ff0000", "name": "Mine B"})).unwrap();
    let d = doc(&s);
    let from_list = d.swatch("Mine A").unwrap().paint.clone();
    let from_color = d.swatch("Mine B").unwrap().paint.clone();
    assert!(matches!(from_color, kition_vector_color::Paint::Solid { color: kition_vector_color::Color::Cmyk { .. }, .. }), "{from_color:?}");
    assert_eq!(from_list, from_color);
}

#[test]
fn a_new_artboards_default_name_is_free() {
    let mut s = session();
    s.execute("artboard.new", &json!({"name": "Artboard 2"})).unwrap();
    // Two artboards: the next number is 3, free.
    s.execute("artboard.new", &json!({})).unwrap();
    s.execute("artboard.setProps", &json!({"index": 0, "name": "Artboard 4"})).unwrap();
    s.execute("artboard.new", &json!({})).unwrap();
    let names: Vec<String> = doc(&s).artboards.iter().map(|a| a.name.clone()).collect();
    let mut unique = names.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), names.len(), "{names:?}");
    // Convert to Artboards names by number too.
    let r = rect(&mut s, 10.0);
    s.execute("select.set", &json!({"ids": [r.0]})).unwrap();
    s.execute("artboard.convertToArtboards", &json!({})).unwrap();
    let names: Vec<String> = doc(&s).artboards.iter().map(|a| a.name.clone()).collect();
    let mut unique = names.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), names.len(), "{names:?}");
}
