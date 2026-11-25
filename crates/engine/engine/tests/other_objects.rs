//! A command acts on the objects it is given and on no other (C8-vector-engine-2, "acts on
//! another object"): what it cannot do to a member of what it was given is refused, never done to
//! a neighbour, a container or a hidden part instead.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use kition_vector_doc::{Document, Node, NodeId, NodeKind};
use kition_vector_engine::{Origin, Session};
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

fn exists(s: &Session, n: NodeId) -> bool {
    doc(s).node(n).is_some()
}

fn rect(s: &mut Session, x: f64, y: f64, w: f64, h: f64) -> NodeId {
    id(&s.execute("shape.rectangle", &json!({"x": x, "y": y, "width": w, "height": h})).unwrap())
}

fn ellipse(s: &mut Session, x: f64, y: f64, w: f64, h: f64) -> NodeId {
    id(&s.execute("shape.ellipse", &json!({"x": x, "y": y, "width": w, "height": h})).unwrap())
}

fn select(s: &mut Session, ids: &[NodeId]) {
    s.execute("select.set", &json!({"ids": ids.iter().map(|i| i.0).collect::<Vec<_>>()})).unwrap();
}

fn point_text(s: &mut Session, x: f64, text: &str) -> NodeId {
    id(&s.execute("text.add", &json!({"x": x, "y": 100, "size": 12, "text": text})).unwrap())
}

fn group_of(s: &mut Session, ids: &[NodeId]) -> NodeId {
    select(s, ids);
    let g = id(&s.execute("arrange.group", &json!({})).unwrap());
    select(s, &[]);
    g
}

/// A clip group `CG` of an ellipse `CA` clipped by a rectangle `CK`: (CG, CA, CK).
fn clip_group(s: &mut Session) -> (NodeId, NodeId, NodeId) {
    let ca = ellipse(s, 300.0, 300.0, 60.0, 60.0);
    let ck = rect(s, 310.0, 310.0, 30.0, 30.0);
    select(s, &[ca, ck]);
    s.execute("object.clippingMask.make", &json!({})).unwrap();
    let cg = doc(s).parent_of(ca).unwrap();
    select(s, &[]);
    (cg, ca, ck)
}

fn is_clipping(n: &Node) -> bool {
    matches!(n.kind, NodeKind::Path { clipping: true, .. })
}

fn blend_of(s: &mut Session) -> (NodeId, NodeId, NodeId) {
    let b1 = ellipse(s, 0.0, 400.0, 20.0, 20.0);
    let b2 = ellipse(s, 200.0, 400.0, 20.0, 20.0);
    let b = id(&s.execute("object.blend.make", &json!({"ids": [b1.0, b2.0], "steps": 3})).unwrap());
    select(s, &[]);
    (b, b1, b2)
}

fn compound(s: &mut Session) -> (NodeId, NodeId, NodeId) {
    let k1 = rect(s, 100.0, 100.0, 50.0, 50.0);
    let k2 = rect(s, 200.0, 100.0, 50.0, 50.0);
    select(s, &[k1, k2]);
    s.execute("object.compoundPath.make", &json!({})).unwrap();
    let cp = doc(s).parent_of(k1).unwrap();
    select(s, &[]);
    (cp, k1, k2)
}

fn path_of(s: &Session, n: NodeId) -> kition_vector_geom::PathData {
    node(s, n).path_data().unwrap().clone()
}

fn layer(s: &Session) -> NodeId {
    doc(s).layers.first().unwrap().id
}

fn agent() -> Origin {
    Origin::agent("agent:test")
}

// ---------- Image Trace ----------

fn image(s: &mut Session, x: f64) -> NodeId {
    use kition_vector_doc::{ImageBlob, ImageObject};
    use kition_vector_geom::Affine;
    let r = kition_vector_trace::Raster::from_fn(20, 20, |x, y| if (x / 5 + y / 5) % 2 == 0 { [0, 0, 0, 255] } else { [255, 255, 255, 255] });
    let png = r.encode_png();
    let key = format!("img{x}");
    s.edit("img", |d, sel| {
        d.images.insert(key.clone(), ImageBlob::new("image/png", png));
        let id = d.alloc_id();
        let l = d.default_layer();
        let xf = Affine::translate((x, 0.0));
        d.insert(
            l,
            usize::MAX,
            Node::new(id, NodeKind::Image(ImageObject { key, width: 20, height: 20, xf, link: None, placement: Default::default() })),
        )?;
        sel.set([id]);
        Ok(id)
    })
    .unwrap()
}

// ---------- object.compoundPath.make ----------

#[test]
fn compound_path_make_refuses_a_group_holding_text() {
    let mut s = session();
    let t = point_text(&mut s, 0.0, "Hello");
    let r = rect(&mut s, 0.0, 200.0, 20.0, 20.0);
    let g2 = group_of(&mut s, &[t, r]);
    let e = ellipse(&mut s, 100.0, 200.0, 20.0, 20.0);
    select(&mut s, &[g2, e]);
    let err = s.execute("object.compoundPath.make", &json!({}));
    assert!(err.is_err(), "a group holding text is refused, not emptied: {err:?}");
    assert!(exists(&s, t) && exists(&s, g2) && exists(&s, e), "nothing is deleted");
}

#[test]
fn compound_path_make_frees_a_clipping_path() {
    let mut s = session();
    let (cg, _ca, ck) = clip_group(&mut s);
    let r2 = rect(&mut s, 500.0, 0.0, 20.0, 20.0);
    select(&mut s, &[cg, r2]);
    let cp = id(&s.execute("object.compoundPath.make", &json!({})).unwrap());
    let members = node(&s, cp).children().unwrap().clone();
    assert!(members.iter().any(|m| m.id == ck), "CK is a member");
    assert!(members.iter().all(|m| !is_clipping(m)), "no member stays flagged as a clipping path");
}

// ---------- object.path.divideObjectsBelow ----------

#[test]
fn divide_objects_below_leaves_blend_keys_and_envelope_content_alone() {
    let mut s = session();
    let (_b, b1, _b2) = blend_of(&mut s);
    let before = path_of(&s, b1);
    let cutter = rect(&mut s, -10.0, 405.0, 50.0, 5.0);
    select(&mut s, &[cutter]);
    s.execute("object.path.divideObjectsBelow", &json!({})).unwrap();
    assert!(exists(&s, b1), "the blend key is not replaced by pieces");
    assert_eq!(path_of(&s, b1), before, "the blend key keeps its shape");
}

// ---------- object.path.simplify / addAnchorPoints ----------

#[test]
fn add_anchor_points_on_a_clip_group_leaves_the_clipping_path_and_locked_members_alone() {
    let mut s = session();
    let (cg, ca, ck) = clip_group(&mut s);
    let ck_before = path_of(&s, ck);
    let ca_before = path_of(&s, ca).anchor_count();
    s.execute("object.setProps", &json!({"ids": [ca.0], "locked": true})).unwrap();
    let r2 = rect(&mut s, 500.0, 500.0, 20.0, 20.0);
    select(&mut s, &[cg, r2]);
    s.execute("object.path.addAnchorPoints", &json!({})).unwrap();
    assert_eq!(path_of(&s, ck), ck_before, "the clipping path is not edited");
    assert_eq!(path_of(&s, ca).anchor_count(), ca_before, "a locked member is not edited");
    assert!(path_of(&s, r2).anchor_count() > 4, "the rectangle is");
}

#[test]
fn add_anchor_points_on_a_group_holding_a_blend_leaves_the_blend_keys_alone() {
    let mut s = session();
    let (b, b1, _) = blend_of(&mut s);
    let r = rect(&mut s, 500.0, 500.0, 20.0, 20.0);
    let g = group_of(&mut s, &[b, r]);
    let before = path_of(&s, b1);
    select(&mut s, &[g]);
    s.execute("object.path.addAnchorPoints", &json!({})).unwrap();
    assert_eq!(path_of(&s, b1), before, "a blend key is not edited");
}

#[test]
fn simplify_keeps_a_live_shape_it_does_not_change() {
    let mut s = session();
    let r = rect(&mut s, 0.0, 0.0, 50.0, 50.0);
    select(&mut s, &[r]);
    s.execute("object.path.simplify", &json!({})).unwrap();
    assert!(matches!(node(&s, r).kind, NodeKind::Path { live: Some(_), .. }), "the rectangle is still a live rectangle");
}

// ---------- object.blend.replaceSpine ----------

#[test]
fn replace_spine_refuses_a_compound_member_and_two_paths() {
    let mut s = session();
    let (b, ..) = blend_of(&mut s);
    let (cp, k1, _) = compound(&mut s);
    select(&mut s, &[b, k1]);
    assert!(s.execute("object.blend.replaceSpine", &json!({})).is_err(), "a compound member is no spine");
    assert_eq!(doc(&s).parent_of(k1), Some(cp), "K1 stays in its compound");
    let p1 = id(&s.execute("shape.line", &json!({"x1": 0, "y1": 500, "x2": 100, "y2": 550})).unwrap());
    let p2 = id(&s.execute("shape.line", &json!({"x1": 0, "y1": 600, "x2": 100, "y2": 650})).unwrap());
    select(&mut s, &[b, p2, p1]);
    assert!(s.execute("object.blend.replaceSpine", &json!({})).is_err(), "two paths: which is the spine is not guessed");
    assert!(exists(&s, p1) && exists(&s, p2));
}

// ---------- object.align {to: artboard} ----------

#[test]
fn align_to_the_artboard_refuses_objects_on_no_artboard() {
    let mut s = session();
    let r = rect(&mut s, 5000.0, 0.0, 20.0, 20.0);
    select(&mut s, &[r]);
    let err = s.execute_as("object.align", &json!({"horizontal": "left", "to": "artboard"}), &agent());
    assert!(err.is_err(), "objects on no artboard: refused for an agent, {err:?}");
    // No test pins the first artboard as a person's fallback: refused too.
    assert!(s.execute("object.align", &json!({"horizontal": "left", "to": "artboard"})).is_err());
    assert_eq!(node(&s, r).geometric_bounds().unwrap().x0, 5000.0);
    // On an artboard it aligns to that one.
    let a = rect(&mut s, 100.0, 100.0, 20.0, 20.0);
    select(&mut s, &[a]);
    s.execute_as("object.align", &json!({"horizontal": "left", "to": "artboard"}), &agent()).unwrap();
    assert_eq!(node(&s, a).geometric_bounds().unwrap().x0, doc(&s).artboards[0].rect.x0);
}

// ---------- object.addTrimMarks ----------

#[test]
fn trim_marks_go_into_the_layer_of_the_selection() {
    let mut s = session();
    let l1 = layer(&s);
    let l2 = id(&s.execute("layer.new", &json!({"name": "Layer 2"})).unwrap());
    let r2 = rect(&mut s, 100.0, 100.0, 50.0, 50.0);
    assert_eq!(doc(&s).parent_of(r2), Some(l2));
    s.execute("layer.setCurrent", &json!({"id": l1.0})).unwrap();
    select(&mut s, &[r2]);
    let g = id(&s.execute("object.addTrimMarks", &json!({})).unwrap());
    assert_eq!(doc(&s).layer_containing(g), Some(l2), "the marks sit with the art they mark");
}

// ---------- object.liquify ----------

#[test]
fn liquify_refuses_locked_and_layer_ids() {
    let mut s = session();
    let r = rect(&mut s, 0.0, 0.0, 100.0, 100.0);
    s.execute("object.setProps", &json!({"ids": [r.0], "locked": true})).unwrap();
    let stroke = json!({"tool": "bloat", "points": [[50, 50], [55, 55]], "ids": [r.0]});
    assert!(s.execute("object.liquify", &stroke).is_err(), "a locked object named is refused, not skipped");
    let l = layer(&s);
    let stroke = json!({"tool": "bloat", "points": [[50, 50], [55, 55]], "ids": [l.0]});
    assert!(s.execute("object.liquify", &stroke).is_err(), "a layer is refused");
}

// ---------- object.flattenTransparency ----------

#[test]
fn flatten_transparency_refuses_a_layer_id() {
    let mut s = session();
    let a = rect(&mut s, 0.0, 0.0, 100.0, 100.0);
    let b = rect(&mut s, 50.0, 50.0, 100.0, 100.0);
    s.execute("transparency.set", &json!({"ids": [b.0], "opacity": 50})).unwrap();
    let l = layer(&s);
    let r = s.execute("object.flattenTransparency", &json!({"ids": [l.0]}));
    assert!(r.is_err(), "a layer is refused, not read as all its art: {r:?}");
    assert!(exists(&s, a) && exists(&s, b));
}

// ---------- object.mesh.add / envelope ----------

#[test]
fn mesh_and_envelope_refuse_a_layer_id() {
    let mut s = session();
    let r = rect(&mut s, 0.0, 0.0, 100.0, 100.0);
    let l = layer(&s);
    assert!(s.execute("object.mesh.add", &json!({"ids": [l.0, r.0]})).is_err(), "mesh: a layer is refused");
    assert!(s.execute("object.envelope.makeWithWarp", &json!({"ids": [l.0, r.0]})).is_err(), "envelope: a layer is refused");
    assert_eq!(doc(&s).parent_of(r), Some(l), "R is untouched");
}

// ---------- object.repeat.* ----------

#[test]
fn repeat_with_one_repeat_and_other_art_selected_is_refused() {
    let mut s = session();
    let a = rect(&mut s, 0.0, 0.0, 20.0, 20.0);
    let rp = id(&s.execute("object.repeat.radial", &json!({"ids": [a.0]})).unwrap());
    let e = ellipse(&mut s, 300.0, 300.0, 20.0, 20.0);
    select(&mut s, &[rp, e]);
    let r = s.execute("object.repeat.grid", &json!({}));
    assert!(r.is_err(), "E would be ignored: {r:?}");
    assert_eq!(doc(&s).parent_of(e), Some(layer(&s)));
}

// ---------- object.pinWarp ----------

#[test]
fn pin_warp_at_rest_defaults_to_the_selected_roots() {
    let mut s = session();
    let (cp, k1, _) = compound(&mut s);
    select(&mut s, &[k1]);
    let r = s.execute("object.pinWarp", &json!({"rest": true, "pins": [], "moved": []})).unwrap();
    assert_eq!(r["ids"], json!([cp.0]), "a compound member stands for its compound, as with rest: false");
}

// ---------- object.pattern.edit ----------

#[test]
fn pattern_edit_without_a_name_takes_the_topmost_selected() {
    let mut s = session();
    let t1 = rect(&mut s, 0.0, 0.0, 10.0, 10.0);
    let p1 = s.execute("object.pattern.make", &json!({"ids": [t1.0], "name": "One", "edit": false})).unwrap()["name"].as_str().unwrap().to_string();
    let t2 = rect(&mut s, 0.0, 0.0, 10.0, 10.0);
    let p2 = s.execute("object.pattern.make", &json!({"ids": [t2.0], "name": "Two", "edit": false})).unwrap()["name"].as_str().unwrap().to_string();
    let lower = rect(&mut s, 100.0, 100.0, 50.0, 50.0);
    let upper = rect(&mut s, 200.0, 100.0, 50.0, 50.0);
    s.execute("paint.setFill", &json!({"ids": [lower.0], "swatch": p1})).unwrap();
    s.execute("paint.setFill", &json!({"ids": [upper.0], "swatch": p2})).unwrap();
    // Selected top first: the lower one was clicked last.
    select(&mut s, &[upper, lower]);
    let a = s.execute("object.pattern.edit", &json!({})).unwrap();
    s.execute("object.pattern.cancel", &json!({})).unwrap();
    select(&mut s, &[lower, upper]);
    let b = s.execute("object.pattern.edit", &json!({})).unwrap();
    assert_eq!(a["name"], b["name"], "the order of the clicks does not decide");
}

// ---------- path.knife / eraseRegion with nothing selected ----------

#[test]
fn knife_with_nothing_selected_leaves_clipping_paths_and_blend_keys_alone() {
    let mut s = session();
    let (_cg, _ca, ck) = clip_group(&mut s);
    let (_b, b1, _) = blend_of(&mut s);
    let ck_before = path_of(&s, ck);
    let b1_before = path_of(&s, b1);
    select(&mut s, &[]);
    s.execute("path.knife", &json!({"points": [[300, 325], [400, 325]]})).unwrap();
    s.execute("path.knife", &json!({"points": [[-10, 410], [40, 410]]})).unwrap();
    assert_eq!(path_of(&s, ck), ck_before, "the clipping path is not cut");
    assert_eq!(path_of(&s, b1), b1_before, "the blend key is not cut");
}

// ---------- path edits by id ----------

#[test]
fn path_edits_by_id_refuse_a_locked_path() {
    let mut s = session();
    let p = id(&s.execute("shape.line", &json!({"x1": 0, "y1": 0, "x2": 100, "y2": 0})).unwrap());
    s.execute("object.setProps", &json!({"ids": [p.0], "locked": true})).unwrap();
    let before = path_of(&s, p);
    for (cmd, params) in [
        ("path.appendAnchor", json!({"id": p.0, "x": 200, "y": 0})),
        ("path.close", json!({"id": p.0})),
        ("path.setHandle", json!({"id": p.0, "anchor": 0, "x": 10, "y": 10})),
        ("path.insertAnchor", json!({"id": p.0, "segment": 0})),
        ("path.reshape", json!({"id": p.0, "x": 50, "y": 0, "dy": 20})),
        ("path.removeAnchor", json!({"id": p.0, "anchor": 0})),
        ("path.split", json!({"id": p.0, "segment": 0, "t": 0.5})),
    ] {
        assert!(s.execute(cmd, &params).is_err(), "{cmd} on a locked path is refused");
    }
    assert_eq!(path_of(&s, p), before);
}

// ---------- edit.cut with direct-selected anchors ----------

#[test]
fn cut_of_direct_selected_anchors_puts_on_the_clipboard_what_it_deletes() {
    let mut s = session();
    let p = id(&s.execute("path.add", &json!({"anchors": [{"x": 0, "y": 0}, {"x": 100, "y": 0}, {"x": 200, "y": 0}, {"x": 300, "y": 0}]})).unwrap());
    s.execute("select.anchors", &json!({"id": p.0, "anchors": [[0, 2], [0, 3]], "mode": "set"})).unwrap();
    s.execute("edit.cut", &json!({})).unwrap();
    let xs = |pd: &kition_vector_geom::PathData| pd.subpaths.iter().flat_map(|sp| sp.anchors.iter().map(|a| a.p.x)).collect::<Vec<_>>();
    assert_eq!(xs(&path_of(&s, p)), [0.0, 100.0], "the anchors and their segments are deleted");
    s.execute("edit.paste", &json!({})).unwrap();
    let pasted: Vec<_> = doc(&s).layers[0].children().unwrap().iter().filter(|c| c.id != p).map(|c| c.path_data().unwrap().clone()).collect();
    assert_eq!(pasted.len(), 1);
    let x = xs(&pasted[0]);
    let dx = x[0] - 100.0;
    assert_eq!(x.iter().map(|v| v - dx).collect::<Vec<_>>(), [100.0, 200.0, 300.0], "the paste is the segments that were cut, not the whole path");
}

// ---------- paint.setFill on a clip group / a blend ----------

#[test]
fn fill_on_a_clip_group_leaves_its_clipping_path_unpainted() {
    let mut s = session();
    let (cg, ca, ck) = clip_group(&mut s);
    s.execute("paint.setFill", &json!({"ids": [cg.0], "color": "#ff0000"})).unwrap();
    assert!(node(&s, ck).appearance.fill_paint().is_none(), "the clipping path stays unpainted");
    assert_eq!(node(&s, ca).appearance.fill_paint().color().unwrap().to_hex(), "#ff0000");
}

#[test]
fn a_blend_is_painted_the_same_way_reached_directly_or_inside_a_group() {
    let mut s = session();
    let (b, b1, b2) = blend_of(&mut s);
    let (c, c1, c2) = blend_of(&mut s);
    let r = rect(&mut s, 600.0, 0.0, 10.0, 10.0);
    let g = group_of(&mut s, &[c, r]);
    s.execute("paint.setFill", &json!({"ids": [b.0], "color": "#00ff00"})).unwrap();
    s.execute("paint.setFill", &json!({"ids": [g.0], "color": "#00ff00"})).unwrap();
    let painted = |s: &Session, n: NodeId| node(s, n).appearance.fill_paint().color().map(|c| c.to_hex());
    assert_eq!(
        (painted(&s, b), painted(&s, b1), painted(&s, b2)),
        (painted(&s, c), painted(&s, c1), painted(&s, c2)),
        "the blend selected and the blend inside the group end up alike"
    );
}

// ---------- text.setFormat / text.fitHeadlineSelection on a layer ----------

#[test]
fn set_format_on_a_layer_formats_the_text_inside() {
    let mut s = session();
    let t = point_text(&mut s, 0.0, "Hello");
    let l = layer(&s);
    s.execute("text.setFormat", &json!({"ids": [l.0], "baselineShift": 3})).unwrap();
    let NodeKind::Text(txt) = &node(&s, t).kind else { panic!() };
    assert!(txt.runs.iter().all(|r| r.style.baseline_shift == 3.0), "the text in the layer is formatted as text.setStyle would");
}

#[test]
fn fit_headline_on_a_group_fits_the_area_type_inside() {
    let mut s = session();
    let t = id(&s.execute("text.add", &json!({"x": 0, "y": 0, "text": "Hi", "size": 12, "area": {"width": 300, "height": 100}})).unwrap());
    let r = rect(&mut s, 500.0, 0.0, 10.0, 10.0);
    let g = group_of(&mut s, &[t, r]);
    let out = s.execute("text.fitHeadlineSelection", &json!({"ids": [g.0]}));
    assert!(out.is_ok(), "the area type in the group is fitted: {out:?}");
}

// ---------- transparency.stopEditingOpacityMask ----------

#[test]
fn stop_editing_an_opacity_mask_returns_to_the_layer_that_was_current() {
    let mut s = session();
    let l1 = layer(&s);
    let r = rect(&mut s, 0.0, 0.0, 100.0, 100.0);
    let e = ellipse(&mut s, 0.0, 0.0, 50.0, 50.0);
    let l2 = id(&s.execute("layer.new", &json!({"name": "Layer 2"})).unwrap());
    let l3 = id(&s.execute("layer.new", &json!({"name": "Layer 3"})).unwrap());
    s.execute("transparency.makeOpacityMask", &json!({"ids": [r.0, e.0]}))
        .unwrap_or_else(|_| s.execute("transparency.makeOpacityMask", &json!({"ids": [e.0, r.0]})).unwrap());
    s.execute("layer.setCurrent", &json!({"id": l2.0})).unwrap();
    let masked = [r, e].into_iter().find(|n| doc(&s).node(*n).is_some_and(|n| n.mask.is_some())).unwrap();
    s.execute("transparency.editOpacityMask", &json!({"id": masked.0})).unwrap();
    s.execute("transparency.stopEditingOpacityMask", &json!({})).unwrap();
    assert_eq!(s.active().unwrap().active_layer, Some(l2), "Layer 2 is current again");
    let _ = (l1, l3);
}

// ---------- brush / symbol ----------

#[test]
fn brush_and_symbol_commands_refuse_a_layer_id() {
    let mut s = session();
    let r = rect(&mut s, 0.0, 0.0, 100.0, 100.0);
    let l = layer(&s);
    let brush = s.execute("brush.list", &json!({})).unwrap()["brushes"][0]["name"].as_str().unwrap().to_string();
    assert!(s.execute("brush.apply", &json!({"name": brush, "ids": [l.0]})).is_err(), "brush.apply: a layer is refused");
    assert!(s.execute("brush.remove", &json!({"ids": [l.0]})).is_err(), "brush.remove: a layer is refused");
    assert!(s.execute("symbol.new", &json!({"ids": [l.0, r.0]})).is_err(), "symbol.new: a layer is refused");
}

#[test]
fn spraying_into_a_selected_locked_symbol_set_is_refused() {
    let mut s = session();
    let r = rect(&mut s, 0.0, 0.0, 10.0, 10.0);
    let name = s.execute("symbol.new", &json!({"ids": [r.0]})).unwrap()["name"].as_str().unwrap().to_string();
    let set = id(&s.execute("symbol.spray", &json!({"points": [[200, 200]], "name": name})).unwrap());
    select(&mut s, &[set]);
    // Locked after it was selected: the set stays the selection.
    let l = doc(&s).layer_containing(set).unwrap();
    s.execute("layer.setProps", &json!({"id": l.0, "locked": true})).unwrap();
    if s.active().unwrap().selection.objects != [set] {
        select(&mut s, &[set]);
    }
    let before = node(&s, set).children().unwrap().len();
    let out = s.execute("symbol.spray", &json!({"points": [[300, 300]], "name": name}));
    assert!(out.is_err() || node(&s, set).children().unwrap().len() == before, "no instance is sprayed into a locked set: {out:?}");
    assert!(out.is_err(), "and the spray says so");
}

// ---------- variables ----------

#[test]
fn a_capture_reads_the_first_bound_object_in_document_order() {
    let mut s = session();
    let t1 = point_text(&mut s, 0.0, "first made");
    let t2 = point_text(&mut s, 200.0, "second made");
    select(&mut s, &[t1]);
    s.execute("arrange.bringToFront", &json!({})).unwrap();
    s.execute("variable.define", &json!({"name": "Name", "kind": "text"})).unwrap();
    s.execute("variable.bind", &json!({"variable": "Name", "ids": [t1.0, t2.0]})).unwrap();
    let ordered = doc(&s).paint_order([t1, t2]);
    let NodeKind::Text(first) = &node(&s, ordered[0]).kind else { panic!() };
    let values = doc(&s).captured_values();
    let got = serde_json::to_value(&values["Name"]).unwrap();
    assert!(got.to_string().contains(&first.plain_text()), "the capture reads {:?} (first in the document), got {got}", first.plain_text());
}

#[test]
fn variable_bind_refuses_a_compound_member() {
    let mut s = session();
    let (_cp, k1, _) = compound(&mut s);
    s.execute("variable.define", &json!({"name": "Shown", "kind": "visibility"})).unwrap();
    assert!(s.execute("variable.bind", &json!({"variable": "Shown", "ids": [k1.0]})).is_err(), "a compound member is not bound on its own");
}

#[test]
fn image_trace_make_refuses_two_images() {
    let mut s = session();
    let a = image(&mut s, 0.0);
    let b = image(&mut s, 100.0);
    assert!(s.execute("imageTrace.make", &json!({"ids": [a.0, b.0]})).is_err(), "the second image is not left untraced silently");
}

#[test]
fn image_trace_release_refuses_a_non_trace_object_named() {
    let mut s = session();
    let a = image(&mut s, 0.0);
    let g = id(&s.execute("imageTrace.make", &json!({"ids": [a.0]})).unwrap());
    let r = rect(&mut s, 300.0, 300.0, 10.0, 10.0);
    assert!(s.execute("imageTrace.release", &json!({"ids": [g.0, r.0]})).is_err(), "R is not dropped silently");
}

#[test]
fn a_group_only_named_image_trace_is_no_image_trace_object() {
    let mut s = session();
    let b = image(&mut s, 200.0);
    let r2 = rect(&mut s, 400.0, 300.0, 10.0, 10.0);
    let fake = group_of(&mut s, &[b, r2]);
    s.execute("object.setProps", &json!({"ids": [fake.0], "name": "Image Trace"})).unwrap();
    assert!(s.execute("imageTrace.expand", &json!({"ids": [fake.0]})).is_err(), "a name does not make an Image Trace object");
    assert!(exists(&s, b), "its image stays");
}

// ---------- asset.add ----------

#[test]
fn assets_add_keeps_the_objects_it_is_given() {
    let mut s = session();
    let c1 = rect(&mut s, 0.0, 0.0, 10.0, 10.0);
    let c2 = rect(&mut s, 20.0, 0.0, 10.0, 10.0);
    let g = group_of(&mut s, &[c1, c2]);
    let r = s.execute("asset.add", &json!({"ids": [g.0, c1.0]})).unwrap();
    assert_eq!(r["assets"].as_array().unwrap().len(), 2, "an asset per object given, C1 included: {r}");
}

// ---------- artboard.rearrange ----------

#[test]
fn rearrange_moves_the_art_of_sublayers_with_its_artboard() {
    let mut s = session();
    s.execute("artboard.new", &json!({"x": 1000, "y": 0, "width": 800, "height": 600})).unwrap();
    s.execute("layer.newSublayer", &json!({})).unwrap();
    // The sublayer holds art on both artboards: its centre is on the first one.
    let a = rect(&mut s, 100.0, 100.0, 50.0, 50.0);
    let r = rect(&mut s, 1100.0, 100.0, 50.0, 50.0);
    assert!(doc(&s).parent_of(r).is_some_and(|p| p != layer(&s)) && doc(&s).parent_of(a) == doc(&s).parent_of(r), "A and R are in a sublayer");
    let ab1 = doc(&s).artboards[1].rect;
    s.execute("artboard.rearrange", &json!({"layout": "column", "spacing": 20})).unwrap();
    let now = doc(&s).artboards[1].rect;
    let moved = now.x0 - ab1.x0;
    assert!(moved.abs() > 1.0, "the artboard moved");
    let b = node(&s, r).geometric_bounds().unwrap();
    assert!((b.x0 - (1100.0 + moved)).abs() < 1e-6, "the sublayer's art moved with it: {b:?}, artboard by {moved}");
    assert_eq!(node(&s, a).geometric_bounds().unwrap().x0, 100.0, "the art on the first artboard stays");
}

// ---------- layer.flatten ----------

#[test]
fn layer_flatten_refuses_an_id_that_is_no_top_level_layer() {
    let mut s = session();
    let r = rect(&mut s, 0.0, 0.0, 10.0, 10.0);
    s.execute("layer.new", &json!({"name": "Layer 2"})).unwrap();
    rect(&mut s, 50.0, 0.0, 10.0, 10.0);
    let n = doc(&s).layers.len();
    assert!(s.execute("layer.flatten", &json!({"id": r.0})).is_err(), "an object is no layer to flatten into");
    assert_eq!(doc(&s).layers.len(), n, "nothing was flattened");
}
