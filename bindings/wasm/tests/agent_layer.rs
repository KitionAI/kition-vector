//! C8-vector-engine: an agent's new art or paste never goes to a layer other than the current
//! one when that one is locked or hidden: it is refused, naming the layer. A person's editor keeps
//! putting it on the topmost layer that takes art, as before (issue-c8-engine-agent-layer).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

use kition_kit::KitDocument;
use kition_vector_engine::Origin;
use kition_vector_kit::VectorDocument;
use serde_json::{Value, json};

/// Two layers, the first current and `prop` (locked or hidden); a rectangle on the clipboard.
fn drawing(prop: &str) -> (VectorDocument, Value, Value) {
    let mut doc = VectorDocument::create("Layers.kvector", 320, 200).unwrap();
    let first = doc.inspect().unwrap()["layers"][0]["id"].clone();
    let rect = doc.execute("shape.rectangle", json!({"x":10,"y":10,"width":20,"height":20})).unwrap()["id"].clone();
    doc.execute("select.set", json!({"ids":[rect]})).unwrap();
    doc.execute("edit.copy", json!({})).unwrap();
    doc.execute("edit.deselectAll", json!({})).unwrap();
    let second = doc.execute("layer.new", json!({"name":"Second"})).unwrap()["id"].clone();
    doc.execute("layer.setCurrent", json!({"id": first})).unwrap();
    doc.execute("layer.setProps", json!({"ids":[first], prop: prop == "locked"})).unwrap();
    (doc, first, second)
}

fn calls() -> [(&'static str, Value); 3] {
    [("shape.ellipse", json!({"x":10,"y":10,"width":20,"height":20})), ("text.add", json!({"x":10,"y":40,"text":"A"})), ("edit.paste", json!({}))]
}

#[test]
fn an_agents_art_is_refused_rather_than_put_elsewhere() {
    for prop in ["locked", "visible"] {
        let (mut doc, _, _) = drawing(prop);
        let before = doc.save(None).unwrap();
        for (command, params) in calls() {
            let error = doc.execute_as(command, params, &Origin::agent("agent:test")).unwrap_err();
            assert!(error.contains("Layer 1") && error.contains("locked or hidden"), "{prop} {command}: {error}");
        }
        assert_eq!(doc.save(None).unwrap(), before, "{prop}: nothing added");
    }
}

#[test]
fn a_persons_art_still_goes_to_the_layer_that_takes_it() {
    for prop in ["locked", "visible"] {
        let (mut doc, _, second) = drawing(prop);
        for (command, params) in calls() {
            doc.execute_as(command, params, &Origin::person("user:test")).unwrap();
        }
        let layers = doc.inspect().unwrap()["layers"].clone();
        let on_second = layers.as_array().unwrap().iter().find(|layer| layer["id"] == second).unwrap()["children"].as_array().map_or(0, Vec::len);
        assert_eq!(on_second, 3, "{prop}: {layers}");
    }
}
