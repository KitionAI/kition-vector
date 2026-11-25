//! C8-vector-5 item 7: two collaborators opening fresh replicas of the same file derive the same
//! persistent identities (`Document::settle_uids`) (issue-c8-identity).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use common::{SharedBlobs, drawing, first};
use kition_vector_doc::Document;

/// The drawing as a file read gives it: every named entry with its identity.
fn settled() -> Document {
    let mut d = drawing();
    d.settle_uids(None);
    d
}

/// Two collaborators, each with a fresh replica of the same file (not one replica's peer).
fn two_readers() -> (SharedBlobs, common::Session, common::Session) {
    let blobs = SharedBlobs::default();
    let a = first(&settled(), &blobs);
    let b = first(&settled(), &blobs);
    (blobs, a, b)
}

fn uids(d: &Document) -> Vec<(String, String)> {
    d.swatches.iter().chain(d.swatch_groups.iter().flat_map(|g| g.swatches.iter())).map(|s| (s.name.clone(), s.uid.as_str().to_owned())).collect()
}

#[test]
fn fresh_reads_of_one_file_give_the_same_identities() {
    let (_, a, b) = two_readers();
    assert_eq!(uids(a.model().document()), uids(b.model().document()));
    assert!(uids(a.model().document()).iter().all(|(_, uid)| !uid.is_empty()));
}
