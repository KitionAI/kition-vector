//! Regression: a drawing draws only with the kit's bundled fonts and
//! the fonts its host adds, never the machine's, so a render is the same on every machine; text a
//! font here cannot draw is reported (`text.fonts` `noGlyph`, the check's `vector.missing-glyphs`)
//! until the host adds a font that has it. A test binary of its own: fonts are the process's.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

use std::path::PathBuf;

use kition_kit::KitDocument;
use kition_vector_kit::VectorDocument;
use serde_json::{Value, json};

fn run(doc: &mut VectorDocument, command: &str, params: Value) -> Value {
    doc.execute(command, params).unwrap_or_else(|error| panic!("{command}: {error}"))
}

fn codes(doc: &VectorDocument) -> Vec<String> {
    doc.check().into_iter().map(|diagnostic| diagnostic.code).collect()
}

#[test]
fn only_bundled_and_added_fonts_draw() {
    let mut doc = VectorDocument::create("greeting.kvector", 600, 200).unwrap();
    // The machine's fonts are not the drawing's.
    let families = run(&mut doc, "text.fontList", json!({}))["families"].clone();
    for system in ["Helvetica", "Arial", "Times New Roman", "Apple SD Gothic Neo", "PingFang SC", "DejaVu Sans", "Segoe UI"] {
        assert!(!families.as_array().unwrap().iter().any(|family| family == system), "{system} is the machine's: {families}");
    }
    assert_eq!(run(&mut doc, "text.rescanFonts", json!({}))["faces"], json!(0), "a rescan finds no installed fonts");

    // Simplified Chinese no bundled font has: reported, not drawn from the machine.
    run(&mut doc, "text.create", json!({ "x": 20, "y": 100, "text": "Hello 你们开发", "size": 40, "font": "Inter" }));
    let fonts = run(&mut doc, "text.fonts", json!({}));
    let inter = fonts.as_array().unwrap().iter().find(|row| row["family"] == "Inter").cloned().unwrap();
    assert_eq!(inter["noGlyph"], json!(4), "four characters no font here has: {inter}");
    assert!(codes(&doc).contains(&"vector.missing-glyphs".to_owned()), "{:?}", codes(&doc));

    // The host adds a font that has them: drawn, nothing to report.
    let dir = std::env::var_os("KITION_FONTS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../kition-fonts")));
    let droid = std::fs::read(dir.join("fonts/droid-sans-fallback/DroidSansFallbackFull.ttf")).expect("needs the kition-fonts checkout");
    assert!(kition_vector_text::FontDb::global().add_font(droid) > 0);
    let fonts = run(&mut doc, "text.fonts", json!({}));
    let inter = fonts.as_array().unwrap().iter().find(|row| row["family"] == "Inter").cloned().unwrap();
    assert_eq!(inter["noGlyph"], json!(0), "{inter}");
    assert!(!codes(&doc).contains(&"vector.missing-glyphs".to_owned()), "{:?}", codes(&doc));
}
