//! C8-vector-engine-2: Region Paint groups in files (issue-c8-engine2-regionpaint-files).
//!
//! A Region Paint group is marked on the node (`Node::region_paint`, C8-vector-engine). The mark
//! goes through SVG (export writes `data-kition-region-paint` on the group when its hidden source
//! paths go with it; import marks the group again). A `.kvector` file written before the mark
//! existed has no mark: it opens with its art as it was, the group a plain group (the repository
//! takes no reader or migration for older files: AGENTS.md, "No compatibility layers").
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

use kition_vector_doc::{Node, NodeId};
use kition_vector_engine::Session;
use serde_json::{Value, json};

fn session() -> Session {
    let mut s = Session::new();
    s.execute("file.new", &json!({"width": 400, "height": 300})).unwrap();
    s
}

fn id(v: &Value) -> NodeId {
    NodeId(v["id"].as_u64().unwrap())
}

/// Two overlapping filled squares made a Region Paint group → its id.
fn live(s: &mut Session) -> NodeId {
    let a = id(&s.execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 100, "height": 100})).unwrap());
    let b = id(&s.execute("shape.rectangle", &json!({"x": 50, "y": 0, "width": 100, "height": 100})).unwrap());
    s.execute("paint.setFill", &json!({"ids": [a.0], "color": "#ff0000"})).unwrap();
    s.execute("paint.setFill", &json!({"ids": [b.0], "color": "#0000ff"})).unwrap();
    s.execute("select.set", &json!({"ids": [a.0, b.0]})).unwrap();
    id(&s.execute("livePaint.make", &json!({})).unwrap())
}

/// The only top-level object of the first layer.
fn only(s: &Session) -> Node {
    let d = &s.doc().unwrap().doc;
    let children = d.layers[0].children().unwrap();
    assert_eq!(children.len(), 1, "one object");
    (*children[0]).clone()
}

fn open(s: &mut Session, name: &str, bytes: &[u8]) -> Value {
    s.execute("document.open", &json!({"name": name, "dataBase64": kition_vector_format::base64_encode(bytes)})).unwrap()
}

fn svg(s: &mut Session, p: Value) -> String {
    let mut p = p;
    p["format"] = json!("svg");
    s.execute("document.serialize", &p).unwrap()["text"].as_str().unwrap().to_string()
}

/// An SVG with its hidden objects (as Save writes it) brings the Region Paint group back as one:
/// its faces and sources are there, and Region Paint commands take it.
#[test]
fn a_region_paint_group_goes_through_svg() {
    let mut s = session();
    let g = live(&mut s);
    let faces = s.execute("livePaint.info", &json!({"group": g.0})).unwrap()["faces"].as_array().unwrap().len();
    let text = svg(&mut s, json!({"hiddenLayers": true}));
    assert!(text.contains("data-kition-region-paint"), "{text}");
    open(&mut s, "live.svg", text.as_bytes());
    let n = only(&s);
    assert!(n.region_paint, "the group is a Region Paint group again");
    assert!(kition_vector_tools::builder::sources(&n).is_some(), "with its source paths");
    let info = s.execute("livePaint.info", &json!({"group": n.id.0})).unwrap();
    assert_eq!(info["faces"].as_array().unwrap().len(), faces, "{info}");
    assert_eq!(info["sources"], 2, "{info}");
}

/// An export that leaves hidden objects out leaves the sources out: no mark, a plain group.
#[test]
fn an_export_without_the_sources_is_a_plain_group() {
    let mut s = session();
    live(&mut s);
    let text = svg(&mut s, json!({}));
    assert!(!text.contains("data-kition-region-paint"), "{text}");
    open(&mut s, "plain.svg", text.as_bytes());
    assert!(!only(&s).region_paint);
}

/// A `.kvector` file written before the mark existed (the same file without `region_paint`): the
/// art is all there, the group named "Region Paint Group" is a plain group, and Region Paint's
/// Release refuses it.
#[test]
fn a_file_from_before_the_mark_opens_as_a_plain_group() {
    let mut s = session();
    live(&mut s);
    let marked = only(&s);
    let saved = kition_vector_format::save(&s.doc().unwrap().doc, false);
    let mut v: Value = serde_json::from_slice(&saved).unwrap();
    fn unmark(v: &mut Value) -> usize {
        match v {
            Value::Object(o) => o.remove("region_paint").map_or(0, |_| 1) + o.values_mut().map(unmark).sum::<usize>(),
            Value::Array(a) => a.iter_mut().map(unmark).sum(),
            _ => 0,
        }
    }
    assert_eq!(unmark(&mut v), 1, "the file carried one mark");
    assert_eq!(v["version"], kition_vector_format::VERSION);
    let old = serde_json::to_vec(&v).unwrap();
    let r = open(&mut s, "old.kvector", &old);
    assert_eq!(r["warnings"], json!([]), "{r}");
    let n = only(&s);
    assert!(!n.region_paint, "no reader turns a name into the mark");
    assert_eq!(n.name.as_deref(), Some(kition_vector_tools::builder::REGION_PAINT_NAME));
    let mut plain = marked.clone();
    plain.region_paint = false;
    assert!(n == plain, "the art came back as it was, the mark aside");
    assert!(s.execute("livePaint.release", &json!({"id": n.id.0})).is_err());
}

/// An Image Trace object goes through SVG the same way: its trace (`Node::trace`) comes back on
/// the group with its source image, and Image Trace commands take it (Release puts the image
/// back).
#[test]
fn an_image_trace_object_goes_through_svg() {
    const PNG: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==";
    let page = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="100"><image x="0" y="0" width="40" height="40" href="data:image/png;base64,{PNG}"/></svg>"#
    );
    let mut s = Session::new();
    open(&mut s, "image.svg", page.as_bytes());
    let image = only(&s).id;
    let traced = id(&s.execute("imageTrace.make", &json!({"id": image.0})).unwrap());
    let trace = s.doc().unwrap().doc.node(traced).unwrap().trace.clone();
    assert!(trace.is_some(), "the trace group carries its trace");
    let text = svg(&mut s, json!({"hiddenLayers": true}));
    assert!(text.contains("data-kition-trace"), "{text}");
    open(&mut s, "traced.svg", text.as_bytes());
    let n = only(&s);
    assert_eq!(n.trace, trace, "the trace came back on the group");
    s.execute("imageTrace.release", &json!({"id": n.id.0})).unwrap();
}
