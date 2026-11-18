//! C8-vector-4 item 8: preferences that change what a write does to the same objects (Scale
//! Strokes & Effects, Scale Corners, Transform Patterns, Use Preview Bounds, Move Locked Artwork
//! with Artboard, the Keyboard Increment) are no premise of a write: they are the editor's, so a
//! command reading one declares `Mode`, the editor state shows them, and a call puts them back.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

use kition_kit::{EditorState, KitDocument};
use kition_vector_kit::VectorDocument;
use serde_json::json;

/// The commands whose handlers read one of them when a parameter leaves it out (read from the
/// handlers and the helpers they call: `scaling`, `transform_patterns`, `artboard_art`, …).
const READERS: &[&str] = &[
    "artboard.copy",
    "artboard.cut",
    "artboard.duplicate",
    "artboard.move",
    "artboard.setProps",
    "edit.copy",
    "edit.cut",
    "graph.create",
    "graph.setData",
    "graph.setType",
    "links.embed",
    "links.unembed",
    "object.align",
    "object.createObjectMosaic",
    "object.distribute",
    "object.distributeSpacing",
    "object.move",
    "object.nudge",
    "object.reflect",
    "object.rotate",
    "object.scale",
    "object.setBounds",
    "object.shear",
    "object.transform",
    "object.transformAgain",
    "object.transformEach",
    "path.lineCut",
    "path.mirrorCut",
    "path.rectCut",
    "stroke.widthPoint.set",
];

#[test]
fn a_command_reading_a_preference_declares_the_mode() {
    let commands = VectorDocument::create("Prefs.kvector", 100, 100).unwrap().commands();
    let missing: Vec<&str> = READERS
        .iter()
        .copied()
        .filter(|id| !commands.iter().find(|command| command.id == *id).unwrap_or_else(|| panic!("no {id}")).reads.contains(&EditorState::Mode))
        .collect();
    assert!(missing.is_empty(), "read a preference without declaring Mode: {missing:?}");
}

#[test]
fn the_editor_state_shows_the_preferences_and_a_call_puts_them_back() {
    let mut doc = VectorDocument::create("Prefs.kvector", 100, 100).unwrap();
    let before = doc.editor_state();
    assert_eq!(before["preferences"]["scaleStrokes"], json!(false), "{before}");
    doc.execute("prefs.set", json!({"key":"scaleStrokes","value":true})).unwrap();
    assert_eq!(doc.editor_state()["preferences"]["scaleStrokes"], json!(true));
    doc.restore_editor_state(&before).unwrap();
    assert_eq!(doc.editor_state(), before);
}

/// C8-vector-5 item 5: the interface language decides how new type and a new text style made
/// from nothing align (em box in Japanese). The macro-made `charStyle.new` and `paraStyle.new`
/// read it without declaring `Mode`, and the editor state did not hold it, so a call that set it
/// left it set.
#[test]
fn the_interface_language_is_a_mode_and_a_call_puts_it_back() {
    let mut doc = VectorDocument::create("Prefs.kvector", 100, 100).unwrap();
    let commands = doc.commands();
    for id in ["charStyle.new", "paraStyle.new", "text.create", "text.createInPath"] {
        let command = commands.iter().find(|command| command.id == id).unwrap_or_else(|| panic!("no {id}"));
        assert!(command.reads.contains(&EditorState::Mode), "{id} reads the interface language without declaring Mode");
    }
    let before = doc.editor_state();
    doc.execute("prefs.set", json!({"key":"interfaceLanguage","value":"ja"})).unwrap();
    assert_ne!(doc.editor_state(), before, "the editor state shows the interface language");
    doc.restore_editor_state(&before).unwrap();
    assert_eq!(doc.editor_state(), before);
}
