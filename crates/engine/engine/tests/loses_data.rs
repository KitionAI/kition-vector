//! C8-vector-engine-2: a command that cannot do all it says does nothing, and an edit of one
//! setting or name leaves the rest of the document as it was (issue-c8-engine2-*).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

use kition_vector_doc::{Document, NodeId};
use kition_vector_engine::Session;
use serde_json::{Value, json};

fn session() -> Session {
    let mut s = Session::new();
    s.execute("file.new", &json!({"width": 800, "height": 600})).unwrap();
    s
}

fn id(v: &Value) -> NodeId {
    NodeId(v["id"].as_u64().unwrap())
}

fn rect(s: &mut Session, x: f64) -> NodeId {
    id(&s.execute("shape.rectangle", &json!({"x": x, "y": 10, "width": 20, "height": 20})).unwrap())
}

fn doc(s: &Session) -> std::sync::Arc<Document> {
    s.doc().unwrap().doc.clone()
}

/// `command` with `params` is refused and leaves the document and its history as they were.
fn refused(s: &mut Session, command: &str, params: Value) -> String {
    let before = doc(s);
    let undo = s.doc().unwrap().history.undo.len();
    let error = s.execute(command, &params).map(|v| format!("accepted: {v}")).unwrap_or_else(|e| e.to_string());
    assert!(!error.starts_with("accepted"), "{command} {params}: {error}");
    assert!(doc(s) == before, "{command} {params} changed the document");
    assert_eq!(s.doc().unwrap().history.undo.len(), undo, "{command} {params} left an undo step");
    error
}

/// Object → Expand of a stroked path whose effect cannot be expanded (its copy would land
/// beyond the canvas) used to outline the stroke and report success with the effect left live.
#[test]
fn expand_with_a_step_that_fails_changes_nothing() {
    let mut s = session();
    let r = rect(&mut s, 10.0);
    s.execute("stroke.set", &json!({"weight": 4})).unwrap();
    s.execute("effect.apply", &json!({"effect": "distort.transform", "params": {"moveH": 5.0e6, "copies": 1}})).unwrap();
    s.execute("select.set", &json!({"ids": [r.0]})).unwrap();
    assert!(s.execute("effect.expandAppearance", &json!({})).is_err(), "the effect alone cannot be expanded");
    refused(&mut s, "object.expand", json!({}));
}

/// Every pattern a paint names, wherever the document holds paints: layer art, symbol
/// definitions, pattern tiles, graphic styles, text runs.
fn patterns_named(s: &Session) -> Vec<String> {
    let mut out = vec![];
    doc(s).visit_paints(&mut |p| {
        if let kition_vector_color::Paint::Pattern { pattern, .. } = p {
            out.push(pattern.clone());
        }
    });
    out
}

/// A pattern "Pat" painting a rectangle, a graphic style, a symbol's art, another pattern's tile
/// and a text object.
fn painted_with_a_pattern() -> Session {
    let mut s = session();
    let tile = rect(&mut s, 10.0);
    s.execute("object.pattern.make", &json!({"ids": [tile.0], "name": "Pat", "edit": false})).unwrap();
    let fill = |s: &mut Session, id: NodeId| {
        s.execute("paint.setFill", &json!({"ids": [id.0], "swatch": "Pat"})).unwrap();
    };
    let styled = rect(&mut s, 100.0);
    fill(&mut s, styled);
    s.execute("graphicStyle.new", &json!({"name": "Patterned", "id": styled.0})).unwrap();
    let in_symbol = rect(&mut s, 200.0);
    fill(&mut s, in_symbol);
    s.execute("symbol.new", &json!({"name": "Sym", "ids": [in_symbol.0]})).unwrap();
    let in_tile = rect(&mut s, 300.0);
    fill(&mut s, in_tile);
    s.execute("object.pattern.make", &json!({"ids": [in_tile.0], "name": "Outer", "edit": false})).unwrap();
    let text = id(&s.execute("text.add", &json!({"x": 10, "y": 200, "text": "Hi"})).unwrap());
    fill(&mut s, text);
    let named = patterns_named(&s);
    assert!(named.iter().filter(|n| *n == "Pat").count() >= 4, "{named:?}");
    s
}

#[test]
fn a_renamed_pattern_is_renamed_wherever_it_paints() {
    let mut s = painted_with_a_pattern();
    s.execute("pattern.options", &json!({"name": "Pat", "newName": "Renamed"})).unwrap();
    let named = patterns_named(&s);
    assert!(!named.iter().any(|n| n == "Pat"), "a paint still names the old pattern: {named:?}");
    let d = doc(&s);
    assert!(d.graphic_styles.iter().any(|g| {
        g.appearance.items.iter().any(|i| matches!(i.paint(), kition_vector_color::Paint::Pattern { pattern, .. } if pattern == "Renamed"))
    }));
    let sym = &d.symbols.iter().find(|x| x.name == "Sym").unwrap().art;
    let mut in_symbol = false;
    sym.walk(&mut |n| {
        in_symbol |=
            n.appearance.items.iter().any(|i| matches!(i.paint(), kition_vector_color::Paint::Pattern { pattern, .. } if pattern == "Renamed"))
    });
    assert!(in_symbol, "the symbol's art follows the rename");
}

#[test]
fn a_deleted_pattern_paints_nowhere() {
    let mut s = painted_with_a_pattern();
    s.execute("pattern.delete", &json!({"name": "Pat"})).unwrap();
    let named = patterns_named(&s);
    assert!(!named.iter().any(|n| n == "Pat"), "a paint still names the deleted pattern: {named:?}");
}

/// Every brush a stroke names: layer art, symbol definitions, pattern tiles, graphic styles.
fn brushes_named(s: &Session) -> Vec<String> {
    let d = doc(s);
    let mut out = vec![];
    let mut items = |items: &[kition_vector_doc::AppearanceItem]| {
        for i in items {
            if let kition_vector_doc::AppearanceItem::Stroke(st) = i
                && let Some(b) = &st.brush
            {
                out.push(b.clone());
            }
        }
    };
    let mut trees: Vec<std::sync::Arc<kition_vector_doc::Node>> = d.layers.clone();
    trees.extend(d.symbols.iter().map(|x| x.art.clone()));
    trees.extend(d.patterns.iter().flat_map(|p| p.art.clone()));
    for t in &trees {
        t.walk(&mut |n| items(&n.appearance.items));
    }
    for g in &d.graphic_styles {
        items(&g.appearance.items);
    }
    out
}

fn brushed() -> Session {
    let mut s = session();
    s.execute("brush.new", &json!({"type": "calligraphic", "name": "Cal"})).unwrap();
    let brush = |s: &mut Session, id: NodeId| {
        s.execute("brush.apply", &json!({"name": "Cal", "ids": [id.0]})).unwrap();
    };
    let styled = rect(&mut s, 100.0);
    brush(&mut s, styled);
    s.execute("graphicStyle.new", &json!({"name": "Brushed", "id": styled.0})).unwrap();
    let in_symbol = rect(&mut s, 200.0);
    brush(&mut s, in_symbol);
    s.execute("symbol.new", &json!({"name": "Sym", "ids": [in_symbol.0]})).unwrap();
    let in_tile = rect(&mut s, 300.0);
    brush(&mut s, in_tile);
    s.execute("object.pattern.make", &json!({"ids": [in_tile.0], "name": "Tile", "edit": false})).unwrap();
    let named = brushes_named(&s);
    assert!(named.iter().filter(|n| *n == "Cal").count() >= 4, "{named:?}");
    s
}

#[test]
fn a_renamed_brush_is_renamed_wherever_it_strokes() {
    let mut s = brushed();
    s.execute("brush.options", &json!({"name": "Cal", "newName": "Renamed"})).unwrap();
    let named = brushes_named(&s);
    assert!(!named.iter().any(|n| n == "Cal"), "a stroke still names the old brush: {named:?}");
    assert!(named.iter().filter(|n| *n == "Renamed").count() >= 4, "{named:?}");
}

#[test]
fn a_deleted_brush_strokes_nowhere() {
    let mut s = brushed();
    s.execute("brush.delete", &json!({"name": "Cal"})).unwrap();
    let named = brushes_named(&s);
    assert!(!named.iter().any(|n| n == "Cal"), "a stroke still names the deleted brush: {named:?}");
}

/// A Lab swatch made spot while the Spot Colors options use CMYK values: the art linked to it
/// shows the CMYK equivalent, as swatch.edit {spot: true} gives.
#[test]
fn set_spot_relinks_the_art() {
    let fill_after = |make_spot: &dyn Fn(&mut Session)| {
        let mut s = session();
        s.execute("swatch.spotOptions", &json!({"useLab": false})).unwrap();
        s.execute("swatch.new", &json!({"name": "L", "color": {"l": 50, "a": 60, "b": 20}, "global": true})).unwrap();
        let r = rect(&mut s, 10.0);
        s.execute("paint.setFill", &json!({"ids": [r.0], "swatch": "L"})).unwrap();
        make_spot(&mut s);
        let d = doc(&s);
        d.node(r).unwrap().appearance.fill_paint().clone()
    };
    let by_edit = fill_after(&|s| {
        s.execute("swatch.edit", &json!({"name": "L", "spot": true})).unwrap();
    });
    let by_set_spot = fill_after(&|s| {
        s.execute("swatch.setSpot", &json!({"name": "L"})).unwrap();
    });
    assert!(matches!(by_edit, kition_vector_color::Paint::Solid { color: kition_vector_color::Color::Cmyk { .. }, .. }), "{by_edit:?}");
    assert_eq!(by_set_spot, by_edit);
}
