//! `document.text` (KitText 1.0): the words of a drawing's type objects, artboard by artboard in
//! reading order, within the limit, with parts inside the text, changing nothing
//! (`kition_kit::testing::check_text`).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;

use kition_kit::KitDocument;
use kition_kit::testing::check_text;
use kition_kit::text::{DEFAULT_LIMIT, DocumentText, TextPart};
use kition_vector_kit::VectorDocument;
use serde_json::json;

fn repo() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
}

fn open(path: &str) -> VectorDocument {
    let bytes = std::fs::read(repo().join(path)).unwrap_or_else(|error| panic!("{path}: {error}"));
    VectorDocument::open(path, &bytes).unwrap()
}

fn part(label: &str, start: usize, end: usize) -> TextPart {
    TextPart { label: label.into(), start, end }
}

const DUSK: &str = "DUSK\nMade in Kition Vector — vectors, pure Rust";

#[test]
fn an_svg_answers_its_text_top_to_bottom() {
    let text = check_text(&open("examples/dusk-poster.svg")).unwrap();
    assert_eq!((text.text.as_str(), text.truncated), (DUSK, false));
    assert_eq!(text.parts.len(), 1, "{text:?}");
    assert_eq!((text.parts[0].start, text.parts[0].end), (0, DUSK.chars().count()));
}

#[test]
fn the_limit_truncates_by_characters() {
    let drawing = open("examples/dusk-poster.svg");
    let cut = drawing.text(30).unwrap();
    assert_eq!((cut.text, cut.truncated), (DUSK.chars().take(30).collect::<String>(), true));
    assert_eq!(cut.parts[0].end, 30);
    let exact = drawing.text(DUSK.chars().count()).unwrap();
    assert_eq!((exact.text.as_str(), exact.truncated), (DUSK, false));
}

#[test]
fn every_example_passes_the_text_check() {
    for entry in std::fs::read_dir(repo().join("examples")).unwrap() {
        let name = entry.unwrap().file_name().to_string_lossy().into_owned();
        check_text(&open(&format!("examples/{name}"))).unwrap_or_else(|error| panic!("{name}: {error}"));
    }
}

#[test]
fn artboards_are_parts_read_top_to_bottom_then_left_to_right() {
    let mut drawing = VectorDocument::create("untitled.kvector", 400, 400).unwrap();
    drawing.execute("artboard.new", json!({ "x": 1000, "y": 0, "width": 400, "height": 400, "name": "Back" })).unwrap();
    for (x, y, text) in [(1100, 100, "back"), (200, 300, "last"), (200, 100, "right"), (20, 100, "left"), (20, 50, "title")] {
        drawing.execute("text.add", json!({ "x": x, "y": y, "text": text })).unwrap();
    }
    drawing.execute("text.add", json!({ "x": 3000, "y": 50, "text": "aside" })).unwrap();
    drawing.execute("text.add", json!({ "x": 20, "y": 200, "text": "hidden" })).unwrap();
    drawing.execute("object.hide", json!({})).unwrap();
    let text = check_text(&drawing).unwrap();
    assert_eq!(text.text, "title\nleft\nright\nlast\nback\naside", "{text:?}");
    assert_eq!(text.parts.len(), 3, "{text:?}");
    assert_eq!(text.parts[1..], [part("Back", 22, 26), part("Pasteboard", 27, 32)]);
}

#[test]
fn reading_changes_nothing() {
    let drawing = open("examples/dusk-poster.kvector");
    let (revision, dirty, bytes) = (drawing.revision(), drawing.dirty(), drawing.save(None).unwrap());
    let state = drawing.inspect().unwrap();
    drawing.text(DEFAULT_LIMIT).unwrap();
    drawing.text(3).unwrap();
    assert_eq!((drawing.revision(), drawing.dirty()), (revision, dirty));
    assert_eq!(drawing.save(None).unwrap(), bytes);
    assert_eq!(drawing.inspect().unwrap(), state);
}

#[test]
fn an_empty_drawing_has_no_text() {
    let drawing = VectorDocument::create("untitled.kvector", 1920, 1080).unwrap();
    assert_eq!(check_text(&drawing).unwrap(), DocumentText::default());
}
