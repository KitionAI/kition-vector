//! C8-vector-engine: an artboard command edits the artboard its `index` names. `artboard.setProps`
//! without one, or with one that is not a whole number, is refused (it edited artboard 1);
//! Fit to Artwork Bounds and Fit to Selected Art keep artboard 1 as their default for a person's
//! menu, and an agent names the artboard (issue-c8-engine-artboard-index).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

use kition_kit::KitDocument;
use kition_vector_engine::Origin;
use kition_vector_kit::VectorDocument;
use serde_json::json;

fn drawing() -> VectorDocument {
    let mut doc = VectorDocument::create("Boards.kvector", 320, 200).unwrap();
    let rect = doc.execute("shape.rectangle", json!({"x":10,"y":10,"width":20,"height":20})).unwrap()["id"].clone();
    doc.execute("select.set", json!({"ids":[rect]})).unwrap();
    doc
}

#[test]
fn set_props_needs_the_index_of_an_artboard() {
    let mut doc = drawing();
    let before = doc.save(None).unwrap();
    for params in [json!({"width": 50}), json!({"index": "1", "width": 50}), json!({"index": 0.5, "width": 50}), json!({"index": -1, "width": 50})] {
        let error = doc.execute("artboard.setProps", params.clone()).unwrap_err();
        assert!(error.contains("index"), "{params}: {error}");
    }
    assert_eq!(doc.save(None).unwrap(), before);
    doc.execute("artboard.setProps", json!({"index": 0, "width": 50})).unwrap();
}

#[test]
fn an_agent_names_the_artboard_it_fits() {
    for command in ["artboard.fitToArt", "artboard.fitToSelection"] {
        let mut doc = drawing();
        let before = doc.save(None).unwrap();
        let error = doc.execute_as(command, json!({}), &Origin::agent("agent:test")).unwrap_err();
        assert!(error.contains("index"), "{command}: {error}");
        assert!(doc.execute_as(command, json!({"index": "x"}), &Origin::person("user:test")).is_err(), "{command}");
        assert_eq!(doc.save(None).unwrap(), before, "{command}");
        doc.execute_as(command, json!({"index": 0}), &Origin::agent("agent:test")).unwrap();
    }
}

#[test]
fn a_persons_menu_fits_the_first_artboard() {
    for command in ["artboard.fitToArt", "artboard.fitToSelection"] {
        let mut doc = drawing();
        doc.execute_as(command, json!({}), &Origin::person("user:test")).unwrap();
        let board = doc.inspect().unwrap()["artboards"][0].clone();
        assert_eq!((board["x"].as_f64(), board["width"].as_f64()), (Some(9.5), Some(21.0)), "{command}: the first artboard fits the art: {board}");
    }
}
