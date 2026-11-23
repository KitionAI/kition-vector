//! A drawing built by the same commands saves the same bytes in a reproducible build, as a
//! showcase sample or a gating test builds it: the document's creation time is `SOURCE_DATE_EPOCH`,
//! not the clock, and with kition-kit's `reproducible-ids` feature (a dev-dependency only) and
//! `KITION_REPRODUCIBLE_IDS` set, the replica's actor, the document id and new objects' ids come
//! from a counter in the process instead of randomness. Each build runs in a process of its own:
//! this test starts itself several times.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;
use std::process::Command;

use kition_kit::KitDocument;
use kition_vector_kit::VectorDocument;
use serde_json::json;

const OUT: &str = "KITION_VECTOR_DETERMINISM_OUT";

fn build() -> Vec<u8> {
    let mut doc = VectorDocument::create("badge.kvector", 400, 300).unwrap();
    doc.execute("shape.ellipse", json!({ "x": 50, "y": 50, "width": 200, "height": 200 })).unwrap();
    doc.execute("paint.setFill", json!({ "color": "#2a9d8f" })).unwrap();
    doc.execute("text.create", json!({ "x": 80, "y": 160, "text": "Trail", "size": 40 })).unwrap();
    doc.save(Some("badge.kvector")).unwrap()
}

/// The child's half: builds the drawing into the file `OUT` names (nothing without it).
#[test]
fn child_builds_a_drawing() {
    if let Ok(path) = std::env::var(OUT) {
        std::fs::write(path, build()).unwrap();
    }
}

fn build_in_a_process(path: &PathBuf) -> Vec<u8> {
    let status = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "child_builds_a_drawing", "--test-threads=1", "--quiet"])
        .env("SOURCE_DATE_EPOCH", "1794182400")
        .env("KITION_REPRODUCIBLE_IDS", "1794182400")
        .env(OUT, path)
        .status()
        .unwrap();
    assert!(status.success(), "the child build failed");
    std::fs::read(path).unwrap()
}

#[test]
fn the_same_commands_save_the_same_bytes() {
    let dir = std::env::temp_dir().join(format!("kition-vector-determinism-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let first = build_in_a_process(&dir.join("0.kvector"));
    for build in 1..3 {
        std::thread::sleep(std::time::Duration::from_millis(1100));
        let again = build_in_a_process(&dir.join(format!("{build}.kvector")));
        assert!(first == again, "build {build} saved other bytes than build 0");
    }
    std::fs::remove_dir_all(&dir).ok();
    assert!(String::from_utf8_lossy(&first).contains("\"created\": 1794182400"), "created at the epoch");
}
