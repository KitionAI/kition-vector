//! regressions/issue-layer-rows: `inspect` reports the Layers panel's highlighted rows. It read them
//! from a scratch session that never copied them, so `layerRows` was always empty, whatever the
//! panel clicked (plan 2026-11-25, stage 4).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

use kition_kit::KitDocument;
use kition_vector_kit::VectorDocument;
use serde_json::{Value, json};

fn run(doc: &mut VectorDocument, command: &str, params: Value) -> Value {
    doc.execute(command, params).unwrap_or_else(|error| panic!("{command}: {error}"))
}

#[test]
fn inspect_reports_the_highlighted_rows() {
    let mut doc = VectorDocument::create("rows.kvector", 400, 300).unwrap();
    let shape = run(&mut doc, "shape.rectangle", json!({ "x": 10, "y": 10, "width": 40, "height": 40 }))["id"].clone();
    let top = run(&mut doc, "layer.new", json!({}))["id"].clone();
    run(&mut doc, "layer.setCurrent", json!({ "id": shape }));
    let inspected = doc.inspect().unwrap();
    assert_eq!(inspected["layerRows"], json!([shape]));
    run(&mut doc, "layer.highlight", json!({ "ids": [top], "mode": "add" }));
    let inspected = doc.inspect().unwrap();
    assert_eq!(inspected["layerRows"], json!([shape, top]));
}
