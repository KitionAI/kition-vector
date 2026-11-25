//! C8-vector-4 item 3: names once per table (tool ids are the entries' identities since
//! C8-vector-5: `swatch:<uid>`, `charStyle:<uid>`).
//!
//! The commands keep a name once per table (a rename to a taken name gets `Brand 2` or is
//! refused), but a file can hold a name twice: the engine's lookups took the first silently, and
//! two objects shared one tool id. A file is read with each name once per table (the later ones
//! renamed as the commands would name them), so a write by name names exactly one object.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

use kition_kit::{KitDocument, Premise};
use kition_vector_kit::VectorDocument;
use serde_json::{Value, json};

/// A saved drawing with a swatch `Brand`, a graphic style `Style`, character styles `Emphasis`
/// and `Quiet`, a paragraph style `Emphasis` and a symbol `Mark`.
fn saved() -> Value {
    let mut doc = VectorDocument::create("Names.kvector", 200, 200).unwrap();
    let r = doc.execute("shape.rectangle", json!({"x":1,"y":1,"width":5,"height":5})).unwrap()["id"].clone();
    doc.execute("swatch.new", json!({"name":"Brand","color":"#123456"})).unwrap();
    doc.execute("graphicStyle.new", json!({"name":"Style","id":r})).unwrap();
    doc.execute("charStyle.new", json!({"name":"Emphasis"})).unwrap();
    doc.execute("charStyle.new", json!({"name":"Quiet"})).unwrap();
    doc.execute("paraStyle.new", json!({"name":"Emphasis"})).unwrap();
    doc.execute("symbol.new", json!({"name":"Mark","ids":[r]})).unwrap();
    serde_json::from_slice(&doc.save(None).unwrap()).unwrap()
}

fn named<'a>(list: &'a Value, name: &str) -> &'a Value {
    list.as_array().unwrap().iter().find(|item| item["name"] == name).unwrap_or_else(|| panic!("no {name}"))
}

fn names(list: &Value) -> Vec<String> {
    list.as_array().unwrap().iter().map(|item| item["name"].as_str().unwrap().to_owned()).collect()
}

#[test]
fn a_file_with_a_name_twice_in_a_table_opens_with_each_name_once() {
    let mut file = saved();
    let d = &mut file["document"];
    // A second `Brand`, red, in the first colour group (swatches and groups share one namespace).
    let mut brand = named(&d["swatches"], "Brand").clone();
    brand["paint"] = json!({"type":"solid","color":{"model":"rgb","r":1.0,"g":0.0,"b":0.0}});
    d["swatch_groups"][0]["swatches"].as_array_mut().unwrap().push(brand);
    // A colour group named `Brand` too.
    d["swatch_groups"].as_array_mut().unwrap().push(json!({"name":"Brand","swatches":[]}));
    for (table, name) in [("graphic_styles", "Style"), ("char_styles", "Quiet"), ("para_styles", "Emphasis"), ("symbols", "Mark")] {
        let copy = named(&d[table], name).clone();
        d[table].as_array_mut().unwrap().push(copy);
    }
    let mut doc = VectorDocument::open("Names.kvector", &serde_json::to_vec(&file).unwrap()).unwrap();
    let again: Value = serde_json::from_slice(&doc.save(None).unwrap()).unwrap();
    let d = &again["document"];
    let mut swatch_names = names(&d["swatches"]);
    for group in d["swatch_groups"].as_array().unwrap() {
        swatch_names.push(group["name"].as_str().unwrap().to_owned());
        swatch_names.extend(names(&group["swatches"]));
    }
    let count = |all: &[String], name: &str| all.iter().filter(|n| *n == name).count();
    assert_eq!(count(&swatch_names, "Brand"), 1, "{swatch_names:?}");
    assert!(swatch_names.contains(&"Brand 2".to_owned()) && swatch_names.contains(&"Brand 3".to_owned()), "{swatch_names:?}");
    for (table, name) in [("graphic_styles", "Style"), ("char_styles", "Quiet"), ("para_styles", "Emphasis"), ("symbols", "Mark")] {
        let all = names(&d[table]);
        assert_eq!(count(&all, name), 1, "{table}: {all:?}");
        assert!(all.contains(&format!("{name} 2")), "{table}: {all:?}");
    }
    // A character style and a paragraph style may share a name: they are different tables.
    assert_eq!(count(&names(&d["char_styles"]), "Emphasis"), 1);
    assert_eq!(count(&names(&d["para_styles"]), "Emphasis"), 1);
    // The first keeps the name: art filled by `Brand` takes the first one's colour.
    let r = doc.execute("shape.rectangle", json!({"x":1,"y":1,"width":5,"height":5})).unwrap()["id"].clone();
    doc.execute("paint.setFill", json!({"ids":[r],"swatch":"Brand"})).unwrap();
    let fill = doc.execute("file.node", json!({"id":r})).unwrap()["appearance"]["items"][0]["paint"]["color"].clone();
    assert!(fill["r"].as_f64().unwrap() < 0.5, "the first Brand (#123456): {fill}");
}

fn premise_ids(premises: &[Premise]) -> Vec<String> {
    premises.iter().map(|premise| premise.id.as_str().map_or_else(|| premise.id.to_string(), str::to_owned)).collect()
}

#[test]
fn a_text_style_write_rests_on_that_style_not_on_the_table() {
    let file = saved();
    // A style's tool id is its persistent identity (C8-vector-5), saved as `uid`.
    let uid = |table: &str| named(&file["document"][table], "Emphasis")["uid"].as_str().unwrap().to_owned();
    let doc = VectorDocument::open("Names.kvector", &serde_json::to_vec(&file).unwrap()).unwrap();
    let under = doc.under("charStyle.setAttrs", &json!({"name":"Emphasis","attrs":{"size":18}})).unwrap().unwrap();
    let ids = premise_ids(&under.premises);
    assert!(ids.contains(&format!("charStyle:{}", uid("char_styles"))), "{ids:?}");
    assert!(!ids.contains(&"drawing".to_owned()), "no table digest: {:?}", under.premises);
    let under = doc.under("paraStyle.setAttrs", &json!({"name":"Emphasis","attrs":{"leading":18}})).unwrap().unwrap();
    assert!(premise_ids(&under.premises).contains(&format!("paraStyle:{}", uid("para_styles"))), "{:?}", under.premises);
}
