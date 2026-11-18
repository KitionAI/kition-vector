//! C8-vector-4 item 5: in memory == saved. Every write command family, on every conformance and
//! example file: after the write, the drawing as the engine holds it equals the drawing its saved
//! file reopens as, and the change the receipt is made from (the replica before and after the
//! write) equals the change between the replica before and the reopened file's.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

use kdm_model::VectorModel;
use kition_kit::session::KitModel;
use kition_kit::session::model::Exchange;
use kition_kit::{KitDocument, document_content};

mod families;
use families::{first_difference, fixtures, labelled, writes};
use kition_vector_kit::VectorDocument;
use serde_json::json;

/// The editor state `inspect` reports beside the drawing.
const VOLATILE: &[&str] = kition_vector_kit::EDITOR_STATE;

#[test]
fn every_write_family_on_every_file_saves_as_it_is_held() {
    let schema = VectorModel::schema();
    let mut tested = 0;
    let mut failures = Vec::new();
    for path in fixtures() {
        let bytes = std::fs::read(&path).unwrap();
        let mut source = VectorDocument::open("Saved.kvector", &bytes).unwrap();
        let scenarios = writes(&mut source);
        let prepared = source.save(None).unwrap();
        let file = path.file_name().unwrap().to_string_lossy().into_owned();
        for (command, params, selection) in scenarios {
            let context = format!("{file} {command} {params}");
            let mut doc = VectorDocument::open("Saved.kvector", &prepared).unwrap();
            if let Some(ids) = selection {
                doc.execute("select.set", json!({ "ids": ids })).unwrap();
            }
            let mut exchange = Exchange::new();
            let before = exchange.replica(&mut doc).unwrap().state_json().unwrap();
            let receipt = match exchange.batch(&mut doc, &json!({"ops":[{"op":"command","id":command,"params":params}]}), false) {
                Ok(receipt) => receipt,
                Err(error) => {
                    failures.push(format!("{context}: failed: {error}"));
                    continue;
                }
            };
            let live = document_content(doc.inspect().unwrap(), VOLATILE);
            let mut reopened = VectorDocument::open("Saved.kvector", &doc.save(None).unwrap()).unwrap();
            let disk = document_content(reopened.inspect().unwrap(), VOLATILE);
            if live != disk {
                failures.push(format!("{context}: the reopened file differs: {}", first_difference(&live, &disk, "")));
                continue;
            }
            let after = exchange.replica(&mut doc).unwrap().state_json().unwrap();
            let saved = Exchange::new().replica(&mut reopened).unwrap().state_json().unwrap();
            // The two replicas name objects by different object ids: each diff names them by tool
            // id (or type and name), as a receipt and a reread do.
            let memory = labelled(kition_kit::kdm::reread::diff_paired(&schema, &before, &after).to_json(), &[&before, &after]);
            let persisted = labelled(kition_kit::kdm::reread::diff_paired(&schema, &before, &saved).to_json(), &[&before, &saved]);
            if std::env::var("SAVED_DEBUG").is_ok_and(|want| context.starts_with(&want)) {
                println!("DEBUG {context}\nMEMORY {memory}\nSAVED {persisted}");
            }
            if memory != persisted {
                failures.push(format!(
                    "{context}: the receipt's change differs from the saved one: {} (receipt {receipt})",
                    first_difference(&memory, &persisted, "")
                ));
                continue;
            }
            if !["changed", "created", "deleted", "moved"].iter().any(|key| persisted[key].as_array().is_some_and(|v| !v.is_empty())) {
                failures.push(format!("{context}: no write"));
                continue;
            }
            tested += 1;
        }
    }
    println!("SAVED {tested} writes held as saved; {} not:", failures.len());
    for failure in &failures {
        println!("  {}", failure.chars().take(900).collect::<String>());
    }
    assert!(failures.is_empty(), "{} writes differ from their saved files", failures.len());
}
