//! The editor's view (plan 2026-11-25, stage 6, feature vector.editor.view): the View menu's
//! toggles the page sets with `View:` requests are the session's (no revision, no undo step, not
//! dirty) and `inspect()` reports them (`view`); Outline, Pixel Preview and the transparency grid
//! change the pixels; Smart Guides off leaves a dragged object where the pointer puts it; Show
//! Bounding Box and Show Edges hide the selection's overlays; a guide dragged out of a ruler is made
//! where it is released over the canvas, in one undo step.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

use kition_kit::KitDocument;
use kition_kit::tools::{KitTools, PointerAction, ToolMods, ToolPointer, ToolResponse};
use kition_vector_kit::VectorDocument;
use serde_json::{Value, json};

fn run(doc: &mut VectorDocument, command: &str, params: Value) -> Value {
    doc.execute(command, params).unwrap_or_else(|error| panic!("{command}: {error}"))
}

fn view(doc: &VectorDocument) -> Value {
    doc.inspect().unwrap()["view"].clone()
}

fn request(doc: &mut VectorDocument, body: Value) -> ToolResponse {
    doc.key(&format!("View:{body}"), ToolMods::default(), 1.0).unwrap_or_else(|error| panic!("{body}: {error}"))
}

fn origin(doc: &VectorDocument) -> (f64, f64) {
    let origin = doc.inspect().unwrap()["origin"].clone();
    (origin[0].as_f64().unwrap(), origin[1].as_f64().unwrap())
}

/// A pointer event `action` at the drawing's point (`x`, `y`).
fn press(doc: &mut VectorDocument, action: PointerAction, x: f64, y: f64) -> ToolResponse {
    let (ox, oy) = origin(doc);
    doc.pointer(&ToolPointer { action, x: x - ox, y: y - oy, mods: ToolMods::default(), pressure: 1.0, zoom: 1.0 }).unwrap()
}

/// The straight RGBA pixel at the drawing's point (`x`, `y`), the canvas at 1:1.
fn pixel(doc: &VectorDocument, x: f64, y: f64) -> [u8; 4] {
    let (ox, oy) = origin(doc);
    let pixels = doc.render(x - ox, y - oy, 1.0, 1, 1).unwrap();
    [pixels[0], pixels[1], pixels[2], pixels[3]]
}

fn x_of(doc: &VectorDocument, id: &Value) -> f64 {
    let inspected = doc.inspect().unwrap();
    let layers = inspected["layers"].as_array().unwrap();
    let node = layers.iter().flat_map(|layer| layer["children"].as_array().unwrap()).find(|node| node["id"] == *id).unwrap();
    node["bounds"]["x"].as_f64().unwrap()
}

#[test]
fn toggles_are_the_sessions_and_inspect_reports_them() {
    let mut doc = VectorDocument::create("view.kvector", 400, 300).unwrap();
    run(&mut doc, "shape.rectangle", json!({ "x": 20, "y": 20, "width": 100, "height": 80 }));
    let shown = view(&doc);
    assert_eq!((shown["outline"].clone(), shown["smartGuides"].clone(), shown["boundingBox"].clone()), (json!(false), json!(true), json!(true)));
    assert_eq!((shown["grid"].clone(), shown["rulers"].clone(), shown["guidesLocked"].clone()), (json!(false), json!(false), json!(false)));
    assert_eq!(shown["gridLook"]["spacing"], json!(72.0));
    let (revision, dirty, history) = (doc.revision(), doc.dirty(), doc.inspect().unwrap()["history"].clone());
    request(&mut doc, json!({ "set": { "outline": true, "smartGuides": false, "grid": true, "rulers": true, "snapToGrid": true } }));
    let shown = view(&doc);
    assert_eq!(
        [&shown["outline"], &shown["smartGuides"], &shown["grid"], &shown["rulers"], &shown["snapToGrid"]],
        [&json!(true), &json!(false), &json!(true), &json!(true), &json!(true)]
    );
    assert_eq!((doc.revision(), doc.dirty(), doc.inspect().unwrap()["history"].clone()), (revision, dirty, history), "a view toggle is no edit");
    // A toggle it does not know, or a value that is not true or false, sets nothing.
    assert!(doc.key(r#"View:{"set": {"outline": false, "wireframe": true}}"#, ToolMods::default(), 1.0).is_err());
    assert!(doc.key(r#"View:{"set": {"outline": 0}}"#, ToolMods::default(), 1.0).is_err());
    assert_eq!(view(&doc)["outline"], json!(true));
    // The engine's own view commands show as checks too.
    run(&mut doc, "view.guides.lock", json!({ "locked": true }));
    run(&mut doc, "view.transparencyGrid", json!({ "on": true }));
    let shown = view(&doc);
    assert_eq!((shown["guidesLocked"].clone(), shown["transparencyGrid"].clone()), (json!(true), json!(true)));
}

#[test]
fn outline_pixel_preview_and_the_transparency_grid_change_the_pixels() {
    let mut doc = VectorDocument::create("view.kvector", 400, 300).unwrap();
    let id = run(&mut doc, "shape.rectangle", json!({ "x": 20, "y": 20, "width": 100, "height": 80 }))["id"].clone();
    run(&mut doc, "paint.setFill", json!({ "ids": [id], "color": "#ff0000" }));
    run(&mut doc, "edit.deselectAll", json!({}));
    assert_eq!(pixel(&doc, 70.0, 60.0), [255, 0, 0, 255], "the fill, in Preview");
    request(&mut doc, json!({ "set": { "outline": true } }));
    assert_eq!(pixel(&doc, 70.0, 60.0), [255, 255, 255, 255], "no fill in Outline: white");
    let edge = pixel(&doc, 20.0, 60.0);
    assert!(edge[0] < 160 && edge[1] < 160, "the path a thin dark line: {edge:?}");
    request(&mut doc, json!({ "set": { "outline": false } }));
    assert_eq!(pixel(&doc, 70.0, 60.0), [255, 0, 0, 255], "back in Preview");

    // The transparency grid replaces the artboard's white: two colours in cells.
    run(&mut doc, "view.transparencyGrid", json!({ "on": true }));
    let (ox, oy) = origin(&doc);
    let tile = doc.render(200.0 - ox, 150.0 - oy, 1.0, 32, 32).unwrap();
    let mut colours: Vec<[u8; 4]> = tile.chunks(4).map(|px| [px[0], px[1], px[2], px[3]]).collect();
    colours.sort_unstable();
    colours.dedup();
    assert_eq!(colours.len(), 2, "the grid's two colours: {colours:?}");
    assert!(colours.iter().all(|c| c[3] == 255));
    assert_eq!(pixel(&doc, 70.0, 60.0), [255, 0, 0, 255], "the art over it");
    run(&mut doc, "view.transparencyGrid", json!({ "on": false }));

    // Pixel Preview at 800%: each point one 8 × 8 square of its colour at 1:1 (point 19 half
    // under the stroke, point 20 the stroke's other half over the fill).
    let (ox, oy) = origin(&doc);
    let (p19, p20) = (pixel(&doc, 19.0, 50.0), pixel(&doc, 20.0, 50.0));
    assert_ne!(p19, p20);
    request(&mut doc, json!({ "set": { "pixelPreview": true } }));
    let block = doc.render(19.5 - ox, 50.0 - oy, 8.0, 32, 8).unwrap();
    for (i, px) in block.chunks(4).enumerate() {
        let (column, row) = (i % 32, i / 32);
        // Document pixel 19.5 is the left edge: half a point (4 pixels), then whole points.
        let square_start = if column < 4 { 0 } else { 4 + ((column - 4) / 8) * 8 };
        let same = &block[(row * 32 + square_start) * 4..][..4];
        assert_eq!(px, same, "pixel {column},{row} is its point's colour");
    }
    assert_eq!(&block[..4], &p19, "point 19 as at 1:1");
    assert_eq!(&block[4 * 4..4 * 4 + 4], &p20, "point 20 as at 1:1");
}

#[test]
fn smart_guides_off_leave_a_dragged_object_where_the_pointer_puts_it() {
    let dragged = |smart: bool| {
        let mut doc = VectorDocument::create("view.kvector", 600, 400).unwrap();
        let a = run(&mut doc, "shape.rectangle", json!({ "x": 20, "y": 20, "width": 100, "height": 80 }))["id"].clone();
        run(&mut doc, "shape.rectangle", json!({ "x": 220, "y": 220, "width": 100, "height": 80 }));
        run(&mut doc, "select.set", json!({ "ids": [a] }));
        doc.select_tool("selection", 1.0).unwrap();
        request(&mut doc, json!({ "set": { "smartGuides": smart, "snapToPoint": false } }));
        // The right edge from 120 to 217.5, 2.5 short of the other rectangle's left edge.
        press(&mut doc, PointerAction::Down, 70.0, 60.0);
        for step in 1..=10 {
            press(&mut doc, PointerAction::Drag, 70.0 + 9.75 * f64::from(step), 60.0);
        }
        press(&mut doc, PointerAction::Up, 167.5, 60.0);
        x_of(&doc, &a)
    };
    assert!((dragged(true) - 120.0).abs() < 1e-6, "Smart Guides on: the edge snaps to the other's left edge");
    assert!((dragged(false) - 117.5).abs() < 1e-6, "Smart Guides off: where the pointer puts it");
}

#[test]
fn bounding_box_and_edges_hide_the_selections_overlays() {
    let mut doc = VectorDocument::create("view.kvector", 400, 300).unwrap();
    let id = run(&mut doc, "shape.rectangle", json!({ "x": 20, "y": 20, "width": 100, "height": 80 }))["id"].clone();
    run(&mut doc, "select.set", json!({ "ids": [id] }));
    let count = |response: &ToolResponse| response.overlays.len();
    let all = count(&doc.select_tool("selection", 1.0).unwrap());
    let without_box = count(&request(&mut doc, json!({ "set": { "boundingBox": false } })));
    let without_either = count(&request(&mut doc, json!({ "set": { "edges": false } })));
    assert!(all > without_box && without_box > without_either, "{all} > {without_box} > {without_either}");
    assert_eq!(without_either, 0, "nothing of the selection shows");
}

#[test]
fn a_guide_dragged_out_of_a_ruler_is_made_where_it_is_released_over_the_canvas() {
    let mut doc = VectorDocument::create("view.kvector", 400, 300).unwrap();
    let (ox, oy) = origin(&doc);
    let steps = doc.inspect().unwrap()["history"].as_array().unwrap().len();
    let ruler = |doc: &mut VectorDocument, kind: &str, x: f64, on: bool| {
        request(doc, json!({ "ruler": { "vertical": true, "kind": kind, "x": x - ox, "y": 100.0 - oy, "onCanvas": on } }));
    };
    // Over the ruler first (nothing yet), then over the canvas: shown as it goes.
    ruler(&mut doc, "drag", -5.0, false);
    assert_eq!(run(&mut doc, "guide.list", json!({})), json!([]));
    ruler(&mut doc, "drag", 80.0, true);
    ruler(&mut doc, "drag", 150.3, true);
    ruler(&mut doc, "up", 150.3, true);
    let guides = run(&mut doc, "guide.list", json!({}));
    assert_eq!(guides.as_array().unwrap().len(), 1, "{guides}");
    assert_eq!(guides[0]["vertical"], json!(true));
    assert!((guides[0]["pos"].as_f64().unwrap() - 150.0).abs() <= 0.5, "snapped to a whole pixel: {guides}");
    assert_eq!(doc.inspect().unwrap()["history"].as_array().unwrap().len(), steps + 1, "one undo step");
    // Released back over the ruler, none is made.
    ruler(&mut doc, "drag", 60.0, true);
    ruler(&mut doc, "up", -5.0, false);
    assert_eq!(run(&mut doc, "guide.list", json!({})).as_array().unwrap().len(), 1);
}
