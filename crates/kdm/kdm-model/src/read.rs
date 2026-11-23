//! Replica -> `Document`, in two passes: the first walks every list and the trees (skipping
//! entries whose object is gone, with what is under them, and every object after its first
//! listing) and gives each object its local id — the one [`Known`] remembers for it, else the one
//! it was written with if no other object of the read has it, else a fresh one above every id the
//! view ever used — so the second can translate every reference stored as data while it reads the
//! fields. Last, each thread's story flows through its area-type frames
//! ([`kition_vector_text::thread::distribute`], as the engine flows it after every edit).
//!
//! With a [`Reuse`] view, a node whose object did not change keeps the view's own node (its
//! caches, and its `Arc` identity when its children did not change either), unless a named thing
//! a node can reference (a swatch, symbol, pattern or graphic style) changed.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::sync::Arc;

use kition_kit::kdm::{KdmError, ObjectId, Replica, TreeEntry};
use kition_kit::session::{BlobRef, BlobSource};
use kition_vector_doc::{Document, FORMAT_VERSION, Guide, ImageBlob, Node, NodeId, NodeKind, TextObject};
use serde_json::{Map, Value, json};

use crate::ids::{Ids, Known, TreeIds, index};
use crate::refs::{Locals, Mapper, SETTINGS_WITH_IDS, map_asset_nodes, map_setting, names_in};
use crate::text::decode;
use crate::write::{TEXT_KEYS, area_text, children_key_of};
use crate::{ImageCache, KdmResult, MAX_DEPTH, kdm_err, root_id};

/// Reads the whole drawing with fresh local ids.
pub fn read_document(replica: &Replica, blobs: &dyn BlobSource) -> KdmResult<(Document, Ids)> {
    let (d, ids, _) = read_with(replica, blobs, &Known::default(), &ImageCache::new(), None)?;
    Ok((d, ids))
}

/// Reads the whole drawing, giving every object `known` remembers the local id it had.
pub fn read_document_known(replica: &Replica, blobs: &dyn BlobSource, known: &Known) -> KdmResult<(Document, Ids)> {
    let (d, ids, _) = read_with(replica, blobs, known, &ImageCache::new(), None)?;
    Ok((d, ids))
}

/// A view to take unchanged nodes from (see the module docs).
pub(crate) struct Reuse<'a> {
    pub view: &'a Document,
    /// The view's ids.
    pub ids: &'a Ids,
    pub changed: &'a BTreeSet<ObjectId>,
}

fn object_of(v: Value) -> Map<String, Value> {
    match v {
        Value::Object(map) => map,
        _ => Map::new(),
    }
}

fn string(v: Value) -> String {
    match v {
        Value::String(s) => s,
        _ => String::new(),
    }
}

/// The layer tree as the replica lists it: entries in pre-order, each one's children, the roots,
/// and the local id of each entry read (`None`: skipped).
struct Tree {
    entries: Vec<TreeEntry>,
    children: Vec<Vec<usize>>,
    roots: Vec<usize>,
    local: Vec<Option<NodeId>>,
}

struct Reader<'r> {
    replica: &'r Replica,
    known: &'r Known,
    used: HashSet<u64>,
    next: u64,
    used_artboards: HashSet<u32>,
    next_artboard: u32,
    used_styles: HashSet<u32>,
    next_style: u32,
    seen: HashSet<ObjectId>,
    ids: Ids,
    locals: Locals,
}

pub(crate) fn read_with(
    replica: &Replica,
    blobs: &dyn BlobSource,
    known: &Known,
    cache: &ImageCache,
    reuse: Option<Reuse<'_>>,
) -> KdmResult<(Document, Ids, ImageCache)> {
    let root = root_id();
    if !replica.exists(&root) {
        return Err(KdmError::NotFound(root.to_string()));
    }
    let mut r = Reader {
        replica,
        known,
        used: HashSet::new(),
        next: known.max,
        used_artboards: HashSet::new(),
        next_artboard: known.max_artboard,
        used_styles: HashSet::new(),
        next_style: known.max_style,
        seen: HashSet::new(),
        ids: Ids::default(),
        locals: Locals::default(),
    };

    // Pass 1: structure and local ids.
    for object in r.live(&root, "swatches")? {
        let name = string(replica.get(&object, "name")?);
        r.locals.swatches.insert(object.clone(), name);
        r.ids.swatches.push(object);
    }
    for group in r.live(&root, "swatchGroups")? {
        let mut members = Vec::new();
        for object in r.live(&group, "swatches")? {
            let name = string(replica.get(&object, "name")?);
            r.locals.swatches.entry(object.clone()).or_insert(name);
            members.push(object);
        }
        r.ids.swatch_groups.push((group, members));
    }
    let mut style_ids = Vec::new();
    for object in r.live(&root, "graphicStyles")? {
        let id = r.mint_style(known.styles.get(&object).copied());
        r.locals.styles.insert(object.clone(), id);
        style_ids.push(id);
        r.ids.graphic_styles.push(object);
    }
    r.ids.char_styles = r.live(&root, "charStyles")?;
    r.ids.para_styles = r.live(&root, "paraStyles")?;
    for object in r.live(&root, "symbols")? {
        let name = string(replica.get(&object, "name")?);
        r.locals.symbols.insert(object.clone(), name);
        r.ids.symbols.push(object);
    }
    for object in r.live(&root, "patterns")? {
        let name = string(replica.get(&object, "name")?);
        r.locals.patterns.insert(object.clone(), name);
        r.ids.patterns.push(object);
    }
    let mut artboards = Vec::new();
    for object in r.live(&root, "artboards")? {
        let stored = replica.get(&object, "id")?.as_u64().and_then(|id| u32::try_from(id).ok());
        let id = r.mint_artboard(known.artboards.get(&object).copied().or(stored));
        r.ids.artboards.insert(object.clone(), id);
        r.locals.artboards.insert(object.clone(), id);
        artboards.push((object, id));
    }
    let tree = r.tree(&root)?;
    let mut art_trees = Vec::with_capacity(r.ids.symbols.len());
    for object in r.ids.symbols.clone() {
        let (art, ids) = r.art_tree(&object)?;
        art_trees.push(art);
        r.ids.symbol_trees.push(ids);
    }
    let mut slices = Vec::new();
    for object in r.live(&root, "slices")? {
        let stored = replica.get(&object, "id")?.as_u64();
        let id = NodeId(r.mint(known.slices.get(&object).map(|s| s.0).or(stored)));
        r.ids.slices.insert(object.clone(), id);
        slices.push((object, id));
    }
    let mut assets = Vec::new();
    for object in r.live(&root, "assets")? {
        let stored = replica.get(&object, "id")?.as_u64();
        let id = r.mint(known.assets.get(&object).copied().or(stored));
        r.ids.assets.insert(object.clone(), id);
        assets.push((object, id));
    }
    let images = r.live(&root, "images")?;
    let guide_objects = r.live(&root, "guides")?;
    let thread_objects = r.live(&root, "threads")?;

    // Pass 2: fields.
    let locals = std::mem::take(&mut r.locals);
    let node_in = |v: &Value| v.as_str().and_then(|s| locals.nodes.get(&ObjectId::from_string(s))).map(|id| Value::from(id.0));
    let artboard_in = |v: &Value| v.as_str().and_then(|s| locals.artboards.get(&ObjectId::from_string(s))).map(|id| Value::from(*id));
    let mapper = Mapper { node: &node_in, artboard: &artboard_in };

    let mut doc: Map<String, Value> = replica.map_entries(&root, "settings")?.into_iter().filter(|(_, v)| !v.is_null()).collect();
    // No print setup is written as an empty one ([`crate::write::settings`]).
    if doc.get("print_setup").is_some_and(|v| v.as_object().is_some_and(Map::is_empty)) {
        doc.remove("print_setup");
    }
    for key in SETTINGS_WITH_IDS {
        if let Some(v) = doc.get_mut(*key) {
            map_setting(key, v, &mapper);
            if v.is_null() {
                doc.remove(*key);
            }
        }
    }
    for key in crate::write::MODELLED {
        doc.remove(*key);
    }
    doc.entry("version").or_insert_with(|| Value::from(FORMAT_VERSION));

    let named = |object: &ObjectId, extra: &[(&str, Value)]| -> KdmResult<Value> {
        let mut map = object_of(replica.get(object, "props")?);
        map.insert("name".into(), Value::String(string(replica.get(object, "name")?)));
        // The entry's persistent identity is its tool id's field (`key`).
        map.insert("uid".into(), Value::String(string(replica.get(object, "key")?)));
        for (k, v) in extra {
            map.insert((*k).to_owned(), v.clone());
        }
        let mut v = Value::Object(map);
        names_in(&mut v, &locals);
        Ok(v)
    };
    let swatches: Vec<Value> = r.ids.swatches.iter().map(|o| named(o, &[])).collect::<KdmResult<_>>()?;
    let mut groups = Vec::new();
    for (group, members) in &r.ids.swatch_groups {
        let swatches: Vec<Value> = members.iter().map(|o| named(o, &[])).collect::<KdmResult<_>>()?;
        groups.push(json!({ "uid": string(replica.get(group, "key")?), "name": string(replica.get(group, "name")?), "swatches": swatches }));
    }
    let mut styles = Vec::new();
    for (object, id) in r.ids.graphic_styles.iter().zip(&style_ids) {
        styles.push(named(object, &[("id", Value::from(*id))])?);
    }
    let chars: Vec<Value> = r.ids.char_styles.iter().map(|o| named(o, &[])).collect::<KdmResult<_>>()?;
    let paras: Vec<Value> = r.ids.para_styles.iter().map(|o| named(o, &[])).collect::<KdmResult<_>>()?;
    // The art is read with the nodes, below.
    let placeholder = serde_json::to_value(empty_group()).map_err(kdm_err)?;
    let mut symbols = Vec::new();
    for object in &r.ids.symbols {
        symbols.push(json!({ "uid": string(replica.get(object, "key")?), "name": string(replica.get(object, "name")?), "art": placeholder }));
    }
    let patterns: Vec<Value> = r.ids.patterns.iter().map(|o| named(o, &[])).collect::<KdmResult<_>>()?;
    let mut boards = Vec::new();
    for (object, id) in &artboards {
        let mut map = object_of(replica.get(object, "props")?);
        map.insert("id".into(), Value::from(*id));
        map.insert("rect".into(), replica.get(object, "rect")?);
        boards.push(Value::Object(map));
    }
    let mut slice_values = Vec::new();
    for (object, id) in &slices {
        let mut map = object_of(replica.get(object, "props")?);
        map.insert("id".into(), Value::from(id.0));
        slice_values.push(Value::Object(map));
    }
    let mut asset_values = Vec::new();
    for (object, id) in &assets {
        let mut props = replica.get(object, "props")?;
        map_asset_nodes(&mut props, &node_in);
        let mut map = object_of(props);
        map.insert("id".into(), Value::from(*id));
        asset_values.push(Value::Object(map));
    }
    let next_id = r.used.iter().copied().max().unwrap_or(0).max(known.max).saturating_add(1);
    doc.insert("artboards".into(), Value::Array(boards));
    doc.insert("layers".into(), Value::Array(Vec::new()));
    doc.insert("swatches".into(), Value::Array(swatches));
    doc.insert("swatch_groups".into(), Value::Array(groups));
    doc.insert("graphic_styles".into(), Value::Array(styles));
    doc.insert("char_styles".into(), Value::Array(chars));
    doc.insert("para_styles".into(), Value::Array(paras));
    doc.insert("symbols".into(), Value::Array(symbols));
    doc.insert("patterns".into(), Value::Array(patterns));
    doc.insert("slices".into(), Value::Array(slice_values));
    doc.insert("assets".into(), Value::Array(asset_values));
    doc.insert("images".into(), Value::Object(Map::new()));
    doc.insert("next_id".into(), Value::from(next_id));
    let mut document: Document = serde_json::from_value(Value::Object(doc)).map_err(kdm_err)?;

    // Nodes, reusing the view's where nothing they depend on changed.
    let old = match &reuse {
        Some(reuse) if !names_changed(replica, reuse) => Some((index(&reuse.view.layers), reuse.changed)),
        _ => None,
    };
    let builder = Builder { replica, tree: &tree, locals: &locals, old: old.as_ref().map(|(index, changed)| (index, *changed)) };
    let mut layers = Vec::new();
    for &i in &tree.roots {
        if let Some(n) = builder.build(i, 0)? {
            layers.push(n);
        }
    }
    document.layers = layers;
    for (symbol, art) in document.symbols.iter_mut().zip(&art_trees) {
        let builder = Builder { replica, tree: art, locals: &locals, old: None };
        let mut first = None;
        for &i in &art.roots {
            if let Some(n) = builder.build(i, 0)? {
                first = Some(n);
                break;
            }
        }
        symbol.art = first.unwrap_or_else(|| Arc::new(empty_group()));
    }

    // Guides: one on a layer or artboard that is gone goes with it.
    for object in guide_objects {
        let on = |field: &str| -> KdmResult<Option<Option<ObjectId>>> {
            Ok(match replica.get(&object, field)? {
                Value::String(id) => Some(Some(ObjectId::from_string(id))),
                Value::Null => Some(None),
                _ => None,
            })
        };
        let (Some(artboard), Some(layer)) = (on("artboard")?, on("layer")?) else { continue };
        let artboard = match artboard {
            Some(a) => match locals.artboards.get(&a) {
                Some(id) => Some(*id),
                None => continue,
            },
            None => None,
        };
        let layer = match layer {
            Some(l) => match locals.nodes.get(&l) {
                Some(id) => Some(*id),
                None => continue,
            },
            None => None,
        };
        let vertical = replica.get(&object, "vertical")?.as_bool().unwrap_or(false);
        let pos = replica.get(&object, "pos")?.as_f64().unwrap_or(0.0);
        document.guides.push(Guide { vertical, pos, artboard, layer });
        r.ids.guides.push(object);
    }

    // Threads: each frame in the first that lists it; the story flows through the area-type ones.
    let mut claimed = HashSet::new();
    for object in thread_objects {
        let frames: Vec<NodeId> = replica
            .list(&object, "frames")?
            .iter()
            .filter_map(|o| locals.nodes.get(o).copied())
            .filter(|id| document.node(*id).is_some() && claimed.insert(*id))
            .collect();
        if frames.is_empty() {
            continue;
        }
        let delta = replica.text_delta(&object, "text")?;
        let area: Vec<NodeId> = frames.iter().copied().filter(|id| document.node(*id).is_some_and(|n| area_text(n).is_some())).collect();
        if !area.is_empty() && !delta.is_empty() {
            let story = decode(&delta, &locals)?;
            let texts: Vec<TextObject> = area.iter().filter_map(|id| document.node(*id).and_then(area_text).cloned()).collect();
            let refs: Vec<&TextObject> = texts.iter().collect();
            let parts = kition_vector_text::thread::distribute(kition_vector_text::FontDb::global(), &refs, &story.runs, &story.paras);
            for (id, part) in area.iter().zip(parts) {
                let same = document.node(*id).and_then(area_text).is_some_and(|t| t.runs == part.runs && t.paragraph_styles() == part.paras);
                if !same && let Some(NodeKind::Text(t)) = document.node_mut(*id).map(|n| &mut n.kind) {
                    t.runs = part.runs;
                    t.set_paragraph_styles(part.paras);
                    t.cached_bounds = None;
                    t.cached_baselines.clear();
                }
            }
        }
        document.text_threads.push(frames);
        r.ids.threads.push(object);
    }

    // Images.
    let mut out_cache = ImageCache::new();
    let mut taken = BTreeMap::new();
    for object in images {
        let key = string(replica.get(&object, "key")?);
        if key.is_empty() || taken.contains_key(&key) {
            continue;
        }
        let mime = string(replica.get(&object, "mime")?);
        let bytes = match blob(replica.get(&object, "blob")?, blobs, cache, &mut out_cache, &key)? {
            Some(bytes) => bytes,
            None => Arc::new(Vec::new()),
        };
        let proxy = blob(replica.get(&object, "proxy")?, blobs, cache, &mut out_cache, &key)?;
        let proxy = proxy.map(|p| if p == bytes { bytes.clone() } else { p });
        taken.insert(key.clone(), ImageBlob { mime, bytes, proxy });
        r.ids.images.insert(key, object);
    }
    document.images = taken;
    document.fix_next_id();
    Ok((document, r.ids, out_cache))
}

/// What a symbol without art (or whose art is gone) reads as: an empty group.
fn empty_group() -> Node {
    Node::new(NodeId(1), NodeKind::Group { children: Vec::new(), clip: false })
}

/// The bytes a blob reference names (from the view's cache when it already holds them).
fn blob(v: Value, blobs: &dyn BlobSource, cache: &ImageCache, out: &mut ImageCache, _key: &str) -> KdmResult<Option<Arc<Vec<u8>>>> {
    if v.is_null() {
        return Ok(None);
    }
    let r: BlobRef = serde_json::from_value(v).map_err(kdm_err)?;
    let bytes = match cache.get(&r.hash).or_else(|| out.get(&r.hash)) {
        Some(bytes) => bytes.clone(),
        None => match blobs.get(&r.hash) {
            Some(bytes) => Arc::new(bytes.to_vec()),
            // Not here yet (blobs travel apart from changes): an empty image, not cached, until
            // the bytes arrive (`VectorSession::add_blobs` reloads it). Failing the read would
            // take the drawing off its replica over bytes that are only late.
            None => return Ok(Some(Arc::new(Vec::new()))),
        },
    };
    out.insert(r.hash, bytes.clone());
    Ok(Some(bytes))
}

/// Whether a swatch, symbol, pattern or graphic style changed (a node naming one reads
/// differently even if its own object did not change). A deleted object counts if the view
/// listed it as one.
fn names_changed(replica: &Replica, reuse: &Reuse<'_>) -> bool {
    const NAMED: &[&str] = &["Swatch", "SwatchGroup", "Symbol", "Pattern", "GraphicStyle"];
    let ids = reuse.ids;
    let was_art = |o: &ObjectId| ids.symbol_trees.iter().any(|t| t.nodes.local(o).is_some());
    let was_named = |o: &ObjectId| {
        ids.swatches.contains(o)
            || ids.swatch_groups.iter().any(|(g, members)| g == o || members.contains(o))
            || ids.graphic_styles.contains(o)
            || ids.symbols.contains(o)
            || ids.patterns.contains(o)
            || was_art(o)
    };
    // A node of a symbol's art: the symbol's instances show it.
    let mut art: Option<BTreeSet<ObjectId>> = None;
    let mut is_art = |o: &ObjectId| {
        if was_art(o) {
            return true;
        }
        if ids.nodes.local(o).is_some() {
            return false;
        }
        let all = art.get_or_insert_with(|| {
            let symbols = replica.list(&root_id(), "symbols").unwrap_or_default();
            symbols.iter().flat_map(|s| replica.tree(s, "nodes").unwrap_or_default()).map(|entry| entry.object).collect()
        });
        all.contains(o)
    };
    reuse.changed.iter().any(|object| match replica.object_type(object) {
        Ok(ty) if ty == "Node" => is_art(object),
        Ok(ty) => NAMED.contains(&ty.as_str()),
        Err(_) => was_named(object),
    })
}

/// The view's nodes by id, and the objects changed since it was read: what a reread may reuse.
type Old<'b> = (&'b HashMap<NodeId, &'b Arc<Node>>, &'b BTreeSet<ObjectId>);

struct Builder<'b> {
    replica: &'b Replica,
    tree: &'b Tree,
    locals: &'b Locals,
    old: Option<Old<'b>>,
}

impl Builder<'_> {
    fn build(&self, i: usize, depth: usize) -> KdmResult<Option<Arc<Node>>> {
        let Some(id) = self.tree.local.get(i).copied().flatten() else { return Ok(None) };
        let Some(entry) = self.tree.entries.get(i) else { return Ok(None) };
        let mut children = Vec::new();
        if depth < MAX_DEPTH {
            for &c in self.tree.children.get(i).map_or(&[][..], |c| &c[..]) {
                if let Some(child) = self.build(c, depth + 1)? {
                    children.push(child);
                }
            }
        }
        if let Some((index, changed)) = self.old
            && !changed.contains(&entry.object)
            && let Some(prev) = index.get(&id)
        {
            let same = match prev.children() {
                Some(c) => c.len() == children.len() && c.iter().zip(&children).all(|(a, b)| Arc::ptr_eq(a, b)),
                None => children.is_empty(),
            };
            if same {
                return Ok(Some(Arc::clone(prev)));
            }
            let mut node = (***prev).clone();
            if let Some(c) = node.children_mut() {
                *c = children;
            }
            return Ok(Some(Arc::new(node)));
        }
        let mut node = self.node(&entry.object, id)?;
        if let Some(c) = node.children_mut() {
            *c = children;
        }
        Ok(Some(Arc::new(node)))
    }

    /// A node from its registers (without its children).
    fn node(&self, object: &ObjectId, id: NodeId) -> KdmResult<Node> {
        let get = |field: &str| self.replica.get(object, field);
        let mut map = object_of(get("props")?);
        if let Some(Value::String(style)) = map.remove("graphic_style")
            && let Some(local) = self.locals.styles.get(&ObjectId::from_string(style))
        {
            map.insert("graphic_style".into(), Value::from(*local));
        }
        let name = get("name")?;
        if !name.is_null() {
            map.insert("name".into(), name);
        }
        for field in ["visible", "locked"] {
            if let Value::Bool(b) = get(field)? {
                map.insert(field.into(), Value::Bool(b));
            }
        }
        let appearance = get("appearance")?;
        if !appearance.is_null() {
            map.insert("appearance".into(), appearance);
        }
        let mut data = object_of(get("data")?);
        if let Some(key) = data.get("type").and_then(Value::as_str).and_then(children_key_of) {
            data.insert(key.into(), Value::Array(Vec::new()));
        }
        let text = data.get("type").and_then(Value::as_str) == Some("text");
        if text {
            // The characters and paragraphs are the `text` field's.
            for key in TEXT_KEYS {
                data.remove(*key);
            }
            data.insert("runs".into(), Value::Array(Vec::new()));
        }
        map.insert("kind".into(), Value::Object(data));
        map.insert("id".into(), Value::from(id.0));
        let mut v = Value::Object(map);
        names_in(&mut v, self.locals);
        let mut node: Node = serde_json::from_value(v).map_err(|error| kdm_err(format!("object {object}: {error}")))?;
        if let NodeKind::Text(t) = &mut node.kind {
            let story = decode(&self.replica.text_delta(object, "text")?, self.locals)?;
            t.runs = story.runs;
            if !story.paras.is_empty() {
                t.set_paragraph_styles(story.paras);
            }
        }
        Ok(node)
    }
}

impl Reader<'_> {
    /// A fresh local id in the node/slice/asset space, or `known` if no object of this read has
    /// it yet.
    fn mint(&mut self, known: Option<u64>) -> u64 {
        if let Some(id) = known
            && id > 0
            && self.used.insert(id)
        {
            return id;
        }
        loop {
            self.next = self.next.saturating_add(1);
            if self.used.insert(self.next) || self.next == u64::MAX {
                return self.next;
            }
        }
    }

    fn mint_artboard(&mut self, known: Option<u32>) -> u32 {
        if let Some(id) = known
            && id > 0
            && self.used_artboards.insert(id)
        {
            return id;
        }
        loop {
            self.next_artboard = self.next_artboard.saturating_add(1);
            if self.used_artboards.insert(self.next_artboard) || self.next_artboard == u32::MAX {
                return self.next_artboard;
            }
        }
    }

    fn mint_style(&mut self, known: Option<u32>) -> u32 {
        if let Some(id) = known
            && id > 0
            && self.used_styles.insert(id)
        {
            return id;
        }
        loop {
            self.next_style = self.next_style.saturating_add(1);
            if self.used_styles.insert(self.next_style) || self.next_style == u32::MAX {
                return self.next_style;
            }
        }
    }

    /// The list's objects that exist and were not listed before (anywhere in this read).
    fn live(&mut self, owner: &ObjectId, field: &str) -> KdmResult<Vec<ObjectId>> {
        let items = self.replica.list(owner, field)?;
        Ok(items.into_iter().filter(|o| self.replica.exists(o) && self.seen.insert(o.clone())).collect())
    }

    /// The tree `owner.nodes` as the replica lists it: its entries in pre-order, each one's
    /// children, and the roots.
    fn shape(&self, owner: &ObjectId) -> KdmResult<(Vec<TreeEntry>, Vec<Vec<usize>>, Vec<usize>)> {
        let entries = self.replica.tree(owner, "nodes")?;
        let mut children = vec![Vec::new(); entries.len()];
        let mut roots = Vec::new();
        // Pre-order: a node's parent is on the stack of its ancestors.
        let mut stack: Vec<usize> = Vec::new();
        for (i, entry) in entries.iter().enumerate() {
            match &entry.parent {
                None => {
                    stack.clear();
                    roots.push(i);
                }
                Some(parent) => {
                    while stack.last().is_some_and(|&top| entries.get(top).is_none_or(|e| &e.node != parent)) {
                        stack.pop();
                    }
                    match stack.last().and_then(|&top| children.get_mut(top)) {
                        Some(list) => list.push(i),
                        None => roots.push(i),
                    }
                }
            }
            stack.push(i);
        }
        Ok((entries, children, roots))
    }

    /// The entries to read, in pre-order: skipping a node whose object is gone or was already
    /// read (with everything under it) and what nests deeper than the bridge goes.
    fn readable(&mut self, entries: &[TreeEntry], children: &[Vec<usize>], roots: &[usize]) -> Vec<usize> {
        let mut out = Vec::new();
        let mut pending: Vec<(usize, usize)> = roots.iter().rev().map(|&i| (i, 0)).collect();
        while let Some((i, depth)) = pending.pop() {
            let Some(entry) = entries.get(i) else { continue };
            if depth > MAX_DEPTH || !self.replica.exists(&entry.object) || !self.seen.insert(entry.object.clone()) {
                continue;
            }
            out.push(i);
            if let Some(list) = children.get(i) {
                for &c in list.iter().rev() {
                    pending.push((c, depth + 1));
                }
            }
        }
        out
    }

    /// The layer tree, with local ids for the nodes read ([`Reader::readable`]).
    fn tree(&mut self, root: &ObjectId) -> KdmResult<Tree> {
        let (entries, children, roots) = self.shape(root)?;
        let mut local = vec![None; entries.len()];
        for i in self.readable(&entries, &children, &roots) {
            let Some(entry) = entries.get(i) else { continue };
            let stored = self.replica.get(&entry.object, "id")?.as_u64();
            let id = NodeId(self.mint(self.known.nodes.get(&entry.object).map(|n| n.0).or(stored)));
            if let Some(slot) = local.get_mut(i) {
                *slot = Some(id);
            }
            self.ids.nodes.insert(entry.object.clone(), id);
            self.ids.tree.insert(entry.object.clone(), entry.node.clone());
            self.locals.nodes.insert(entry.object.clone(), id);
        }
        Ok(Tree { entries, children, roots, local })
    }

    /// A symbol's art tree, with local ids for the nodes read: the one [`Known`] remembers for each,
    /// else a fresh one unique in the symbol (the art's ids are its own: [`crate::ids`]).
    fn art_tree(&mut self, symbol: &ObjectId) -> KdmResult<(Tree, TreeIds)> {
        let (entries, children, roots) = self.shape(symbol)?;
        let order = self.readable(&entries, &children, &roots);
        let known: Vec<Option<NodeId>> =
            order.iter().map(|&i| entries.get(i).and_then(|e| self.known.symbol_nodes.get(&e.object).copied())).collect();
        let mut next = known.iter().flatten().map(|n| n.0).max().unwrap_or(0);
        let mut used = HashSet::new();
        let mut local = vec![None; entries.len()];
        let mut ids = TreeIds::default();
        for (k, &i) in order.iter().enumerate() {
            let Some(entry) = entries.get(i) else { continue };
            let id = match known.get(k).copied().flatten() {
                Some(id) if id.0 > 0 && used.insert(id) => id,
                _ => loop {
                    next = next.saturating_add(1);
                    if used.insert(NodeId(next)) || next == u64::MAX {
                        break NodeId(next);
                    }
                },
            };
            if let Some(slot) = local.get_mut(i) {
                *slot = Some(id);
            }
            ids.nodes.insert(entry.object.clone(), id);
            ids.tree.insert(entry.object.clone(), entry.node.clone());
        }
        Ok((Tree { entries, children, roots, local }, ids))
    }
}
