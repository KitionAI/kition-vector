//! Fonts the editor gives the engine (plan 2026-11-25, stage 5, feature vector.editor.type): a
//! served family picked in the font menu or a font file loaded (`add_face`) joins the families
//! `text.fontList` names, and the drawing's type is laid out again in it; text with characters no
//! face has asks for more fonts (`wants_fonts`, the editor's lazy Chinese face) until one has them.
//! A test binary of its own: fonts are the process's.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

use std::path::PathBuf;

use kition_kit::KitDocument;
use kition_vector_kit::VectorDocument;
use serde_json::{Value, json};

fn run(doc: &mut VectorDocument, command: &str, params: Value) -> Value {
    doc.execute(command, params).unwrap_or_else(|error| panic!("{command}: {error}"))
}

fn kition_fonts() -> PathBuf {
    std::env::var_os("KITION_FONTS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../kition-fonts")))
}

#[test]
fn a_font_given_joins_the_families_and_the_type_is_laid_out_in_it() {
    let mut doc = VectorDocument::create("fonts.kvector", 600, 200).unwrap();
    let id = run(&mut doc, "text.add", json!({ "x": 20, "y": 100, "text": "Serif 你好", "size": 40, "font": "Lora" }))["id"].clone();
    let width = |doc: &mut VectorDocument| run(doc, "file.inspect", json!({}))["layers"][0]["children"][0]["bounds"]["width"].as_f64().unwrap();
    let before = width(&mut doc);
    let steps = run(&mut doc, "file.inspect", json!({}))["history"].as_array().unwrap().len();
    // Laid out with characters no face has: the engine asks for more fonts, once.
    let _ = run(&mut doc, "text.fonts", json!({}));
    assert!(kition_vector_kit::wants_fonts(), "Chinese without a Chinese face wants fonts");
    assert!(!kition_vector_kit::wants_fonts(), "the call clears it");

    let lora = std::fs::read(kition_fonts().join("fonts/lora/Lora-Variable.ttf")).expect("needs the kition-fonts checkout");
    let answer = doc.add_face(&lora);
    assert_eq!(answer["families"], json!(["Lora"]), "{answer}");
    assert!(run(&mut doc, "text.fontList", json!({}))["families"].as_array().unwrap().contains(&json!("Lora")));
    assert_ne!(width(&mut doc), before, "the type is laid out again in Lora");
    assert_eq!(run(&mut doc, "file.inspect", json!({}))["history"].as_array().unwrap().len(), steps, "no history step");
    // The same file again adds no family.
    assert_eq!(doc.add_face(&lora)["families"], json!([]));
    // Bytes that are not a font add nothing.
    assert_eq!(doc.add_face(b"not a font")["faces"], json!(0));

    // The editor's lazy Chinese face: then nothing is missing.
    let droid = std::fs::read(kition_fonts().join("fonts/droid-sans-fallback/DroidSansFallbackFull.ttf")).unwrap();
    assert!(kition_vector_kit::add_font(&droid) > 0);
    assert!(doc.refresh_type());
    let fonts = run(&mut doc, "text.fonts", json!({}));
    assert!(fonts.as_array().unwrap().iter().all(|row| row["noGlyph"] == json!(0)), "{fonts}");
    assert_eq!(id, run(&mut doc, "file.inspect", json!({}))["layers"][0]["children"][0]["id"]);
}
