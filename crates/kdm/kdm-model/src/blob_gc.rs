//! Which blobs a [`crate::VectorSession`] still needs, so the rest can be dropped (copied from
//! Kition Photo's `kdm-model` by way of Kition Slide's, same owner and licence; only
//! [`object_blobs`] is Vector's own).
//!
//! A blob is kept while the replica's current state references it, while an undo or redo step
//! this session can still take might bring it back, or while it waits to be imported (blobs a
//! peer sent before its changes). Everything else (the bytes of an image deleted and its undo
//! step trimmed from history) is garbage: without this a long session keeps every image it ever
//! held.
//!
//! The current state's references are counted per object, updated from the objects each
//! transaction, undo, redo or import changed, so nothing rescans the whole document. Each undo
//! step keeps the blobs its transaction displaced (to undo it) and added (to redo it after an
//! undo, even if a collaborator overwrote them meanwhile); a redo step keeps the same for the undo
//! that made it.

use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};

use kition_kit::kdm::{ObjectId, Replica, UndoScope};
use kition_kit::session::MemoryBlobs;
use serde_json::Value;

/// The blob hashes one replica operation stopped and started referencing.
#[derive(Default)]
pub(crate) struct Moved {
    pub displaced: HashSet<String>,
    pub added: HashSet<String>,
}

impl Moved {
    fn all(self) -> HashSet<String> {
        let mut all = self.displaced;
        all.extend(self.added);
        all
    }
}

/// How many imports a blob added ahead of its changes waits for them before it is garbage.
const PENDING_IMPORTS: u32 = 256;

/// One undo scope's steps.
#[derive(Default)]
struct Steps {
    /// What each undo step can bring back, oldest first.
    undo: VecDeque<HashSet<String>>,
    /// What each redo step can bring back, the next redo last.
    redo: Vec<HashSet<String>>,
}

fn slot(scope: UndoScope) -> usize {
    match scope {
        UndoScope::Person => 0,
        UndoScope::Agent => 1,
    }
}

#[derive(Default)]
pub(crate) struct BlobIndex {
    by_object: HashMap<ObjectId, Vec<String>>,
    refs: HashMap<String, u32>,
    /// The person's steps and the agents' (Kition platform KitModel 1.1 M12a: they undo
    /// separately, as the replica keeps them).
    steps: [Steps; 2],
    /// Blobs added for changes not imported yet, with how many imports they have waited.
    pending: HashMap<String, u32>,
}

/// Every `{"hash": …, "mime": …}` (a blob reference) inside `value`.
fn collect(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            if let (Some(Value::String(hash)), Some(Value::String(_))) = (map.get("hash"), map.get("mime")) {
                out.push(hash.clone());
            }
            for item in map.values() {
                collect(item, out);
            }
        }
        Value::Array(items) => items.iter().for_each(|item| collect(item, out)),
        _ => {}
    }
}

/// The blobs object `id` references (none if it no longer exists): an `Image` object's `blob`
/// and `proxy`, the only blob fields of `kit.vector`'s schema.
fn object_blobs(replica: &Replica, id: &ObjectId) -> Vec<String> {
    let mut out = Vec::new();
    if replica.exists(id) && matches!(replica.object_type(id).as_deref(), Ok("Image")) {
        for field in ["blob", "proxy"] {
            if let Ok(value) = replica.get(id, field) {
                collect(&value, &mut out);
            }
        }
    }
    out
}

impl BlobIndex {
    /// Indexes every object of a freshly loaded or created replica.
    pub fn build(replica: &Replica) -> Self {
        let mut index = Self::default();
        let all: BTreeSet<ObjectId> = replica.objects_of_type("Image").into_iter().collect();
        index.update(replica, &all);
        index
    }

    /// Re-reads the references of the `changed` objects.
    ///
    /// An object's field can list the same hash more than once (two tiles sharing one default
    /// pixel): `before` and `now` are counted as multisets, and only a hash's *net* change in
    /// how many times this one object lists it touches `refs`. Without that, a hash an object
    /// keeps listing (one tile painted while another of the same object is untouched) would
    /// drop to a transient zero and look "gained" again from nothing, even though nothing about
    /// it changed; that falsely marked it `Moved::added` for every later change to the object,
    /// not just the one that actually first added it.
    pub fn update(&mut self, replica: &Replica, changed: &BTreeSet<ObjectId>) -> Moved {
        let mut dropped = Vec::new();
        let mut gained = Vec::new();
        for id in changed {
            let now = object_blobs(replica, id);
            let before = if now.is_empty() { self.by_object.remove(id) } else { self.by_object.insert(id.clone(), now.clone()) }.unwrap_or_default();
            let mut before_counts: HashMap<&str, i64> = HashMap::new();
            for hash in &before {
                *before_counts.entry(hash.as_str()).or_insert(0) += 1;
            }
            let mut now_counts: HashMap<&str, i64> = HashMap::new();
            for hash in &now {
                *now_counts.entry(hash.as_str()).or_insert(0) += 1;
            }
            let mut hashes: BTreeSet<&str> = before_counts.keys().copied().collect();
            hashes.extend(now_counts.keys().copied());
            for hash in hashes {
                let delta = now_counts.get(hash).copied().unwrap_or(0) - before_counts.get(hash).copied().unwrap_or(0);
                if delta > 0 {
                    let count = self.refs.entry(hash.to_owned()).or_insert(0);
                    if *count == 0 {
                        gained.push(hash.to_owned());
                    }
                    *count += u32::try_from(delta).unwrap_or(u32::MAX);
                } else if delta < 0
                    && let Some(count) = self.refs.get_mut(hash)
                {
                    *count = count.saturating_sub(u32::try_from(-delta).unwrap_or(u32::MAX));
                    if *count == 0 {
                        self.refs.remove(hash);
                        dropped.push(hash.to_owned());
                    }
                }
            }
        }
        let mut moved = Moved::default();
        for hash in dropped {
            if !self.refs.contains_key(&hash) {
                moved.displaced.insert(hash);
            }
        }
        for hash in gained {
            if !moved.displaced.remove(&hash) {
                moved.added.insert(hash);
            }
        }
        moved
    }

    fn scope(&mut self, scope: UndoScope) -> &mut Steps {
        let [person, agent] = &mut self.steps;
        if slot(scope) == 0 { person } else { agent }
    }

    /// A transaction that became a new undo step of `scope`.
    pub fn push_step(&mut self, scope: UndoScope, moved: Moved) {
        let steps = self.scope(scope);
        steps.undo.push_back(moved.all());
        steps.redo.clear();
    }

    pub fn undone(&mut self, scope: UndoScope, moved: Moved) {
        let steps = self.scope(scope);
        steps.undo.pop_back();
        steps.redo.push(moved.all());
    }

    pub fn redone(&mut self, scope: UndoScope, moved: Moved) {
        let steps = self.scope(scope);
        steps.redo.pop();
        steps.undo.push_back(moved.all());
    }

    /// Drops the top step of `scope`'s undo (`undo`) or redo stack: one the replica passed over.
    pub fn drop_top(&mut self, scope: UndoScope, undo: bool) {
        let steps = self.scope(scope);
        if undo {
            steps.undo.pop_back();
        } else {
            steps.redo.pop();
        }
    }

    pub fn undo_steps(&self, scope: UndoScope) -> usize {
        self.steps.get(slot(scope)).map_or(0, |s| s.undo.len())
    }

    pub fn redo_steps(&self, scope: UndoScope) -> usize {
        self.steps.get(slot(scope)).map_or(0, |s| s.redo.len())
    }

    /// Every hash the current state references (not the extra copies undo, redo or a pending
    /// import keep around): what a peer starting from nothing needs.
    pub fn referenced(&self) -> Vec<String> {
        self.refs.keys().cloned().collect()
    }

    /// Keeps only the newest `undo` undo steps and the next `redo` redo steps of `scope`: the
    /// older ones can no longer be taken, so their blobs can go.
    pub fn retain(&mut self, scope: UndoScope, undo: usize, redo: usize) {
        let steps = self.scope(scope);
        while steps.undo.len() > undo {
            steps.undo.pop_front();
        }
        if steps.redo.len() > redo {
            let extra = steps.redo.len() - redo;
            steps.redo.drain(..extra);
        }
    }

    pub fn protect(&mut self, hashes: impl IntoIterator<Item = String>) {
        self.pending.extend(hashes.into_iter().map(|hash| (hash, 0)));
    }

    /// After an import: blobs the state now references are kept by that; the others go on waiting
    /// for their changes, which may arrive later than the blobs (a change imported before what it
    /// depends on waits too, and a host may hand over many blobs before replaying many changes),
    /// up to [`PENDING_IMPORTS`] imports.
    pub fn imported(&mut self) {
        let refs = &self.refs;
        self.pending.retain(|hash, waited| {
            *waited += 1;
            !refs.contains_key(hash) && *waited < PENDING_IMPORTS
        });
    }

    /// Whether the current state references `hash`.
    pub fn references(&self, hash: &str) -> bool {
        self.refs.contains_key(hash)
    }

    /// Drops every blob nothing above needs.
    pub fn collect_garbage(&self, blobs: &mut MemoryBlobs) {
        blobs.0.retain(|hash, _| {
            self.refs.contains_key(hash)
                || self.pending.contains_key(hash)
                || self.steps.iter().any(|steps| steps.undo.iter().chain(&steps.redo).any(|s| s.contains(hash)))
        });
    }
}
