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

/// The kit's save writes the drawing only: two sessions that open the same file and save without
/// editing write the same bytes, whatever each person looked at or selected (nothing of one
/// person's editing, no thumbnail, is in a `.kvector`).
#[test]
fn two_sessions_that_open_the_same_file_and_save_without_editing_write_identical_files() {
    let mut doc = VectorDocument::create("Logo.kvector", 800, 600).unwrap();
    doc.execute("shape.rectangle", json!({ "x": 40, "y": 40, "width": 120, "height": 80 })).unwrap();
    doc.execute("select.all", json!({})).unwrap();
    doc.execute("object.group", json!({})).unwrap();
    let file = doc.save(None).unwrap();

    let a = VectorDocument::open("Logo.kvector", &file).unwrap();
    let mut b = VectorDocument::open("Logo.kvector", &file).unwrap();
    // One person selects and looks; the other does nothing.
    b.execute("select.all", json!({})).unwrap();
    let first = a.save(None).unwrap();
    let second = b.save(None).unwrap();
    assert!(first == second, "two sessions saved other bytes");
    let text = String::from_utf8_lossy(&first);
    for key in ["\"last_view\"", "\"layers_open\"", "\"preview\""] {
        assert!(!text.contains(key), "{key} in the file");
    }
}
