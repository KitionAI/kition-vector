//! C8-vector-5 item 1: named entries have a persistent identity (issue-c8-identity).
//!
//! A swatch's, graphic style's, character or paragraph style's, symbol's or pattern's tool id was
//! its name (`swatch:Brand`), and a fresh read of the file made its object id from it. So a
//! rename told "changed name" in the receipt while a read of the saved file told `swatch:Old`
//! deleted and `swatch:New` created (kit-env `receipt-differs`), and an agent holding the id lost
//! the object. Each entry now has a uid saved with it (`swatch:<uid>`); the name stays an input.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

use kdm_model::VectorModel;
use kition_kit::session::KitModel;
use kition_kit::session::model::Exchange;
use kition_kit::{KitDocument, Premise};
use kition_vector_kit::VectorDocument;
use serde_json::{Value, json};

/// A saved drawing with a swatch `Brand`, a graphic style `Style`, character and paragraph styles
/// `Emphasis`, a symbol `Mark` and a pattern `Dots`.
fn saved() -> Vec<u8> {
    let mut doc = VectorDocument::create("Identity.kvector", 200, 200).unwrap();
    let r = doc.execute("shape.rectangle", json!({"x":1,"y":1,"width":5,"height":5})).unwrap()["id"].clone();
    doc.execute("swatch.new", json!({"name":"Brand","color":"#123456"})).unwrap();
    doc.execute("graphicStyle.new", json!({"name":"Style","id":r})).unwrap();
    doc.execute("charStyle.new", json!({"name":"Emphasis"})).unwrap();
    doc.execute("paraStyle.new", json!({"name":"Emphasis"})).unwrap();
    doc.execute("symbol.new", json!({"name":"Mark","ids":[r]})).unwrap();
    let p = doc.execute("shape.rectangle", json!({"x":50,"y":1,"width":5,"height":5})).unwrap()["id"].clone();
    doc.execute("select.set", json!({"ids":[p]})).unwrap();
    doc.execute("object.pattern.make", json!({"name":"Dots","edit":false})).unwrap();
    doc.save(None).unwrap()
}

/// `(type, key)` of every named entry of the replica's state.
fn keys(state: &Value) -> Vec<(String, String, String)> {
    let mut out: Vec<(String, String, String)> = state["objects"]
        .as_object()
        .unwrap()
        .values()
        .filter(|o| ["Swatch", "GraphicStyle", "CharStyle", "ParaStyle", "Symbol", "Pattern"].contains(&o["$type"].as_str().unwrap_or("")))
        .map(|o| (o["$type"].as_str().unwrap().to_owned(), o["key"].as_str().unwrap_or("").to_owned(), o["name"].as_str().unwrap_or("").to_owned()))
        .collect();
    out.sort();
    out
}

fn key_of(state: &Value, ty: &str, name: &str) -> String {
    keys(state).into_iter().find(|(t, _, n)| t == ty && n == name).unwrap_or_else(|| panic!("no {ty} {name}")).1
}

/// The change from `before` as the receipt's replica tells it and as a read of the saved file
/// tells it, each naming objects by tool id.
fn both(doc: &mut VectorDocument, exchange: &mut Exchange, before: &Value) -> (Value, Value) {
    let schema = VectorModel::schema();
    let after = exchange.replica(doc).unwrap().state_json().unwrap();
    let mut reopened = VectorDocument::open("Identity.kvector", &doc.save(None).unwrap()).unwrap();
    let saved = Exchange::new().replica(&mut reopened).unwrap().state_json().unwrap();
    let memory = kition_kit::kdm::reread::diff_paired(&schema, before, &after);
    let disk = kition_kit::kdm::reread::diff_paired(&schema, before, &saved);
    (memory.to_json(), disk.to_json())
}

fn ids(diff: &Value, list: &str) -> Vec<String> {
    diff[list].as_array().into_iter().flatten().map(|item| item["id"].as_str().map_or_else(|| item["id"].to_string(), str::to_owned)).collect()
}

#[test]
fn a_rename_keeps_the_tool_id_and_the_saved_file_tells_the_same() {
    let file = saved();
    for (ty, prefix, command, params) in [
        ("Swatch", "swatch:", "swatch.edit", json!({"name":"Brand","newName":"Brand Blue"})),
        ("GraphicStyle", "graphicStyle:", "graphicStyle.rename", json!({"name":"Style","to":"Look"})),
        ("CharStyle", "charStyle:", "charStyle.rename", json!({"name":"Emphasis","to":"Strong"})),
        ("ParaStyle", "paraStyle:", "paraStyle.rename", json!({"name":"Emphasis","to":"Body"})),
        ("Pattern", "pattern:", "pattern.options", json!({"name":"Dots","newName":"Spots"})),
    ] {
        let mut doc = VectorDocument::open("Identity.kvector", &file).unwrap();
        let mut exchange = Exchange::new();
        let before = exchange.replica(&mut doc).unwrap().state_json().unwrap();
        let old = params["name"].as_str().unwrap();
        let key = key_of(&before, ty, old);
        assert!(!key.is_empty() && key != old, "{ty}: the tool id is no name: {key}");
        exchange
            .batch(&mut doc, &json!({"ops":[{"op":"command","id":command,"params":params}]}), false)
            .unwrap_or_else(|e| panic!("{command}: {e:?}"));
        let (memory, disk) = both(&mut doc, &mut exchange, &before);
        assert_eq!(memory, disk, "{command}: the receipt's change differs from the saved one");
        assert!(ids(&disk, "created").is_empty() && ids(&disk, "deleted").is_empty(), "{command}: {disk}");
        assert!(ids(&disk, "changed").contains(&format!("{prefix}{key}")), "{command}: {disk}");
    }
}

#[test]
fn a_rename_through_the_object_model_keeps_the_tool_id() {
    let file = saved();
    let mut doc = VectorDocument::open("Identity.kvector", &file).unwrap();
    let mut exchange = Exchange::new();
    let before = exchange.replica(&mut doc).unwrap().state_json().unwrap();
    let key = key_of(&before, "Symbol", "Mark");
    exchange.batch(&mut doc, &json!({"ops":[{"op":"set","target":format!("symbol:{key}"),"field":"name","value":"Sign"}]}), false).unwrap();
    let (memory, disk) = both(&mut doc, &mut exchange, &before);
    assert_eq!(memory, disk);
    // The symbol, and its instance, which names it by its name (a read gives it the new one).
    assert!(ids(&disk, "changed").contains(&format!("symbol:{key}")), "{disk}");
    assert!(ids(&disk, "created").is_empty() && ids(&disk, "deleted").is_empty(), "{disk}");
}

#[test]
fn an_entry_a_batch_creates_has_its_identity_in_the_receipt_and_the_file() {
    let file = saved();
    let mut doc = VectorDocument::open("Identity.kvector", &file).unwrap();
    let mut exchange = Exchange::new();
    let before = exchange.replica(&mut doc).unwrap().state_json().unwrap();
    let receipt = exchange
        .batch(&mut doc, &json!({"ops":[{"op":"create","type":"Swatch","in":"root.swatches","at":1,"fields":{"name":"Fresh"}}]}), false)
        .unwrap();
    let (memory, disk) = both(&mut doc, &mut exchange, &before);
    assert_eq!(memory, disk, "receipt {receipt}");
    let created = ids(&disk, "created");
    assert_eq!(created.len(), 1, "{disk}");
    assert!(created[0].starts_with("swatch:") && created[0] != "swatch:Fresh", "{created:?}");
}

#[test]
fn a_file_without_identities_reads_the_same_ones_every_time_and_its_first_save_keeps_them() {
    let mut file: Value = serde_json::from_slice(&saved()).unwrap();
    // As a file written before identities: no `uid`.
    for table in ["swatches", "graphic_styles", "char_styles", "para_styles", "symbols", "patterns"] {
        for entry in file["document"][table].as_array_mut().unwrap() {
            entry.as_object_mut().unwrap().remove("uid");
        }
    }
    for group in file["document"]["swatch_groups"].as_array_mut().unwrap() {
        for entry in group["swatches"].as_array_mut().unwrap() {
            entry.as_object_mut().unwrap().remove("uid");
        }
    }
    let bytes = serde_json::to_vec(&file).unwrap();
    let state = |bytes: &[u8]| {
        let mut doc = VectorDocument::open("Identity.kvector", bytes).unwrap();
        let state = Exchange::new().replica(&mut doc).unwrap().state_json().unwrap();
        (keys(&state), doc.save(None).unwrap())
    };
    let (first, saved_once) = state(&bytes);
    let (second, _) = state(&bytes);
    assert_eq!(first, second, "two reads of one file give the same identities");
    assert!(first.iter().all(|(_, key, _)| !key.is_empty()), "{first:?}");
    let (third, _) = state(&saved_once);
    assert_eq!(first, third, "the first save writes them");
}

fn premise_ids(premises: &[Premise]) -> Vec<String> {
    premises.iter().map(|premise| premise.id.as_str().map_or_else(|| premise.id.to_string(), str::to_owned)).collect()
}

#[test]
fn a_write_by_name_rests_on_the_entry_the_name_names_by_its_identity() {
    let mut doc = VectorDocument::open("Identity.kvector", &saved()).unwrap();
    let state = Exchange::new().replica(&mut doc).unwrap().state_json().unwrap();
    let key = key_of(&state, "Swatch", "Brand");
    let under = doc.under("swatch.edit", &json!({"name":"Brand","color":"#ff0000"})).unwrap().unwrap();
    assert!(premise_ids(&under.premises).contains(&format!("swatch:{key}")), "{:?}", under.premises);
}
