//! Bridges Kition Vector's [`Document`] ([`kition_vector_doc`]) to a `kit.vector` KDM replica.
//!
//! [`VectorModel`] implements [`KitModel`]: the engine's materialised view of a replica, the same
//! [`Document`] the rest of the engine already knows. [`write_document`] and [`write_changes`]
//! are the engine command bridge's entry points: writing a whole drawing in one creating
//! transaction, or only what a command changed.
//!
//! What lives where (`model/schema.json`):
//! - the layer tree is the `Drawing.nodes` KDM **tree**: each layer, group and object is a `Node`
//!   object named by one tree node, so a reorder or a reparent is a tree move and merges as one
//!   (two peers moving one object at once end with it in one place). A node's registers are its
//!   `name`, `visible`, `locked`, `appearance` (fills, strokes, effects: a recolour), `data` (its
//!   kind and geometry without its children: a move or a reshape) and `props` (the rest of the
//!   object: opacity, blending, the opacity mask with its art, attributes, perspective…). So
//!   moving one object and recolouring another, or one object moved and recoloured by two peers,
//!   both survive a merge;
//! - artboards, swatches (and swatch groups), graphic styles, character and paragraph styles,
//!   symbols, patterns, slices, export assets and images are objects in lists of the `Drawing`;
//!   image bytes are blobs, content-addressed through [`BlobSink`]/[`BlobSource`];
//! - every other key of the document (setup, guides, threads, views, saved selections,
//!   variables, metadata, foreign `unknown` data, keys a newer version wrote…) is one entry of the
//!   `Drawing.settings` map, so two peers changing different settings both keep theirs and
//!   nothing is lost on a round trip.
//!
//! **Ids.** Engine ids (`NodeId` from the document's counter, artboard and graphic style ids,
//! slice and asset ids) are never replicated: [`read_document`] mints them (keeping, through
//! [`Known`], the ones a session's view already gave an object), and [`Ids`] maps them to object
//! ids. Every id stored as data — a node's graphic style, text threads, guides' layers and
//! artboards, saved selections, variable bindings, export assets, the pattern-editing layer — is
//! written as the object id it names and translated back on read; a reference to nothing is not
//! replicated (it means nothing). References by **name** to a swatch, a pattern or a symbol
//! (paints, gradient stops, tints, symbol instances, inline graphics, pattern editing) are written
//! as `{"ref": <object id>, "name": <name>}`: they follow a rename by another peer, and read back
//! as the name they had if the object is gone.
//!
//! **Per person.** The opacity-mask editing layer and warp pins are the editing person's
//! own (see [`shared_part`]): never written. The saved view and the open Layers rows are view
//! state the engine keeps outside the document.
//!
//! **Repair.** [`KitModel::validate`] clears what a merge left dangling: a symbol instance of a
//! deleted symbol is deleted (as `symbol.delete` without expanding does), a graphic style link to
//! a deleted style is dropped, threads, guides, saved selections, variable bindings, assets and
//! pattern editing let go of deleted objects, and list entries naming deleted objects go.
#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

mod blob_gc;
mod compare;
mod ids;
mod list;
mod read;
mod refs;
mod session;
mod validate;
mod write;

use std::borrow::Cow;
use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;

use kition_kit::kdm::{ActorId, DocId, KdmError, ObjectId, Origin, Replica, Schema, Tx};
use kition_kit::session::{BlobSink, BlobSource, Damage, KitError, KitModel};
use kition_vector_doc::Document;

pub use compare::normalize;
pub use ids::{Ids, Known};
pub use read::{read_document, read_document_known};
pub use session::VectorSession;
pub use write::{write_changes, write_document};

/// The settling step of a model batch (`kition_kit::KitDocument::batch_settler`): a named entry
/// the batch created gets its persistent identity (`key`, the field of its tool id) in the same
/// transaction, so the receipt names it as a read of the saved file does, and the references
/// to a swatch, symbol or pattern the batch renamed carry its new name, as the saved file does
/// (C8-vector-5).
pub fn settle_batch(tx: &mut Tx<'_>) -> std::result::Result<(), String> {
    validate::complete_entries(tx).map_err(|error| error.to_string())?;
    validate::complete_objects(tx).map_err(|error| error.to_string())?;
    validate::settle_keys(tx, true).map_err(|error| error.to_string())?;
    validate::follow_renames(tx).map(|_| ()).map_err(|error| error.to_string())
}

/// Re-exported so downstream crates (the engine command bridge) share this crate's exact pin.
pub use kition_kit;

type Result<T> = kition_kit::session::Result<T>;
pub(crate) type KdmResult<T> = std::result::Result<T, KdmError>;

const SCHEMA: &str = include_str!("../model/schema.json");

/// How deep groups may nest for the bridge (writes refuse deeper trees, reads stop there).
pub(crate) const MAX_DEPTH: usize = 200;

/// The `Drawing` object's id: one per replica, minted by [`write_document`] with
/// [`Tx::create_with_id`], so every reader finds it without an index.
pub(crate) fn root_id() -> ObjectId {
    ObjectId::from_string("drawing")
}

fn kit_err(error: impl std::fmt::Display) -> KitError {
    KitError::new(error.to_string())
}

pub(crate) fn kdm_err(error: impl std::fmt::Display) -> KdmError {
    KdmError::Engine(error.to_string())
}

/// Image bytes the view already holds, by blob hash, so a reload does not copy them again.
pub(crate) type ImageCache = HashMap<String, Arc<Vec<u8>>>;

/// The part of `doc` a replica holds: without the opacity-mask editing layer (the editing
/// person's working copy of art the mask already holds, which the engine syncs into the mask
/// after every edit). Borrowed when no mask is being edited. warp pins and the mask
/// editing mode itself are never serialized, so they are not written either.
pub fn shared_part(doc: &Document) -> Cow<'_, Document> {
    doc.without_edit_modes()
}

/// A file's drawing, as [`KitModel::import`] reads it: `.kvector` (by content), else SVG
/// (by content or `name`'s extension), without the view state a file keeps for its editor (the
/// saved view and the open Layers rows) and with duplicate ids renumbered (a replica needs every
/// object, slice, asset and artboard id unique).
pub fn decode(name: &str, bytes: &[u8]) -> Result<Document> {
    let lower = name.to_ascii_lowercase();
    let svg = lower.ends_with(".svg") || lower.ends_with(".svgz") || kition_vector_svg::is_svgz(bytes) || looks_like_svg(bytes);
    let mut doc = if kition_vector_format::sniff(bytes) {
        kition_vector_format::load(bytes).map_err(kit_err)?
    } else if svg {
        let text = kition_vector_svg::text_of(bytes).map_err(kit_err)?;
        kition_vector_svg::import(&text).map_err(kit_err)?
    } else {
        kition_vector_format::load(bytes).map_err(kit_err)?
    };
    doc.last_view = None;
    doc.layers_open = None;
    doc.drop_edit_modes();
    doc.settle_uids(None);
    ids::dedupe(&mut doc);
    Ok(doc)
}

fn looks_like_svg(bytes: &[u8]) -> bool {
    let head = String::from_utf8_lossy(bytes.get(..512).unwrap_or(bytes)).to_ascii_lowercase();
    head.contains("<svg")
}

/// The engine's view of a `kit.vector` replica: the same [`Document`] the rest of the engine
/// knows, plus the id maps [`write_changes`] needs to find each object again.
pub struct VectorModel {
    document: Arc<Document>,
    ids: Ids,
    known: Known,
    images: ImageCache,
}

impl VectorModel {
    /// A view built by the caller itself: `document` as written by [`write_document`] or
    /// [`write_changes`], with the ids they filled.
    pub fn from_parts(document: Arc<Document>, ids: Ids) -> Self {
        let mut known = Known::default();
        known.remember(&document, &ids);
        Self { document, ids, known, images: ImageCache::new() }
    }

    /// Replaces the view after the caller wrote `document` to the replica itself (a diff bridge's
    /// [`write_changes`]), with the ids that write left.
    pub fn adopt(&mut self, document: Arc<Document>, ids: Ids) {
        self.known.remember(&document, &ids);
        self.document = document;
        self.ids = ids;
    }

    /// The view: the drawing as of the last load, apply or adopt.
    pub fn document(&self) -> &Arc<Document> {
        &self.document
    }

    /// The id maps, current as of the last `load`/`apply`/`adopt`. A diff bridge clones them into
    /// its own working copy for a [`write_changes`] transaction, then hands them back through
    /// [`VectorModel::adopt`].
    pub fn ids(&self) -> &Ids {
        &self.ids
    }

    /// Swaps the view for `document` when it holds exactly the same content with the same local
    /// ids (the engine's own undo snapshot of what the replica now holds): its caches and its
    /// identity (a saved document stays clean) are kept. Answers whether it did.
    pub fn adopt_equal(&mut self, document: &Arc<Document>) -> bool {
        if Arc::ptr_eq(document, &self.document) {
            return true;
        }
        if !compare::same_content(&self.document, document) {
            return false;
        }
        self.document = document.clone();
        true
    }

    fn reload(&mut self, replica: &Replica, blobs: &dyn BlobSource, changed: Option<&BTreeSet<ObjectId>>) -> Result<()> {
        let reuse = changed.map(|changed| read::Reuse { view: &self.document, ids: &self.ids, changed });
        let (document, ids, images) = read::read_with(replica, blobs, &self.known, &self.images, reuse).map_err(kit_err)?;
        self.known.remember(&document, &ids);
        self.document = Arc::new(document);
        self.ids = ids;
        self.images = images;
        Ok(())
    }
}

impl std::fmt::Debug for VectorModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VectorModel").field("layers", &self.document.layers.len()).finish_non_exhaustive()
    }
}

impl KitModel for VectorModel {
    fn schema() -> Arc<Schema> {
        #[allow(clippy::expect_used)]
        Arc::new(Schema::parse(SCHEMA).expect("model/schema.json is a fixed, tested file"))
    }

    fn load(replica: &Replica, blobs: &dyn BlobSource) -> Result<Self> {
        let (document, ids, images) = read::read_with(replica, blobs, &Known::default(), &ImageCache::new(), None).map_err(kit_err)?;
        let mut model = Self::from_parts(Arc::new(document), ids);
        model.images = images;
        Ok(model)
    }

    /// Reloads the view, keeping every surviving object's local id, image bytes it already holds
    /// and every node whose object (and subtree) did not change, caches and all.
    fn apply(&mut self, replica: &Replica, changed: &BTreeSet<ObjectId>, blobs: &dyn BlobSource) -> Result<Damage> {
        self.reload(replica, blobs, Some(changed))?;
        Ok(Damage::Full)
    }

    fn validate(tx: &mut Tx<'_>, changed: &BTreeSet<ObjectId>) -> Result<bool> {
        validate::validate(tx, changed).map_err(kit_err)
    }

    fn import(name: &str, bytes: &[u8], blobs: &mut dyn BlobSink, actor: ActorId) -> Result<Replica> {
        let document = decode(name, bytes)?;
        let mut ids = Ids::default();
        let (replica, ()) = Replica::create_with(Self::schema(), DocId::new(), actor, &Origin::system("import"), |tx| {
            write_document(tx, &document, &mut ids, blobs)?;
            Ok(())
        })
        .map_err(kit_err)?;
        Ok(replica)
    }

    /// `.svg` when `name` says so, otherwise `.kvector`.
    fn export(&self, _replica: &Replica, name: &str, _blobs: &dyn BlobSource) -> Result<Vec<u8>> {
        if name.to_ascii_lowercase().ends_with(".svg") {
            Ok(kition_vector_svg::export(&self.document, &kition_vector_svg::ExportOptions::default()).into_bytes())
        } else {
            Ok(kition_vector_format::save_file(&self.document))
        }
    }
}
