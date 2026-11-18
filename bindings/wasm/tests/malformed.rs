//! Damaged files are refused without a panic or a hang (kit rule E2): every damaged variant of
//! the conformance samples opens to an error, or to a document that inspects and renders.
#![allow(clippy::unwrap_used, clippy::panic)]

use std::path::Path;
use std::time::Duration;

#[test]
fn damaged_files_are_refused_without_a_panic_or_a_hang() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance");
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
        .filter(|name| (name.starts_with("basic.") || name.starts_with("check-")) && !name.ends_with(".json"))
        .collect();
    names.sort_by_key(|name| (!name.starts_with("basic."), name.clone()));
    names.truncate(3);
    assert!(!names.is_empty(), "no conformance samples in {}", dir.display());
    let mut problems = Vec::new();
    for name in &names {
        let bytes = std::fs::read(dir.join(name)).unwrap();
        if let Err(found) = kition_kit::testing::check_malformed::<kition_vector_kit::VectorDocument>(name, &[&bytes], Duration::from_secs(30)) {
            problems.extend(found.into_iter().map(|problem| format!("{name}: {problem}")));
        }
    }
    assert!(problems.is_empty(), "{problems:#?}");
}
