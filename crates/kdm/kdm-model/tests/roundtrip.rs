//! Round trips: a drawing written to a replica reads back with the same content (ids aside), for
//! the web editor's sample and the repository's examples, a drawing exercising every replicated
//! reference, and (when `KITION_VECTOR_CORPUS=<dir>` is set) up to 40 drawings of a local corpus.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use std::sync::Arc;

use common::{SharedBlobs, assert_same_content, drawing, new_replica};
use kdm_model::{VectorModel, decode, read_document, write_changes};
use kition_kit::kdm::ActorId;
use kition_kit::session::{KitModel, KitSession};
use kition_vector_doc::{Document, Node};

/// A copy of `nodes` sharing no `Arc` with them, so a diff against it compares every field.
fn deep(nodes: &[Arc<Node>]) -> Vec<Arc<Node>> {
    nodes
        .iter()
        .map(|n| {
            let mut c = (**n).clone();
            if let Some(children) = c.children_mut() {
                *children = deep(children);
            }
            Arc::new(c)
        })
        .collect()
}

/// `d` through `import`'s path (write_document in a creating transaction) and back. Also checks
/// that diffing the read view against a deep copy of itself writes nothing.
fn round_trip(d: &Document) -> Document {
    let blobs = SharedBlobs::default();
    let (mut replica, _) = new_replica(d, &blobs);
    let (read, mut ids) = read_document(&replica, &blobs).unwrap_or_else(|error| panic!("read: {error}"));
    let mut copy = read.clone();
    copy.layers = deep(&copy.layers);
    let mut sink = blobs.clone();
    let wrote = replica
        .transact(&common::person("t"), |tx| write_changes(tx, &read, &copy, &mut ids, &mut sink))
        .unwrap_or_else(|error| panic!("write_changes: {error}"));
    assert!(!wrote, "an unchanged view wrote changes");
    read
}

fn check(name: &str, original: &Document) {
    let read = round_trip(original);
    assert_same_content(name, original, &read);
    for (key, img) in &original.images {
        let back = read.images.get(key).unwrap_or_else(|| panic!("{name}: image {key} is gone"));
        assert_eq!(img.bytes, back.bytes, "{name}: image {key} bytes");
        assert_eq!(img.proxy, back.proxy, "{name}: image {key} proxy");
    }
}

#[test]
fn sample_and_examples_round_trip() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../..");
    let mut files = vec![format!("{root}/web/public/sample/Sample.kvector")];
    for entry in std::fs::read_dir(format!("{root}/examples")).unwrap_or_else(|error| panic!("examples: {error}")) {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e| e == "kvector") {
            files.push(path.to_string_lossy().into_owned());
        }
    }
    assert!(files.len() > 1, "the examples are there");
    for file in files {
        let bytes = std::fs::read(&file).unwrap_or_else(|error| panic!("{file}: {error}"));
        let original = decode(&file, &bytes).unwrap_or_else(|error| panic!("{file}: decode: {error}"));
        assert!(!original.layers.is_empty(), "{file} has layers");
        check(&file, &original);
    }
}

#[test]
fn rejects_foreign_native_format_tags() {
    let file = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../web/public/sample/Sample.kvector");
    let ours = std::fs::read_to_string(file).unwrap();
    let foreign = ours.replacen("\"format\":\"kvector\"", "\"format\":\"otherapp\"", 1);
    assert_ne!(foreign, ours);
    for name in ["Sample.otherapp", "Sample.kvector"] {
        assert!(decode(name, foreign.as_bytes()).is_err(), "{name}");
    }
    assert!(decode("Sample.kvector", ours.as_bytes()).is_ok());
}

#[test]
fn drawing_with_every_reference_round_trips() {
    let d = drawing();
    check("drawing", &d);
    let read = round_trip(&d);
    // The references read back as references to the same things.
    let a = common::named(&read, "A");
    let style = a.graphic_style.unwrap_or_else(|| panic!("A keeps its graphic style link"));
    assert_eq!(read.graphic_styles[0].id, style);
    assert_eq!(a.appearance.fill_paint(), d.node(common::named_id(&d, "A")).unwrap().appearance.fill_paint(), "the swatch link reads back by name");
    assert_eq!(read.text_threads, vec![vec![common::named_id(&read, "B"), common::named_id(&read, "C")]]);
    assert_eq!(read.guides[1].artboard, Some(read.artboards[1].id));
    assert_eq!(read.guides[0].layer, Some(read.layers[0].id));
    assert_eq!(read.assets[0].nodes, vec![common::named_id(&read, "A")]);
    assert!(read.variables.bindings.contains_key(&common::named_id(&read, "A")));
}

#[test]
fn import_and_export_through_a_session() {
    let d = drawing();
    let bytes = kition_vector_format::save_file(&d);
    let session: KitSession<VectorModel> =
        KitSession::import("Logo.kvector", &bytes, Box::new(SharedBlobs::default()), ActorId::from_u64(7).unwrap_or_else(|_| panic!("actor")))
            .unwrap_or_else(|error| panic!("import: {error}"));
    let exported = session.export("Logo.kvector").unwrap_or_else(|error| panic!("export: {error}"));
    let again = decode("Logo.kvector", &exported).unwrap_or_else(|error| panic!("decode: {error}"));
    let original = decode("Logo.kvector", &bytes).unwrap_or_else(|error| panic!("decode: {error}"));
    assert_same_content("export", &original, &again);
    let svg = session.export("Logo.svg").unwrap_or_else(|error| panic!("svg export: {error}"));
    assert!(String::from_utf8_lossy(&svg).contains("<svg"));
}

#[test]
fn schema_parses() {
    let schema = VectorModel::schema();
    assert_eq!(schema.kit, "kit.vector");
}

/// `KITION_VECTOR_CORPUS=<dir>`: up to 40 drawings there (native files, SVG), each decoded,
/// written, read back and compared. Files that do not decode are skipped. Skipped when unset.
#[test]
fn corpus_round_trips() {
    let Ok(dir) = std::env::var("KITION_VECTOR_CORPUS") else { return };
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .unwrap_or_else(|error| panic!("{dir}: {error}"))
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .collect();
    files.sort();
    files.truncate(40);
    let mut checked = 0;
    for path in &files {
        let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let bytes = std::fs::read(path).unwrap_or_else(|error| panic!("{name}: {error}"));
        let Ok(original) = decode(&name, &bytes) else {
            eprintln!("{name}: not decodable, skipped");
            continue;
        };
        check(&name, &original);
        checked += 1;
    }
    eprintln!("corpus: {checked} of {} files round-tripped", files.len());
}

/// An opacity mask made of several parts (or none) is a group with the engine's id
/// `u64::MAX - 1`, beyond what a replica takes (±2^53): the drawing still gets a replica, records
/// edits to it, and reads back the same.
#[test]
fn a_mask_group_with_an_id_beyond_2_pow_53_replicates() {
    use kition_vector_doc::{NodeId, OpacityMask};
    let mut d = Document::new(300.0, 200.0);
    let layer = d.layers[0].id;
    let mut masked = common::rect_node(&mut d, "masked", 10.0, 10.0);
    let part = common::rect_node(&mut d, "part", 0.0, 0.0);
    masked.mask = Some(Box::new(OpacityMask::new(Node::group(NodeId(u64::MAX - 1), vec![Arc::new(part)]), true)));
    d.insert(Some(layer), usize::MAX, masked).unwrap();
    check("mask group", &d);
    let doc = Arc::new(d.clone());
    let mut session = kdm_model::VectorSession::create(&doc).unwrap();
    let mut moved = d.clone();
    let id = common::named_id(&moved, "masked");
    moved.node_mut(id).unwrap().mask.as_mut().unwrap().invert = true;
    assert_eq!(session.record(&common::person("p"), &Arc::new(moved)).unwrap(), 1, "the edit is one undo step");
}
