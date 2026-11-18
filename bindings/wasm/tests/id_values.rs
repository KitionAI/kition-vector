//! C8-vector-engine: a value that is not an object id, where a command takes one, is refused
//! (invalid-params) rather than dropped, so the command never acts on its default instead: the
//! current layer, the selection, the top level (issue-c8-engine-id-values).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

use kition_kit::KitDocument;
use kition_vector_engine::Origin;
use kition_vector_kit::VectorDocument;
use serde_json::{Value, json};

/// Two layers with art, the second current, a rectangle of the first selected.
fn drawing() -> (VectorDocument, Value, Value, Value) {
    let mut doc = VectorDocument::create("Ids.kvector", 320, 200).unwrap();
    let rect = doc.execute("shape.rectangle", json!({"x":10,"y":10,"width":20,"height":20})).unwrap()["id"].clone();
    let first = doc.inspect().unwrap()["layers"][0]["id"].clone();
    let second = doc.execute("layer.new", json!({"name":"Second"})).unwrap()["id"].clone();
    doc.execute("shape.ellipse", json!({"x":60,"y":10,"width":20,"height":20})).unwrap();
    doc.execute("select.set", json!({"ids":[rect]})).unwrap();
    (doc, rect, first, second)
}

fn layer_names(doc: &VectorDocument) -> Vec<Value> {
    doc.inspect().unwrap()["layers"].as_array().unwrap().iter().map(|layer| layer["name"].clone()).collect()
}

#[test]
fn a_string_id_deletes_no_layer() {
    let (mut doc, _, _, _) = drawing();
    let before = layer_names(&doc);
    let error = doc.execute("layer.delete", json!({"id":"x"})).unwrap_err();
    assert!(error.contains("`id`"), "{error}");
    assert_eq!(layer_names(&doc), before, "the current layer is still there");
}

#[test]
fn a_string_parent_moves_no_layer_to_the_top() {
    let (mut doc, _, first, second) = drawing();
    let sub = doc.execute("layer.newSublayer", json!({"parent": first})).unwrap()["id"].clone();
    let before = doc.save(None).unwrap();
    assert!(second.is_u64());
    let error = doc.execute("node.move", json!({"id": sub, "parent": "Second", "index": 0})).unwrap_err();
    assert!(error.contains("`parent`"), "{error}");
    assert_eq!(doc.save(None).unwrap(), before);
}

#[test]
fn a_string_layer_puts_no_guide_on_the_current_layer() {
    let (mut doc, _, _, _) = drawing();
    let before = doc.save(None).unwrap();
    let error = doc.execute("guide.add", json!({"pos":50,"layer":"x"})).unwrap_err();
    assert!(error.contains("`layer`"), "{error}");
    assert_eq!(doc.save(None).unwrap(), before);
}

#[test]
fn a_string_among_ids_is_refused_not_dropped() {
    let (mut doc, rect, _, _) = drawing();
    let before = doc.save(None).unwrap();
    for ids in [json!(["x"]), json!([rect, "x"]), json!([1.5])] {
        let error = doc.execute("plugin.run", json!({"id":"kition.test", "ids": ids})).unwrap_err();
        assert!(error.contains("`ids`"), "{error}");
    }
    assert_eq!(doc.save(None).unwrap(), before);
}

/// A numeric string is not an id either: the SDK's client hands an agent's `"9"` on as the
/// integer before the call reaches the kit, and the engine refuses strings in id lists.
#[test]
fn a_numeric_string_is_refused_too() {
    let (mut doc, _, _, second) = drawing();
    let before = layer_names(&doc);
    assert!(doc.execute("layer.delete", json!({"id": second.as_u64().unwrap().to_string()})).is_err());
    assert_eq!(layer_names(&doc), before);
}

/// The same refusal for an agent and a person: the value is simply not an id.
#[test]
fn the_refusal_is_the_same_for_an_agent_and_a_person() {
    for origin in [Origin::agent("agent:test"), Origin::person("user:test")] {
        let (mut doc, _, _, _) = drawing();
        let before = layer_names(&doc);
        assert!(doc.execute_as("layer.delete", json!({"id":"x"}), &origin).is_err());
        assert_eq!(layer_names(&doc), before, "{origin:?}");
    }
}

/// Every command, every object parameter its declaration names at the top level: a value that is
/// no id ("x", or a list holding one) is refused. A parameter that also takes a name (an effect's
/// catalogue id) is left out: a string there is that name.
#[test]
fn every_object_parameter_refuses_a_value_that_is_no_id() {
    let commands = drawing().0.commands();
    let mut accepted = vec![];
    let mut tried = 0;
    for command in &commands {
        let Some(schema) = command.schema.as_ref() else { continue };
        let params: Vec<&str> =
            command.targets.into_iter().chain(command.objects.iter().copied()).filter(|path| !path.contains(['.', '['])).collect();
        for param in params {
            let node = &schema[param];
            if node.get("oneOf").is_some() || node.get("type").and_then(Value::as_str) == Some("string") {
                continue;
            }
            let list = node.get("type").and_then(Value::as_str) == Some("array");
            let value = if list { json!(["x"]) } else { json!("x") };
            let (mut doc, _, _, _) = drawing();
            tried += 1;
            if let Ok(answer) = doc.execute(&command.id, json!({ param: value })) {
                accepted.push(format!("{} {param}: {answer}", command.id));
            }
        }
    }
    assert!(tried > 100, "{tried}");
    assert!(accepted.is_empty(), "{} of {tried} accepted a value that is no id:\n{}", accepted.len(), accepted.join("\n"));
}
