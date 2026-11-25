//! [`VectorSession`]: one drawing's replica, its [`VectorModel`] view and its blobs, for the
//! engine's command bridge.
//!
//! It is `kition_kit::session::KitSession<VectorModel>`'s flow (transactions, per-actor undo, the
//! change exchange, the post-merge repair through [`KitModel::validate`], the view patched through
//! [`KitModel::apply`]) on concrete parts, as Kition Photo's and Slide's sessions do: the engine
//! must enumerate, ship and garbage-collect the image blobs, which `KitSession`'s boxed blob store
//! does not allow. Edits are diff-bridged: the engine changes its own [`Document`], and
//! [`VectorSession::record`] writes what changed as one transaction ([`write_changes`]), then
//! adopts it as the view. Everything here is `Send`. The documents it takes are the
//! [`crate::shared_part`] of the engine's (the caller strips the editing person's own layers).

use std::collections::BTreeSet;
use std::sync::Arc;

use kition_kit::kdm::{ActorId, DocId, KdmError, ObjectId, Origin, Replica, Tx, UndoScope, Version};
use kition_kit::session::{BlobSource, Damage, KitModel, MemoryBlobs};
use kition_vector_doc::Document;

use crate::blob_gc::{BlobIndex, Moved};
use crate::{Ids, Result, VectorModel, kit_err, normalize, read_document, write_changes, write_document};

/// One open drawing as a KDM replica and the engine's view of it. See the module docs.
pub struct VectorSession {
    replica: Replica,
    model: VectorModel,
    blobs: MemoryBlobs,
    /// Which blobs are still needed ([`crate::blob_gc`]); the rest are dropped after each change.
    index: BlobIndex,
    /// The hashes each record, import, undo and redo started referencing, with the version right
    /// after it, for [`VectorSession::blobs_since`] (chronological, so `since` cleanly splits it
    /// into "already has" and "needs"; trimmed with the store).
    blob_log: Vec<(Version, Vec<String>)>,
}

impl std::fmt::Debug for VectorSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VectorSession").field("doc", &self.replica.doc_id()).field("actor", &self.replica.actor()).finish_non_exhaustive()
    }
}

impl VectorSession {
    /// A new replica holding `document`, written in its creating transaction (not an undo step).
    /// The view is `document` itself, with its own local ids.
    pub fn create(document: &Arc<Document>) -> Result<Self> {
        let mut blobs = MemoryBlobs::default();
        let mut ids = Ids::default();
        let (mut replica, ()) = Replica::create_with(VectorModel::schema(), DocId::new(), ActorId::random(), &Origin::system("create"), |tx| {
            write_document(tx, document, &mut ids, &mut blobs).map(|_| ())
        })
        .map_err(kit_err)?;
        replica.take_changed();
        let index = BlobIndex::build(&replica);
        let model = VectorModel::from_parts(document.clone(), ids);
        Ok(Self { replica, model, blobs, index, blob_log: Vec::new() })
    }

    /// Opens a snapshot (or a full change export) of another replica as a new replica of a fresh
    /// actor. `blobs` must hold the blobs it references.
    pub fn open_snapshot(bytes: &[u8], blobs: MemoryBlobs) -> Result<Self> {
        let mut replica = Replica::load(VectorModel::schema(), bytes, ActorId::random()).map_err(kit_err)?;
        replica.take_changed();
        let model = VectorModel::load(&replica, &blobs)?;
        let mut index = BlobIndex::build(&replica);
        // Blobs handed over beside the snapshot may be for changes replayed next (a host's
        // stored commits): they wait like blobs a peer adds.
        index.protect(blobs.0.keys().cloned());
        let mut session = Self { replica, model, blobs, index, blob_log: Vec::new() };
        session.index.collect_garbage(&mut session.blobs);
        Ok(session)
    }

    /// The view: the drawing as of the last record, undo, redo or import.
    pub fn document(&self) -> &Arc<Document> {
        self.model.document()
    }

    pub fn model(&self) -> &VectorModel {
        &self.model
    }

    /// Writes what differs between the view and `after` as one transaction of `origin`, then
    /// makes `after` the view. Answers how many undo steps that made: 1, or 0 when nothing
    /// replicated changed or `origin` is a `system`/`import` one (never an undo step). On failure
    /// nothing is written and the view is unchanged.
    pub fn record(&mut self, origin: &Origin, after: &Arc<Document>) -> Result<u32> {
        let mut ids = self.model.ids().clone();
        let before = self.replica.version();
        let (model, blobs) = (&self.model, &mut self.blobs);
        let result = self.replica.transact(origin, |tx| write_changes(tx, model.document(), after, &mut ids, blobs).map(|_| ()));
        let changed = self.replica.take_changed();
        if let Err(error) = result {
            self.index.collect_garbage(&mut self.blobs);
            return Err(kit_err(error));
        }
        let moved = self.index.update(&self.replica, &changed);
        self.model.adopt(after.clone(), ids);
        let wrote = self.replica.version() != before;
        let scope = UndoScope::of(origin.kind).filter(|_| wrote);
        self.log_added(moved.added.iter().cloned());
        if let Some(scope) = scope {
            self.index.push_step(scope, moved);
        }
        self.index.collect_garbage(&mut self.blobs);
        Ok(u32::from(scope.is_some()))
    }

    /// Whether the replica holds `document`'s content, compared id-blind ([`normalize`]) as
    /// [`read_document`] reads both back. Errs if `document` cannot be written to a replica.
    pub fn holds(&self, document: &Document) -> Result<bool> {
        let mut blobs = MemoryBlobs::default();
        let mut ids = Ids::default();
        let (scratch, ()) = Replica::create_with(VectorModel::schema(), DocId::new(), ActorId::random(), &Origin::system("create"), |tx| {
            write_document(tx, document, &mut ids, &mut blobs).map(|_| ())
        })
        .map_err(kit_err)?;
        let (theirs, _) = read_document(&scratch, &blobs).map_err(kit_err)?;
        let (mine, _) = read_document(&self.replica, &self.blobs).map_err(kit_err)?;
        Ok(normalize(&theirs) == normalize(&mine))
    }

    /// Makes `document` the view when it holds exactly the view's content with the view's ids
    /// (see [`VectorModel::adopt_equal`]). Answers whether it did.
    pub fn adopt_equal(&mut self, document: &Arc<Document>) -> bool {
        self.model.adopt_equal(document)
    }

    /// Undoes this replica's last undo step in `scope` (never imports or repairs, and never the
    /// other scope's steps, Kition platform KitModel 1.1 M12a), repairs the result and patches the
    /// view. Answers whether there was one, and the damage.
    pub fn undo_in(&mut self, scope: UndoScope) -> Result<(bool, Damage)> {
        self.step_in(scope, true).map(|(steps, damage)| (steps > 0, damage))
    }

    pub fn redo_in(&mut self, scope: UndoScope) -> Result<(bool, Damage)> {
        self.step_in(scope, false).map(|(steps, damage)| (steps > 0, damage))
    }

    /// [`VectorSession::undo_in`] (`undo`) or [`VectorSession::redo_in`], answering how many of
    /// `scope`'s steps the replica took off its stack: the steps it passed over (they changed
    /// nothing any more) and the one it took back, or 0 when it took none. A caller that keeps a
    /// history of its own beside the replica's counts its steps with it.
    pub fn step_in(&mut self, scope: UndoScope, undo: bool) -> Result<(usize, Damage)> {
        let can = if undo { self.can_undo_in(scope) } else { self.can_redo_in(scope) };
        if !can {
            return Ok((0, Damage::None));
        }
        let outcome = if undo { self.replica.undo_in(scope) } else { self.replica.redo_in(scope) }.map_err(kit_err)?;
        let (damage, moved) = self.settle();
        let steps = if outcome.is_done() { outcome.steps() } else { 0 };
        if steps > 0 {
            self.log_added(moved.added.iter().cloned());
            // The steps passed over joined the one taken back: one step moves to the other stack.
            for _ in 1..steps {
                self.index.drop_top(scope, undo);
            }
            if undo {
                self.index.undone(scope, moved);
            } else {
                self.index.redone(scope, moved);
            }
        }
        self.index.collect_garbage(&mut self.blobs);
        Ok((steps, damage?))
    }

    /// Whether `scope` has an undo step to take: one of this replica's, not trimmed by
    /// [`VectorSession::retain_steps`].
    pub fn can_undo_in(&self, scope: UndoScope) -> bool {
        self.replica.can_undo_in(scope) && self.index.undo_steps(scope) > 0
    }

    pub fn can_redo_in(&self, scope: UndoScope) -> bool {
        self.replica.can_redo_in(scope) && self.index.redo_steps(scope) > 0
    }

    /// Keeps only the newest `undo` undo steps and the next `redo` redo steps of `scope` reachable
    /// (the engine history's own) and drops the blobs only older steps needed: the replica has no
    /// limit of its own, so this is what keeps a long session's images bounded.
    pub fn retain_steps(&mut self, scope: UndoScope, undo: usize, redo: usize) {
        self.index.retain(scope, undo, redo);
        self.index.collect_garbage(&mut self.blobs);
        let blobs = &self.blobs;
        self.blob_log.retain(|(_, hashes)| hashes.iter().any(|hash| blobs.0.contains_key(hash)));
    }

    /// Undo and redo steps of `scope` reachable, as [`VectorSession::retain_steps`] left them.
    pub fn steps(&self, scope: UndoScope) -> (usize, usize) {
        (self.index.undo_steps(scope), self.index.redo_steps(scope))
    }

    /// Imports remote or stored changes (their blobs added first, [`VectorSession::add_blobs`]),
    /// repairs the merged state and patches the view. An import whose repair or view load fails
    /// has still imported its changes: [`VectorSession::view_loaded`] tells whether the view
    /// followed.
    pub fn import_changes(&mut self, bytes: &[u8]) -> Result<Damage> {
        self.replica.import_changes(bytes).map_err(kit_err)?;
        let (damage, moved) = self.settle();
        self.log_added(moved.added.iter().cloned());
        self.index.imported();
        self.index.collect_garbage(&mut self.blobs);
        damage
    }

    /// Takes a draft's changes (made on a fork of this replica) in as one transaction of `origin`
    /// (`Replica::merge_changes`): one undo step of its scope unless `origin` is a `system` or
    /// `import` one. Repairs the result and patches the view as an import does; add the draft's
    /// blobs first ([`VectorSession::add_blobs`]). Answers the undo steps it made (1 or 0) and the
    /// damage. A refused merge changes nothing.
    pub fn merge_changes(&mut self, bytes: &[u8], origin: &Origin) -> Result<(u32, Damage)> {
        let before = self.replica.version();
        self.replica.merge_changes(bytes, origin).map_err(kit_err)?;
        self.took_in(&before, origin)
    }

    /// Writes the state `snapshot` holds (a snapshot or content of this drawing, as a host's
    /// checkpoint keeps it) over the replica as one transaction of `origin`, keyed by object id
    /// (`kition_kit::session::write_state`), so every object the two share keeps its id: one undo
    /// step of its scope unless `origin` is a `system` or `import` one. Nothing changes when the
    /// states are equal. Refuses another document's snapshot. Add the blobs it references first.
    pub fn restore_state(&mut self, snapshot: &[u8], origin: &Origin) -> Result<(u32, Damage)> {
        let schema = VectorModel::schema();
        let loaded = Replica::load(Arc::clone(&schema), snapshot, ActorId::random()).map_err(kit_err)?;
        if loaded.doc_id() != self.replica.doc_id() {
            return Err(kit_err(KdmError::WrongDocument { expected: self.replica.doc_id().to_string(), found: loaded.doc_id().to_string() }));
        }
        let (now, then) = (kition_kit::session::state(&self.replica)?, kition_kit::session::state(&loaded)?);
        if now == then {
            return Ok((0, Damage::None));
        }
        let before = self.replica.version();
        let result = self.replica.transact(origin, |tx| kition_kit::session::write_state(tx, &schema, &now, &then).map_err(crate::kdm_err));
        if let Err(error) = result {
            self.replica.take_changed();
            return Err(kit_err(error));
        }
        self.took_in(&before, origin)
    }

    /// After a merge or a restore wrote one transaction of `origin` since `before`: repairs and
    /// patches the view as an import does, and counts the transaction as an undo step of its
    /// scope. A failed repair or view load is the error, as an import's is: the transaction stays
    /// ([`VectorSession::view_loaded`] tells whether the view followed).
    fn took_in(&mut self, before: &Version, origin: &Origin) -> Result<(u32, Damage)> {
        let wrote = self.replica.version() != *before;
        let (damage, moved) = self.settle();
        self.log_added(moved.added.iter().cloned());
        let scope = UndoScope::of(origin.kind).filter(|_| wrote);
        if let Some(scope) = scope {
            self.index.push_step(scope, moved);
        }
        self.index.imported();
        self.index.collect_garbage(&mut self.blobs);
        damage.map(|damage| (u32::from(scope.is_some()), damage))
    }

    /// Whether the view matches the replica as it is now (its own read succeeds).
    pub fn view_loaded(&self) -> bool {
        read_document(&self.replica, &self.blobs).is_ok()
    }

    /// The blobs a peer at `since` needs to reach this session's current state: the hashes
    /// logged for a version `since` does not include, still in the store. An empty `since`
    /// answers every blob the current state references (a creating transaction or a snapshot is
    /// never logged).
    pub fn blobs_since(&self, since: &Version) -> Vec<(String, Arc<[u8]>)> {
        if *since == Version::empty() {
            return self.index.referenced().into_iter().filter_map(|hash| self.blobs.0.get(&hash).cloned().map(|bytes| (hash, bytes))).collect();
        }
        let mut seen = BTreeSet::new();
        let mut out = Vec::new();
        for (version, hashes) in &self.blob_log {
            if since.includes(version) {
                continue;
            }
            for hash in hashes {
                if seen.insert(hash.clone())
                    && let Some(bytes) = self.blobs.0.get(hash)
                {
                    out.push((hash.clone(), bytes.clone()));
                }
            }
        }
        out
    }

    fn log_added(&mut self, added: impl IntoIterator<Item = String>) {
        let added: Vec<String> = added.into_iter().collect();
        if !added.is_empty() {
            self.blob_log.push((self.replica.version(), added));
        }
    }

    pub fn export_changes(&self, since: &Version) -> Result<Vec<u8>> {
        self.replica.export_changes(since).map_err(kit_err)
    }

    pub fn version(&self) -> Version {
        self.replica.version()
    }

    pub fn snapshot(&self) -> Result<Vec<u8>> {
        self.replica.snapshot().map_err(kit_err)
    }

    pub fn state_hash(&self) -> Result<String> {
        self.replica.state_hash().map_err(kit_err)
    }

    pub fn doc_id(&self) -> DocId {
        self.replica.doc_id()
    }

    pub fn replica(&self) -> &Replica {
        &self.replica
    }

    /// The blobs the replica references or an undo, redo or pending import can still need.
    pub fn blobs(&self) -> &MemoryBlobs {
        &self.blobs
    }

    /// Adds blobs (from a peer, before importing its changes): kept until changes reference them
    /// (or for a while, see `BlobIndex::imported`). Blobs the state already references that were
    /// missing (they arrived after their changes) reload the images that show them; answers
    /// whether the view changed.
    pub fn add_blobs(&mut self, blobs: &MemoryBlobs) -> bool {
        let mut late = false;
        for (hash, bytes) in &blobs.0 {
            if !self.blobs.0.contains_key(hash) {
                late |= self.index.references(hash);
                self.blobs.0.insert(hash.clone(), bytes.clone());
            }
        }
        self.index.protect(blobs.0.keys().cloned());
        if !late {
            return false;
        }
        let images: BTreeSet<ObjectId> = self.replica.objects_of_type("Image").into_iter().collect();
        self.refresh(&images).is_ok()
    }

    /// As `KitSession`'s: repairs what changed in a `system` transaction (not an undo step), then
    /// patches the view with both; the view follows the replica even if the repair fails. Also
    /// answers which blobs moved (the index is updated either way).
    fn settle(&mut self) -> (Result<Damage>, Moved) {
        let mut changed = self.replica.take_changed();
        let repaired = if changed.is_empty() {
            Ok(())
        } else {
            let mut kit_error = None;
            let result = self.replica.transact(&Origin::system("repair"), |tx: &mut Tx<'_>| {
                VectorModel::validate(tx, &changed).map(|_| ()).map_err(|error| {
                    let message = error.to_string();
                    kit_error = Some(error);
                    KdmError::Engine(message)
                })
            });
            match (result, kit_error) {
                (_, Some(error)) => Err(error),
                (result, None) => result.map_err(kit_err),
            }
        };
        changed.extend(self.replica.take_changed());
        let moved = self.index.update(&self.replica, &changed);
        let damage = self.refresh(&changed).and_then(|damage| repaired.map(|()| damage));
        (damage, moved)
    }

    fn refresh(&mut self, changed: &BTreeSet<ObjectId>) -> Result<Damage> {
        if changed.is_empty() {
            return Ok(Damage::None);
        }
        let blobs: &dyn BlobSource = &self.blobs;
        self.model.apply(&self.replica, changed, blobs)
    }
}
