//! Commands act on the objects and with the parameters they are given (C8-vector-engine, group B):
//! an id names that object and no other, every parameter given is used or refused, and nothing
//! the caller didn't name is overwritten or lost.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use kition_vector_color::Paint;
use kition_vector_doc::{Document, Node, NodeId, NodeKind};
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

fn doc(s: &Session) -> &Document {
    &s.active().unwrap().doc
}

fn node(s: &Session, n: NodeId) -> Node {
    doc(s).node(n).unwrap().clone()
}

fn rect(s: &mut Session, x: f64, y: f64, w: f64, h: f64) -> NodeId {
    id(&s.execute("shape.rectangle", &json!({"x": x, "y": y, "width": w, "height": h})).unwrap())
}

fn select(s: &mut Session, ids: &[NodeId]) {
    s.execute("select.set", &json!({"ids": ids.iter().map(|i| i.0).collect::<Vec<_>>()})).unwrap();
}

fn left(s: &Session, n: NodeId) -> f64 {
    doc(s).node(n).unwrap().geometric_bounds().unwrap().x0
}

/// A group `G` of two rectangles `C1`, `C2` (nothing selected afterwards).
fn group(s: &mut Session) -> (NodeId, NodeId, NodeId) {
    let c1 = rect(s, 0.0, 0.0, 10.0, 10.0);
    let c2 = rect(s, 20.0, 0.0, 10.0, 10.0);
    select(s, &[c1, c2]);
    let g = id(&s.execute("object.group", &json!({})).unwrap());
    select(s, &[]);
    (g, c1, c2)
}

/// A compound path `CP` of two rectangles `K1`, `K2` with a 10 pt stroke.
fn compound(s: &mut Session) -> (NodeId, NodeId, NodeId) {
    let k1 = rect(s, 100.0, 100.0, 50.0, 50.0);
    let k2 = rect(s, 200.0, 100.0, 50.0, 50.0);
    select(s, &[k1, k2]);
    s.execute("object.compoundPath.make", &json!({})).unwrap();
    let cp = doc(s).parent_of(k1).unwrap();
    s.execute("stroke.set", &json!({"ids": [cp.0], "weight": 10})).unwrap();
    (cp, k1, k2)
}

fn layer(s: &Session) -> NodeId {
    doc(s).layers.first().unwrap().id
}

// ---------- type.size.increase / decrease ----------

fn point_text(s: &mut Session, x: f64, text: &str) -> NodeId {
    id(&s.execute("text.create", &json!({"x": x, "y": 100, "size": 12, "text": text})).unwrap())
}

fn sizes(s: &Session, t: NodeId) -> Vec<f64> {
    let NodeKind::Text(t) = &node(s, t).kind else { panic!("not text") };
    t.runs.iter().map(|r| r.style.size).collect()
}

#[test]
fn type_size_steps_steps_the_text_it_is_given() {
    let mut s = session();
    let t = point_text(&mut s, 0.0, "selected");
    let ta = point_text(&mut s, 300.0, "named");
    select(&mut s, &[t]);
    s.execute("type.size.increase", &json!({"ids": [ta.0]})).unwrap();
    assert_eq!(sizes(&s, t), [12.0], "the selected text is not the one named");
    assert!(sizes(&s, ta).iter().all(|z| *z > 12.0), "the named text steps up: {:?}", sizes(&s, ta));
    // A range of `id` steps only that range.
    s.execute("type.size.decrease", &json!({"id": t.0, "start": 0, "end": 2})).unwrap();
    let NodeKind::Text(txt) = &node(&s, t).kind else { panic!() };
    let first = &txt.runs[0];
    assert!(first.style.size < 12.0 && txt.runs.iter().any(|r| r.style.size == 12.0), "only characters 0..2 step down: {:?}", sizes(&s, t));
    assert_eq!(first.text.len(), 2);
}

// ---------- paint.sampleColor {stop} ----------

fn set_gradient(s: &mut Session, n: NodeId, colors: &[&str]) {
    let k = (colors.len() - 1) as f64;
    let stops: Vec<Value> = colors.iter().enumerate().map(|(i, c)| json!({"offset": i as f64 / k, "color": c})).collect();
    s.execute("paint.setFill", &json!({"ids": [n.0], "gradient": {"stops": stops}})).unwrap();
}

fn stop_colors(s: &Session, n: NodeId) -> Vec<String> {
    let Paint::Gradient(g) = node(s, n).appearance.fill_paint() else { panic!("no gradient fill") };
    g.gradient.stops.iter().map(|st| st.color.to_hex()).collect()
}

#[test]
fn sample_color_into_a_stop_recolours_each_targets_own_gradient() {
    let mut s = session();
    let r = rect(&mut s, 0.0, 0.0, 50.0, 50.0);
    let e = rect(&mut s, 100.0, 0.0, 50.0, 50.0);
    set_gradient(&mut s, r, &["#ff0000", "#00ff00", "#0000ff"]);
    set_gradient(&mut s, e, &["#000000", "#ffffff"]);
    select(&mut s, &[r]);
    s.execute("paint.sampleColor", &json!({"color": "#ffff00", "stop": 1, "stroke": false, "ids": [e.0]})).unwrap();
    assert_eq!(stop_colors(&s, e), ["#000000", "#ffff00"], "E keeps its own stops");
    assert_eq!(stop_colors(&s, r), ["#ff0000", "#00ff00", "#0000ff"], "R, selected but not named, is untouched");
    // Two targets with different gradients each keep theirs.
    s.execute("paint.sampleColor", &json!({"color": "#00ffff", "stop": 0, "stroke": false, "ids": [r.0, e.0]})).unwrap();
    assert_eq!(stop_colors(&s, r), ["#00ffff", "#00ff00", "#0000ff"]);
    assert_eq!(stop_colors(&s, e), ["#00ffff", "#ffff00"]);
    // A stop a target hasn't fails and changes nothing.
    assert!(s.execute("paint.sampleColor", &json!({"color": "#123456", "stop": 2, "stroke": false, "ids": [r.0, e.0]})).is_err());
    assert_eq!(stop_colors(&s, r), ["#00ffff", "#00ff00", "#0000ff"]);
}

fn fill_hex(s: &Session, n: NodeId) -> Option<String> {
    match node(s, n).appearance.fill_paint() {
        Paint::Solid { color, .. } => Some(color.to_hex()),
        _ => None,
    }
}

#[test]
fn sample_color_uses_id_and_refuses_what_it_does_not_take() {
    let mut s = session();
    let r = rect(&mut s, 0.0, 0.0, 50.0, 50.0);
    let e = rect(&mut s, 100.0, 0.0, 50.0, 50.0);
    select(&mut s, &[r]);
    s.execute("paint.sampleColor", &json!({"color": "#ff0000", "stroke": false, "id": e.0})).unwrap();
    assert_eq!(fill_hex(&s, e), Some("#ff0000".into()), "`id` names E");
    assert_ne!(fill_hex(&s, r), Some("#ff0000".into()), "R is not named");
    // A parameter the command does not take is refused, not dropped.
    assert!(s.execute("paint.sampleColor", &json!({"color": "#00ff00", "stroke": false, "ids": [e.0], "swatch": "Nope"})).is_err());
    assert_eq!(fill_hex(&s, e), Some("#ff0000".into()));
}

// ---------- text.areaOptions ----------

fn area(s: &mut Session, x: f64) -> NodeId {
    id(&s.execute("text.create", &json!({"x": x, "y": 0, "size": 12, "area": {"width": 200, "height": 100}, "text": "Some words to flow."})).unwrap())
}

fn area_of(s: &Session, n: NodeId) -> kition_vector_doc::AreaOptions {
    let NodeKind::Text(t) = &node(s, n).kind else { panic!("not text") };
    t.area.clone()
}

#[test]
fn area_options_change_only_the_options_given_on_each_target() {
    let mut s = session();
    let a = area(&mut s, 0.0);
    let b = area(&mut s, 300.0);
    s.execute("text.areaOptions", &json!({"ids": [b.0], "columns": 3, "inset": 4})).unwrap();
    s.execute("text.areaOptions", &json!({"ids": [a.0, b.0], "gutter": 10})).unwrap();
    let (oa, ob) = (area_of(&s, a), area_of(&s, b));
    assert_eq!((oa.columns, oa.inset, oa.gutter), (1, 0.0, 10.0));
    assert_eq!((ob.columns, ob.inset, ob.gutter), (3, 4.0, 10.0), "B keeps its own columns and inset");
}

// ---------- width points on a compound member ----------

#[test]
fn width_commands_given_a_compound_member_name_the_compound_they_edit() {
    let mut s = session();
    let (cp, k1, k2) = compound(&mut s);
    let r = s.execute("stroke.widthPoint.set", &json!({"id": k1.0, "t": 0.5, "left": 4, "right": 4})).unwrap();
    assert_eq!(r["id"], json!(cp.0), "the result names the compound, whose stroke changed: {r}");
    let r = s.execute("stroke.widthPoint.copy", &json!({"id": k2.0, "index": 1, "t": 0.25})).unwrap();
    assert_eq!(r["id"], json!(cp.0), "{r}");
    let r = s.execute("stroke.widthPoint.remove", &json!({"id": k1.0, "index": 1})).unwrap();
    assert_eq!(r, json!({"id": cp.0}));
    let r = s.execute("stroke.widthProfile.set", &json!({"ids": [k1.0, k2.0], "points": null})).unwrap();
    assert_eq!(r, json!({"ids": [cp.0]}));
}

// ---------- explicit ids reduced to roots ----------

#[test]
fn transform_ids_are_roots_and_must_exist() {
    let mut s = session();
    let (g, c1, c2) = group(&mut s);
    s.execute("object.transform", &json!({"ids": [g.0, c1.0], "matrix": [1, 0, 0, 1, 10, 0]})).unwrap();
    assert_eq!((left(&s, c1), left(&s, c2)), (10.0, 30.0), "C1 moves once, with G");
    for ids in [json!([c2.0, 99_999]), json!([layer(&s).0])] {
        assert!(s.execute("object.transform", &json!({"ids": ids, "matrix": [1, 0, 0, 1, 10, 0]})).is_err(), "{ids}");
        assert_eq!((left(&s, c1), left(&s, c2)), (10.0, 30.0), "{ids}");
    }
}

#[test]
fn distort_ids_are_roots_and_must_exist() {
    let mut s = session();
    let (g, c1, c2) = group(&mut s);
    // A plain shift of the box by 10.
    let corners = json!([[10, 0], [40, 0], [40, 10], [10, 10]]);
    s.execute("object.distort", &json!({"ids": [g.0, c1.0], "corners": corners, "from": [0, 0, 30, 10]})).unwrap();
    assert_eq!((left(&s, c1), left(&s, c2)), (10.0, 30.0), "C1 is warped once, with G");
    assert!(s.execute("object.distort", &json!({"ids": [layer(&s).0], "corners": corners, "from": [0, 0, 30, 10]})).is_err());
    assert_eq!((left(&s, c1), left(&s, c2)), (10.0, 30.0));
}

#[test]
fn pixel_perfect_ids_must_name_objects() {
    let mut s = session();
    let r = rect(&mut s, 0.3, 0.3, 10.0, 10.0);
    for ids in [json!([r.0, 99_999]), json!([layer(&s).0])] {
        assert!(s.execute("object.makePixelPerfect", &json!({"ids": ids})).is_err(), "{ids}");
        assert_eq!(left(&s, r), 0.3, "{ids}");
    }
    s.execute("object.makePixelPerfect", &json!({"ids": [r.0]})).unwrap();
    assert_eq!(left(&s, r), 0.5, "a 1 pt stroke centres on a half pixel");
}

#[test]
fn repeat_and_pattern_ids_are_roots_and_must_exist() {
    let mut s = session();
    let (g, c1, _) = group(&mut s);
    select(&mut s, &[g]);
    let r = id(&s.execute("object.repeat.grid", &json!({"ids": [g.0, c1.0]})).unwrap());
    let NodeKind::Repeat(spec) = &node(&s, r).kind else { panic!("not a repeat") };
    assert_eq!(spec.source.len(), 1, "G alone is the source; C1 goes with it");
    s.execute("edit.undo", &json!({})).unwrap();
    // A compound member stands for its compound.
    let (cp, k1, _) = compound(&mut s);
    let r = id(&s.execute("object.repeat.grid", &json!({"ids": [k1.0]})).unwrap());
    let NodeKind::Repeat(spec) = &node(&s, r).kind else { panic!("not a repeat") };
    assert_eq!(spec.source.iter().map(|n| n.id).collect::<Vec<_>>(), [cp]);
    s.execute("edit.undo", &json!({})).unwrap();
    assert!(s.execute("object.pattern.make", &json!({"ids": [g.0, 99_999], "edit": false})).is_err());
}

#[test]
fn symbol_ids_are_roots() {
    let mut s = session();
    let (g, c1, _) = group(&mut s);
    select(&mut s, &[g]);
    let r = s.execute("symbol.new", &json!({"name": "Mark", "ids": [g.0, c1.0]})).unwrap();
    assert!(doc(&s).node(g).is_none() && doc(&s).node(id(&r)).is_some());
    let art = &doc(&s).symbols.iter().find(|x| x.name == "Mark").unwrap().art;
    let mut paths = 0;
    art.walk(&mut |n| paths += usize::from(matches!(n.kind, NodeKind::Path { .. })));
    assert_eq!(paths, 2, "C1 is in the symbol once");
    assert!(s.execute("symbol.update", &json!({"name": "Mark", "ids": [id(&r).0, 99_999]})).is_err());
}

// ---------- transparency.set ----------

#[test]
fn transparency_set_refuses_what_it_does_not_take() {
    let mut s = session();
    let r = rect(&mut s, 0.0, 0.0, 10.0, 10.0);
    s.execute("object.setProps", &json!({"id": r.0, "name": "Keep"})).unwrap();
    for extra in [json!({"locked": true}), json!({"name": ""}), json!({"visible": false}), json!({"data": {"k": "v"}})] {
        let mut p = json!({"ids": [r.0], "opacity": 50});
        p.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
        assert!(s.execute("transparency.set", &p).is_err(), "{extra}");
        let n = node(&s, r);
        assert_eq!((n.locked, n.visible, n.name.as_deref(), n.opacity), (false, true, Some("Keep"), 1.0), "{extra}");
    }
    s.execute("transparency.set", &json!({"ids": [r.0], "opacity": 50, "isolate": true, "knockout": "on", "knockoutShape": true})).unwrap();
    assert_eq!(node(&s, r).opacity, 0.5);
}

// ---------- Region Paint groups ----------

fn live(s: &mut Session) -> NodeId {
    let a = rect(s, 0.0, 0.0, 100.0, 100.0);
    let b = rect(s, 50.0, 0.0, 100.0, 100.0);
    select(s, &[a, b]);
    id(&s.execute("livePaint.make", &json!({})).unwrap())
}

#[test]
fn region_paint_is_a_mark_on_the_group_not_its_name() {
    let mut s = session();
    let lp = live(&mut s);
    s.execute("object.setProps", &json!({"id": lp.0, "name": "Shapes"})).unwrap();
    assert!(s.execute("livePaint.info", &json!({"group": lp.0})).is_ok(), "a renamed Region Paint group stays one");
    let r = s.execute("livePaint.release", &json!({"id": lp.0})).unwrap();
    assert_eq!(r["ids"].as_array().unwrap().len(), 2, "its two sources come back");

    // A plain group named like one is not one.
    let (g, c1, c2) = group(&mut s);
    s.execute("object.setProps", &json!({"id": g.0, "name": "Region Paint 2"})).unwrap();
    assert!(s.execute("livePaint.release", &json!({"id": g.0})).is_err());
    assert!(doc(&s).node(c1).is_some() && doc(&s).node(c2).is_some(), "its art is kept");
    assert!(s.execute("livePaint.info", &json!({"group": g.0})).is_err());
}

#[test]
fn region_paint_mark_survives_save_and_expand_clears_it() {
    let mut s = session();
    let lp = live(&mut s);
    let saved = serde_json::to_value(doc(&s)).unwrap();
    let reread: Document = serde_json::from_value(saved).unwrap();
    assert!(kition_vector_tools::builder::is_region_paint(reread.node(lp).unwrap()));
    s.execute("livePaint.expand", &json!({"id": lp.0})).unwrap();
    assert!(!kition_vector_tools::builder::is_region_paint(&node(&s, lp)));
}

// ---------- the rest of the reading's notes (same kinds) ----------

#[test]
fn envelope_mesh_point_index_must_be_a_point_index() {
    let mut s = session();
    let r = rect(&mut s, 0.0, 0.0, 100.0, 100.0);
    select(&mut s, &[r]);
    let env = id(&s.execute("object.envelope.makeWithMesh", &json!({"rows": 2, "cols": 2})).unwrap());
    let before = node(&s, env);
    for index in [json!(-1), json!(0.5), json!("0")] {
        assert!(s.execute("object.envelope.setMeshPoint", &json!({"id": env.0, "index": index, "x": 5, "y": 5})).is_err(), "{index}");
        assert_eq!(node(&s, env).kind, before.kind, "{index}: mesh point 0 is not the one named");
    }
}

#[test]
fn text_wrap_options_change_only_wrap_objects_and_only_what_is_given() {
    let mut s = session();
    let a = rect(&mut s, 0.0, 0.0, 10.0, 10.0);
    let b = rect(&mut s, 20.0, 0.0, 10.0, 10.0);
    let plain = rect(&mut s, 40.0, 0.0, 10.0, 10.0);
    select(&mut s, &[a, b, plain]);
    s.execute("object.textWrap.make", &json!({"ids": [a.0, b.0], "offset": 6})).unwrap();
    s.execute("object.textWrap.options", &json!({"ids": [b.0], "invert": true})).unwrap();
    s.execute("object.textWrap.options", &json!({"ids": [a.0, b.0, plain.0], "offset": 12})).unwrap();
    let wrap = |n: NodeId| node(&s, n).wrap.map(|w| (w.offset, w.invert));
    assert_eq!(wrap(a), Some((12.0, false)));
    assert_eq!(wrap(b), Some((12.0, true)), "B keeps its own invert");
    assert_eq!(wrap(plain), None, "an object that doesn't wrap is not made to");
    assert!(s.execute("object.textWrap.options", &json!({"ids": [plain.0], "offset": 3})).is_err());
}

#[test]
fn reflect_and_shear_refuse_an_axis_they_do_not_know() {
    let mut s = session();
    let r = rect(&mut s, 0.0, 0.0, 10.0, 20.0);
    select(&mut s, &[r]);
    let before = node(&s, r).kind;
    for (cmd, axis) in [("object.reflect", "Horizontal"), ("object.reflect", "diagonal"), ("object.shear", "diagonal")] {
        assert!(s.execute(cmd, &json!({"axis": axis, "angle": 30})).is_err(), "{cmd} {axis}");
        assert_eq!(node(&s, r).kind, before, "{cmd} {axis}");
    }
    s.execute("object.reflect", &json!({"axis": "vertical"})).unwrap();
    s.execute("object.shear", &json!({"axis": "vertical", "angle": 10})).unwrap();
}

#[test]
fn errors_name_the_command_that_failed() {
    let mut s = session();
    let err = s.execute("edit.redo", &json!({"scope": "nobody"})).unwrap_err();
    assert!(err.to_string().contains("edit.redo"), "{err}");
    let r = rect(&mut s, 0.0, 0.0, 10.0, 10.0);
    select(&mut s, &[r]);
    let err = s.execute("text.tabs.clear", &json!({"start": "x"})).unwrap_err();
    assert!(err.to_string().contains("text.tabs.clear"), "{err}");
}

#[test]
fn char_style_apply_uses_the_range_it_is_given_and_refuses_non_text() {
    let mut s = session();
    s.execute("charStyle.new", &json!({"name": "Big", "attrs": {"size": 30.0}})).unwrap();
    let t = point_text(&mut s, 0.0, "styled");
    let r = rect(&mut s, 0.0, 200.0, 10.0, 10.0);
    select(&mut s, &[t]);
    assert!(s.execute("charStyle.apply", &json!({"name": "Big", "ids": [t.0], "start": 0, "end": 2})).is_err(), "a range goes with `id`");
    assert_eq!(sizes(&s, t), [12.0]);
    assert!(s.execute("charStyle.apply", &json!({"name": "Big", "id": r.0})).is_err(), "a rectangle has no characters");
    s.execute("charStyle.apply", &json!({"name": "Big", "id": t.0, "start": 0, "end": 2})).unwrap();
    assert_eq!(sizes(&s, t), [30.0, 12.0]);
}

#[test]
fn threading_a_path_keeps_the_object_it_was() {
    let mut s = session();
    let a = area(&mut s, 0.0);
    let p = rect(&mut s, 300.0, 0.0, 200.0, 100.0);
    s.execute("object.setProps", &json!({"id": p.0, "name": "Second frame", "opacity": 50})).unwrap();
    select(&mut s, &[a, p]);
    s.execute("text.thread.create", &json!({})).unwrap();
    let n = node(&s, p);
    assert!(matches!(n.kind, NodeKind::Text(_)));
    assert_eq!((n.name.as_deref(), n.opacity), (Some("Second frame"), 0.5));
}

#[test]
fn type_in_a_path_refuses_compound_members_and_clipping_paths() {
    let mut s = session();
    let (cp, k1, _) = compound(&mut s);
    assert!(s.execute("text.createInPath", &json!({"path": k1.0, "text": "x"})).is_err());
    assert_eq!(doc(&s).parent_of(k1), Some(cp));
    let art = rect(&mut s, 0.0, 300.0, 50.0, 50.0);
    let clip = rect(&mut s, 10.0, 310.0, 20.0, 20.0);
    select(&mut s, &[art, clip]);
    s.execute("object.clippingMask.make", &json!({})).unwrap();
    assert!(s.execute("text.createInPath", &json!({"path": clip.0, "text": "x"})).is_err());
    assert!(doc(&s).node(clip).is_some());
}

#[test]
fn opacity_mask_ids_are_roots() {
    let mut s = session();
    let e = rect(&mut s, 90.0, 90.0, 200.0, 100.0);
    let (cp, k1, k2) = compound(&mut s);
    select(&mut s, &[e]);
    s.execute("transparency.makeOpacityMask", &json!({"ids": [e.0, k1.0]})).unwrap();
    assert!(node(&s, e).mask.is_some());
    assert!(doc(&s).node(cp).is_none(), "the compound, not its member, is the mask");
    assert!(doc(&s).node(k2).is_none());
}

#[test]
fn pattern_rename_refuses_a_swatch_name() {
    let mut s = session();
    let r = rect(&mut s, 0.0, 0.0, 10.0, 10.0);
    select(&mut s, &[r]);
    let name = s.execute("object.pattern.make", &json!({"edit": false})).unwrap()["name"].as_str().unwrap().to_string();
    s.execute("swatch.new", &json!({"name": "Brand", "color": "#336699"})).unwrap();
    assert!(s.execute("pattern.options", &json!({"name": name, "newName": "Brand"})).is_err());
    assert_eq!(doc(&s).swatches.iter().filter(|w| w.name == "Brand").count(), 1);
}

#[test]
fn symbol_adjust_refuses_a_style_that_does_not_exist() {
    let mut s = session();
    let err = s.execute("symbol.adjust", &json!({"tool": "style", "style": "Nope", "points": [[0, 0]]})).unwrap_err();
    assert!(err.to_string().contains("Nope"), "{err}");
}

#[test]
fn graphic_style_from_a_library_applies_to_the_id_given() {
    let mut s = session();
    let r = rect(&mut s, 0.0, 0.0, 10.0, 10.0);
    select(&mut s, &[]);
    let out = s.execute("graphicStyle.addFromLibrary", &json!({"library": "shadows-glows", "name": "Halo", "apply": true, "id": r.0})).unwrap();
    assert_eq!(out["applied"], json!("Halo"), "{out}");
    assert!(node(&s, r).graphic_style.is_some());
}

#[test]
fn graph_commands_refuse_an_id_that_is_no_graph() {
    let mut s = session();
    let g = id(&s.execute("graph.create", &json!({"type": "column", "x": 0, "y": 0, "width": 100, "height": 100, "rows": [[1, 2]]})).unwrap());
    let r = rect(&mut s, 300.0, 0.0, 10.0, 10.0);
    select(&mut s, &[g]);
    let before = node(&s, g).graph.clone();
    assert!(s.execute("graph.setData", &json!({"id": r.0, "rows": [[9]]})).is_err());
    assert!(s.execute("graph.setType", &json!({"id": r.0, "type": "pie"})).is_err());
    assert_eq!(node(&s, g).graph, before, "the selected graph is not the one named");
}
