//! Menu rows that cannot run for what is selected are off instead of failing (plan 2026-11-25,
//! stage 4): each predicate is its handler's own check, so a row is on exactly when its run would
//! take the selection, and a call naming its objects is judged with those objects selected.

use serde_json::{Value, json};

use super::*;

fn session() -> Session {
    let mut s = Session::new();
    s.execute("file.new", &json!({"width": 800, "height": 600})).unwrap();
    s
}

fn id_of(v: &Value) -> u64 {
    v["id"].as_u64().unwrap()
}

/// Whether the menu offers `id` now (its `enabled` predicate).
fn offered(s: &Session, id: &str) -> bool {
    (find_command(id).unwrap().enabled)(s).is_ok()
}

#[test]
fn rows_needing_objects_of_a_kind_are_off_without_them() {
    let mut s = session();
    let a = id_of(&s.execute("shape.rectangle", &json!({"x": 10, "y": 10, "width": 40, "height": 40})).unwrap());
    let b = id_of(&s.execute("shape.ellipse", &json!({"x": 80, "y": 10, "width": 40, "height": 40})).unwrap());
    let t = id_of(&s.execute("text.add", &json!({"x": 10, "y": 200, "text": "Kition"})).unwrap());
    s.execute("edit.deselectAll", &json!({})).unwrap();
    for id in [
        "path.reverse",
        "object.blend.make",
        "text.convertToOutlines",
        "text.orientation.vertical",
        "select.same.fontSize",
        "object.clippingMask.editContents",
        "object.expandShape",
        "view.guides.make",
        "livePaint.release",
        "imageTrace.release",
        "shaper.release",
        "object.textWrap.release",
        "object.transformAgain",
        "asset.add",
        "edit.colors.blendHorizontally",
    ] {
        assert!(!offered(&s, id), "{id} is off with nothing selected");
    }
    // A rectangle: paths, shapes and guides are offered; text, blends and clip groups are not.
    s.execute("select.set", &json!({"ids": [a]})).unwrap();
    for id in ["path.reverse", "object.expandShape", "view.guides.make", "asset.add"] {
        assert!(offered(&s, id), "{id} is offered with a rectangle selected");
    }
    for id in [
        "object.blend.make",
        "text.convertToOutlines",
        "object.clippingMask.editMask",
        "livePaint.expand",
        "object.transformAgain",
        "edit.colors.blendVertically",
    ] {
        assert!(!offered(&s, id), "{id} is off with one rectangle selected");
    }
    // Text: the type rows come on.
    s.execute("select.set", &json!({"ids": [t]})).unwrap();
    for id in ["text.convertToOutlines", "text.orientation.horizontal", "select.same.fontFamily", "text.convertToAreaType"] {
        assert!(offered(&s, id), "{id} is offered with text selected");
    }
    // Two objects blend; a move makes Transform Again available.
    s.execute("select.set", &json!({"ids": [a, b]})).unwrap();
    assert!(offered(&s, "object.blend.make"));
    assert!(!offered(&s, "object.transformAgain"));
    s.execute("object.move", &json!({"dx": 5, "dy": 0})).unwrap();
    assert!(offered(&s, "object.transformAgain"));
}

#[test]
fn an_off_row_refuses_with_its_handlers_words_and_a_named_call_still_runs() {
    let mut s = session();
    let a = id_of(&s.execute("shape.rectangle", &json!({"x": 10, "y": 10, "width": 40, "height": 40})).unwrap());
    let b = id_of(&s.execute("shape.ellipse", &json!({"x": 80, "y": 10, "width": 40, "height": 40})).unwrap());
    s.execute("edit.deselectAll", &json!({})).unwrap();
    let error = s.execute("text.convertToOutlines", &json!({})).unwrap_err().to_string();
    assert!(error.contains("not available right now") && error.contains("select text objects"), "{error}");
    let error = s.execute("object.blend.make", &json!({})).unwrap_err().to_string();
    assert!(error.contains("select at least two objects"), "{error}");
    // Named objects stand for the selection: the blend is made with nothing selected.
    s.execute("object.blend.make", &json!({"ids": [a, b]})).unwrap();
    let mut blends = 0;
    s.doc().unwrap().doc.walk(|n| blends += usize::from(matches!(n.kind, kition_vector_doc::NodeKind::Blend { .. })));
    assert_eq!(blends, 1);
}
