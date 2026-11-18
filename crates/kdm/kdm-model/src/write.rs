//! `Document` -> replica: a full write (a new replica, an import) and an incremental diff (the
//! engine command bridge) that writes only what changed.
//!
//! Both run the same passes, a full write being a diff from nothing:
//! 1. every new artboard, swatch, swatch group, style, symbol, pattern, slice, asset and image
//!    gets its object (named lists are paired with their previous version by content, then by
//!    name), and what `after` no longer holds is deleted;
//! 2. every new node gets its object, written in full;
//! 3. the tree is made to hold `after`'s layers: under each parent whose children changed, the
//!    longest run of children already in place (in order, from the first) stays and the rest are
//!    moved (or, new, created) after it in order — a tree only ever appends — so bringing one
//!    object to the front, or moving it into another layer on top, is one tree move; a subtree
//!    whose `Arc` did not change is skipped whole; removed nodes are then deleted child-first;
//! 4. every field that differs is set (a node's registers only when its `Arc` changed), every
//!    list is made to hold `after`'s order, and the settings map gets the keys that changed.

use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::sync::Arc;

use kition_kit::kdm::{NodeRef, ObjectId, Tx};
use kition_kit::session::{BlobSink, IdMap};
use kition_vector_doc::{Artboard, Document, ExportAsset, ImageBlob, Node, NodeId, NodeKind, Slice};
use serde::Serialize;
use serde_json::{Map, Value, json};

use crate::ids::{Ids, check_unique, index};
use crate::list::{align, sync_list};
use crate::refs::{Mapper, Names, SETTINGS_WITH_IDS, map_asset_nodes, map_setting, names_out};
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
/// it: a replica refuses such integers, and leaves the drawing without a replica. Live objects' ids
/// are never written (they are minted on read); the ids left are those inside art that is data
/// (opacity masks, symbols, patterns), which get fresh ids when used, and a mask's group of
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

/// A node's own registers: its name, visibility, lock, appearance, kind and geometry (`data`,
/// without its children) and the rest of the object (`props`). Names and the graphic style link
/// are written as object references through `names`.
pub(crate) fn node_fields(n: &Node, names: &Names) -> KdmResult<Fields> {
    let mut shallow = n.clone();
    if let Some(children) = shallow.children_mut() {
        children.clear();
    }
    let mut map = object_of(value(&shallow)?);
    map.remove("id");
    let mut data = map.remove("kind").unwrap_or(Value::Null);
    if let (Some(key), Value::Object(d)) = (children_key(&n.kind), &mut data) {
        d.remove(key);
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
pub(crate) fn settings(doc: &Document, ids: &Ids, names: &Names) -> KdmResult<Map<String, Value>> {
    let mut shell = doc.clone();
    shell.layers = Vec::new();
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
    if let Some(edit) = map.get_mut("pattern_edit") {
        names_out(edit, names);
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

struct Writer<'w, 'tx> {
    tx: &'w mut Tx<'tx>,
    ids: &'w mut Ids,
    blobs: &'w mut dyn BlobSink,
    /// Objects this write created: written in full, whatever `before` held.
    fresh: HashSet<ObjectId>,
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
        Self { tx, ids, blobs, fresh: HashSet::new(), wrote: false, stable }
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
        let settings_before = before.map(|b| settings(b, self.ids, &names_before)).transpose()?.unwrap_or_default();

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
        let symbols =
            self.plan("Symbol", before.map_or(&[][..], |b| &b.symbols[..]), &self.ids.symbols.clone(), &after.symbols, |s| pairing(&s.uid, &s.name))?;
        let patterns = self.plan("Pattern", before.map_or(&[][..], |b| &b.patterns[..]), &self.ids.patterns.clone(), &after.patterns, |p| {
            pairing(&p.uid, &p.name)
        })?;
        self.ids.swatches = swatches.objects.clone();
        self.ids.swatch_groups = groups.objects.iter().cloned().zip(group_swatches.iter().map(|p| p.objects.clone())).collect();
        self.ids.graphic_styles = styles.objects.clone();
        self.ids.char_styles = chars.objects.clone();
        self.ids.para_styles = paras.objects.clone();
        self.ids.symbols = symbols.objects.clone();
        self.ids.patterns = patterns.objects.clone();

        let artboard_keys = |d: Option<&Document>| d.map(|d| d.artboards.iter().map(|a| a.id).collect::<Vec<_>>()).unwrap_or_default();
        let slice_keys = |d: Option<&Document>| d.map(|d| d.slices.iter().map(|s| s.id).collect::<Vec<_>>()).unwrap_or_default();
        let asset_keys = |d: Option<&Document>| d.map(|d| d.assets.iter().map(|a| a.id).collect::<Vec<_>>()).unwrap_or_default();
        let artboards = self.plan_keyed("Artboard", |ids| &mut ids.artboards, &artboard_keys(before), &artboard_keys(Some(after)))?;
        let slices = self.plan_keyed("Slice", |ids| &mut ids.slices, &slice_keys(before), &slice_keys(Some(after)))?;
        let assets = self.plan_keyed("Asset", |ids| &mut ids.assets, &asset_keys(before), &asset_keys(Some(after)))?;
        self.write_images(before, after)?;

        let names_after = Names::of(after, self.ids);

        // 2. Node objects.
        let before_index = before.map(|b| index(&b.layers)).unwrap_or_default();
        let layers_same = before.is_some_and(|b| same_list(&b.layers, &after.layers));
        if !layers_same {
            self.node_objects(&before_index, after, &names_after)?;
        }

        // 3. The tree, and node registers.
        if !layers_same {
            let old = before.map_or(&[][..], |b| &b.layers[..]);
            self.reconcile(None, old, &after.layers, &before_index, &names_before, &names_after, 0)?;
            self.delete_nodes(old, after)?;
        }

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
            |s, n| {
                let mut art = value(&s.art)?;
                names_out(&mut art, n);
                Ok(vec![("key", key(&s.uid)), ("name", json!(s.name)), ("art", art)])
            },
        )?;
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

        let settings_after = settings(after, self.ids, &names_after)?;
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

    /// Deletes `object` and the swatches a swatch group lists.
    fn delete_deep(&mut self, object: &ObjectId) -> KdmResult<()> {
        if !self.tx.exists(object) {
            return Ok(());
        }
        if self.tx.object_type(object)? == "SwatchGroup" {
            for swatch in self.tx.list_items(object, "swatches")? {
                if self.tx.exists(&swatch) {
                    self.tx.delete(&swatch)?;
                }
            }
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

    /// Creates (in full) the objects of new nodes. Removed parents must survive reconciliation
    /// so retained children can move out before their former container is deleted.
    fn node_objects(&mut self, before_index: &HashMap<NodeId, &Arc<Node>>, after: &Document, names: &Names) -> KdmResult<()> {
        let mut new = Vec::new();
        crate::ids::walk(&after.layers, 0, &mut |n| {
            let known = before_index.contains_key(&n.id) && self.ids.nodes.object(n.id).is_some_and(|o| self.ids.tree.contains_key(o));
            if !known {
                new.push(n.clone());
            }
        });
        for n in new {
            // An object the view lost track of (no tree node) is replaced, not shown twice.
            if let Some(stale) = self.ids.nodes.remove_local(n.id)
                && self.tx.exists(&stale)
            {
                self.tx.delete(&stale)?;
            }
            let object = self.create("Node", &n.id.0.to_string(), fields_map(node_fields(&n, names)?))?;
            self.fresh.insert(object.clone());
            self.ids.nodes.insert(object, n.id);
            self.wrote = true;
        }
        Ok(())
    }

    fn delete_nodes(&mut self, before: &[Arc<Node>], after: &Document) -> KdmResult<()> {
        let kept = index(&after.layers);
        let mut gone = Vec::new();
        crate::ids::walk(before, 0, &mut |n| {
            if !kept.contains_key(&n.id) {
                gone.push(n.id);
            }
        });
        for id in gone.into_iter().rev() {
            if let Some(object) = self.ids.nodes.remove_local(id) {
                if self.tx.exists(&object) {
                    self.tx.delete(&object)?;
                }
                self.ids.tree.remove(&object);
                self.wrote = true;
            }
        }
        Ok(())
    }

    /// Makes the tree under `parent` hold `after` (it held `before`; nothing if `parent` is new),
    /// writing each changed node's registers on the way (see the module docs).
    #[allow(clippy::too_many_arguments)]
    fn reconcile(
        &mut self,
        parent: Option<&NodeRef>,
        before: &[Arc<Node>],
        after: &[Arc<Node>],
        before_index: &HashMap<NodeId, &Arc<Node>>,
        names_before: &Names,
        names_after: &Names,
        depth: usize,
    ) -> KdmResult<()> {
        if depth > MAX_DEPTH {
            return Err(kdm_err(format!("groups nest deeper than {MAX_DEPTH}")));
        }
        if same_list(before, after) {
            return Ok(());
        }
        let root = root_id();
        let before_pos: HashMap<NodeId, usize> = before.iter().enumerate().map(|(i, n)| (n.id, i)).collect();
        let mut prefix = 0;
        let mut previous: Option<usize> = None;
        for n in after {
            let fresh = self.ids.nodes.object(n.id).is_none_or(|o| self.fresh.contains(o));
            let Some(&pos) = before_pos.get(&n.id).filter(|_| !fresh) else { break };
            if previous.is_some_and(|p| pos <= p) {
                break;
            }
            previous = Some(pos);
            prefix += 1;
        }
        for (k, n) in after.iter().enumerate() {
            let object = self.ids.nodes.object(n.id).cloned().ok_or_else(|| kdm_err(format!("object {} has no replica object", n.id)))?;
            let fresh = self.fresh.contains(&object);
            let old = if fresh { None } else { before_index.get(&n.id).copied() };
            if !fresh && old.is_none_or(|o| !Arc::ptr_eq(o, n)) {
                let old_fields = old.map(|o| node_fields(o, names_before)).transpose()?;
                self.set_fields(&object, old_fields, node_fields(n, names_after)?)?;
            }
            if k >= prefix {
                match self.ids.tree.get(&object).cloned() {
                    Some(node) if !fresh => self.tx.tree_move(&root, "nodes", &node, parent)?,
                    _ => {
                        let node = self.tx.tree_create(&root, "nodes", parent, &object)?;
                        self.ids.tree.insert(object.clone(), node);
                    }
                }
                self.wrote = true;
            }
            let Some(children) = n.children() else { continue };
            if old.is_some_and(|o| Arc::ptr_eq(o, n)) {
                continue;
            }
            let old_children = old.and_then(|o| o.children()).map_or(&[][..], |c| &c[..]);
            let node = self.ids.tree.get(&object).cloned().ok_or_else(|| kdm_err(format!("object {} has no tree node", n.id)))?;
            self.reconcile(Some(&node), old_children, children, before_index, names_before, names_after, depth + 1)?;
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
