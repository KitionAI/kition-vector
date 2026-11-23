//! Foreign product tags are rejected, even when the payload otherwise matches a native file.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::io::Write as _;

use kition_vector_doc::{Document, ImageBlob, ImageObject, Node, NodeKind};
use kition_vector_format::{EXTENSION, FormatError, INLINE_MAX, SaveOptions, load_file, save_with, sniff};
use kition_vector_geom::Affine;

/// A document showing an image of `len` bytes.
fn drawing(len: usize) -> Document {
    let mut d = Document::new(120.0, 80.0);
    d.images.insert("img".into(), ImageBlob::new("image/png", (0..len).map(|i| (i * 13 % 251) as u8).collect()));
    let l = d.layers[0].id;
    let id = d.alloc_id();
    let im = ImageObject { key: "img".into(), width: 4, height: 4, xf: Affine::IDENTITY, link: None, placement: Default::default() };
    d.insert(Some(l), 0, Node::new(id, NodeKind::Image(im))).unwrap();
    d
}

/// `bytes` (a plain, uncompressed file, compact or pretty) with its format tag replaced by `tag`.
fn retag(bytes: &[u8], tag: &str) -> Vec<u8> {
    for sep in [":", ": "] {
        let from = format!("\"format\"{sep}\"{EXTENSION}\"");
        if let Some(at) = bytes.windows(from.len()).position(|w| w == from.as_bytes()) {
            let mut out = bytes[..at].to_vec();
            out.extend_from_slice(format!("\"format\"{sep}\"{tag}\"").as_bytes());
            out.extend_from_slice(&bytes[at + from.len()..]);
            return out;
        }
    }
    panic!("the tag")
}

fn gzip(bytes: &[u8]) -> Vec<u8> {
    let mut e = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    e.write_all(bytes).unwrap();
    e.finish().unwrap()
}

#[test]
fn rejects_foreign_tags_in_all_supported_encodings() {
    for (pretty, len) in [(false, 100), (true, 100), (false, INLINE_MAX + 10), (true, INLINE_MAX + 10)] {
        let d = drawing(len);
        let ours = save_with(&d, &SaveOptions { pretty, ..SaveOptions::default() }).unwrap();
        for bytes in [ours.clone(), gzip(&ours)] {
            assert!(sniff(&bytes));
            assert_eq!(load_file(&bytes).unwrap().doc.images["img"].bytes, d.images["img"].bytes);
        }
        for tag in ["otherapp", "kitionvector", "someotherapp"] {
            let foreign = retag(&ours, tag);
            assert_ne!(ours, foreign);
            for bytes in [foreign.clone(), gzip(&foreign)] {
                assert!(!sniff(&bytes), "pretty {pretty}, {len} bytes, {tag}");
                assert!(matches!(load_file(&bytes), Err(FormatError::NotKitionVector(_))), "pretty {pretty}, {len} bytes, {tag}");
            }
        }
    }
}
