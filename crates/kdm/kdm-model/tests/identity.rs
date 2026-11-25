//! C8-vector-5 item 7: two collaborators opening fresh replicas of the same file derive the same
//! persistent identities (`Document::settle_uids`); when a draft and the person meanwhile both
//! create or rename named entries, the draft merges and no identity is held twice
//! (issue-c8-identity).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use std::collections::HashSet;

use common::{SharedBlobs, draft, drawing, edit, first, merge};
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

#[test]
fn fresh_reads_of_one_file_give_the_same_identities() {
    let (_, a, b) = two_readers();
    assert_eq!(uids(a.model().document()), uids(b.model().document()));
    assert!(uids(a.model().document()).iter().all(|(_, uid)| !uid.is_empty()));
}

/// Adds a swatch named `name` of colour `color`, with the identity a command gives it.
fn add(d: &mut Document, name: &str, color: Color) {
    let before = d.clone();
    d.swatches.push(Swatch { uid: Default::default(), name: name.into(), paint: Paint::solid(color), global: false, spot: false });
    d.settle_uids(Some(&before));
}

/// The swatches as (name, colour, identity), in their order.
fn swatches(d: &Document) -> Vec<(String, Paint, String)> {
    d.swatches.iter().map(|s| (s.name.clone(), s.paint.clone(), s.uid.as_str().to_owned())).collect()
}

/// A draft adds a swatch "Fresh" while the person adds one of that name at the same place: each
/// derives the same identity for its own, the draft merges, the person's comes first (data model
/// 6.6 D6) and keeps its identity, and the draft's gets another.
#[test]
fn a_draft_adding_a_swatch_named_like_one_the_person_added_meanwhile_merges_with_two_identities() {
    let blobs = SharedBlobs::default();
    let mut a = first(&settled(), &blobs);
    let (mut b, base) = draft(&a, 2, &blobs);
    let (mine, theirs) = (Color::rgb(0.2, 0.4, 0.6), Color::rgb(0.6, 0.4, 0.2));
    edit(&mut b, &blobs, |d| add(d, "Fresh", theirs));
    edit(&mut a, &blobs, |d| add(d, "Fresh", mine));
    let fresh = |s: &common::Session| uids(s.model().document()).into_iter().find(|(n, _)| n == "Fresh").unwrap().1;
    let key = fresh(&a);
    assert_eq!(fresh(&b), key, "each derived the same identity for its own");
    let before = swatches(a.model().document());
    merge(&b, &base, &mut a).unwrap();
    let after = swatches(a.model().document());
    assert_eq!(after.len(), before.len() + 1, "{after:?}");
    assert_eq!(after[..before.len()], before[..], "the person's swatches stay as they were: {after:?}");
    let (name, paint, uid) = after.last().unwrap();
    assert_eq!((name.as_str(), paint), ("Fresh", &Paint::solid(theirs)), "the draft's comes after the person's");
    assert!(!uid.is_empty() && *uid != key, "the draft's gets another identity: {after:?}");
    let all = uids(a.model().document());
    let distinct: HashSet<&String> = all.iter().map(|(_, uid)| uid).collect();
    assert_eq!(distinct.len(), all.len(), "an identity held twice after the merge: {all:?}");
}

/// A draft adds a swatch "Brand" while the person renames "Brand" to "Brand Red": the draft
/// merges, the renamed swatch keeps its identity and no identity is held twice.
#[test]
fn a_draft_creating_a_name_the_person_renamed_away_meanwhile_merges_and_keeps_the_identity() {
    let blobs = SharedBlobs::default();
    let mut a = first(&settled(), &blobs);
    let brand = uids(a.model().document()).into_iter().find(|(n, _)| n == "Brand").unwrap().1;
    let (mut b, base) = draft(&a, 2, &blobs);
    edit(&mut b, &blobs, |d| add(d, "Brand", Color::rgb(0.1, 0.1, 0.9)));
    edit(&mut a, &blobs, |d| {
        d.swatches.iter_mut().find(|s| s.name == "Brand").unwrap().name = "Brand Red".into();
    });
    merge(&b, &base, &mut a).unwrap();
    let all = uids(a.model().document());
    assert!(all.contains(&("Brand Red".to_owned(), brand.clone())), "the renamed swatch keeps its identity: {all:?}");
    assert!(all.iter().any(|(name, uid)| name == "Brand" && *uid != brand), "the draft's swatch is there: {all:?}");
    let distinct: HashSet<&String> = all.iter().map(|(_, uid)| uid).collect();
    assert_eq!(distinct.len(), all.len(), "{all:?}");
}
