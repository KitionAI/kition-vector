//! `vector_check` against its catalogue (`agent/checks.json`, KHP A38): the catalogue file is the
//! check's own (`check::CATALOGUE`), errors first, one sentence each; every fixture in
//! `conformance/` reports its code and only catalogued codes; `basic.kvector` is clean; and
//! checking never changes the drawing. The fixtures are written by the crate's `conformance` test.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;

use kition_kit::KitDocument;
use kition_kit::diagnostics::{Severity, check_diagnostics};
use kition_vector_kit::VectorDocument;
use kition_vector_kit::check::{CATALOGUE, catalogue_json};
use serde_json::Value;

fn repo() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
}

fn open(name: &str) -> VectorDocument {
    let bytes = std::fs::read(repo().join("conformance").join(name)).unwrap_or_else(|error| panic!("{name}: {error}"));
    VectorDocument::open(name, &bytes).unwrap()
}

#[test]
fn the_catalogue_file_is_the_checks_own() {
    let file: Value = serde_json::from_str(&std::fs::read_to_string(repo().join("agent/checks.json")).unwrap()).unwrap();
    assert_eq!(file, catalogue_json(), "regenerate with KITION_WRITE_CONFORMANCE=1 cargo test -p kition-vector-kit conformance");
    let mut previous = Severity::Error;
    for entry in CATALOGUE {
        assert!(kition_kit::diagnostics::valid_code(entry.code) && entry.code.starts_with("vector."), "{}", entry.code);
        assert!(entry.severity >= previous, "errors first: {}", entry.code);
        previous = entry.severity;
        assert!(entry.summary.ends_with('.') && !entry.summary.contains('\n'), "{}: one sentence", entry.code);
    }
}

#[test]
fn every_fixture_reports_its_code_and_only_catalogued_codes() {
    let known: Vec<&str> = CATALOGUE.iter().map(|entry| entry.code).collect();
    for entry in CATALOGUE {
        let name = format!("check-{}.kvector", entry.code.trim_start_matches("vector."));
        let drawing = open(&name);
        let found = drawing.check();
        check_diagnostics(&found).unwrap();
        assert!(found.iter().any(|diagnostic| diagnostic.code == entry.code), "{name}: {found:?}");
        for diagnostic in &found {
            assert!(known.contains(&diagnostic.code.as_str()), "{name}: {} is not catalogued", diagnostic.code);
        }
    }
}

#[test]
fn basic_is_clean_and_checking_changes_nothing() {
    let drawing = open("basic.kvector");
    let before = drawing.inspect().unwrap();
    assert_eq!(drawing.check(), Vec::new());
    assert_eq!(drawing.inspect().unwrap(), before);
    assert!(!drawing.dirty());
}
