//! `document.text` (KitText 1.0) through `kition-vector` as `kition-headless` serves it: the
//! manifest's capability reaches `kit.initialize`, and the executable answers the drawing's words
//! without changing it.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;

use kition_headless::{Headless, HostFolder, KitInfo};
use kition_kit_server::protocol::DOCUMENT_TEXT_LIMIT;
use kition_kit_server::{ErrorReason, KitBackend, OpenRequest};
use kition_vector_kit::VectorDocument;
use serde_json::json;

const DUSK: &str = "DUSK\nMade in Kition Vector — vectors, pure Rust";

#[test]
fn answers_document_text_as_the_manifest_declares() {
    let mut kit = Headless::<VectorDocument>::with_replica_port(
        KitInfo::from_manifest(include_str!("../../../kit.json"), &[], kition_vector_kit::EDITOR_STATE).unwrap(),
    );
    assert_eq!(kit.capabilities()["capabilities"], json!([kition_kit::text::CAPABILITY]));
    let root = std::env::temp_dir().join(format!("kition-vector-text-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let root = std::fs::canonicalize(root).unwrap();
    let path = root.join("dusk.kvector");
    std::fs::copy(Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/dusk-poster.kvector")), &path).unwrap();
    let folder = HostFolder::new(&root).unwrap();
    kit.set_host(folder.host());
    let opened = kit.open(OpenRequest { file: &folder.file("dusk.kvector").unwrap(), read_only: false, create: None }).unwrap();
    let text = kit.text(&opened.id, DOCUMENT_TEXT_LIMIT).unwrap();
    assert_eq!((text["text"].as_str(), text["truncated"].as_bool()), (Some(DUSK), Some(false)));
    assert_eq!(text["parts"].as_array().map(Vec::len), Some(1), "{text}");
    let cut = kit.text(&opened.id, 4).unwrap();
    assert_eq!((cut["text"].as_str(), cut["truncated"].as_bool()), (Some("DUSK"), Some(true)));
    assert!(!kit.document(&opened.id).unwrap().dirty);
    assert_eq!(kit.text("no-such-document", 10).unwrap_err().reason(), Some(ErrorReason::DocumentNotFound));
    let _ = std::fs::remove_dir_all(root);
}
