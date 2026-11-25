//! The command bridge to each drawing's KDM replica, the same design as Kition Slide's, Film's
//! and Photo's.
//!
//! A document's replica ([`kdm_model::VectorSession`]) is its shared source of truth;
//! [`DocState::doc`] is the engine's own view of it. Commands, tools and keys run exactly as
//! before on the in-memory [`Document`]; at the end of every top-level call
//! ([`Session::execute`], and the interaction and undo-group ends tools and fields use) the bridge
//! writes what changed since the replica last saw it ([`Link::synced`]) as one transaction per
//! new history step, found by their serials ([`crate::HistoryEntry::serial`]): so nested
//! commands, the post-passes of [`Session::edit`] (text wrap, threads, Shaper, inline graphics,
//! opacity-mask sync, asset and binding pruning), direct writes, the panic guard's rollback and a
//! revert are all replicated, each in the transaction of the step they belong to. Edits made in an
//! undo group extend its step. A live interaction (a drag, typing) is written when it commits, as
//! the one step it is; while it lasts, the replica gets what was committed before it (its base),
//! and an import rebases it on the merged drawing and replays its preview.
//!
//! Undo is per person: each history step remembers how many replica undo steps it took
//! ([`crate::HistoryEntry::steps`]); undo and redo step the replica that many times and take the
//! replica's result (with every collaborator's edit since) as the document, or the engine's own
//! snapshot when the replica holds just that (no collaborator's edit came in). A step the history drops from the top (a command merging two steps into one, a
//! cancelled undo group, discarded empty type) hands its replica steps to the step under it, so
//! undoing that one undoes them too. The selection, the current layer, isolation, the tool, the
//! clipboard, the view, the opacity-mask and pattern editing layers, the active data set (and warp
//! pins) are this person's own: never written ([`kdm_model::shared_part`]), kept across imports, undo and redo.
//!
//! A replica that refuses a write or an undo is left ([`Session::kdm_resets`]): the document keeps
//! working from its own history snapshots, and the next edit makes a fresh replica.

use std::collections::HashSet;
use std::sync::Arc;

use kdm_model::VectorSession;
use kdm_model::kition_kit::kdm::{Origin, OriginKind, UndoScope, Version};
use kdm_model::kition_kit::session::{Damage, MemoryBlobs};
use kition_vector_doc::Document;

use crate::{DocState, EngineError, Result, Selection, Session};

/// The origin of the local person's commands (used when nothing else was set with
/// [`Session::execute_as`]/[`Session::set_origin`]).
pub fn local_person() -> Origin {
    Origin::person("person:local")
}

fn kdm_error(error: impl std::fmt::Display) -> EngineError {
    EngineError::Other(format!("drawing replica: {error}"))
}

fn no_replica() -> EngineError {
    EngineError::Other("the drawing has no replica and none could be made".into())
}

/// One document's bridge state.
#[derive(Debug)]
pub struct Link {
    /// The replica, made lazily (the first edit or passthrough); `None` again after a reset.
    pub(crate) kdm: Option<VectorSession>,
    /// The document as last written to (or read from) the replica, the editing person's own
    /// layers included.
    pub(crate) synced: Arc<Document>,
    pub(crate) resets: u64,
    /// [`crate::History::pushed`] when the bridge last looked.
    pub(crate) mark: u64,
    /// Every history step (serial and replica steps) as the bridge last left them, to find the
    /// steps a command dropped from the history.
    pub(crate) last: Vec<(u64, u32)>,
    /// `mark` and `last` of the other undo scope's history ([`DocState::use_scope`] swaps them
    /// with the histories).
    pub(crate) other: (u64, Vec<(u64, u32)>),
}

impl Link {
    pub(crate) fn new(doc: &Arc<Document>) -> Self {
        Self { kdm: None, synced: doc.clone(), resets: 0, mark: 0, last: Vec::new(), other: (0, Vec::new()) }
    }

    /// This link's replica and state, for the document that takes this one's place (File →
    /// Revert): it goes on as the same KDM document. This one is left without a replica.
    pub(crate) fn take_over(&mut self) -> Self {
        let synced = self.synced.clone();
        std::mem::replace(self, Self::new(&synced))
    }
}

/// A clone holds no replica (a replica has one owner): it makes a fresh one on its next edit.
impl Clone for Link {
    fn clone(&self) -> Self {
        Self { kdm: None, synced: self.synced.clone(), resets: self.resets, mark: self.mark, last: Vec::new(), other: (self.other.0, Vec::new()) }
    }
}

/// What a replica holds of `doc` ([`kdm_model::shared_part`]), as an `Arc` (the same one when
/// nothing is the editing person's own).
fn shared(doc: &Arc<Document>) -> Arc<Document> {
    match kdm_model::shared_part(doc) {
        std::borrow::Cow::Borrowed(_) => doc.clone(),
        std::borrow::Cow::Owned(d) => Arc::new(d),
    }
}

/// Puts the editing person's own state from `from` into `to`, a view of the replica: the
/// opacity-mask editing layer (while the masked object is still there), pattern editing (its tile
/// layer, while the pattern is still there), the active data set (while it is still there) and
/// warp pins (while their objects are).
fn carry_local(from: &Document, to: &mut Document) {
    if let Some(edit) = &from.pattern_edit
        && to.pattern_edit.is_none()
        && to.node(edit.layer).is_none()
        && to.patterns.iter().any(|p| p.name == edit.pattern)
        && let Some(at) = from.layers.iter().position(|l| l.id == edit.layer)
        && let Some(layer) = from.layers.get(at)
    {
        to.layers.insert(at.min(to.layers.len()), layer.clone());
        to.pattern_edit = Some(edit.clone());
    }
    if let Some(active) = &from.variables.active_dataset
        && to.variables.active_dataset.is_none()
        && to.variables.datasets.iter().any(|d| &d.name == active)
    {
        to.variables.active_dataset = Some(active.clone());
    }
    if let Some(me) = from.mask_edit
        && to.mask_edit.is_none()
        && to.node(me.object).is_some()
        && to.node(me.layer).is_none()
        && let Some(at) = from.layers.iter().position(|l| l.id == me.layer)
        && let Some(layer) = from.layers.get(at)
    {
        to.layers.insert(at.min(to.layers.len()), layer.clone());
        to.mask_edit = Some(me);
    }
    if let Some(pins) = &from.pin_warp
        && pins.ids.iter().all(|id| to.node(*id).is_some())
    {
        // The pins hold while their objects are the very nodes the last warp left
        // (`WarpPins::holds`); a view's nodes are new, so where one is that node unchanged the
        // pins take it.
        let mut pins = pins.clone();
        let pins_mut = Arc::make_mut(&mut pins);
        for (id, result) in pins_mut.ids.iter().zip(pins_mut.result.iter_mut()) {
            if let Some(node) = to.node_arc(*id)
                && from.node_arc(*id).is_some_and(|mine| Arc::ptr_eq(mine, result))
                && **node == **result
            {
                *result = node.clone();
            }
        }
        to.pin_warp = Some(pins);
    }
}

/// The replica's view as the document: with an id counter never below `floor`, the editing
/// person's own state from `local`, and text layout bounds where they are missing.
fn view_of(kdm: &VectorSession, floor: u64, local: &Document) -> Arc<Document> {
    let mut d = (**kdm.document()).clone();
    d.raise_next_id(floor);
    carry_local(local, &mut d);
    Session::refresh_text_bounds(&mut d);
    Arc::new(d)
}

impl DocState {
    /// The document's replica, if it has made one.
    pub fn kdm(&self) -> Option<&VectorSession> {
        self.link.kdm.as_ref()
    }

    /// How many times the document left its replica (a write or undo the replica refused).
    pub fn kdm_resets(&self) -> u64 {
        self.link.resets
    }

    /// The document as committed: the base of an interaction in progress, else the document.
    fn committed(&self) -> Arc<Document> {
        self.interaction.as_ref().map_or_else(|| self.doc.clone(), |it| it.doc.clone())
    }

    /// The replica steps of the history steps the bridge accounted that are gone from the history
    /// from the top (newer than the oldest one still there): they go to the step now on top.
    /// Steps dropped from the bottom (the history limit) are simply out of reach.
    fn dropped_steps(&self) -> u32 {
        let present: HashSet<u64> = self.history.undo.iter().chain(&self.history.redo).map(|e| e.serial).collect();
        let Some(oldest) = self.link.last.iter().map(|(serial, _)| *serial).filter(|s| present.contains(s)).min() else { return 0 };
        self.link
            .last
            .iter()
            .filter(|(serial, _)| !present.contains(serial) && *serial > oldest)
            .map(|(_, steps)| *steps)
            .fold(0, u32::saturating_add)
    }

    /// Remembers the history as the bridge leaves it.
    fn mark(&mut self) {
        self.link.mark = self.history.pushed;
        self.link.last = self.history.undo.iter().chain(&self.history.redo).map(|e| (e.serial, e.steps)).collect();
    }

    /// Writes what the document committed since the replica last saw it: one transaction per
    /// history step pushed since the last look; a change that pushed none belongs to the top step
    /// (a `system` one right after an undo, so the redo steps survive).
    pub(crate) fn sync_kdm(&mut self, origin: &Origin) {
        let current = self.committed();
        let dropped = self.dropped_steps();
        let pushed = self.history.pushed_since(self.link.mark);
        if pushed.is_empty() && dropped == 0 && Arc::ptr_eq(&self.link.synced, &current) {
            self.mark();
            return;
        }
        if pushed.is_empty() {
            let mut steps = dropped;
            if !Arc::ptr_eq(&self.link.synced, &current) {
                let origin = if self.history.redo.is_empty() { origin.clone() } else { Origin::system("view") };
                steps = steps.saturating_add(self.record_kdm(&origin, &current).unwrap_or(0));
            }
            if steps > 0
                && let Some(top) = self.history.undo.last_mut()
            {
                top.steps = top.steps.saturating_add(steps);
            }
        } else {
            if dropped > 0
                && let Some(e) = pushed.start.checked_sub(1).and_then(|i| self.history.undo.get_mut(i))
            {
                e.steps = e.steps.saturating_add(dropped);
            }
            for i in pushed {
                // What changed before this step (a direct write between commands) is no part of
                // it: it goes first, to the step under it, so undoing this step keeps it.
                if let Some(before) = self.history.undo.get(i).map(|e| e.doc.clone())
                    && !Arc::ptr_eq(&self.link.synced, &before)
                {
                    let Some(steps) = self.record_kdm(origin, &before) else { break };
                    if steps > 0
                        && let Some(below) = i.checked_sub(1).and_then(|j| self.history.undo.get_mut(j))
                    {
                        below.steps = below.steps.saturating_add(steps);
                    }
                }
                let after = self.history.undo.get(i + 1).map(|e| e.doc.clone()).unwrap_or_else(|| current.clone());
                let mut steps = 0;
                if !Arc::ptr_eq(&self.link.synced, &after) {
                    match self.record_kdm(origin, &after) {
                        Some(n) => steps = n,
                        None => break,
                    }
                }
                if let Some(e) = self.history.undo.get_mut(i) {
                    e.steps = steps;
                }
            }
        }
        self.mark();
        self.trim_kdm();
    }

    /// Writes `after` on top of the replica (made now, baselined on [`Link::synced`], if there is
    /// none) and answers the undo steps that took; `None` when the replica refused and was left.
    fn record_kdm(&mut self, origin: &Origin, after: &Arc<Document>) -> Option<u32> {
        if self.link.kdm.is_none() {
            match VectorSession::create(&shared(&self.link.synced)) {
                Ok(kdm) => self.link.kdm = Some(kdm),
                Err(error) => {
                    log::warn!("the drawing can't be held in a replica: {error}");
                    self.link.synced = after.clone();
                    return Some(0);
                }
            }
        }
        let kdm = self.link.kdm.as_mut()?;
        match kdm.record(origin, &shared(after)) {
            Ok(steps) => {
                self.link.synced = after.clone();
                Some(steps)
            }
            Err(error) => {
                log::warn!("the drawing replica refused an edit: {error}");
                self.detach_kdm();
                None
            }
        }
    }

    /// The replica, made now from the committed document if there is none yet (anything not
    /// written yet is written first; a live interaction stays for its commit).
    pub(crate) fn ensure_kdm(&mut self, origin: &Origin) -> Option<&mut VectorSession> {
        self.sync_kdm(origin);
        if self.link.kdm.is_none() {
            let current = self.committed();
            match VectorSession::create(&shared(&current)) {
                Ok(kdm) => {
                    self.link.kdm = Some(kdm);
                    self.link.synced = current;
                }
                Err(_) => return None,
            }
        }
        self.link.kdm.as_mut()
    }

    /// Leaves the replica: every step's replica steps were the dropped replica's, and a fresh
    /// one never holds them.
    pub(crate) fn detach_kdm(&mut self) {
        self.link.kdm = None;
        self.link.resets = self.link.resets.saturating_add(1);
        self.link.synced = self.committed();
        for history in [&mut self.history, &mut self.other_history] {
            for e in history.undo.iter_mut().chain(history.redo.iter_mut()) {
                e.steps = 0;
            }
        }
        self.mark();
    }

    /// Lets the replica forget the steps the history can no longer take, and their blobs.
    pub(crate) fn trim_kdm(&mut self) {
        let sum = |v: &[crate::HistoryEntry]| v.iter().map(|e| e.steps as usize).sum::<usize>();
        let (undo, redo) = (sum(&self.history.undo), sum(&self.history.redo));
        let (other_undo, other_redo) = (sum(&self.other_history.undo), sum(&self.other_history.redo));
        if let Some(kdm) = self.link.kdm.as_mut() {
            kdm.retain_steps(self.history_scope, undo, redo);
            kdm.retain_steps(other_scope(self.history_scope), other_undo, other_redo);
        }
    }

    /// Makes `history` the one of `scope`.
    pub(crate) fn use_scope(&mut self, scope: UndoScope) {
        if self.history_scope != scope {
            std::mem::swap(&mut self.history, &mut self.other_history);
            std::mem::swap(&mut self.link.mark, &mut self.link.other.0);
            std::mem::swap(&mut self.link.last, &mut self.link.other.1);
            self.history_scope = scope;
        }
    }

    /// After undo or redo set the document to the step's own snapshot `reference`: steps the
    /// replica too and takes its view (with every collaborator's edit since) instead, or the
    /// snapshot itself when the replica holds just that (nobody else's edit came in since the
    /// step; always so for a drawing nobody shares): then undo is exactly what it is without a
    /// replica (the snapshot's shared nodes, caches, numbers as written and id counter), and
    /// undoing back to the saved document leaves it clean. `floor` is the document's id counter
    /// before, which a view never lowers.
    pub(crate) fn step_kdm(&mut self, steps: u32, undo: bool, floor: u64, reference: &Arc<Document>) {
        if steps > 0
            && let Some(kdm) = self.link.kdm.as_mut()
        {
            match take_steps(kdm, self.history_scope, undo, steps) {
                // Only a replica that refuses is left.
                None => {
                    self.detach_kdm();
                    return;
                }
                // The replica had fewer steps to take (it passed over steps whose objects are gone
                // since): the history step, now on the other stack, counts those it took.
                Some(taken) if taken != steps => {
                    let moved = if undo { self.history.redo.last_mut() } else { self.history.undo.last_mut() };
                    if let Some(e) = moved {
                        e.steps = taken;
                    }
                }
                Some(_) => {}
            }
        }
        if let Some(kdm) = self.link.kdm.as_mut() {
            self.doc = if kdm.adopt_equal(&shared(reference)) { reference.clone() } else { view_of(kdm, floor, reference) };
        }
        self.link.synced = self.doc.clone();
        self.mark();
        self.trim_kdm();
    }

    /// After an import: the document follows the replica (an interaction in progress rebased on
    /// it), and the selection and panel state let go of what is gone.
    fn follow_kdm(&mut self) {
        let Some(kdm) = self.link.kdm.as_ref() else { return };
        let view = view_of(kdm, self.doc.peek_next_id(), &self.doc);
        self.link.synced = view.clone();
        if let Some(it) = &mut self.interaction {
            it.doc = view.clone();
            it.selection.prune(&view);
        }
        self.doc = view;
        self.selection.prune(&self.doc);
        if self.isolation.is_some_and(|i| self.doc.node(i).is_none()) {
            self.isolation = None;
        }
        let doc = &self.doc;
        self.layer_rows.retain(|id| doc.node(*id).is_some());
        self.revision += 1;
    }
}

impl Session {
    /// Who the running (or next) command acts for: the origin of [`Session::execute_as`], or the
    /// local person.
    pub fn origin(&self) -> Origin {
        self.origin.clone().unwrap_or_else(local_person)
    }

    /// Sets who the following commands act for (`None`: the local person); answers the previous
    /// setting, to put back.
    pub fn set_origin(&mut self, origin: Option<Origin>) -> Option<Origin> {
        let previous = std::mem::replace(&mut self.origin, origin);
        let scope = scope_of(&self.origin());
        for st in &mut self.docs {
            st.use_scope(scope);
        }
        previous
    }

    /// Runs `step` (an undo or a redo) in `scope` (`None`: the caller's, engine rule E14a).
    pub(crate) fn in_scope<T>(&mut self, scope: Option<UndoScope>, step: impl FnOnce(&mut Self) -> T) -> T {
        let own = scope_of(&self.origin());
        let scope = scope.unwrap_or(own);
        for st in &mut self.docs {
            st.use_scope(scope);
        }
        let done = step(self);
        for st in &mut self.docs {
            st.use_scope(own);
        }
        done
    }

    /// [`Session::execute`] as `origin` (a person, an agent): the replica records the edit as
    /// theirs, and it undoes as this replica's own step.
    pub fn execute_as(&mut self, id: &str, params: &serde_json::Value, origin: &Origin) -> Result<serde_json::Value> {
        let previous = self.set_origin(Some(origin.clone()));
        let r = self.execute(id, params);
        self.set_origin(previous);
        r
    }

    /// After a top-level call: replicates every document's changes.
    pub(crate) fn sync_all_kdm(&mut self) {
        let origin = self.origin();
        for st in &mut self.docs {
            st.sync_kdm(&origin);
        }
    }

    /// The active document's replica, if it has made one (never makes one).
    pub fn kdm(&self) -> Option<&VectorSession> {
        self.active()?.kdm()
    }

    /// How many times the active document left its replica. Hosts watch this: the next edit's
    /// fresh replica is a new KDM document collaborators must join from its snapshot.
    pub fn kdm_resets(&self) -> u64 {
        self.active().map_or(0, DocState::kdm_resets)
    }

    /// The active document's KDM identity, `None` before it has made a replica.
    pub fn doc_id(&self) -> Option<String> {
        Some(self.kdm()?.doc_id().to_string())
    }

    fn active_kdm(&mut self) -> Result<&mut VectorSession> {
        let origin = self.origin();
        self.doc_mut()?.ensure_kdm(&origin).ok_or_else(no_replica)
    }

    /// The active document's replica version.
    pub fn version(&mut self) -> Result<Version> {
        Ok(self.active_kdm()?.version())
    }

    /// The active document's replica as a snapshot, for a peer ([`Session::open_snapshot`]).
    pub fn snapshot(&mut self) -> Result<Vec<u8>> {
        self.active_kdm()?.snapshot().map_err(kdm_error)
    }

    /// SHA-256 of the active document's replicated state: equal on replicas that converged.
    pub fn state_hash(&mut self) -> Result<String> {
        self.active_kdm()?.state_hash().map_err(kdm_error)
    }

    /// The active document's changes since `since` (`Version::empty()` for all of them).
    pub fn export_changes(&mut self, since: &Version) -> Result<Vec<u8>> {
        self.active_kdm()?.export_changes(since).map_err(kdm_error)
    }

    /// Every blob the active document's replica references or can still bring back.
    pub fn blobs(&mut self) -> Result<MemoryBlobs> {
        Ok(self.active_kdm()?.blobs().clone())
    }

    /// Adds a peer's blobs (before importing its changes): kept until the next import.
    pub fn add_blobs(&mut self, blobs: &MemoryBlobs) -> Result<()> {
        // Bytes that arrived after the changes that need them reload the images showing them.
        if self.active_kdm()?.add_blobs(blobs) {
            self.doc_mut()?.follow_kdm();
        }
        Ok(())
    }

    /// The blobs a peer at `since` needs (`Version::empty()`: every blob the current state
    /// references), so a live change ships only its own images.
    pub fn blobs_since(&mut self, since: &Version) -> Result<MemoryBlobs> {
        Ok(MemoryBlobs(self.active_kdm()?.blobs_since(since).into_iter().collect()))
    }

    /// Imports a collaborator's (or stored) changes into the active document's replica (their
    /// blobs added first, [`Session::add_blobs`]). Local edits not written yet are written first;
    /// a drag in progress goes on from the merged drawing. The document and its selection follow
    /// the replica; one whose view could not be loaded from it leaves it (a reset) rather than
    /// diff against it later.
    pub fn import_changes(&mut self, bytes: &[u8]) -> Result<Damage> {
        let kdm = self.active_kdm()?;
        let before = kdm.version();
        let result = kdm.import_changes(bytes);
        let changed = kdm.version() != before;
        let follow = result.is_ok() || (changed && kdm.view_loaded());
        let st = self.doc_mut()?;
        if follow {
            st.follow_kdm();
        } else if changed {
            st.detach_kdm();
        }
        st.trim_kdm();
        let preview = st.interaction.as_ref().and_then(|it| it.preview.clone());
        if follow && let Some((cmd, params)) = preview {
            // The drag's own change, on top of the merged drawing.
            if self.preview(&cmd, &params).is_err() {
                self.cancel_interaction()?;
            }
        }
        result.map_err(kdm_error)
    }

    /// Takes a draft's changes (made on a replica forked from this one, `mergeChanges`) into the
    /// active document as one undo step of the person, as a command's edit is, even when an agent
    /// made the draft (the person accepted it, KitModel M12a, as the replica records it): local edits
    /// not written yet are written first, a gesture in progress is cut short, the document and its
    /// selection follow the replica, and the step undoes the whole draft. Add the draft's blobs
    /// first ([`Session::add_blobs`]). A refused merge changes nothing.
    pub fn merge_changes(&mut self, bytes: &[u8], origin: &Origin) -> Result<Damage> {
        self.take_in(UndoScope::Person, "Merge Draft", |kdm| kdm.merge_changes(bytes, origin))
    }

    /// Writes the state a snapshot holds (a checkpoint of this drawing, `replica/restore`) over the
    /// active document by object id, as one undo step of `origin`'s scope, taken in as a merged
    /// draft is ([`Session::merge_changes`]). Nothing changes when the states are equal.
    pub fn restore_state(&mut self, snapshot: &[u8], origin: &Origin) -> Result<Damage> {
        self.take_in(scope_of(origin), "Restore Version", |kdm| kdm.restore_state(snapshot, origin))
    }

    /// One transaction `write` makes on the active document's replica, taken into the document as
    /// a command's step: the view follows the replica, and the history of `scope` gets the step
    /// (its undo step the replica's, which the replica records in the same scope) when the
    /// transaction is one.
    fn take_in(
        &mut self,
        scope: UndoScope,
        label: &str,
        write: impl FnOnce(&mut VectorSession) -> kdm_model::kition_kit::session::Result<(u32, Damage)>,
    ) -> Result<Damage> {
        // A gesture cut short belongs to the step on top, as for undo.
        self.doc_mut()?.undo_interaction();
        let kdm = self.active_kdm()?;
        let before_version = kdm.version();
        let result = write(&mut *kdm);
        let changed = kdm.version() != before_version;
        let follow = changed && (result.is_ok() || kdm.view_loaded());
        let st = self.doc_mut()?;
        let (before, selection) = (st.doc.clone(), st.selection.clone());
        if follow {
            st.follow_kdm();
        } else if changed {
            st.detach_kdm();
        }
        let steps = result.as_ref().map_or(0, |(steps, _)| *steps);
        if follow && steps > 0 {
            let own = st.history_scope;
            st.use_scope(scope);
            let mut entry = crate::HistoryEntry::new(label, before, selection);
            entry.steps = steps;
            st.push_undo(entry);
            // The step is written already: the bridge has nothing of it left to record.
            st.mark();
            st.use_scope(own);
        }
        st.trim_kdm();
        result.map(|(_, damage)| damage).map_err(kdm_error)
    }

    /// Opens another replica's snapshot (with the blobs it references) as a new document, a
    /// replica of the same KDM document under a fresh actor, and makes it active: clean, with an
    /// empty history.
    pub fn open_snapshot(&mut self, bytes: &[u8], blobs: MemoryBlobs) -> Result<usize> {
        let kdm = VectorSession::open_snapshot(bytes, blobs).map_err(kdm_error)?;
        let index = self.add_document((**kdm.document()).clone(), None);
        let st = self.doc_mut()?;
        st.link.kdm = Some(kdm);
        st.link.synced = st.doc.clone();
        st.mark_saved();
        Ok(index)
    }

    /// Reads the active document's file again while the document is clean (kition-sdk
    /// `KitDocument::reconcile_reread`): `fresh` is the file as a new session opened it. Only what
    /// differs from the document is written to its replica, as one `import` transaction outside
    /// both undo histories, so the person's and the agents' steps stay undoable and undoing one
    /// keeps the file's edits. Object ids, named entries' identities, the editor state and the
    /// blobs stay, and the document is saved. Answers `false`, changing nothing, while an
    /// interaction or an undo group is open. On an error the session is as it was.
    pub fn reread_file(&mut self, fresh: &Document) -> Result<bool> {
        let origin = self.origin();
        let st = self.doc_mut()?;
        if st.interaction.is_some() || st.undo_group.is_some() {
            return Ok(false);
        }
        let mut after = fresh.clone();
        after.raise_next_id(st.doc.peek_next_id());
        let after = shared(&Arc::new(after));
        let current = shared(&st.doc);
        let kdm = st.ensure_kdm(&origin).ok_or_else(no_replica)?;
        let before = kdm.version();
        kdm.record(&Origin::new(OriginKind::Import, "file reread"), &after).map_err(kdm_error)?;
        if kdm.version() == before {
            // The file holds the document: it stays the very same one (its shared nodes, its
            // caches), as the replica's view.
            kdm.adopt_equal(&current);
        } else {
            st.follow_kdm();
        }
        st.mark_saved();
        st.trim_kdm();
        Ok(true)
    }

    /// [`Session::open_snapshot`] for a host that keeps a document's replica beside its saved
    /// file. The file is what the person saved: when `file` is not empty and the replica does not
    /// hold its content (id-blind), the file wins, recorded onto the replica as one `import`
    /// transaction (not an undo step) under the replica's own identity, so collaborators get it as
    /// ordinary changes. The document opens clean either way. `name` gives the file's format.
    pub fn open_with_replica(&mut self, name: &str, file: &[u8], snapshot: &[u8], blobs: MemoryBlobs) -> Result<usize> {
        let loaded = if file.is_empty() { None } else { Some(crate::cmd::fileio::load(name, file)?.doc) };
        let index = self.open_snapshot(snapshot, blobs)?;
        let st = self.doc_mut()?;
        if let Some(mut doc) = loaded {
            doc.adopt_guides();
            doc.drop_edit_modes();
            // As a file opens (`Session::add_document`): with each named entry's identity, so a
            // file without them is compared with the replica as it opens (C8-vector-5).
            doc.settle_uids(None);
            let kdm = st.link.kdm.as_mut().ok_or_else(no_replica)?;
            if !kdm.holds(&doc).map_err(kdm_error)? {
                doc.raise_next_id(st.doc.peek_next_id());
                Session::refresh_text_bounds(&mut doc);
                let doc = Arc::new(doc);
                kdm.record(&Origin::system("import"), &doc).map_err(kdm_error)?;
                st.doc = doc.clone();
                st.saved_doc = doc.clone();
                st.link.synced = doc;
                st.selection = Selection::default();
                st.revision += 1;
            }
        }
        Ok(index)
    }
}

/// Takes `steps` of `scope`'s replica steps back (`undo`) or forward and answers how many the
/// replica took off its stack, counting the steps a call passed over (they change nothing any
/// more), at most `steps`; fewer when the replica had no more. `None` when the replica refused.
fn take_steps(kdm: &mut VectorSession, scope: UndoScope, undo: bool, steps: u32) -> Option<u32> {
    let mut taken = 0u32;
    while taken < steps {
        match kdm.step_in(scope, undo) {
            Err(_) => return None,
            Ok((0, _)) => break,
            Ok((n, _)) => taken = taken.saturating_add(u32::try_from(n).unwrap_or(u32::MAX)),
        }
    }
    Some(taken.min(steps))
}

/// The undo scope `origin`'s steps join (Kition platform KitModel 1.1 M12a): the agents' for an
/// agent, else the person's (imports and repairs are no undo steps and record nothing in a history).
pub(crate) fn scope_of(origin: &Origin) -> UndoScope {
    UndoScope::of(origin.kind).unwrap_or(UndoScope::Person)
}

fn other_scope(scope: UndoScope) -> UndoScope {
    match scope {
        UndoScope::Person => UndoScope::Agent,
        UndoScope::Agent => UndoScope::Person,
    }
}
