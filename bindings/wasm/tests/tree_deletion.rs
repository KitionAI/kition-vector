//! C8-land-11-vector: structural edits preserve replica identity under child-first deletion.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use kition_kit::KitDocument;
use kition_vector_kit::VectorDocument;
use serde_json::json;

#[test]
fn deleting_and_ungrouping_containers_preserves_the_replica() {
    for command in ["arrange.ungroup", "edit.delete"] {
        let mut doc = VectorDocument::create("Tree.kvector", 200, 100).unwrap();
        let a = doc.execute("shape.rectangle", json!({"x":10,"y":10,"width":20,"height":20})).unwrap()["id"].clone();
        let b = doc.execute("shape.rectangle", json!({"x":50,"y":10,"width":20,"height":20})).unwrap()["id"].clone();
        doc.execute("select.set", json!({"ids":[a,b]})).unwrap();
        doc.execute("arrange.group", json!({})).unwrap();
        doc.version().unwrap();
        let identity = doc.doc_id();
        doc.execute(command, json!({})).unwrap();
        assert_eq!(doc.kdm_resets(), 0, "{command} reset the replica");
        assert_eq!(doc.doc_id(), identity, "{command} changed document identity");
        assert!(VectorDocument::open("Tree.kvector", &doc.save(None).unwrap()).is_ok());
    }
}
