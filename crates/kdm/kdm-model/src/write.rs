//! `Document` -> replica: a full write (a new replica, an import) and an incremental diff (the
//! engine command bridge) that writes only what changed.
//!
//! Both run the same passes, a full write being a diff from nothing:
//! 1. every new artboard, swatch, swatch group, style, symbol, pattern, slice, asset, image, guide
//!    and text thread gets its object (named lists and guides are paired with their previous
//!    version by content, then by name; threads by their frames), and what `after` no longer
//!    holds is deleted;
//! 2. every new node gets its object, written in full;
//! 3. the tree is made to hold `after`'s layers: under each parent whose children changed, the
//!    longest run of children already in place (in order, from the first) stays and the rest are
//!    moved (or, new, created) after it in order — a tree only ever appends — so bringing one
//!    object to the front, or moving it into another layer on top, is one tree move; a subtree
//!    whose `Arc` did not change is skipped whole; removed nodes are then deleted child-first;
//! 4. every field that differs is set (a node's registers and text only when its `Arc` changed,
//!    or its text when it joined or left a thread), each symbol's art tree is made to hold its art
//!    the way the layer tree is, every list is made to hold `after`'s order, a thread's story is
//!    written to its text, and the settings map gets the keys that changed.
//!
//! A text field is written as the characters and marks that changed ([`write_rich_text`]); when
//! the replica's text is not as long as the one the view says it holds (a merge put another
//! person's characters into a frame that was threaded meanwhile), it is replaced whole.

use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::sync::Arc;

use kition_kit::kdm::{NodeRef, ObjectId, Tx};
use kition_kit::session::{BlobSink, IdMap, TextRun as Delta, write_rich_text};
use kition_vector_doc::{Artboard, Document, ExportAsset, Guide, ImageBlob, Node, NodeId, NodeKind, Slice, TextKind, TextObject};
use serde::Serialize;
use serde_json::{Map, Value, json};

use crate::ids::{Ids, TreeIds, check_unique, index};
use crate::list::{align, sync_list};
use crate::refs::{Mapper, Names, SETTINGS_WITH_IDS, map_asset_nodes, map_setting, names_out};
use crate::text::encode;
use crate::{KdmResult, MAX_DEPTH, kdm_err, root_id};

/// The keys of a serialized `Document` that are objects of their own (or view state), not
/// settings.
pub(crate) const MODELLED: &[&str] = &[
    "artboards",
    "layers",
    "swatches",
    "swatch_groups",
    "graphic_styles",
    "char_styles",
    "para_styles",
    "symbols",
    "patterns",
    "images",
    "slices",
    "assets",
    "guides",
    "text_threads",
    "next_id",
    "last_view",
    "layers_open",
];

/// The register fields of one object, as written (references already object ids).
pub(crate) type Fields = Vec<(&'static str, Value)>;

/// `v` as the JSON a register holds, with every `id` beyond ±2^53 brought into range (see
/// [`safe_ids`]).
pub(crate) fn value(v: &impl Serialize) -> KdmResult<Value> {
    let mut v = serde_json::to_value(v).map_err(kdm_err)?;
    safe_ids(&mut v, 0);
    Ok(v)
}

/// The largest integer every KDM implementation reads exactly (JavaScript's 2^53 - 1).
const MAX_SAFE: u64 = (1 << 53) - 1;

/// Brings every `id` beyond ±2^53 inside `v` into range, keeping its low bits, so a replica takes
/// it: a replica refuses such integers, and leaves the drawing without a replica. The ids written
/// in objects' `id` fields come from the document's counter; the large ones are those inside art
/// that is data (opacity masks, patterns), which get fresh ids when used, and a mask's group of
/// several parts has `u64::MAX - 1` (the engine's `maskedit::mask_group`), as can a damaged file.
pub(crate) fn safe_ids(v: &mut Value, depth: usize) {
    if depth > MAX_DEPTH * 4 {
        return;
    }
    match v {
        Value::Object(map) => {
            for (key, item) in map.iter_mut() {
                if key == "id"
                    && let Some(id) = item.as_u64().filter(|id| *id > MAX_SAFE)
                {
                    *item = Value::from(id & MAX_SAFE);
                } else {
                    safe_ids(item, depth + 1);
                }
            }
        }
        Value::Array(items) => items.iter_mut().for_each(|item| safe_ids(item, depth + 1)),
        _ => {}
    }
}

fn object_of(v: Value) -> Map<String, Value> {
    match v {
        Value::Object(map) => map,
        _ => Map::new(),
    }
}

fn fields_map(fields: Fields) -> Map<String, Value> {
    fields.into_iter().filter(|(_, v)| !v.is_null()).map(|(k, v)| (k.to_owned(), v)).collect()
}

fn oid(object: &ObjectId) -> Value {
    Value::String(object.as_str().to_owned())
}

/// Writes a whole drawing into a fresh replica's creating transaction. Answers the `Drawing`
/// object's id. `doc` should be the [`crate::shared_part`] of the engine's document.
pub fn write_document(tx: &mut Tx<'_>, doc: &Document, ids: &mut Ids, blobs: &mut dyn BlobSink) -> KdmResult<ObjectId> {
    let root = root_id();
    tx.create_with_id("Drawing", &root, Map::new())?;
    *ids = Ids::default();
    Writer::new(tx, ids, blobs, true).write(None, doc)?;
    Ok(root)
}

/// Writes only what differs between `before` (the drawing `ids` describe) and `after`. Answers
/// whether it wrote anything. Errs, writing nothing usable, if `after` holds an id twice where it
/// must be unique (see [`crate::decode`]) or nests deeper than the bridge goes.
pub fn write_changes(tx: &mut Tx<'_>, before: &Document, after: &Document, ids: &mut Ids, blobs: &mut dyn BlobSink) -> KdmResult<bool> {
    Writer::new(tx, ids, blobs, false).write(Some(before), after)
}

/// The keys of a type object's serialized kind its `text` field holds, not `data`.
pub(crate) const TEXT_KEYS: &[&str] = &["runs", "para", "paras"];

/// A node's own registers: its name, visibility, lock, appearance, kind and geometry (`data`,
/// without its children, and for a type object without its characters and paragraphs, which
/// are its text: [`node_text`]) and the rest of the object (`props`). Names and the graphic style
/// link are written as object references through `names`.
pub(crate) fn node_fields(n: &Node, names: &Names) -> KdmResult<Fields> {
    let mut shallow = n.clone();
    if let Some(children) = shallow.children_mut() {
        children.clear();
    }
    if let NodeKind::Text(t) = &mut shallow.kind {
        t.runs = Vec::new();
        t.paras = Vec::new();
    }
    let mut map = object_of(value(&shallow)?);
    map.remove("id");
    let mut data = map.remove("kind").unwrap_or(Value::Null);
    if let (Some(key), Value::Object(d)) = (children_key(&n.kind), &mut data) {
        d.remove(key);
    }
    if let (NodeKind::Text(_), Value::Object(d)) = (&n.kind, &mut data) {
        for key in TEXT_KEYS {
            d.remove(*key);
        }
    }
    let name = map.remove("name").unwrap_or(Value::Null);
    map.remove("visible");
    map.remove("locked");
    let mut appearance = map.remove("appearance").unwrap_or(Value::Null);
    if let Some(style) = map.remove("graphic_style")
        && let Some(object) = style.as_u64().and_then(|s| u32::try_from(s).ok()).and_then(|s| names.styles.get(&s))
    {
        map.insert("graphic_style".into(), oid(object));
    }
    let mut props = if map.is_empty() { Value::Null } else { Value::Object(map) };
    names_out(&mut data, names);
    names_out(&mut appearance, names);
    names_out(&mut props, names);
    Ok(vec![
        ("id", json!(n.id.0)),
        ("name", name),
        ("visible", Value::Bool(n.visible)),
        ("locked", Value::Bool(n.locked)),
        ("appearance", appearance),
        ("props", props),
        ("data", data),
    ])
}

/// A node's `text` field: a type object's story ([`crate::text`]), none for other objects and for
/// area-type frames of a thread (`threaded`), whose story is the thread's.
pub(crate) fn node_text(n: &Node, threaded: &HashSet<NodeId>, names: &Names) -> KdmResult<Vec<Delta>> {
    match &n.kind {
        NodeKind::Text(t) if !threaded.contains(&n.id) => encode(&t.runs, &t.paragraph_styles(), names),
        _ => Ok(Vec::new()),
    }
}

/// `n`'s type object when it is area type (what a thread flows through).
pub(crate) fn area_text(n: &Node) -> Option<&TextObject> {
    match &n.kind {
        NodeKind::Text(t) if matches!(t.kind, TextKind::Area { .. }) => Some(t),
        _ => None,
    }
}

/// The area-type frames of `doc`'s threads: their story is their thread's text, not their own.
pub(crate) fn threaded(doc: &Document) -> HashSet<NodeId> {
    doc.text_threads.iter().flatten().copied().filter(|id| doc.node(*id).is_some_and(|n| area_text(n).is_some())).collect()
}

/// A thread's `text`: the story its area-type frames show, in order (none without one).
fn thread_text(doc: &Document, frames: &[NodeId], names: &Names) -> KdmResult<Vec<Delta>> {
    let texts: Vec<&TextObject> = frames.iter().filter_map(|id| doc.node(*id)).filter_map(area_text).collect();
    if texts.is_empty() {
        return Ok(Vec::new());
    }
    let story = kition_vector_text::thread::story(&texts);
    let paras = kition_vector_text::thread::story_paras(&texts, &[]);
    encode(&story, &paras, names)
}

/// A guide's registers: its layer and artboard as object references through `ids` (none when it
/// is on none, or on one the drawing no longer has).
fn guide_fields(g: &Guide, ids: &Ids) -> Fields {
    let artboard = g.artboard.and_then(|a| ids.artboards.object(a)).map_or(Value::Null, oid);
    let layer = g.layer.and_then(|l| ids.nodes.object(l)).map_or(Value::Null, oid);
    vec![("vertical", Value::Bool(g.vertical)), ("pos", json!(g.pos)), ("artboard", artboard), ("layer", layer)]
}

/// What pairs two versions of a guide that changed: its direction and what it is on.
fn guide_key(g: &Guide) -> String {
    format!("{}a{}l{}", if g.vertical { "v" } else { "h" }, g.artboard.unwrap_or(0), g.layer.map_or(0, |l| l.0))
}

/// The key a container kind keeps its children under (they are tree children, not data).
pub(crate) fn children_key(kind: &NodeKind) -> Option<&'static str> {
    match kind {
        NodeKind::Layer { .. } | NodeKind::Group { .. } | NodeKind::Compound { .. } | NodeKind::Blend { .. } => Some("children"),
        NodeKind::Envelope { .. } => Some("content"),
        NodeKind::Repeat(_) => Some("source"),
        _ => None,
    }
}

/// The same key, by the serialized kind's `type`.
pub(crate) fn children_key_of(ty: &str) -> Option<&'static str> {
    match ty {
        "layer" | "group" | "compound" | "blend" => Some("children"),
        "envelope" => Some("content"),
        "repeat" => Some("source"),
        _ => None,
    }
}

/// Whether two child lists hold the very same nodes (`Arc` identity).
fn same_list(a: &[Arc<Node>], b: &[Arc<Node>]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| Arc::ptr_eq(x, y))
}

/// The document's settings: every key of its serialization that is not an object of its own,
/// with the local ids inside them as object ids (see [`map_setting`]).
pub(crate) fn settings(doc: &Document, ids: &Ids) -> KdmResult<Map<String, Value>> {
    let mut shell = doc.clone();
    shell.layers = Vec::new();
    shell.guides = Vec::new();
    shell.text_threads = Vec::new();
    // The person's own ([`crate::shared_part`]).
    shell.pattern_edit = None;
    shell.variables.active_dataset = None;
    shell.artboards = Vec::new();
    shell.swatches = Vec::new();
    shell.swatch_groups = Vec::new();
    shell.graphic_styles = Vec::new();
    shell.char_styles = Vec::new();
    shell.para_styles = Vec::new();
    shell.symbols = Vec::new();
    shell.patterns = Vec::new();
    shell.images = Default::default();
    shell.slices = Vec::new();
    shell.assets = Vec::new();
    shell.last_view = None;
    shell.layers_open = None;
    let mut map = object_of(value(&shell)?);
    for key in MODELLED {
        map.remove(*key);
    }
    // The settings groups are written even when they hold their defaults, so a first change of one
    // setting in them is a change of that setting, not of the whole group (C8-vector-engine-2: a
    // write that rests on another setting of the group is not refused for it).
    for (key, group) in [("setup", value(&doc.setup)?), ("metadata", value(&doc.metadata)?), ("raster_effects", value(&doc.raster_effects)?)] {
        map.entry(key).or_insert(group);
    }
    // The print setup holds only the settings changed from the defaults (`print.setup`): none is an
    // empty one, so the first setting given is a change of that setting.
    map.entry("print_setup").or_insert_with(|| Value::Object(Map::new()));
    let node = |v: &Value| v.as_u64().and_then(|n| ids.nodes.object(NodeId(n))).map(oid);
    let artboard = |v: &Value| v.as_u64().and_then(|n| u32::try_from(n).ok()).and_then(|n| ids.artboards.object(n)).map(oid);
    let m = Mapper { node: &node, artboard: &artboard };
    for key in SETTINGS_WITH_IDS {
        if let Some(v) = map.get_mut(*key) {
            map_setting(key, v, &m);
            if v.is_null() {
                map.remove(*key);
            }
        }
    }
    Ok(map)
}

/// Named list elements: everything but the name (and a graphic style's local id).
/// What pairs two versions of a named entry: its persistent identity, else (a document never
/// settled, `Document::settle_uids`) its name.
fn pairing(uid: &kition_vector_doc::Uid, name: &str) -> String {
    if uid.is_empty() { name.to_owned() } else { uid.as_str().to_owned() }
}

/// A swatch's `role`: `builtIn` for None and Registration, which the engine never edits, moves or
/// deletes (the object model then refuses to), none for the rest.
fn role(s: &kition_vector_doc::color::Swatch) -> Value {
    if s.is_reserved() { json!("builtIn") } else { Value::Null }
}

/// A named entry's `key` (its tool id, `swatch:<uid>`): its persistent identity
/// (`Document::settle_uids`), none while it has none.
fn key(uid: &kition_vector_doc::Uid) -> Value {
    if uid.is_empty() { Value::Null } else { json!(uid.as_str()) }
}

fn named_props(v: &impl Serialize, drop: &[&str], names: &Names) -> KdmResult<Value> {
    let mut map = object_of(value(v)?);
    map.remove("name");
    map.remove("uid");
    for key in drop {
        map.remove(*key);
    }
    let mut props = Value::Object(map);
    names_out(&mut props, names);
    Ok(props)
}

pub(crate) fn artboard_fields(a: &Artboard) -> KdmResult<Fields> {
    let mut map = object_of(value(a)?);
    map.remove("id");
    let rect = map.remove("rect").unwrap_or(Value::Null);
    Ok(vec![("id", json!(a.id)), ("rect", rect), ("props", Value::Object(map))])
}

pub(crate) fn slice_fields(s: &Slice) -> KdmResult<Fields> {
    let mut map = object_of(value(s)?);
    map.remove("id");
    Ok(vec![("id", json!(s.id.0)), ("props", Value::Object(map))])
}

fn asset_fields(a: &ExportAsset, ids: &Ids) -> KdmResult<Fields> {
    let mut map = object_of(value(a)?);
    map.remove("id");
    let mut props = Value::Object(map);
    let node = |v: &Value| v.as_u64().and_then(|n| ids.nodes.object(NodeId(n))).map(oid);
    map_asset_nodes(&mut props, &node);
    Ok(vec![("id", json!(a.id)), ("props", props)])
}

fn image_fields(key: &str, img: &ImageBlob, blobs: &mut dyn BlobSink) -> KdmResult<Fields> {
    let blob = if img.bytes.is_empty() { Value::Null } else { value(&blobs.put(&img.bytes, &img.mime))? };
    let proxy = match &img.proxy {
        Some(p) if Arc::ptr_eq(p, &img.bytes) && !blob.is_null() => blob.clone(),
        Some(p) if !p.is_empty() => value(&blobs.put(p, "image/png"))?,
        _ => Value::Null,
    };
    Ok(vec![("key", json!(key)), ("mime", json!(img.mime)), ("blob", blob), ("proxy", proxy)])
}

fn same_image(a: &ImageBlob, b: &ImageBlob) -> bool {
    let same_bytes = |x: &Arc<Vec<u8>>, y: &Arc<Vec<u8>>| Arc::ptr_eq(x, y) || x == y;
    a.mime == b.mime
        && same_bytes(&a.bytes, &b.bytes)
        && match (&a.proxy, &b.proxy) {
            (None, None) => true,
            (Some(x), Some(y)) => same_bytes(x, y),
            _ => false,
        }
}

/// A named list's objects in `after`'s order, and each one's partner in `before`.
struct Plan {
    objects: Vec<ObjectId>,
    partner: Vec<Option<usize>>,
}

/// Where a tree of nodes lives: the drawing's layers (`Drawing.nodes`) or a symbol's art
/// (`Symbol.nodes`).
struct Place {
    owner: ObjectId,
    /// A symbol's art: its nodes' ids are not written ([`crate::ids`]), and a fresh read names
    /// their objects after this (the symbol's) prefix.
    art: Option<String>,
}

/// One side of a write (`before` or `after`): its names and its threaded frames.
struct Side<'a> {
    names: &'a Names,
    threaded: &'a HashSet<NodeId>,
}

/// `fields` as a symbol's art node writes them: without the art's own id.
fn art_fields(mut fields: Fields, art: bool) -> Fields {
    if art && let Some(slot) = fields.iter_mut().find(|(field, _)| *field == "id") {
        slot.1 = Value::Null;
    }
    fields
}

struct Writer<'w, 'tx> {
    tx: &'w mut Tx<'tx>,
    ids: &'w mut Ids,
    blobs: &'w mut dyn BlobSink,
    /// Objects this write created: written in full, whatever `before` held.
    fresh: HashSet<ObjectId>,
    /// Objects whose `text` this write wrote.
    texts: HashSet<ObjectId>,
    wrote: bool,
    /// A whole drawing written into a fresh replica (a file read): each object gets the object id
    /// its tool id or name gives ([`stable_id`]), so two reads of one file name an object alike
    /// and a file read again pairs with the read before by object id, not by place (C8-vector-4).
    stable: bool,
}

/// The object id a fresh read gives an object of type `ty` whose tool id or name is `what`:
/// `node-12`, `swatch-<hash of the name>`.
fn stable_id(ty: &str, what: &str) -> ObjectId {
    let readable = what.len() <= 40 && what.chars().all(|c| c.is_ascii_alphanumeric());
    if readable {
        return ObjectId::from_string(format!("{}-{what}", ty.to_ascii_lowercase()));
    }
    // FNV-1a: the same on every platform and run.
    let hash = what.bytes().fold(0xcbf2_9ce4_8422_2325_u64, |h, b| (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3));
    ObjectId::from_string(format!("{}-h{hash:016x}", ty.to_ascii_lowercase()))
}

impl<'w, 'tx> Writer<'w, 'tx> {
    fn new(tx: &'w mut Tx<'tx>, ids: &'w mut Ids, blobs: &'w mut dyn BlobSink, stable: bool) -> Self {
        Self { tx, ids, blobs, fresh: HashSet::new(), texts: HashSet::new(), wrote: false, stable }
    }

    /// A new object of type `ty`: with the [`stable_id`] of `what` in a fresh read (a fresh id
    /// when that one is taken), else a fresh id.
    fn create(&mut self, ty: &str, what: &str, fields: Map<String, Value>) -> KdmResult<ObjectId> {
        if self.stable
            && let Ok(object) = self.tx.create_with_id(ty, &stable_id(ty, what), fields.clone())
        {
            return Ok(object);
        }
        self.tx.create(ty, fields)
    }

    fn write(mut self, before: Option<&Document>, after: &Document) -> KdmResult<bool> {
        check_unique(after)?;
        let root = root_id();
        let names_before = before.map(|b| Names::of(b, self.ids)).unwrap_or_default();
        let settings_before = before.map(|b| settings(b, self.ids)).transpose()?.unwrap_or_default();
        let threaded_before = before.map(threaded).unwrap_or_default();
        let threaded_after = threaded(after);

        // 1. Objects of the lists.
        let swatches = self
            .plan("Swatch", before.map_or(&[][..], |b| &b.swatches[..]), &self.ids.swatches.clone(), &after.swatches, |s| pairing(&s.uid, &s.name))?;
        let group_objects: Vec<ObjectId> = self.ids.swatch_groups.iter().map(|(o, _)| o.clone()).collect();
        let groups = self.plan("SwatchGroup", before.map_or(&[][..], |b| &b.swatch_groups[..]), &group_objects, &after.swatch_groups, |g| {
            pairing(&g.uid, &g.name)
        })?;
        let mut group_swatches = Vec::with_capacity(after.swatch_groups.len());
        for (j, group) in after.swatch_groups.iter().enumerate() {
            let old = groups.partner.get(j).copied().flatten();
            let (old_swatches, old_objects) = match (old, before) {
                (Some(i), Some(b)) => (
                    b.swatch_groups.get(i).map_or(&[][..], |g| &g.swatches[..]),
                    self.ids.swatch_groups.get(i).map(|(_, o)| o.clone()).unwrap_or_default(),
                ),
                _ => (&[][..], Vec::new()),
            };
            group_swatches.push(self.plan("Swatch", old_swatches, &old_objects, &group.swatches, |s| pairing(&s.uid, &s.name))?);
        }
        let styles = self.plan(
            "GraphicStyle",
            before.map_or(&[][..], |b| &b.graphic_styles[..]),
            &self.ids.graphic_styles.clone(),
            &after.graphic_styles,
            |g| pairing(&g.uid, &g.name),
        )?;
        let chars =
            self.plan("CharStyle", before.map_or(&[][..], |b| &b.char_styles[..]), &self.ids.char_styles.clone(), &after.char_styles, |t| {
                pairing(&t.uid, &t.name)
            })?;
        let paras =
            self.plan("ParaStyle", before.map_or(&[][..], |b| &b.para_styles[..]), &self.ids.para_styles.clone(), &after.para_styles, |t| {
                pairing(&t.uid, &t.name)
            })?;
        let mut old_trees = std::mem::take(&mut self.ids.symbol_trees);
        if old_trees.len() != self.ids.symbols.len() {
            old_trees.clear();
        }
        let symbols =
            self.plan("Symbol", before.map_or(&[][..], |b| &b.symbols[..]), &self.ids.symbols.clone(), &after.symbols, |s| pairing(&s.uid, &s.name))?;
        let patterns = self.plan("Pattern", before.map_or(&[][..], |b| &b.patterns[..]), &self.ids.patterns.clone(), &after.patterns, |p| {
            pairing(&p.uid, &p.name)
        })?;
        let guides = self.plan("Guide", before.map_or(&[][..], |b| &b.guides[..]), &self.ids.guides.clone(), &after.guides, guide_key)?;
        let threads = self.plan_threads(before.map_or(&[][..], |b| &b.text_threads[..]), &after.text_threads)?;
        // Each symbol's art: its partner's, a new symbol's empty.
        let mut symbol_trees: Vec<TreeIds> = Vec::with_capacity(after.symbols.len());
        for (j, object) in symbols.objects.iter().enumerate() {
            let partner = symbols.partner.get(j).copied().flatten().filter(|_| !self.fresh.contains(object));
            symbol_trees.push(partner.and_then(|i| old_trees.get(i).cloned()).unwrap_or_default());
        }
        self.ids.swatches = swatches.objects.clone();
        self.ids.swatch_groups = groups.objects.iter().cloned().zip(group_swatches.iter().map(|p| p.objects.clone())).collect();
        self.ids.graphic_styles = styles.objects.clone();
        self.ids.char_styles = chars.objects.clone();
        self.ids.para_styles = paras.objects.clone();
        self.ids.symbols = symbols.objects.clone();
        self.ids.patterns = patterns.objects.clone();
        self.ids.guides = guides.objects.clone();
        self.ids.threads = threads.objects.clone();

        let artboard_keys = |d: Option<&Document>| d.map(|d| d.artboards.iter().map(|a| a.id).collect::<Vec<_>>()).unwrap_or_default();
        let slice_keys = |d: Option<&Document>| d.map(|d| d.slices.iter().map(|s| s.id).collect::<Vec<_>>()).unwrap_or_default();
        let asset_keys = |d: Option<&Document>| d.map(|d| d.assets.iter().map(|a| a.id).collect::<Vec<_>>()).unwrap_or_default();
        let artboards = self.plan_keyed("Artboard", |ids| &mut ids.artboards, &artboard_keys(before), &artboard_keys(Some(after)))?;
        let slices = self.plan_keyed("Slice", |ids| &mut ids.slices, &slice_keys(before), &slice_keys(Some(after)))?;
        let assets = self.plan_keyed("Asset", |ids| &mut ids.assets, &asset_keys(before), &asset_keys(Some(after)))?;
        self.write_images(before, after)?;

        let names_after = Names::of(after, self.ids);
        let side_before = Side { names: &names_before, threaded: &threaded_before };
        let side_after = Side { names: &names_after, threaded: &threaded_after };

        // 2. and 3. Node objects, the tree, and node registers.
        let before_index = before.map(|b| index(&b.layers)).unwrap_or_default();
        let layers_same = before.is_some_and(|b| same_list(&b.layers, &after.layers));
        let mut tree = TreeIds { nodes: std::mem::take(&mut self.ids.nodes), tree: std::mem::take(&mut self.ids.tree) };
        let place = Place { owner: root.clone(), art: None };
        if !layers_same {
            let old = before.map_or(&[][..], |b| &b.layers[..]);
            self.node_objects(&mut tree, &place, &before_index, &after.layers, &side_after)?;
            self.reconcile(&mut tree, &place, None, old, &after.layers, &before_index, &side_before, &side_after, 0)?;
            self.delete_nodes(&mut tree, old, &after.layers)?;
        }
        // A frame that joined or left a thread: its own text is now none, or its slice.
        let moved: Vec<NodeId> = threaded_before.symmetric_difference(&threaded_after).copied().collect();
        if !moved.is_empty() {
            let after_index = index(&after.layers);
            for id in moved {
                let (Some(n), Some(object)) = (after_index.get(&id), tree.nodes.object(id).cloned()) else { continue };
                if self.texts.contains(&object) || self.fresh.contains(&object) {
                    continue;
                }
                let old = before_index.get(&id).map(|o| node_text(o, &threaded_before, &names_before)).transpose()?.unwrap_or_default();
                let new = node_text(n, &threaded_after, &names_after)?;
                self.write_text(&object, &old, &new)?;
            }
        }
        self.ids.nodes = tree.nodes;
        self.ids.tree = tree.tree;

        // 4. Fields, lists and settings.
        self.write_named(
            &swatches,
            before.map_or(&[][..], |b| &b.swatches[..]),
            &after.swatches,
            &names_before,
            &names_after,
            |s| &s.uid,
            |s, n| Ok(vec![("key", key(&s.uid)), ("role", role(s)), ("name", json!(s.name)), ("props", named_props(s, &[], n)?)]),
        )?;
        self.order(&root, "swatches", &swatches.objects)?;
        for (j, group) in after.swatch_groups.iter().enumerate() {
            let (Some(object), Some(plan)) = (groups.objects.get(j).cloned(), group_swatches.get(j)) else { continue };
            let old = groups.partner.get(j).copied().flatten().filter(|_| !self.fresh.contains(&object));
            let old_group = old.and_then(|i| before.and_then(|b| b.swatch_groups.get(i)));
            if old_group.map(|g| &g.name) != Some(&group.name) {
                self.tx.set(&object, "name", json!(group.name))?;
                self.wrote = true;
            }
            if old_group.map(|g| g.uid.as_str()) != Some(group.uid.as_str()) {
                self.tx.set(&object, "key", key(&group.uid))?;
                self.wrote = true;
            }
            let old_swatches = old_group.map_or(&[][..], |g| &g.swatches[..]);
            self.write_named(
                plan,
                old_swatches,
                &group.swatches,
                &names_before,
                &names_after,
                |s| &s.uid,
                |s, n| Ok(vec![("key", key(&s.uid)), ("role", role(s)), ("name", json!(s.name)), ("props", named_props(s, &[], n)?)]),
            )?;
            self.order(&object, "swatches", &plan.objects)?;
        }
        self.order(&root, "swatchGroups", &groups.objects)?;
        self.write_named(
            &styles,
            before.map_or(&[][..], |b| &b.graphic_styles[..]),
            &after.graphic_styles,
            &names_before,
            &names_after,
            |g| &g.uid,
            |g, n| Ok(vec![("key", key(&g.uid)), ("name", json!(g.name)), ("props", named_props(g, &["id"], n)?)]),
        )?;
        self.order(&root, "graphicStyles", &styles.objects)?;
        for (plan, b, a, field) in [
            (&chars, before.map_or(&[][..], |b| &b.char_styles[..]), &after.char_styles, "charStyles"),
            (&paras, before.map_or(&[][..], |b| &b.para_styles[..]), &after.para_styles, "paraStyles"),
        ] {
            self.write_named(
                plan,
                b,
                a,
                &names_before,
                &names_after,
                |t| &t.uid,
                |t, n| Ok(vec![("key", key(&t.uid)), ("name", json!(t.name)), ("props", named_props(t, &[], n)?)]),
            )?;
            self.order(&root, field, &plan.objects)?;
        }
        self.write_named(
            &symbols,
            before.map_or(&[][..], |b| &b.symbols[..]),
            &after.symbols,
            &names_before,
            &names_after,
            |s| &s.uid,
            |s, _| Ok(vec![("key", key(&s.uid)), ("name", json!(s.name))]),
        )?;
        let no_threads = HashSet::new();
        let art_before = Side { names: &names_before, threaded: &no_threads };
        let art_after = Side { names: &names_after, threaded: &no_threads };
        for (j, symbol) in after.symbols.iter().enumerate() {
            let (Some(object), Some(tree)) = (symbols.objects.get(j).cloned(), symbol_trees.get_mut(j)) else { continue };
            let fresh = self.fresh.contains(&object);
            let old_art =
                symbols.partner.get(j).copied().flatten().filter(|_| !fresh).and_then(|i| before.and_then(|b| b.symbols.get(i))).map(|s| &s.art);
            if old_art.is_some_and(|art| Arc::ptr_eq(art, &symbol.art)) {
                continue;
            }
            let old = old_art.map_or(&[][..], std::slice::from_ref);
            let new = std::slice::from_ref(&symbol.art);
            let old_index = index(old);
            let prefix = if symbol.uid.is_empty() { format!("s{j}") } else { symbol.uid.as_str().to_owned() };
            let place = Place { owner: object, art: Some(prefix) };
            self.node_objects(tree, &place, &old_index, new, &art_after)?;
            self.reconcile(tree, &place, None, old, new, &old_index, &art_before, &art_after, 0)?;
            self.delete_nodes(tree, old, new)?;
        }
        self.ids.symbol_trees = symbol_trees;
        self.order(&root, "symbols", &symbols.objects)?;
        self.write_named(
            &patterns,
            before.map_or(&[][..], |b| &b.patterns[..]),
            &after.patterns,
            &names_before,
            &names_after,
            |p| &p.uid,
            |p, n| Ok(vec![("key", key(&p.uid)), ("name", json!(p.name)), ("props", named_props(p, &[], n)?)]),
        )?;
        self.order(&root, "patterns", &patterns.objects)?;

        let before_artboards: HashMap<u32, &Artboard> = before.map(|b| b.artboards.iter().map(|a| (a.id, a)).collect()).unwrap_or_default();
        for (a, object) in after.artboards.iter().zip(&artboards) {
            let old = before_artboards.get(&a.id).copied().filter(|_| !self.fresh.contains(object));
            if old != Some(a) {
                self.set_fields(object, old.map(artboard_fields).transpose()?, artboard_fields(a)?)?;
            }
        }
        self.order(&root, "artboards", &artboards)?;
        let before_slices: HashMap<NodeId, &Slice> = before.map(|b| b.slices.iter().map(|s| (s.id, s)).collect()).unwrap_or_default();
        for (s, object) in after.slices.iter().zip(&slices) {
            let old = before_slices.get(&s.id).copied().filter(|_| !self.fresh.contains(object));
            if old != Some(s) {
                self.set_fields(object, old.map(slice_fields).transpose()?, slice_fields(s)?)?;
            }
        }
        self.order(&root, "slices", &slices)?;
        let before_assets: HashMap<u64, &ExportAsset> = before.map(|b| b.assets.iter().map(|a| (a.id, a)).collect()).unwrap_or_default();
        for (a, object) in after.assets.iter().zip(&assets) {
            let old = before_assets.get(&a.id).copied().filter(|_| !self.fresh.contains(object));
            let old_fields = old.map(|o| asset_fields(o, self.ids)).transpose()?;
            let new_fields = asset_fields(a, self.ids)?;
            self.set_fields(object, old_fields, new_fields)?;
        }
        self.order(&root, "assets", &assets)?;

        for (j, guide) in after.guides.iter().enumerate() {
            let Some(object) = guides.objects.get(j).cloned() else { continue };
            let old =
                guides.partner.get(j).copied().flatten().and_then(|i| before.and_then(|b| b.guides.get(i))).filter(|_| !self.fresh.contains(&object));
            if old == Some(guide) {
                continue;
            }
            let old_fields = old.map(|o| guide_fields(o, self.ids));
            let new_fields = guide_fields(guide, self.ids);
            self.set_fields(&object, old_fields, new_fields)?;
        }
        self.order(&root, "guides", &guides.objects)?;

        for (j, frames) in after.text_threads.iter().enumerate() {
            let Some(object) = threads.objects.get(j).cloned() else { continue };
            let objects: Vec<ObjectId> = frames.iter().filter_map(|id| self.ids.nodes.object(*id).cloned()).collect();
            self.order(&object, "frames", &objects)?;
            let fresh = self.fresh.contains(&object);
            let old =
                threads.partner.get(j).copied().flatten().filter(|_| !fresh).and_then(|i| before.and_then(|b| b.text_threads.get(i).map(|t| (b, t))));
            // Neither the frames nor any of them changed: the story did not either.
            let same = old.is_some_and(|(b, t)| {
                t == frames
                    && frames.iter().all(|id| match (b.node_arc(*id), after.node_arc(*id)) {
                        (Some(x), Some(y)) => Arc::ptr_eq(x, y),
                        (None, None) => true,
                        _ => false,
                    })
            });
            if same {
                continue;
            }
            let old_text = match old {
                Some((b, t)) => thread_text(b, t, &names_before)?,
                None => Vec::new(),
            };
            let new_text = thread_text(after, frames, &names_after)?;
            self.write_text(&object, &old_text, &new_text)?;
        }
        self.order(&root, "threads", &threads.objects)?;

        let settings_after = settings(after, self.ids)?;
        if settings_after != settings_before {
            for (key, v) in &settings_after {
                if settings_before.get(key) != Some(v) {
                    self.tx.map_set(&root, "settings", key, v.clone())?;
                    self.wrote = true;
                }
            }
            for key in settings_before.keys() {
                if !settings_after.contains_key(key) {
                    self.tx.map_delete(&root, "settings", key)?;
                    self.wrote = true;
                }
            }
        }
        Ok(self.wrote)
    }

    fn order(&mut self, owner: &ObjectId, field: &str, objects: &[ObjectId]) -> KdmResult<()> {
        self.wrote |= sync_list(self.tx, owner, field, objects)? > 0;
        Ok(())
    }

    /// Makes `object`'s `text` hold `new`, the view saying it holds `old` (see the module docs).
    fn write_text(&mut self, object: &ObjectId, old: &[Delta], new: &[Delta]) -> KdmResult<()> {
        self.texts.insert(object.clone());
        if old == new {
            return Ok(());
        }
        let held = self.tx.text_len(object, "text")?;
        let said: usize = old.iter().map(|run| run.insert.chars().count()).sum();
        let old = if held == said {
            old
        } else {
            if held > 0 {
                self.tx.text_delete(object, "text", 0, held)?;
            }
            &[]
        };
        write_rich_text(self.tx, object, "text", old, new).map_err(kdm_err)?;
        self.wrote = true;
        Ok(())
    }

    /// Deletes `object`, the swatches a swatch group lists and the art a symbol holds.
    fn delete_deep(&mut self, object: &ObjectId) -> KdmResult<()> {
        if !self.tx.exists(object) {
            return Ok(());
        }
        match self.tx.object_type(object)?.as_str() {
            "SwatchGroup" => {
                for swatch in self.tx.list_items(object, "swatches")? {
                    if self.tx.exists(&swatch) {
                        self.tx.delete(&swatch)?;
                    }
                }
            }
            "Symbol" => {
                // Children before their parents (the tree lists parents first).
                for entry in self.tx.tree(object, "nodes")?.into_iter().rev() {
                    if self.tx.exists(&entry.object) {
                        self.tx.delete(&entry.object)?;
                    }
                }
            }
            _ => {}
        }
        self.wrote = true;
        self.tx.delete(object)
    }

    /// Pairs `after` with `before` (whose objects `before_objects` are, in order) by content,
    /// then by `name` (an entry's persistent identity, else its name: [`pairing`]); deletes the objects of unpaired `before` elements and creates (empty) ones
    /// for unpaired `after` elements.
    fn plan<T: Serialize + PartialEq>(
        &mut self,
        ty: &str,
        before: &[T],
        before_objects: &[ObjectId],
        after: &[T],
        name: impl Fn(&T) -> String,
    ) -> KdmResult<Plan> {
        // A view that does not match the objects it was read with is rewritten.
        let before = if before.len() == before_objects.len() {
            before
        } else {
            for object in before_objects {
                self.delete_deep(object)?;
            }
            &[]
        };
        let partner: Vec<Option<usize>> = if before == after && before.iter().map(&name).eq(after.iter().map(&name)) {
            (0..after.len()).map(Some).collect()
        } else {
            let b: Vec<(Value, String)> = before.iter().map(|x| Ok((value(x)?, name(x)))).collect::<KdmResult<_>>()?;
            let a: Vec<(Value, String)> = after.iter().map(|x| Ok((value(x)?, name(x)))).collect::<KdmResult<_>>()?;
            align(&b, &a)
        };
        let kept: HashSet<usize> = partner.iter().flatten().copied().collect();
        if before.len() == before_objects.len() {
            for (i, object) in before_objects.iter().enumerate() {
                if !kept.contains(&i) {
                    self.delete_deep(object)?;
                }
            }
        }
        let mut objects = Vec::with_capacity(after.len());
        for (j, p) in partner.iter().enumerate() {
            match p.and_then(|i| before_objects.get(i)) {
                Some(object) => objects.push(object.clone()),
                None => {
                    let what = after.get(j).map(&name).unwrap_or_default();
                    let object = self.create(ty, &what, Map::new())?;
                    self.fresh.insert(object.clone());
                    self.wrote = true;
                    objects.push(object);
                }
            }
        }
        Ok(Plan { objects, partner })
    }

    /// The `Thread` objects of `after`'s threads: each paired with the thread of `before` (whose
    /// objects [`Ids`] holds) that starts with the same frame, else that shares a frame with it;
    /// the rest of `before`'s are deleted and the rest of `after`'s get new ones.
    fn plan_threads(&mut self, before: &[Vec<NodeId>], after: &[Vec<NodeId>]) -> KdmResult<Plan> {
        let before_objects = self.ids.threads.clone();
        let before = if before.len() == before_objects.len() {
            before
        } else {
            for object in &before_objects {
                self.delete_deep(object)?;
            }
            &[]
        };
        let mut taken = vec![false; before.len()];
        let mut partner: Vec<Option<usize>> = vec![None; after.len()];
        for (j, frames) in after.iter().enumerate() {
            let first = before
                .iter()
                .enumerate()
                .position(|(i, b)| !taken.get(i).copied().unwrap_or(true) && !frames.is_empty() && b.first() == frames.first());
            let found = first.or_else(|| {
                before.iter().enumerate().position(|(i, b)| !taken.get(i).copied().unwrap_or(true) && b.iter().any(|f| frames.contains(f)))
            });
            if let Some(i) = found {
                if let Some(slot) = taken.get_mut(i) {
                    *slot = true;
                }
                if let Some(slot) = partner.get_mut(j) {
                    *slot = Some(i);
                }
            }
        }
        if before.len() == before_objects.len() {
            for (i, object) in before_objects.iter().enumerate() {
                if !taken.get(i).copied().unwrap_or(false) {
                    self.delete_deep(object)?;
                }
            }
        }
        let mut objects = Vec::with_capacity(after.len());
        for (j, p) in partner.iter().enumerate() {
            match p.and_then(|i| before_objects.get(i)) {
                Some(object) => objects.push(object.clone()),
                None => {
                    let what = after.get(j).and_then(|t| t.first()).map_or(0, |f| f.0).to_string();
                    let object = self.create("Thread", &what, Map::new())?;
                    self.fresh.insert(object.clone());
                    self.wrote = true;
                    objects.push(object);
                }
            }
        }
        Ok(Plan { objects, partner })
    }

    /// Writes the fields of a planned named list's elements that changed (`==` ignores an entry's
    /// uid, `uid` tells it).
    #[allow(clippy::too_many_arguments)]
    fn write_named<T: PartialEq>(
        &mut self,
        plan: &Plan,
        before: &[T],
        after: &[T],
        names_before: &Names,
        names_after: &Names,
        uid: impl Fn(&T) -> &kition_vector_doc::Uid,
        fields: impl Fn(&T, &Names) -> KdmResult<Fields>,
    ) -> KdmResult<()> {
        for (j, a) in after.iter().enumerate() {
            let Some(object) = plan.objects.get(j) else { continue };
            let old = plan.partner.get(j).copied().flatten().and_then(|i| before.get(i)).filter(|_| !self.fresh.contains(object));
            if old == Some(a) && old.is_some_and(|o| uid(o).as_str() == uid(a).as_str()) {
                continue;
            }
            let old_fields = old.map(|o| fields(o, names_before)).transpose()?;
            self.set_fields(object, old_fields, fields(a, names_after)?)?;
        }
        Ok(())
    }

    /// Objects by local id: deletes those of `before` keys `after` lacks, creates (empty) ones for
    /// new keys. Answers `after`'s objects in order.
    fn plan_keyed<K: Copy + Eq + Hash + std::fmt::Display>(
        &mut self,
        ty: &str,
        map: impl Fn(&mut Ids) -> &mut IdMap<K>,
        before: &[K],
        after: &[K],
    ) -> KdmResult<Vec<ObjectId>> {
        let after_set: HashSet<K> = after.iter().copied().collect();
        let before_set: HashSet<K> = before.iter().copied().collect();
        for key in before {
            if !after_set.contains(key)
                && let Some(object) = map(self.ids).remove_local(*key)
            {
                self.delete_deep(&object)?;
            }
        }
        let mut objects = Vec::with_capacity(after.len());
        for key in after {
            match map(self.ids).object(*key).cloned() {
                Some(object) if before_set.contains(key) => objects.push(object),
                _ => {
                    let object = self.create(ty, &key.to_string(), Map::new())?;
                    self.fresh.insert(object.clone());
                    self.wrote = true;
                    map(self.ids).insert(object.clone(), *key);
                    objects.push(object);
                }
            }
        }
        Ok(objects)
    }

    fn write_images(&mut self, before: Option<&Document>, after: &Document) -> KdmResult<()> {
        let root = root_id();
        if let Some(b) = before {
            for key in b.images.keys() {
                if !after.images.contains_key(key)
                    && let Some(object) = self.ids.images.remove(key)
                {
                    self.delete_deep(&object)?;
                }
            }
        }
        self.ids.images.retain(|key, _| after.images.contains_key(key));
        let mut objects = Vec::with_capacity(after.images.len());
        for (key, img) in &after.images {
            let old = before.and_then(|b| b.images.get(key));
            match (self.ids.images.get(key).cloned(), old) {
                (Some(object), Some(old)) => {
                    if !same_image(old, img) {
                        for (field, v) in image_fields(key, img, self.blobs)? {
                            if self.tx.get(&object, field)? != v {
                                self.tx.set(&object, field, v)?;
                                self.wrote = true;
                            }
                        }
                    }
                    objects.push(object);
                }
                (Some(stale), None) => {
                    self.delete_deep(&stale)?;
                    let fields = fields_map(image_fields(key, img, self.blobs)?);
                    let object = self.create("Image", key, fields)?;
                    self.fresh.insert(object.clone());
                    self.ids.images.insert(key.clone(), object.clone());
                    self.wrote = true;
                    objects.push(object);
                }
                _ => {
                    let fields = fields_map(image_fields(key, img, self.blobs)?);
                    let object = self.create("Image", key, fields)?;
                    self.fresh.insert(object.clone());
                    self.ids.images.insert(key.clone(), object.clone());
                    self.wrote = true;
                    objects.push(object);
                }
            }
        }
        self.order(&root, "images", &objects)
    }

    /// Creates (in full, with their text) the objects of the new nodes of the tree at `place`.
    /// Removed parents must survive reconciliation so retained children can move out before their
    /// former container is deleted.
    fn node_objects(
        &mut self,
        t: &mut TreeIds,
        place: &Place,
        before_index: &HashMap<NodeId, &Arc<Node>>,
        after: &[Arc<Node>],
        side: &Side<'_>,
    ) -> KdmResult<()> {
        let mut new = Vec::new();
        crate::ids::walk(after, 0, &mut |n| {
            let known = before_index.contains_key(&n.id) && t.nodes.object(n.id).is_some_and(|o| t.tree.contains_key(o));
            if !known {
                new.push(n.clone());
            }
        });
        for n in new {
            // An object the view lost track of (no tree node) is replaced, not shown twice.
            if let Some(stale) = t.nodes.remove_local(n.id)
                && self.tx.exists(&stale)
            {
                self.tx.delete(&stale)?;
            }
            let fields = art_fields(node_fields(&n, side.names)?, place.art.is_some());
            let what = match &place.art {
                Some(prefix) => format!("{prefix}n{}", n.id.0),
                None => n.id.0.to_string(),
            };
            let object = self.create("Node", &what, fields_map(fields))?;
            self.fresh.insert(object.clone());
            t.nodes.insert(object.clone(), n.id);
            self.write_text(&object, &[], &node_text(&n, side.threaded, side.names)?)?;
            self.wrote = true;
        }
        Ok(())
    }

    fn delete_nodes(&mut self, t: &mut TreeIds, before: &[Arc<Node>], after: &[Arc<Node>]) -> KdmResult<()> {
        let kept = index(after);
        let mut gone = Vec::new();
        crate::ids::walk(before, 0, &mut |n| {
            if !kept.contains_key(&n.id) {
                gone.push(n.id);
            }
        });
        for id in gone.into_iter().rev() {
            if let Some(object) = t.nodes.remove_local(id) {
                if self.tx.exists(&object) {
                    self.tx.delete(&object)?;
                }
                t.tree.remove(&object);
                self.wrote = true;
            }
        }
        Ok(())
    }

    /// Makes the tree at `place` hold `after` under `parent` (it held `before`; nothing if
    /// `parent` is new), writing each changed node's registers and text on the way (see the module
    /// docs).
    #[allow(clippy::too_many_arguments)]
    fn reconcile(
        &mut self,
        t: &mut TreeIds,
        place: &Place,
        parent: Option<&NodeRef>,
        before: &[Arc<Node>],
        after: &[Arc<Node>],
        before_index: &HashMap<NodeId, &Arc<Node>>,
        side_before: &Side<'_>,
        side_after: &Side<'_>,
        depth: usize,
    ) -> KdmResult<()> {
        if depth > MAX_DEPTH {
            return Err(kdm_err(format!("groups nest deeper than {MAX_DEPTH}")));
        }
        if same_list(before, after) {
            return Ok(());
        }
        let art = place.art.is_some();
        let before_pos: HashMap<NodeId, usize> = before.iter().enumerate().map(|(i, n)| (n.id, i)).collect();
        let mut prefix = 0;
        let mut previous: Option<usize> = None;
        for n in after {
            let fresh = t.nodes.object(n.id).is_none_or(|o| self.fresh.contains(o));
            let Some(&pos) = before_pos.get(&n.id).filter(|_| !fresh) else { break };
            if previous.is_some_and(|p| pos <= p) {
                break;
            }
            previous = Some(pos);
            prefix += 1;
        }
        for (k, n) in after.iter().enumerate() {
            let object = t.nodes.object(n.id).cloned().ok_or_else(|| kdm_err(format!("object {} has no replica object", n.id)))?;
            let fresh = self.fresh.contains(&object);
            let old = if fresh { None } else { before_index.get(&n.id).copied() };
            if !fresh && old.is_none_or(|o| !Arc::ptr_eq(o, n)) {
                let old_fields = old.map(|o| node_fields(o, side_before.names).map(|f| art_fields(f, art))).transpose()?;
                self.set_fields(&object, old_fields, art_fields(node_fields(n, side_after.names)?, art))?;
                let old_text = old.map(|o| node_text(o, side_before.threaded, side_before.names)).transpose()?.unwrap_or_default();
                let new_text = node_text(n, side_after.threaded, side_after.names)?;
                self.write_text(&object, &old_text, &new_text)?;
            }
            if k >= prefix {
                match t.tree.get(&object).cloned() {
                    Some(node) if !fresh => self.tx.tree_move(&place.owner, "nodes", &node, parent)?,
                    _ => {
                        let node = self.tx.tree_create(&place.owner, "nodes", parent, &object)?;
                        t.tree.insert(object.clone(), node);
                    }
                }
                self.wrote = true;
            }
            let Some(children) = n.children() else { continue };
            if old.is_some_and(|o| Arc::ptr_eq(o, n)) {
                continue;
            }
            let old_children = old.and_then(|o| o.children()).map_or(&[][..], |c| &c[..]);
            let node = t.tree.get(&object).cloned().ok_or_else(|| kdm_err(format!("object {} has no tree node", n.id)))?;
            self.reconcile(t, place, Some(&node), old_children, children, before_index, side_before, side_after, depth + 1)?;
        }
        Ok(())
    }

    /// Sets each field of `object` whose value differs from `before` (all of them, `null`s aside,
    /// when there is no `before`).
    fn set_fields(&mut self, object: &ObjectId, before: Option<Fields>, after: Fields) -> KdmResult<()> {
        for (i, (field, v)) in after.into_iter().enumerate() {
            let old = before.as_ref().and_then(|b| b.get(i)).map(|(_, v)| v);
            let differs = match old {
                Some(old) => *old != v,
                None => !v.is_null(),
            };
            if differs {
                self.tx.set(object, field, v)?;
                self.wrote = true;
            }
        }
        Ok(())
    }
}
