//! Shared pinned corpus gate: an installed corpus must be complete and unmodified.
#![allow(dead_code)]

use std::path::PathBuf;

#[path = "../../../../../tools/xtask/src/sha256.rs"]
mod sha256;

pub fn pinned() -> Option<PathBuf> {
    // This helper is compiled in both affinity and engine integration tests.
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../corpus/affinity");
    let required = std::env::var_os("AFFINITY_CORPUS_REQUIRED").is_some_and(|v| v != "0");
    if !dir.exists() {
        assert!(!required, "required Affinity corpus absent: see docs/features/affinity-validation.md");
        eprintln!("Affinity corpus absent: copy it manually into corpus/affinity (see docs/features/affinity-validation.md)");
        return None;
    }
    assert!(dir.join("SOURCES.md").is_file(), "incomplete Affinity corpus: missing SOURCES.md (see docs/features/affinity-validation.md)");
    let manifest = include_str!("../../../../../tools/xtask/affinity-corpus.sha256");
    let mut count = 0;
    for line in manifest.lines().filter(|l| !l.trim().is_empty()) {
        let (digest, name) = line.split_once("  ").expect("malformed pinned Affinity SHA256 manifest");
        assert_eq!(digest.len(), 64, "invalid SHA256 for {name}");
        let bytes = std::fs::read(dir.join(name)).unwrap_or_else(|e| panic!("missing pinned Affinity file {name}: {e}"));
        assert_eq!(sha256::hex(&bytes), digest, "{name}: pinned corpus modified (see docs/features/affinity-validation.md)");
        count += 1;
    }
    assert!(count >= 4, "pinned Affinity manifest must include the four Affinity 3 .af fixtures");
    Some(dir)
}
