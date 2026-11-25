//! TEMPORARY diagnostic (plain model): undo all and redo all of plain rectangles; prints the
//! layer's child ids and the replica's tree after each phase, then fails so the output shows.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use kition_vector_testkit::fixtures;
use kition_vector_testkit::invariants::{redo_all, undo_all};
use serde_json::json;

fn ids(s: &kition_vector_engine::Session) -> Vec<u64> {
    let d = &s.doc().unwrap().doc;
    d.layers[0].children().map(|c| c.iter().map(|n| n.id.0).collect()).unwrap_or_default()
}

fn tree(s: &kition_vector_engine::Session) -> String {
    let Some(kdm) = s.kdm() else { return "no replica".into() };
    let r = kdm.replica();
    let root = kdm_model::kition_kit::kdm::ObjectId::from_string("drawing");
    let field = r
        .tree(&root, "nodes")
        .map(|t| t.iter().map(|e| format!("{}<{:?}", e.object, e.parent.as_ref().map(|p| p.object().to_string()))).collect::<Vec<_>>().join(" "));
    format!("{field:?}")
}

#[test]
fn diag_undo_redo_order() {
    let mut out = String::new();
    for (n, batch) in [(3usize, false), (5, false), (5, true), (8, false)] {
        let mut s = fixtures::session();
        for i in 0..n {
            let rect = json!({"x": 0, "y": 0, "width": 1, "height": 1});
            if batch && i == 1 {
                s.execute("command.batch", &json!({"commands": [{"command": "shape.rectangle", "params": rect}]})).unwrap();
            } else {
                s.execute("shape.rectangle", &rect).unwrap();
            }
        }
        let fin = ids(&s);
        let t0 = tree(&s);
        let u = undo_all(&mut s);
        let t1 = tree(&s);
        let r = redo_all(&mut s);
        let again = ids(&s);
        let t2 = tree(&s);
        out.push_str(&format!("n={n} batch={batch} undo={u} redo={r}\n final {fin:?}\n again {again:?}\n tree0 {t0}\n tree1 {t1}\n tree2 {t2}\n"));
    }
    panic!("DIAG\n{out}");
}
