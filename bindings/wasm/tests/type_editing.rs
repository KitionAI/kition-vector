//! Type by hand in the editor (plan 2026-11-25, stage 5, feature vector.editor.type): what
//! `inspect()` reports of the Type tool (`typing`: the text edited, its selected characters, the caret
//! line, the Character and Paragraph panels' attributes, what new type is set in), the page's `Type:`
//! requests (an input method's marked text, a new click count, new type's size), a selection
//! tool's double-click putting the caret where it was clicked (regressions/issue-dblclick-caret), and
//! a colour given to selected characters colouring only them (regressions/issue-range-colour).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

use kition_kit::KitDocument;
use kition_kit::tools::{KitTools, PointerAction, ToolMods, ToolPointer};
use kition_vector_kit::VectorDocument;
use serde_json::{Value, json};

fn run(doc: &mut VectorDocument, command: &str, params: Value) -> Value {
    doc.execute(command, params).unwrap_or_else(|error| panic!("{command}: {error}"))
}

fn typing(doc: &VectorDocument) -> Value {
    doc.inspect().unwrap()["typing"].clone()
}

/// A press of `action` at the drawing's point (`x`, `y`).
fn press(doc: &mut VectorDocument, action: PointerAction, x: f64, y: f64) {
    let origin = doc.inspect().unwrap()["origin"].clone();
    let (ox, oy) = (origin[0].as_f64().unwrap(), origin[1].as_f64().unwrap());
    doc.pointer(&ToolPointer { action, x: x - ox, y: y - oy, mods: ToolMods::default(), pressure: 1.0, zoom: 1.0 }).unwrap();
}

fn request(doc: &mut VectorDocument, body: Value) {
    doc.key(&format!("Type:{body}"), ToolMods::default(), 1.0).unwrap();
}

fn text_of(doc: &VectorDocument, id: &Value) -> String {
    let inspected = doc.inspect().unwrap();
    let layers = inspected["layers"].as_array().unwrap();
    let found = layers.iter().flat_map(|layer| layer["children"].as_array().unwrap()).find(|node| node["id"] == *id).unwrap();
    found["text"].as_str().unwrap().to_owned()
}

#[test]
fn inspect_reports_the_type_edited_and_its_attributes() {
    let mut doc = VectorDocument::create("type.kvector", 600, 400).unwrap();
    let id = run(&mut doc, "text.add", json!({ "x": 50, "y": 100, "text": "Hello world", "size": 24 }))["id"].clone();
    // Selected, not edited: the whole object's attributes, no caret.
    let shown = typing(&doc);
    assert_eq!(shown["editing"], Value::Null);
    assert_eq!(shown["caret"], Value::Null);
    assert_eq!(shown["character"]["size"], json!(24.0));
    assert_eq!(shown["paragraph"]["justify"], "auto");
    // "world" at 36: the object's sizes differ (null, named in `mixed`).
    run(&mut doc, "text.setRangeStyle", json!({ "id": id, "start": 6, "end": 11, "size": 36 }));
    let shown = typing(&doc);
    assert_eq!(shown["character"]["size"], Value::Null, "{shown}");
    assert!(shown["character"]["mixed"].as_array().unwrap().contains(&json!("size")), "{shown}");
    // Nothing selected: no attributes, and new type's defaults are the engine's (null).
    run(&mut doc, "edit.deselectAll", json!({}));
    let shown = typing(&doc);
    assert_eq!(shown["character"], Value::Null);
    assert_eq!(shown["newType"], json!({ "font": null, "style": null, "size": null }));
    // New type's size, set with nothing selected, whichever tool is chosen.
    request(&mut doc, json!({ "newType": { "size": 30 } }));
    assert_eq!(typing(&doc)["newType"]["size"], json!(30.0));
}

#[test]
fn a_selection_tools_double_click_puts_the_caret_where_it_was_clicked() {
    let mut doc = VectorDocument::create("type.kvector", 600, 400).unwrap();
    let id = run(&mut doc, "text.add", json!({ "x": 50, "y": 100, "text": "Hello world", "size": 24 }))["id"].clone();
    doc.select_tool("selection", 1.0).unwrap();
    for action in [PointerAction::Down, PointerAction::Up, PointerAction::Down, PointerAction::Up, PointerAction::DoubleClick] {
        press(&mut doc, action, 70.0, 95.0);
    }
    assert_eq!(doc.tool(), "type");
    let shown = typing(&doc);
    assert_eq!(shown["editing"], id);
    assert_eq!(shown["start"], shown["end"], "a caret, not the word: {shown}");
    let caret = shown["start"].as_u64().unwrap();
    assert!((1..5).contains(&caret), "between the letters clicked: {shown}");
    assert!(shown["caret"].as_array().is_some_and(|line| line.len() == 4), "{shown}");
    // Typing inserts there; choosing the Type tool afterwards (the page's switch) keeps editing.
    KitTools::text(&mut doc, "!", 1.0).unwrap();
    doc.select_tool("type", 1.0).unwrap();
    let text = text_of(&doc, &id);
    assert_eq!(text.len(), 12, "{text}");
    assert_eq!(text.replace('!', ""), "Hello world");
    assert_eq!(typing(&doc)["editing"], id);
}

#[test]
fn an_input_methods_marked_text_shows_in_place_and_its_commit_lands_once() {
    let mut doc = VectorDocument::create("type.kvector", 600, 400).unwrap();
    let id = run(&mut doc, "text.add", json!({ "x": 50, "y": 100, "text": "AB", "size": 24 }))["id"].clone();
    doc.select_tool("type", 1.0).unwrap();
    press(&mut doc, PointerAction::Down, 52.0, 95.0);
    press(&mut doc, PointerAction::Up, 52.0, 95.0);
    assert_eq!(typing(&doc)["editing"], id);
    request(&mut doc, json!({ "preedit": { "text": "zhong" } }));
    let shown = typing(&doc);
    assert_eq!(shown["composing"], json!(true));
    assert!(text_of(&doc, &id).contains("zhong"), "{}", text_of(&doc, &id));
    KitTools::text(&mut doc, "中", 1.0).unwrap();
    let text = text_of(&doc, &id);
    assert!(text.contains('中') && !text.contains("zhong"), "{text}");
    assert_eq!(typing(&doc)["composing"], json!(false));
    // A composition given up (an empty marked text) leaves nothing behind.
    request(&mut doc, json!({ "preedit": { "text": "wen" } }));
    request(&mut doc, json!({ "preedit": { "text": "" } }));
    assert_eq!(text_of(&doc, &id), text);
    // A request that is not JSON is refused, not a crash.
    assert!(doc.key("Type:{nope", ToolMods::default(), 1.0).is_err());
}

#[test]
fn a_colour_given_to_selected_characters_colours_only_them() {
    let mut doc = VectorDocument::create("type.kvector", 600, 400).unwrap();
    let id = run(&mut doc, "text.add", json!({ "x": 50, "y": 100, "text": "Hello world", "size": 24, "color": "#000000" }))["id"].clone();
    doc.select_tool("type", 1.0).unwrap();
    press(&mut doc, PointerAction::Down, 52.0, 95.0);
    press(&mut doc, PointerAction::Up, 52.0, 95.0);
    doc.key("Home", ToolMods::default(), 1.0).unwrap();
    for _ in 0..5 {
        doc.key("ArrowRight", ToolMods::default(), 1.0).unwrap();
    }
    for _ in 0..5 {
        doc.key("ArrowRight", ToolMods { shift: true, ..ToolMods::default() }, 1.0).unwrap();
    }
    let shown = typing(&doc);
    assert_eq!((shown["start"].as_u64(), shown["end"].as_u64()), (Some(5), Some(10)), "{shown}");
    run(&mut doc, "paint.setFill", json!({ "color": "#e03131" }));
    let runs = run(&mut doc, "text.getRange", json!({ "id": id }))["runs"].clone();
    let colours: Vec<(String, String)> =
        runs.as_array().unwrap().iter().map(|r| (r["text"].as_str().unwrap().to_owned(), r["style"]["fill"].to_string())).collect();
    assert_eq!(colours.len(), 3, "{colours:?}");
    assert_eq!(colours[1].0, " worl");
    assert_ne!(colours[0].1, colours[1].1, "{colours:?}");
    assert_eq!(colours[0].1, colours[2].1, "the characters outside the selection keep their colour: {colours:?}");
    // The panels read the selected characters' colour.
    assert_eq!(typing(&doc)["character"]["fill"], "#e03131");
    // With the type selected but not edited, the colour is the whole object's again.
    doc.key("Escape", ToolMods::default(), 1.0).unwrap();
    run(&mut doc, "paint.setFill", json!({ "color": "#2266ff" }));
    let runs = run(&mut doc, "text.getRange", json!({ "id": id }))["runs"].clone();
    let fills: Vec<String> = runs.as_array().unwrap().iter().map(|r| r["style"]["fill"].to_string()).collect();
    assert!(fills.windows(2).all(|pair| pair[0] == pair[1]), "every character takes it: {fills:?}");
    assert_eq!(typing(&doc)["character"]["fill"], "#2266ff");
}
