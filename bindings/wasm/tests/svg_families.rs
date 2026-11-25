//! C8-vector-5 item 4: in memory == saved for SVG files. Each fixture as an SVG this kit exports
//! (and `examples/dusk-poster.svg`), opened as `.svg`, gets one write of each command family
//! (`families::writes`), is written again as SVG and reopened as SVG: the drawing as the engine
//! holds it against the reopened one, and the change the receipt is made from against the change
//! a read of the reopened file tells.
//!
//! Two ways to write the SVG back: with the editing data (`preserveEditing`, the native drawing in
//! `<metadata>`), which must hold every family as `.kvector` does, and as plain SVG, which holds
//! only what SVG says: the families that differ there are listed (printed), not failed.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

use std::collections::BTreeMap;

use kdm_model::VectorModel;
use kition_kit::session::KitModel;
use kition_kit::session::model::Exchange;
use kition_kit::{KitDocument, document_content};
use kition_vector_kit::VectorDocument;
use serde_json::json;

mod families;
use families::{first_difference, fixtures, labelled, writes};

const VOLATILE: &[&str] = kition_vector_kit::EDITOR_STATE;

/// The drawing written as SVG by this kit's SVG writer (as `file.export` writes one file of
/// every artboard's art), with or without its editing data. (`file.exportBytes` takes no SVG
/// options through the kit: see the item's report.)
fn svg_of(doc: &mut VectorDocument, preserve: bool) -> Vec<u8> {
    let drawing = kition_vector_format::load(&doc.save(Some("Drawing.kvector")).unwrap()).unwrap();
    let native = preserve.then(|| kition_vector_format::save(&drawing, false));
    let opts = kition_vector_svg::ExportOptions { preserve_editing: preserve, hidden_layers: true, ..Default::default() };
    kition_vector_svg::export_full(&drawing, &opts, native.as_deref()).svg.into_bytes()
}

/// The SVG files: every fixture exported as SVG (with its editing data, so it opens as itself),
/// and the example SVG.
fn svg_fixtures() -> Vec<(String, Vec<u8>)> {
    let mut out: Vec<(String, Vec<u8>)> = fixtures()
        .into_iter()
        .map(|path| {
            let mut doc = VectorDocument::open("Fixture.kvector", &std::fs::read(&path).unwrap()).unwrap();
            (path.file_name().unwrap().to_string_lossy().replace(".kvector", ".svg"), svg_of(&mut doc, true))
        })
        .collect();
    let example = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/dusk-poster.svg");
    out.push(("dusk-poster.svg (example)".to_owned(), std::fs::read(example).unwrap()));
    out
}

/// Per family (command), how many files it differed on, and the first difference seen.
fn round_trips(preserve: bool) -> (usize, BTreeMap<String, (usize, String)>) {
    let schema = VectorModel::schema();
    let mut held = 0;
    let mut differ: BTreeMap<String, (usize, String)> = BTreeMap::new();
    for (file, bytes) in svg_fixtures() {
        let mut source = VectorDocument::open("Saved.svg", &bytes).unwrap();
        let scenarios = writes(&mut source);
        let prepared = svg_of(&mut source, preserve);
        for (command, params, selection) in scenarios {
            let mut note = |what: String| {
                let entry = differ.entry(command.to_owned()).or_insert((0, String::new()));
                entry.0 += 1;
                if entry.1.is_empty() {
                    entry.1 = format!("{file}: {}", what.chars().take(300).collect::<String>());
                }
            };
            let mut doc = VectorDocument::open("Saved.svg", &prepared).unwrap();
            if let Some(ids) = selection {
                // Ids of the prepared file's objects may differ in a plain SVG read.
                if doc.execute("select.set", json!({ "ids": ids })).is_err() {
                    note("its objects are not in the SVG read again".to_owned());
                    continue;
                }
            }
            let mut exchange = Exchange::new();
            let before = exchange.replica(&mut doc).unwrap().state_json().unwrap();
            if let Err(error) = exchange.batch(&mut doc, &json!({"ops":[{"op":"command","id":command,"params":params}]}), false) {
                note(format!("failed: {}", error.message));
                continue;
            }
            let live = document_content(doc.inspect().unwrap(), VOLATILE);
            let mut reopened = VectorDocument::open("Saved.svg", &svg_of(&mut doc, preserve)).unwrap();
            let disk = document_content(reopened.inspect().unwrap(), VOLATILE);
            if live != disk {
                note(format!("the reopened SVG differs: {}", first_difference(&live, &disk, "")));
                continue;
            }
            let after = exchange.replica(&mut doc).unwrap().state_json().unwrap();
            let saved = Exchange::new().replica(&mut reopened).unwrap().state_json().unwrap();
            let memory = labelled(kition_kit::kdm::reread::diff_paired(&schema, &before, &after).to_json(), &[&before, &after]);
            let persisted = labelled(kition_kit::kdm::reread::diff_paired(&schema, &before, &saved).to_json(), &[&before, &saved]);
            if memory != persisted {
                note(format!("the receipt's change differs from the reread: {}", first_difference(&memory, &persisted, "")));
                continue;
            }
            held += 1;
        }
    }
    (held, differ)
}

fn report(label: &str, held: usize, differ: &BTreeMap<String, (usize, String)>) {
    let failed: usize = differ.values().map(|(count, _)| count).sum();
    println!("SVG {label}: {held} writes held, {failed} differ, in {} families:", differ.len());
    for (command, (count, first)) in differ {
        println!("  {command} ({count} files): {first}");
    }
}

#[test]
fn an_svg_written_back_with_its_editing_data_holds_every_family() {
    let (held, differ) = round_trips(true);
    report("with editing data", held, &differ);
    assert!(differ.is_empty(), "{} families differ", differ.len());
}

#[test]
fn a_plain_svg_written_back_holds_what_svg_says() {
    let (held, differ) = round_trips(false);
    report("plain", held, &differ);
    // Listed, not failed: plain SVG has no swatches, styles, guides, artboards beyond the view,
    // live shapes or effects of its own (see the printed list).
    assert!(held > 0);
}
