//! C8-vector-5 item 7: two collaborators opening fresh replicas of the same file derive the same
//! persistent identities (`Document::settle_uids`); when both then create or rename named
//! entries, the replicas converge and no identity is held twice (issue-c8-identity).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use std::collections::HashSet;

use common::{SharedBlobs, canonical, drawing, edit, first, sync};
use kition_vector_doc::Document;
use kition_vector_doc::color::{Color, Paint, Swatch};

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

fn add(d: &mut Document, name: &str) {
    let before = d.clone();
    d.swatches.push(Swatch {
        uid: Default::default(),
        name: name.into(),
        paint: Paint::solid(Color::rgb(0.2, 0.4, 0.6)),
        global: false,
        spot: false,
    });
    d.settle_uids(Some(&before));
}

#[test]
fn fresh_reads_of_one_file_give_the_same_identities() {
    let (_, a, b) = two_readers();
    assert_eq!(uids(a.model().document()), uids(b.model().document()));
    assert!(uids(a.model().document()).iter().all(|(_, uid)| !uid.is_empty()));
}

#[test]
fn both_creating_the_same_name_at_the_same_place_converge_with_two_identities() {
    let blobs = SharedBlobs::default();
    let mut a = first(&settled(), &blobs);
    let mut b = common::peer(&a, 2, &blobs);
    edit(&mut a, &blobs, |d| add(d, "Fresh"));
    edit(&mut b, &blobs, |d| add(d, "Fresh"));
    // Each derived the same identity for its own `Fresh`.
    let mine = |s: &common::Session| uids(s.model().document()).into_iter().find(|(n, _)| n == "Fresh").unwrap().1;
    assert_eq!(mine(&a), mine(&b));
    sync(&mut a, &mut b);
    assert_eq!(a.state_hash().ok(), b.state_hash().ok(), "same replica state");
    assert_eq!(canonical(a.model().document()), canonical(b.model().document()), "same view");
    let all = uids(a.model().document());
    let fresh: Vec<&(String, String)> = all.iter().filter(|(n, _)| n == "Fresh").collect();
    assert_eq!(fresh.len(), 2, "{all:?}");
    let distinct: HashSet<&String> = all.iter().map(|(_, uid)| uid).collect();
    assert_eq!(distinct.len(), all.len(), "an identity held twice after the merge: {all:?}");
}

#[test]
fn a_rename_on_one_side_and_a_create_on_the_other_converge_and_keep_the_identity() {
    let blobs = SharedBlobs::default();
    let mut a = first(&settled(), &blobs);
    let mut b = common::peer(&a, 2, &blobs);
    let brand = uids(a.model().document()).into_iter().find(|(n, _)| n == "Brand").unwrap().1;
    edit(&mut a, &blobs, |d| {
        d.swatches.iter_mut().find(|s| s.name == "Brand").unwrap().name = "Brand Red".into();
    });
    edit(&mut b, &blobs, |d| add(d, "Brand"));
    sync(&mut a, &mut b);
    assert_eq!(a.state_hash().ok(), b.state_hash().ok());
    assert_eq!(canonical(a.model().document()), canonical(b.model().document()));
    let all = uids(a.model().document());
    assert!(all.contains(&("Brand Red".to_owned(), brand.clone())), "the renamed swatch keeps its identity: {all:?}");
    let distinct: HashSet<&String> = all.iter().map(|(_, uid)| uid).collect();
    assert_eq!(distinct.len(), all.len(), "{all:?}");
}
