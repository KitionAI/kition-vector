//! `document.identity` and `document.fork` (Kit Process Protocol section 5.6, KitModel M7a)
//! through the real `kition-vector` process. A `.kvector` embeds no replica: it holds the drawing's
//! document model as JSON (`kition_vector_format`), and the kit makes a replica under a new
//! document id each time it opens a file. So a drawing carries no identity, a byte copy carries
//! none either, and a fork is the same bytes with no document id; a fork still never replaces a
//! file.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

use std::path::{Path, PathBuf};
use std::time::Duration;

use kition_kit::KitDocument;
use kition_kit_host::{HeadlessKit, LaunchOptions};
use kition_vector_kit::VectorDocument;
use serde_json::json;

const LIMIT: Duration = Duration::from_secs(30);

fn workspace(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("kition-vector-identity-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::canonicalize(dir).unwrap()
}

async fn launch(dir: &Path) -> HeadlessKit {
    let mut options = LaunchOptions::new(env!("CARGO_BIN_EXE_kition-vector"));
    options.workspace = Some(dir.to_path_buf());
    HeadlessKit::launch(options).await.unwrap()
}

/// A drawing the kit made and edited: saved, opened again, a shape added and saved, as the kit
/// saves a person's or an agent's edits.
fn drawing() -> Vec<u8> {
    let mut doc = VectorDocument::create("Logo.kvector", 800, 600).unwrap();
    doc.execute("shape.rectangle", json!({ "x": 40, "y": 40, "width": 120, "height": 80 })).unwrap();
    let first = doc.save(None).unwrap();
    let mut doc = VectorDocument::open("Logo.kvector", &first).unwrap();
    doc.execute("shape.ellipse", json!({ "x": 200, "y": 150, "width": 60, "height": 60 })).unwrap();
    doc.save(None).unwrap()
}

#[tokio::test]
async fn a_drawing_carries_no_identity_and_forks_as_the_same_bytes() {
    let dir = workspace("no-replica");
    let bytes = drawing();
    std::fs::write(dir.join("Logo.kvector"), &bytes).unwrap();
    let kit = launch(&dir).await;
    assert!(kit.serves("document.identity"), "{}", kit.capabilities());

    // The file carries no identity, and neither does a byte copy of it.
    assert_eq!(kit.identity("Logo.kvector", LIMIT).await.unwrap().doc_id, None, "a .kvector embeds no replica");
    std::fs::copy(dir.join("Logo.kvector"), dir.join("Copy.kvector")).unwrap();
    assert_eq!(kit.identity("Copy.kvector", LIMIT).await.unwrap().doc_id, None);

    // A fork writes the same bytes to a new file, with no document id.
    let fork = kit.fork("Logo.kvector", "Fork.kvector", LIMIT).await.unwrap();
    assert_eq!(fork.doc_id, None);
    assert_eq!(fork.name, "Fork.kvector");
    assert!(!fork.version.is_empty());
    assert_eq!(std::fs::read(dir.join("Fork.kvector")).unwrap(), bytes, "a file without a replica forks unchanged");
    assert_eq!(std::fs::read(dir.join("Logo.kvector")).unwrap(), bytes, "the source is unchanged");
    assert_eq!(kit.identity("Fork.kvector", LIMIT).await.unwrap().doc_id, None);

    // The fork opens as any drawing.
    VectorDocument::open("Fork.kvector", &std::fs::read(dir.join("Fork.kvector")).unwrap()).unwrap();

    // A fork never replaces a file.
    std::fs::write(dir.join("Taken.kvector"), b"a file already there").unwrap();
    let taken = kit.fork("Logo.kvector", "Taken.kvector", LIMIT).await.unwrap_err();
    assert_eq!(taken.reason(), Some("file_conflict"), "{taken}");
    assert_eq!(std::fs::read(dir.join("Taken.kvector")).unwrap(), b"a file already there", "the file there is kept");
    kit.shutdown(Duration::from_secs(5)).await;
    let _ = std::fs::remove_dir_all(dir);
}
