//! C8-vector-5 item 3: what kit-env's `create` edits do. kit-env makes a new object in a list
//! with the scalar values of an object of its type (never a json field, a read-only field or the
//! tool id's) and its label from a word list, as one model batch, and keeps it when the receipt is
//! the difference of the file read again after saving. A create the kit refuses, or whose receipt
//! differs, is discarded (26 `refused` and 2 `receipt-differs` at seed 7 before this card).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

use kdm_model::VectorModel;
use kition_kit::KitDocument;
use kition_kit::session::KitModel;
use kition_kit::session::model::Exchange;
use kition_vector_kit::VectorDocument;
use serde_json::{Value, json};

/// A drawing with an entry of every named table, a slice, an export asset and an image.
fn drawing() -> Vec<u8> {
    let mut doc = VectorDocument::create("Creates.kvector", 300, 200).unwrap();
    let r = doc.execute("shape.rectangle", json!({"x":1,"y":1,"width":5,"height":5})).unwrap()["id"].clone();
    doc.execute("swatch.new", json!({"name":"Brand","color":"#123456"})).unwrap();
    doc.execute("graphicStyle.new", json!({"name":"Style","id":r})).unwrap();
    doc.execute("charStyle.new", json!({"name":"Emphasis"})).unwrap();
    doc.execute("paraStyle.new", json!({"name":"Body"})).unwrap();
    doc.execute("symbol.new", json!({"name":"Mark","ids":[r]})).unwrap();
    let p = doc.execute("shape.rectangle", json!({"x":50,"y":1,"width":5,"height":5})).unwrap()["id"].clone();
    doc.execute("select.set", json!({"ids":[p]})).unwrap();
    doc.execute("object.pattern.make", json!({"name":"Dots","edit":false})).unwrap();
    doc.execute("object.slice.make", json!({})).unwrap();
    doc.execute("edit.deselectAll", json!({})).unwrap();
    doc.save(None).unwrap()
}

/// The scalar fields of the first object of `ty` in `state` that kit-env would copy: strings,
/// numbers and booleans that are no json, no read-only field and not the tool id's (`key`, `id`).
fn scalars(state: &Value, ty: &str) -> serde_json::Map<String, Value> {
    let schema: Value = serde_json::from_str(include_str!("../../../crates/kdm/kdm-model/model/schema.json")).unwrap();
    let fields = &schema["types"][ty]["fields"];
    let object = state["objects"].as_object().unwrap().values().find(|o| o["$type"] == ty);
    let mut out = serde_json::Map::new();
    if let (Some(object), Some(fields)) = (object, fields.as_object()) {
        for (name, def) in fields {
            let scalar = matches!(def["kind"].as_str(), Some("string" | "integer" | "number" | "boolean" | "enum"));
            if scalar && def["readOnly"] != true && def["sample"] != false && !object[name].is_null() {
                out.insert(name.clone(), object[name].clone());
            }
        }
    }
    out
}

#[test]
fn every_list_of_the_drawing_takes_a_create_as_kit_env_makes_it() {
    let schema = VectorModel::schema();
    let file = drawing();
    let mut problems = Vec::new();
    for (ty, field, label) in [
        ("Swatch", "swatches", Some("name")),
        ("SwatchGroup", "swatchGroups", Some("name")),
        ("GraphicStyle", "graphicStyles", Some("name")),
        ("CharStyle", "charStyles", Some("name")),
        ("ParaStyle", "paraStyles", Some("name")),
        ("Symbol", "symbols", Some("name")),
        ("Pattern", "patterns", Some("name")),
        ("Artboard", "artboards", None),
        ("Slice", "slices", None),
        ("Node", "nodes", Some("name")),
    ] {
        let mut doc = VectorDocument::open("Creates.kvector", &file).unwrap();
        let mut exchange = Exchange::new();
        let before = exchange.replica(&mut doc).unwrap().state_json().unwrap();
        let mut fields = scalars(&before, ty);
        if let Some(label) = label {
            fields.insert(label.to_owned(), json!("Harbor"));
        }
        let mut op = json!({"op":"create","type":ty,"in":format!("root.{field}"),"at":0,"fields":fields});
        if ty == "Node" {
            // Into the first layer (a tree: the node goes last among its children).
            op["parent"] = json!("1");
            op.as_object_mut().unwrap().remove("at");
        }
        let receipt = match exchange.batch(&mut doc, &json!({"ops":[op]}), false) {
            Ok(receipt) => receipt,
            Err(error) => {
                problems.push(format!("{ty}: refused: {}", error.message));
                continue;
            }
        };
        let after = exchange.replica(&mut doc).unwrap().state_json().unwrap();
        let mut reopened = VectorDocument::open("Creates.kvector", &doc.save(None).unwrap()).unwrap();
        let saved = Exchange::new().replica(&mut reopened).unwrap().state_json().unwrap();
        let memory = kition_kit::kdm::reread::diff_paired(&schema, &before, &after).to_json();
        let disk = kition_kit::kdm::reread::diff_paired(&schema, &before, &saved).to_json();
        if memory != disk {
            problems.push(format!("{ty}: receipt differs: {memory} != {disk} (receipt {receipt})"));
        } else if memory["created"].as_array().is_none_or(Vec::is_empty) {
            problems.push(format!("{ty}: nothing created: {memory}"));
        } else {
            println!("{ty}: created {}", memory["created"][0]["id"]);
        }
    }
    // An export asset of no objects is refused (the drawing would drop it).
    let mut doc = VectorDocument::open("Creates.kvector", &file).unwrap();
    let refused = Exchange::new().batch(&mut doc, &json!({"ops":[{"op":"create","type":"Asset","in":"root.assets","fields":{}}]}), false);
    match refused {
        Err(error) if error.message.contains("needs the objects it exports") => {}
        other => problems.push(format!("Asset: {other:?}")),
    }
    // The built-in swatches are read-only, as the engine treats them.
    let mut doc = VectorDocument::open("Creates.kvector", &file).unwrap();
    let mut exchange = Exchange::new();
    let state = exchange.replica(&mut doc).unwrap().state_json().unwrap();
    for name in ["[None]", "[Registration]"] {
        let Some(swatch) = state["objects"].as_object().unwrap().values().find(|o| o["$type"] == "Swatch" && o["name"] == name) else { continue };
        assert_eq!(swatch["role"], "builtIn", "{swatch}");
        let target = format!("swatch:{}", swatch["key"].as_str().unwrap());
        for op in [json!({"op":"delete","target":target}), json!({"op":"set","target":target,"field":"name","value":"Mine"})] {
            if exchange.batch(&mut doc, &json!({"ops":[op.clone()]}), false).is_ok() {
                problems.push(format!("{name}: {op} applied"));
            }
        }
    }
    for problem in &problems {
        println!("  {}", problem.chars().take(700).collect::<String>());
    }
    assert!(problems.is_empty(), "{} creates fail", problems.len());
}

/// The drawing's derived digests (`~graphicStyles`, …) change only with their table: a model batch
/// that reloads the view gives a graphic style no file links (local id 0) a new local id, and the
/// digest held that id, so every such batch's receipt told `~graphicStyles` changed and a read of
/// the saved file did not (kit-env `receipt-differs`).
#[test]
fn a_batch_elsewhere_leaves_the_table_digests_alone() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/dusk-poster.kvector");
    let mut doc = VectorDocument::open("Digest.kvector", &std::fs::read(path).unwrap()).unwrap();
    let before = doc.derived(&[], &[]).unwrap();
    let op = json!({"op":"create","type":"Node","in":"root.nodes","parent":"1","fields":{"visible":true,"locked":false}});
    Exchange::new().batch(&mut doc, &json!({"ops":[op]}), false).unwrap();
    let after = doc.derived(&[], &[]).unwrap();
    assert_eq!(before[0].values["graphicStyles"], after[0].values["graphicStyles"]);
    assert_eq!(before[0].values["swatches"], after[0].values["swatches"]);
}

/// C8-vector-engine-2 item 6: kit-env's create put a new node into a path or a mesh (a tree child of
/// an object that holds none); the batch took it, its receipt told it created, and the drawing,
/// which cannot hold it there, dropped it on the next read (16 `receipt-differs` at seed 7). A node
/// goes only into a layer, a group, a compound path, a blend, an envelope or a repeat: elsewhere the
/// batch is refused.
#[test]
fn a_node_goes_only_into_an_object_that_holds_others() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/dusk-poster.kvector");
    let file = std::fs::read(path).unwrap();
    let mut doc = VectorDocument::open("Into.kvector", &file).unwrap();
    // A rectangle of the drawing (a path, holding nothing).
    let inspected = doc.inspect().unwrap();
    let rectangle =
        inspected["layers"][0]["children"].as_array().unwrap().iter().find(|n| n["kind"] == "Rounded Rectangle").unwrap()["id"].as_u64().unwrap();
    let into = |parent: u64| {
        json!({"ops":[
            {"op":"create","type":"Node","in":"root.nodes","parent":parent.to_string(),"ref":"$new","fields":{"visible":true,"locked":false}},
            {"op":"tree.move","target":"root","field":"nodes","item":"$new","parent":parent.to_string()}
        ]})
    };
    match Exchange::new().batch(&mut doc, &into(rectangle), false) {
        Err(error) => assert!(error.message.contains("holds no objects"), "{}", error.message),
        Ok(answer) => panic!("a node made inside a rectangle was taken: {answer}"),
    }
    // Into the layer it is taken, and kept by the saved file.
    let mut doc = VectorDocument::open("Into.kvector", &file).unwrap();
    let answer = Exchange::new().batch(&mut doc, &into(1), false).unwrap();
    let id: u64 = answer["refs"]["$new"].as_str().unwrap().parse().unwrap();
    let mut reopened = VectorDocument::open("Into.kvector", &doc.save(None).unwrap()).unwrap();
    assert!(reopened.execute("file.node", json!({"id": id})).is_ok());
}
