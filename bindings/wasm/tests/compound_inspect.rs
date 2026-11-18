//! Regression: `inspect` reports a compound path's paint (it is the
//! compound's, which every member draws with) on the compound and on each member.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

use kition_kit::KitDocument;
use kition_vector_kit::VectorDocument;
use serde_json::{Value, json};

fn run(doc: &mut VectorDocument, command: &str, params: Value) -> Value {
    doc.execute(command, params).unwrap_or_else(|error| panic!("{command}: {error}"))
}

#[test]
fn a_compound_path_reports_its_paint() {
    let mut doc = VectorDocument::create("ring.kvector", 400, 400).unwrap();
    let outer = run(&mut doc, "shape.ellipse", json!({ "x": 50, "y": 50, "width": 300, "height": 300 }))["id"].clone();
    let inner = run(&mut doc, "shape.ellipse", json!({ "x": 125, "y": 125, "width": 150, "height": 150 }))["id"].clone();
    run(&mut doc, "select.set", json!({ "ids": [outer, inner] }));
    run(&mut doc, "object.compoundPath.make", json!({}));
    run(&mut doc, "paint.setFill", json!({ "color": "#2a9d8f" }));
    run(&mut doc, "paint.setStroke", json!({ "color": "#264653" }));
    let inspected = doc.inspect().unwrap();
    let compound = inspected["layers"][0]["children"][0].clone();
    assert_eq!(compound["kind"], json!("Compound Path"), "{compound}");
    let fill = compound["fill"].as_str().unwrap_or_else(|| panic!("the compound has a fill: {compound}")).to_lowercase();
    assert!(fill.contains("2a9d8f"), "{compound}");
    assert!(compound["stroke"].as_str().is_some_and(|stroke| stroke.to_lowercase().contains("264653")), "{compound}");
    let members = compound["children"].as_array().unwrap();
    assert_eq!(members.len(), 2);
    for member in members {
        assert_eq!(member["fill"].as_str().map(str::to_lowercase), Some(fill.clone()), "a member draws with the compound's fill: {member}");
    }
}
