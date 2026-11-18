//! Kition's collaboration members (engine contract 2): the drawing as a KDM replica.
//!
//! The engine keeps its own `Session` (the command bridge to the replica,
//! `kition_vector_engine::kdm_bridge`), not a `kition_kit::session::KitSession` directly, so
//! `kition_kit::export_kit_session!` does not apply here; these members are added by hand with
//! the exact JS names and shapes `kition-sdk`'s `session::web` gives
//! (`crates/kition-kit/src/session/web.rs`), so Kition's host code is the same for every kit. The
//! wasm bindings live in [`crate::web`].
//!
//! Drawings embed images in the replica, so [`VectorDocument::blobs`],
//! [`VectorDocument::add_blobs`] and [`VectorDocument::blobs_since`] move real content, encoded the
//! same way Kition Photo's and Slide's do: `(64-char ASCII-hex sha256, u64 LE length, bytes) * N`.

use std::collections::HashMap;
use std::sync::Arc;

use kition_kit::KitDocument;
use kition_vector_engine::{Damage, MemoryBlobs, Origin, Session, Version};
use serde_json::{Value, json};

use crate::{VectorDocument, engine_error};

impl VectorDocument {
    /// Opens a file with the replica a host stored for it (the Kit UI protocol's `replica`
    /// capability: `.kvector` and SVG cannot carry their own replica): the drawing is the replica's, so
    /// it keeps its KDM identity across saves. The host vouches that the replica was saved with
    /// these bytes; when they disagree (an older save, a file edited outside the replica) the
    /// engine records the file on top as one import. A replica that does not open
    /// (damaged, a newer schema) is ignored and the file opens as [`VectorDocument::open`] opens
    /// it.
    ///
    /// Empty `bytes` mean there is no file yet, only the replica (a host opening a shared
    /// drawing for the first time from its sync room's snapshot, to save it as a new file):
    /// the drawing is the replica's alone.
    pub fn open_with_replica(name: &str, bytes: &[u8], snapshot: &[u8], blobs: &[u8]) -> Result<Self, String> {
        let restored = (|| -> Result<Session, String> {
            let blobs = decode_blobs(blobs)?;
            let mut session = crate::kit_session();
            // The new drawing is the active one, as after `open`.
            session.open_with_replica(name, bytes, snapshot, blobs).map_err(engine_error)?;
            Ok(session)
        })();
        match restored {
            Ok(session) => Ok(Self::from_session(session, name)),
            Err(_) => Self::open(name, bytes),
        }
    }

    /// Whether the drawing has made its replica yet: it makes one lazily, on the first
    /// shared edit or when asked for it ([`VectorDocument::version`] and the rest).
    pub fn has_replica(&self) -> bool {
        self.session.active().is_some_and(|state| state.kdm().is_some())
    }

    /// The replica's own document id (a `kition-sdk` `DocId`, stable across saves and across
    /// opening another replica of the same document — never Kition Vector's own `.kvector`
    /// path/name): Kit UI's generic runtime reads this for the Kit UI protocol's `replica/info`
    /// message. `None` (`undefined` in JS, through [`crate::web`]) before the drawing has
    /// made a replica yet ([`VectorDocument::has_replica`]).
    pub fn doc_id(&self) -> Option<String> {
        self.session.doc_id()
    }

    /// The drawing's replica version (empty before it has made one).
    pub fn version(&mut self) -> Result<Vec<u8>, String> {
        Ok(self.session.version().map_err(engine_error)?.encode())
    }

    /// The drawing's changes since `since` (empty bytes: every change).
    pub fn export_changes(&mut self, since: &[u8]) -> Result<Vec<u8>, String> {
        let since = if since.is_empty() { Version::empty() } else { Version::decode(since).map_err(engine_error)? };
        self.session.export_changes(&since).map_err(engine_error)
    }

    /// Imports a peer's (or stored) changes; add their blobs first with
    /// [`VectorDocument::add_blobs`]. Answers what went stale on screen.
    pub fn import_changes(&mut self, bytes: &[u8]) -> Result<Damage, String> {
        self.session.import_changes(bytes).map_err(engine_error)
    }

    /// The drawing's replica as a snapshot, for a peer to open as a new replica of the same
    /// drawing ([`VectorDocument::open`]'ing the saved bytes does not: `.kvector` carries
    /// no replica).
    pub fn snapshot(&mut self) -> Result<Vec<u8>, String> {
        self.session.snapshot().map_err(engine_error)
    }

    /// Undoes the drawing's last local step on its replica. `done` is `false`, not an error,
    /// when there is nothing to undo.
    pub fn undo(&mut self) -> Result<(bool, Damage), String> {
        let done = self.can_undo();
        if done {
            self.execute("edit.undo", json!({}))?;
        }
        Ok((done, if done { Damage::Full } else { Damage::None }))
    }

    pub fn redo(&mut self) -> Result<(bool, Damage), String> {
        let done = self.can_redo();
        if done {
            self.execute("edit.redo", json!({}))?;
        }
        Ok((done, if done { Damage::Full } else { Damage::None }))
    }

    pub fn can_undo(&self) -> bool {
        self.session.active().is_some_and(|state| !state.history.undo.is_empty())
    }

    pub fn can_redo(&self) -> bool {
        self.session.active().is_some_and(|state| !state.history.redo.is_empty())
    }

    /// Runs a command as `origin` (a person, an agent): the replica records it as theirs, and it
    /// undoes as that replica's own step. Contract 1's [`VectorDocument::execute`](kition_kit::KitDocument::execute)
    /// is this with the local person.
    pub fn execute_as(&mut self, command: &str, params: Value, origin: &Origin) -> Result<Value, String> {
        self.pass.reset();
        self.refuse_unknown(command, &params)?;
        self.session.execute_as(command, &params, origin).map_err(engine_error)
    }

    /// Every blob the drawing's replica references or can still bring back, as one buffer a
    /// peer feeds straight into [`VectorDocument::add_blobs`].
    pub fn blobs(&mut self) -> Result<Vec<u8>, String> {
        let blobs = self.session.blobs().map_err(engine_error)?;
        Ok(encode_blobs(&blobs))
    }

    /// Adds a peer's blobs (from their [`VectorDocument::blobs`]): kept until the next
    /// [`VectorDocument::import_changes`], which a host calls right after.
    pub fn add_blobs(&mut self, bytes: &[u8]) -> Result<(), String> {
        let blobs = decode_blobs(bytes)?;
        self.session.add_blobs(&blobs).map_err(engine_error)
    }

    /// The blobs the drawing's replica has newly needed since `since` (empty bytes: every
    /// blob the current state references), encoded the same way as [`VectorDocument::blobs`]: for
    /// live sync, so one change ships only the blobs it needs rather than the whole store.
    pub fn blobs_since(&mut self, since: &[u8]) -> Result<Vec<u8>, String> {
        let since = if since.is_empty() { Version::empty() } else { Version::decode(since).map_err(engine_error)? };
        let blobs = self.session.blobs_since(&since).map_err(engine_error)?;
        Ok(encode_blobs(&blobs))
    }

    /// How many times the drawing left its replica because it refused a write or an undo (a
    /// drawing it cannot hold, or a bug). The drawing keeps working, undoing from its
    /// own history; the next shared edit makes a fresh replica, which collaborators must resync
    /// (its full [`VectorDocument::snapshot`]). Hosts watch this.
    pub fn kdm_resets(&self) -> u32 {
        u32::try_from(self.session.kdm_resets()).unwrap_or(u32::MAX)
    }
}

/// Encodes blobs for one wasm call's worth of blob transfer
/// ([`VectorDocument::blobs`]/[`VectorDocument::add_blobs`]): `(64-byte ASCII-hex hash, u64 LE
/// length, bytes) * N`. Every hash the KDM model produces is a 64-char hex SHA-256, so the hash
/// field needs no length prefix of its own.
fn encode_blobs(blobs: &MemoryBlobs) -> Vec<u8> {
    let mut out = Vec::new();
    for (hash, data) in &blobs.0 {
        out.extend_from_slice(hash.as_bytes());
        out.extend_from_slice(&(data.len() as u64).to_le_bytes());
        out.extend_from_slice(data);
    }
    out
}

/// Reverses [`encode_blobs`]; truncated or malformed input is an error, never a panic.
fn decode_blobs(bytes: &[u8]) -> Result<MemoryBlobs, String> {
    let mut map = HashMap::new();
    let mut i = 0usize;
    while i < bytes.len() {
        let hash_end = i.checked_add(64).ok_or("blob bundle: overflow")?;
        let hash = bytes.get(i..hash_end).ok_or("truncated blob bundle: hash")?;
        let hash = std::str::from_utf8(hash).map_err(|_| "blob bundle: hash is not ASCII".to_owned())?.to_owned();
        let len_end = hash_end.checked_add(8).ok_or("blob bundle: overflow")?;
        let len_bytes: [u8; 8] =
            bytes.get(hash_end..len_end).ok_or("truncated blob bundle: length")?.try_into().map_err(|_| "blob bundle: length".to_owned())?;
        let len = usize::try_from(u64::from_le_bytes(len_bytes)).map_err(|_| "blob bundle: length too large".to_owned())?;
        let data_end = len_end.checked_add(len).ok_or("blob bundle: overflow")?;
        let data = bytes.get(len_end..data_end).ok_or("truncated blob bundle: data")?;
        map.insert(hash, Arc::from(data));
        i = data_end;
    }
    Ok(MemoryBlobs(map))
}

/// The object model (`KitObjectModel 1.0`) over the document's change exchange: `kition-vector` serves
/// `model.describe` and `model.batch` headless through `kition_kit::session::model::Exchange`.
impl kition_kit::session::model::ReplicaPort for VectorDocument {
    fn model_schema(&self) -> std::sync::Arc<kition_kit::kdm::Schema> {
        <kdm_model::VectorModel as kition_kit::session::KitModel>::schema()
    }

    fn model_export(&mut self, since: &[u8]) -> Result<Vec<u8>, String> {
        self.export_changes(since)
    }

    fn model_import(&mut self, bytes: &[u8]) -> Result<(), String> {
        self.import_changes(bytes).map(|_| ())
    }

    /// Every call kition-headless runs for the agent or the host, and every `command` operation of
    /// `model.batch`: the agent's gate first ([`crate::gate`]), and for the commands it runs in
    /// turn (a batch's steps).
    fn model_command(&mut self, command: &str, params: Value, origin: &Origin) -> Result<Value, String> {
        crate::gate::learn(|| self.commands());
        crate::gate::may_run(command, &params)?;
        let outer = self.session.step_gate.replace(crate::gate::step_gate());
        let answer = self.execute_as(command, params, origin);
        self.session.step_gate = outer;
        answer
    }

    fn model_commands(&self) -> Vec<kition_kit::KitCommand> {
        self.commands()
    }

    /// A shared drawing with no file yet (a server's headless kit opening a sync room): the
    /// replica alone, failing rather than falling back to an empty drawing.
    fn model_open_replica(name: &str, snapshot: &[u8], blobs: &[u8]) -> Result<Self, String> {
        let blobs = decode_blobs(blobs)?;
        let mut session = crate::kit_session();
        session.open_with_replica(name, &[], snapshot, blobs).map_err(engine_error)?;
        Ok(Self::from_session(session, name))
    }
}

#[cfg(test)]
mod tests {
    use kition_kit::KitDocument;
    use serde_json::json;

    use super::*;

    fn fixture() -> Vec<u8> {
        let mut doc = VectorDocument::create("Logo.kvector", 400, 300).unwrap();
        doc.execute("shape.rectangle", json!({ "x": 40, "y": 40, "width": 120, "height": 80 })).unwrap();
        doc.save(None).unwrap()
    }

    fn extra_edit(doc: &mut VectorDocument) {
        doc.execute("shape.ellipse", json!({ "x": 200, "y": 150, "width": 60, "height": 60 })).unwrap();
        doc.execute("paint.setFill", json!({ "color": "#0000ff" })).unwrap();
    }

    #[test]
    fn version_and_snapshot_force_a_replica_lazily() {
        let mut doc = VectorDocument::open("Logo.kvector", &fixture()).unwrap();
        assert!(!doc.has_replica(), "opening a .kvector plainly makes no replica");
        let version = doc.version().unwrap();
        assert!(doc.has_replica());
        assert_eq!(doc.version().unwrap(), version, "asking twice makes no further replica");
        let snap = doc.snapshot().unwrap();
        assert!(!snap.is_empty());
    }

    #[test]
    fn undo_redo_report_done_and_damage() {
        let mut doc = VectorDocument::open("Logo.kvector", &fixture()).unwrap();
        assert!(!doc.can_undo());
        doc.execute("shape.ellipse", json!({ "x": 10, "y": 10, "width": 20, "height": 20 })).unwrap();
        assert!(doc.can_undo());
        let (done, damage) = doc.undo().unwrap();
        assert!(done);
        assert_ne!(damage, Damage::None);
        assert!(!doc.can_undo());
        assert!(doc.can_redo());
        let (done, _) = doc.redo().unwrap();
        assert!(done);
        // Undoing with nothing to undo is a normal `false`, not an error.
        doc.undo().unwrap();
        let (done, damage) = doc.undo().unwrap();
        assert!(!done);
        assert_eq!(damage, Damage::None);
    }

    #[test]
    fn execute_as_records_the_given_origin() {
        let mut doc = VectorDocument::open("Logo.kvector", &fixture()).unwrap();
        let origin = Origin::person("user:agent-1");
        doc.execute_as("shape.ellipse", json!({ "x": 10, "y": 10, "width": 20, "height": 20 }), &origin).unwrap();
        assert!(doc.can_undo());
    }

    /// `docId` is `None` before any replica exists, and the replica's own id (not the file's
    /// `name`, and stable across a save/reopen with that same replica) once one does.
    #[test]
    fn doc_id_is_none_until_a_replica_exists_then_the_replicas_own_id() {
        let mut doc = VectorDocument::open("Logo.kvector", &fixture()).unwrap();
        assert_eq!(doc.doc_id(), None, "no replica yet");
        doc.version().unwrap(); // makes the replica lazily
        let id = doc.doc_id().unwrap_or_else(|| panic!("doc_id should be set once a replica exists"));
        assert!(!id.is_empty());
        assert_eq!(doc.doc_id().as_deref(), Some(id.as_str()), "stable across further asks");

        let bytes = doc.save(None).unwrap();
        let snapshot = doc.snapshot().unwrap();
        let blobs = doc.blobs().unwrap();
        let reopened = VectorDocument::open_with_replica("Logo.kvector", &bytes, &snapshot, &blobs).unwrap();
        assert_eq!(reopened.doc_id(), Some(id), "the reopened replica keeps the same document id");
    }

    /// With no file yet (empty bytes), the replica alone opens: a peer's full change export
    /// becomes the same drawing, same identity, and it saves as a new `.kvector`.
    #[test]
    fn a_replica_opens_without_a_file() {
        let mut a = VectorDocument::open("Logo.kvector", &fixture()).unwrap();
        extra_edit(&mut a);
        let changes = a.export_changes(&[]).unwrap();
        let mut b = VectorDocument::open_with_replica("Shared.kvector", &[], &changes, &a.blobs().unwrap()).unwrap();
        assert_eq!(b.doc_id(), a.doc_id());
        assert_eq!(b.version().unwrap(), a.version().unwrap());
        assert!(!b.dirty());
        assert!(VectorDocument::open("Shared.kvector", &b.save(None).unwrap()).is_ok());
        assert!(VectorDocument::open_with_replica("Shared.kvector", &[], b"not a replica", &[]).is_err());
    }

    #[test]
    fn blobs_round_trip_through_encode_and_decode() {
        let mut a = MemoryBlobs::default();
        a.0.insert("a".repeat(64), Arc::from(b"hello".as_slice()));
        a.0.insert("b".repeat(64), Arc::from(b"".as_slice()));
        let bytes = encode_blobs(&a);
        let b = decode_blobs(&bytes).unwrap();
        assert_eq!(b.0.len(), 2);
        assert_eq!(b.0.get(&"a".repeat(64)).map(|v| v.as_ref()), Some(b"hello".as_ref()));
        assert_eq!(b.0.get(&"b".repeat(64)).map(|v| v.as_ref()), Some(b"".as_ref()));
    }

    #[test]
    fn decode_blobs_rejects_truncated_input_without_panicking() {
        assert!(decode_blobs(&[1, 2, 3]).is_err());
        assert!(decode_blobs(&"a".repeat(64).into_bytes()).is_err(), "hash with no length");
        let mut malformed = "a".repeat(64).into_bytes();
        malformed.extend_from_slice(&u64::MAX.to_le_bytes());
        assert!(decode_blobs(&malformed).is_err(), "a length claiming more data than exists");
    }

    /// A `.kvector` saved with its replica reopens as that replica: same version, same
    /// rendered pixels, not dirty, and it takes new edits (which undo) at once.
    #[test]
    fn a_drawing_reopens_with_its_stored_replica() {
        let mut a = VectorDocument::open("Logo.kvector", &fixture()).unwrap();
        extra_edit(&mut a);
        let bytes = a.save(None).unwrap();
        let (snapshot, blobs) = (a.snapshot().unwrap(), a.blobs().unwrap());
        let version = a.version().unwrap();

        let mut b = VectorDocument::open_with_replica("Logo.kvector", &bytes, &snapshot, &blobs).unwrap();
        assert!(b.has_replica());
        assert_eq!(b.version().unwrap(), version);
        assert_eq!(b.render(0.0, 0.0, 1.0, b.width(), b.height()).unwrap(), a.render(0.0, 0.0, 1.0, a.width(), a.height()).unwrap());
        assert!(!b.dirty());
        assert!(!b.can_undo());
        extra_edit(&mut b);
        assert!(b.dirty());
        assert_ne!(b.version().unwrap(), version);
        assert!(b.undo().unwrap().0);
        assert!(VectorDocument::open("Logo.kvector", &b.save(None).unwrap()).is_ok(), "it still saves as .kvector");
    }

    /// A damaged (or missing) stored replica never stops the file from opening: it opens plainly,
    /// never as a blank drawing.
    #[test]
    fn a_damaged_stored_replica_opens_the_file_plainly() {
        let bytes = fixture();
        let a = VectorDocument::open("Logo.kvector", &bytes).unwrap();
        for (snapshot, blobs) in [(&b"not a snapshot"[..], &[][..]), (&[][..], &[1u8, 2, 3][..])] {
            let doc = VectorDocument::open_with_replica("Logo.kvector", &bytes, snapshot, blobs).unwrap();
            assert_eq!((doc.width(), doc.height()), (a.width(), a.height()));
            assert!(!doc.has_replica(), "the file opened plainly");
        }
        assert!(VectorDocument::open_with_replica("Logo.kvector", b"not a drawing", b"not a snapshot", &[]).is_err());
    }

    /// `a` saves with its replica; `b` opens with it, so both start from the same replica
    /// identity. `a` edits further;
    /// `b.importChanges(a.exportChanges(b.version()))` converges `b` to the same replicated
    /// state as `a`, through the public SDK surface alone.
    #[test]
    fn two_documents_converge_through_export_and_import_changes() {
        let mut a = VectorDocument::open("Logo.kvector", &fixture()).unwrap();
        extra_edit(&mut a);
        let bytes = a.save(None).unwrap();
        let snapshot = a.snapshot().unwrap();
        let blobs = a.blobs().unwrap();

        let mut b = VectorDocument::open_with_replica("Logo.kvector", &bytes, &snapshot, &blobs).unwrap();
        assert_eq!(b.version().unwrap(), a.version().unwrap(), "b starts from a's own replica");

        a.execute("shape.rectangle", json!({ "x": 300, "y": 200, "width": 50, "height": 50 })).unwrap();

        let since = b.version().unwrap();
        let changes = a.export_changes(&since).unwrap();
        b.add_blobs(&a.blobs().unwrap()).unwrap();
        b.import_changes(&changes).unwrap();

        assert_eq!(a.version().unwrap(), b.version().unwrap());
        assert_eq!(
            a.render(0.0, 0.0, 1.0, a.width(), a.height()).unwrap(),
            b.render(0.0, 0.0, 1.0, b.width(), b.height()).unwrap(),
            "converged replicas render the same drawing"
        );
    }
    /// `kition-vector` serves the object model through `ReplicaPort`: a command and a model edit in
    /// batches reach the engine, and a read-only batch refuses writes.
    #[test]
    fn serves_the_object_model_over_its_change_exchange() {
        use kition_kit::session::model::{Exchange, ModelErrorCode};
        use serde_json::{Value, json};

        let mut doc = VectorDocument::open("Logo.kvector", &fixture()).unwrap();
        let mut exchange = Exchange::new();
        let described = exchange.describe(&mut doc).unwrap();
        assert_eq!(described["capability"], "KitObjectModel 1.0");
        assert!(described["commands"].as_array().unwrap().iter().any(|command| command["id"] == "shape.ellipse"));

        let command = json!({ "ops": [{ "op": "command", "id": "shape.ellipse", "params": { "x": 10, "y": 10, "width": 20, "height": 20 } }] });
        assert_eq!(exchange.batch(&mut doc, &command, true).unwrap_err().code, ModelErrorCode::ReadOnly);
        let before = doc.version().unwrap();
        exchange.batch(&mut doc, &command, false).unwrap();
        assert_ne!(doc.version().unwrap(), before, "the command ran in the engine");

        let load = |exchange: &mut Exchange, doc: &mut VectorDocument, target: &str, field: &str| -> Value {
            let answer = exchange.batch(doc, &json!({ "ops": [], "load": [{ "target": target, "fields": [field] }] }), false).unwrap();
            answer["loaded"][0]["fields"][field].clone()
        };
        let nodes = load(&mut exchange, &mut doc, "root", "nodes");
        let node = nodes[0]["object"].as_str().unwrap().to_owned();
        let lock = json!({ "ops": [{ "op": "set", "target": node, "field": "locked", "value": true }] });
        exchange.batch(&mut doc, &lock, false).unwrap();
        assert_eq!(load(&mut Exchange::new(), &mut doc, &node, "locked"), true);
    }
}
