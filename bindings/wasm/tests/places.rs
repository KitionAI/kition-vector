//! C8-land-11-vector item 4: what each call rests on, read from the handler (issue-c8-places).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

use kition_kit::{KitDocument, Premise};
use kition_vector_kit::VectorDocument;
use serde_json::{Value, json};

fn drawing() -> (VectorDocument, u64, u64, u64, u64) {
    let mut doc = VectorDocument::create("Places.kvector", 400, 300).unwrap();
    let a = doc.execute("shape.rectangle", json!({"x":10,"y":10,"width":20,"height":20})).unwrap()["id"].as_u64().unwrap();
    let b = doc.execute("shape.ellipse", json!({"x":60,"y":10,"width":20,"height":20})).unwrap()["id"].as_u64().unwrap();
    let c = doc.execute("shape.ellipse", json!({"x":100,"y":10,"width":20,"height":20})).unwrap()["id"].as_u64().unwrap();
    doc.execute("select.set", json!({"ids":[b, c]})).unwrap();
    let group = doc.execute("arrange.group", json!({})).unwrap()["id"].as_u64().unwrap();
    doc.execute("select.set", json!({"ids":[a]})).unwrap();
    (doc, a, b, c, group)
}

fn premise(premises: &[Premise], id: u64) -> &Premise {
    premises.iter().find(|premise| premise.id == json!(id)).unwrap_or_else(|| panic!("no premise on {id}: {premises:?}"))
}

fn fields(premise: &Premise) -> Vec<String> {
    premise.fields.clone().unwrap_or_else(|| panic!("{premise:?} holds every field"))
}

#[test]
fn a_property_write_holds_the_fields_it_sets_and_no_place() {
    let (doc, a, ..) = drawing();
    let under = doc.under("object.setProps", &json!({"ids":[a],"name":"N","opacity":50})).unwrap().unwrap();
    let held = premise(&under.premises, a);
    assert_eq!(fields(held), ["name", "props.opacity"]);
    assert_eq!(held.positions, Some(vec![]));
    let under = doc.under("transparency.set", &json!({"ids":[a],"opacity":50,"item":0})).unwrap().unwrap();
    assert_eq!(fields(premise(&under.premises, a)), ["appearance"]);
}

#[test]
fn a_fill_holds_the_painted_objects_appearance_and_the_groups_membership() {
    let (doc, a, b, c, group) = drawing();
    let under = doc.under("paint.setFill", &json!({"ids":[a],"color":"#ff0000"})).unwrap().unwrap();
    assert_eq!(fields(premise(&under.premises, a)), ["appearance"]);
    assert!(under.within.is_empty());
    let under = doc.under("paint.setFill", &json!({"ids":[group],"color":"#ff0000"})).unwrap().unwrap();
    assert_eq!(fields(premise(&under.premises, group)), Vec::<String>::new(), "the group only has to exist");
    for leaf in [b, c] {
        assert_eq!(fields(premise(&under.premises, leaf)), ["appearance"]);
    }
    assert_eq!((under.within[0].parent.clone(), under.within[0].from, under.within[0].to), (json!(group), 0, 2));
    // Without ids: the selection (the agent's `on`).
    let under = doc.under("paint.setFill", &json!({"color":"#ff0000"})).unwrap().unwrap();
    assert_eq!(fields(premise(&under.premises, a)), ["appearance"]);
}

#[test]
fn a_move_holds_the_geometry_of_what_it_moves() {
    let (doc, a, b, c, group) = drawing();
    let under = doc.under("object.move", &json!({"dx":3,"dy":0})).unwrap().unwrap();
    assert_eq!(fields(premise(&under.premises, a)), ["appearance", "data"]);
    let mut doc = doc;
    doc.execute("select.set", json!({"ids":[group]})).unwrap();
    let under = doc.under("object.move", &json!({"dx":3,"dy":0})).unwrap().unwrap();
    for id in [group, b, c] {
        assert!(fields(premise(&under.premises, id)).contains(&"data".to_owned()), "{id}");
    }
}

#[test]
fn a_rename_is_not_held_by_geometry_and_a_lock_holds_only_locked() {
    let (doc, a, ..) = drawing();
    let under = doc.under("object.lock", &json!({})).unwrap().unwrap();
    assert_eq!(fields(premise(&under.premises, a)), ["locked"]);
}

#[test]
fn new_art_holds_the_layer_it_goes_into() {
    let (doc, ..) = drawing();
    let layer = doc.inspect().unwrap()["currentLayer"].as_u64().unwrap();
    let under = doc.under("shape.rectangle", &json!({"x":0,"y":0,"width":5,"height":5})).unwrap().unwrap();
    assert_eq!(fields(premise(&under.premises, layer)), ["locked", "visible"]);
}

#[test]
fn an_artboard_by_index_is_its_window_in_the_drawing() {
    let (doc, ..) = drawing();
    let under = doc.under("artboard.setProps", &json!({"index":0,"name":"A"})).unwrap().unwrap();
    assert_eq!(
        (under.within[0].parent.clone(), under.within[0].field.as_str(), under.within[0].from, under.within[0].to),
        (json!("drawing"), "artboards", 0, 1)
    );
}

#[test]
fn a_structural_write_holds_its_objects_whole() {
    let (doc, a, ..) = drawing();
    let under = doc.under("edit.delete", &json!({"ids":[a]})).unwrap().unwrap();
    assert_eq!(premise(&under.premises, a).fields, None);
    let commands = doc.commands();
    let value: Value =
        serde_json::to_value(commands.iter().find(|c| c.id == "arrange.group").unwrap().reads.iter().map(|r| r.as_str()).collect::<Vec<_>>())
            .unwrap();
    assert_eq!(value, json!(["selection"]));
    let under = doc.under("arrange.group", &json!({})).unwrap().unwrap();
    assert_eq!(premise(&under.premises, a).fields, Some(vec![]), "grouped where it is, its fields as they are");
    assert_eq!(premise(&under.premises, a).positions, Some(vec!["nodes".to_owned()]));
}

/// C8-vector-5 item 6: the pinned SDK keeps one premise per object id, the last one. A layer
/// write without ids acts on the current layer, which is also where new art would go: the
/// layer's own premise (the name it sets) and the insertion layer's (shown, unlocked, a layer)
/// name one object, and are joined, not the later one kept.
#[test]
fn two_premises_on_one_object_are_joined() {
    let (doc, ..) = drawing();
    let under = doc.under("layer.setProps", &json!({"name":"Renamed"})).unwrap().unwrap();
    let ids: Vec<String> = under.premises.iter().map(|premise| premise.id.to_string()).collect();
    let unique: std::collections::BTreeSet<&String> = ids.iter().collect();
    assert_eq!(ids.len(), unique.len(), "one premise per object: {:?}", under.premises);
    let layer =
        under.premises.iter().find(|premise| fields(premise).contains(&"visible".to_owned())).unwrap_or_else(|| panic!("{:?}", under.premises));
    assert!(fields(layer).contains(&"name".to_owned()), "the name the write sets is still held: {layer:?}");
}

/// C8-vector-engine-2: the guard asks only for an agent's writes, whose new art is refused on a
/// locked or hidden current layer: the place is then no layer at all (not the layer a person's art
/// would fall back to), and the current layer when it takes the art.
#[test]
fn an_agents_new_art_rests_on_the_layer_it_will_go_into_or_none() {
    let (mut doc, ..) = drawing();
    let current = doc.inspect().unwrap()["currentLayer"].as_u64().unwrap();
    let other = doc.execute("layer.new", json!({"name": "Other"})).unwrap()["id"].as_u64().unwrap();
    doc.execute("layer.setCurrent", json!({"id": current})).unwrap();
    let rectangle = json!({"x": 0, "y": 0, "width": 5, "height": 5});
    let under = doc.under("shape.rectangle", &rectangle).unwrap().unwrap();
    assert!(under.premises.iter().any(|p| p.id == json!(current)), "{:?}", under.premises);
    doc.execute("layer.setProps", json!({"ids": [current], "locked": true})).unwrap();
    doc.execute("layer.setCurrent", json!({"id": current})).unwrap();
    let under = doc.under("shape.rectangle", &rectangle).unwrap().unwrap();
    for layer in [current, other] {
        assert!(!under.premises.iter().any(|p| p.id == json!(layer)), "an agent's art on a locked layer goes nowhere: {:?}", under.premises);
    }
}
