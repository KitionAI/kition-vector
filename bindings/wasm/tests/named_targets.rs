//! C8-vector-engine: a command that takes its objects as `ids` (or `id`) runs on them with nothing
//! selected; its menu item still needs a selection, as before (issue-c8-engine-named-targets).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

use kition_kit::KitDocument;
use kition_vector_kit::VectorDocument;
use serde_json::{Value, json};

/// A rectangle, a text and a symbol instance; nothing selected.
fn drawing() -> (VectorDocument, Value, Value, Value) {
    let mut doc = VectorDocument::create("Named.kvector", 320, 200).unwrap();
    let rect = doc.execute("shape.rectangle", json!({"x":10,"y":10,"width":20,"height":20})).unwrap()["id"].clone();
    let text = doc.execute("text.add", json!({"x":10,"y":80,"text":"abc"})).unwrap()["id"].clone();
    let art = doc.execute("shape.ellipse", json!({"x":60,"y":10,"width":20,"height":20})).unwrap()["id"].clone();
    doc.execute("select.set", json!({"ids":[art]})).unwrap();
    let made = doc.execute("symbol.new", json!({"name":"Dot"})).unwrap();
    let instance = made.get("instance").or_else(|| made.get("id")).cloned().unwrap_or(art);
    doc.execute("edit.deselectAll", json!({})).unwrap();
    (doc, rect, text, instance)
}

const DISABLED: &str = "nothing selected";

#[test]
fn named_objects_need_no_selection() {
    let (mut doc, rect, text, instance) = drawing();
    doc.execute("symbol.breakLink", json!({"ids":[instance]})).unwrap();
    doc.execute("object.slice.make", json!({"ids":[rect]})).unwrap();
    doc.execute("text.changeCase", json!({"case":"upper","ids":[text]})).unwrap();
    let inspected = doc.inspect().unwrap().to_string();
    assert!(inspected.contains("ABC"), "the named text changed case");
}

#[test]
fn the_menu_still_needs_a_selection() {
    let (doc, _, _, _) = drawing();
    let disabled = doc.disabled_commands();
    for id in ["symbol.breakLink", "object.slice.make", "text.changeCase", "object.textWrap.make"] {
        assert!(disabled.iter().any(|d| d == id), "{id} is still off in the menu with nothing selected");
    }
}

/// Every command whose targets are `ids`: given an object, it is never refused for want of a
/// selection (it may refuse the object for what it is).
#[test]
fn no_command_that_takes_ids_asks_for_a_selection_instead() {
    let (doc, _, _, _) = drawing();
    let mut refused = vec![];
    for command in doc.commands().iter().filter(|command| command.targets == Some("ids")) {
        let (mut doc, rect, _, _) = drawing();
        if let Err(error) = doc.execute(&command.id, json!({"ids":[rect]}))
            && error.contains("not available right now")
            && error.contains(DISABLED)
        {
            refused.push(format!("{}: {error}", command.id));
        }
    }
    assert!(refused.is_empty(), "{} commands:\n{}", refused.len(), refused.join("\n"));
}
