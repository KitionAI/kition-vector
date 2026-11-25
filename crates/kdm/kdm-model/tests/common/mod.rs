//! Shared test helpers: a drawing to edit, a blob store shared between replicas, replicas over
//! `KitSession<VectorModel>` that take each other's changes in order, and drafts merged back.
//! Each `tests/*.rs` binary uses only some.
#![allow(dead_code, clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::{BTreeSet, HashMap};
use std::sync::{Arc, Mutex};

use kdm_model::{Ids, VectorModel, write_changes, write_document};
use kition_kit::kdm::{ActorId, DocId, ObjectId, Origin, Replica, Version};
use kition_kit::session::{BlobRef, BlobSink, BlobSource, KitError, KitModel, KitSession, sha256_hex};
use kition_vector_doc::color::{Color, Paint, Swatch};
use kition_vector_doc::geom::{Affine, Rect, shapes};
use kition_vector_doc::{
    Appearance, Artboard, Document, ExportAsset, Guide, ImageBlob, ImageObject, Node, NodeId, NodeKind, PatternDef, SavedSelection, Slice, Symbol,
    TextStyleDef,
};

pub type Session = KitSession<VectorModel>;

/// One blob store every replica of a test shares (blobs live outside the replica).
#[derive(Clone, Default)]
pub struct SharedBlobs(pub Arc<Mutex<HashMap<String, Arc<[u8]>>>>);

impl BlobSource for SharedBlobs {
    fn get(&self, hash: &str) -> Option<Arc<[u8]>> {
        self.0.lock().unwrap_or_else(std::sync::PoisonError::into_inner).get(hash).cloned()
    }
}

impl BlobSink for SharedBlobs {
    fn put(&mut self, bytes: &[u8], mime: &str) -> BlobRef {
        let hash = sha256_hex(bytes);
        self.0.lock().unwrap_or_else(std::sync::PoisonError::into_inner).entry(hash.clone()).or_insert_with(|| Arc::from(bytes));
        BlobRef { hash, mime: mime.to_owned(), size: bytes.len() as u64 }
    }
}

pub fn actor(value: u64) -> ActorId {
    ActorId::from_u64(value).unwrap_or_else(|_| panic!("actor {value}"))
}

pub fn person(name: &str) -> Origin {
    Origin::person(format!("user:{name}"))
}

/// A filled rectangle path named `name`.
pub fn rect_node(d: &mut Document, name: &str, x: f64, y: f64) -> Node {
    let id = d.alloc_id();
    let mut n = Node::path(
        id,
        shapes::rectangle(Rect::new(x, y, x + 40.0, y + 30.0)),
        Appearance::basic(Paint::solid(Color::rgb(0.2, 0.4, 0.8)), Paint::None, 0.0),
    );
    n.name = Some(name.into());
    n
}

/// A drawing exercising every replicated reference: two layers; on the first a path filled with
/// a global swatch and linked to a graphic style, a group of two paths, a symbol instance, an
/// embedded image; a second artboard; a symbol, a pattern, a character style; guides on a layer
/// and on an artboard; a text thread, a saved selection, a variable binding, an export asset and
/// a slice.
pub fn drawing() -> Document {
    let mut d = Document::new(600.0, 400.0);
    let layer = d.layers[0].id;
    let second = d.add_layer(Some("Notes"));

    d.swatches.push(Swatch {
        uid: Default::default(),
        name: "Brand".into(),
        paint: Paint::solid(Color::rgb(0.9, 0.1, 0.1)),
        global: true,
        spot: false,
    });
    if let Some(style) = d.graphic_styles.first_mut() {
        style.id = 7;
    }
    let mut a = rect_node(&mut d, "A", 10.0, 10.0);
    a.appearance.set_fill(Paint::Solid { color: Color::rgb(0.9, 0.1, 0.1), swatch: Some("Brand".into()), tint: 1.0 });
    a.graphic_style = Some(7);
    let a_id = a.id;
    d.insert(Some(layer), usize::MAX, a).unwrap();

    let b = rect_node(&mut d, "B", 100.0, 10.0);
    let c = rect_node(&mut d, "C", 160.0, 10.0);
    let (b_id, c_id) = (b.id, c.id);
    let gid = d.alloc_id();
    let mut group = Node::group(gid, vec![Arc::new(b), Arc::new(c)]);
    group.name = Some("Group".into());
    d.insert(Some(layer), usize::MAX, group).unwrap();

    let art_id = d.alloc_id();
    let art = Node::path(art_id, shapes::ellipse(Rect::new(0.0, 0.0, 20.0, 20.0)), Appearance::basic(Paint::solid(Color::BLACK), Paint::None, 0.0));
    d.symbols.push(Symbol { uid: Default::default(), name: "Star".into(), art: Arc::new(art) });
    let iid = d.alloc_id();
    let mut instance = Node::new(iid, NodeKind::SymbolInstance { symbol: "Star".into(), xf: Affine::translate((300.0, 50.0)) });
    instance.name = Some("Instance".into());
    d.insert(Some(layer), usize::MAX, instance).unwrap();

    let png = vec![0x89, b'P', b'N', b'G', 1, 2, 3, 4];
    d.images.insert("img-1".into(), ImageBlob::new("image/png", png));
    let image_id = d.alloc_id();
    let mut image = Node::new(
        image_id,
        NodeKind::Image(ImageObject {
            key: "img-1".into(),
            width: 2,
            height: 2,
            xf: Affine::translate((400.0, 200.0)),
            link: None,
            placement: Default::default(),
        }),
    );
    image.name = Some("Picture".into());
    d.insert(Some(layer), usize::MAX, image).unwrap();

    let note = rect_node(&mut d, "Note", 20.0, 300.0);
    d.insert(Some(second), usize::MAX, note).unwrap();

    d.artboards.push(Artboard {
        id: 2,
        name: "Second".into(),
        rect: Rect::new(700.0, 0.0, 900.0, 200.0),
        show_center_mark: false,
        show_cross_hairs: false,
    });
    let tile = rect_node(&mut d, "Tile", 0.0, 0.0);
    d.patterns.push(PatternDef::new("Dots", vec![Arc::new(tile)]));
    d.char_styles.push(TextStyleDef { uid: Default::default(), name: "Emphasis".into(), attrs: Default::default() });
    d.guides.push(Guide { vertical: true, pos: 50.0, artboard: None, layer: Some(layer) });
    d.guides.push(Guide { vertical: false, pos: 20.0, artboard: Some(2), layer: Some(second) });
    d.text_threads.push(vec![b_id, c_id]);
    d.saved_selections.push(SavedSelection { name: "Both".into(), objects: vec![a_id, b_id] });
    d.variables.bindings.insert(a_id, BTreeSet::from(["Title".to_string()]));
    let asset = d.alloc_id().0;
    d.assets.push(ExportAsset { id: asset, name: "Asset 1".into(), nodes: vec![a_id] });
    let slice = d.alloc_id();
    d.slices.push(Slice { id: slice, rect: Rect::new(0.0, 0.0, 50.0, 50.0), options: Default::default() });
    d
}

/// The node named `name` (depth first).
pub fn named<'a>(d: &'a Document, name: &str) -> &'a Node {
    let mut found = None;
    d.walk(|n| {
        if found.is_none() && n.name.as_deref() == Some(name) {
            found = Some(n);
        }
    });
    found.unwrap_or_else(|| panic!("no object named {name}"))
}

pub fn named_id(d: &Document, name: &str) -> NodeId {
    named(d, name).id
}

/// Moves the path `id` by `dx`.
pub fn move_path(d: &mut Document, id: NodeId, dx: f64) {
    if let Some(NodeKind::Path { path, .. }) = d.node_mut(id).map(|n| &mut n.kind) {
        path.transform(Affine::translate((dx, 0.0)));
    }
}

/// Fills `id` with `color` (no swatch link).
pub fn recolor(d: &mut Document, id: NodeId, color: Color) {
    if let Some(n) = d.node_mut(id) {
        n.appearance.set_fill(Paint::solid(color));
    }
}

/// A fresh replica of `d` (its creating transaction, not an undo step) and the ids it minted.
pub fn new_replica(d: &Document, blobs: &SharedBlobs) -> (Replica, Ids) {
    let mut ids = Ids::default();
    let mut sink = blobs.clone();
    let (replica, ()) = Replica::create_with(VectorModel::schema(), DocId::new(), actor(1), &Origin::system("create"), |tx| {
        write_document(tx, d, &mut ids, &mut sink).map(|_| ())
    })
    .unwrap_or_else(|error| panic!("create: {error}"));
    (replica, ids)
}

pub fn first(d: &Document, blobs: &SharedBlobs) -> Session {
    let (replica, _) = new_replica(d, blobs);
    KitSession::from_replica(replica, Box::new(blobs.clone())).unwrap_or_else(|error| panic!("attach: {error}"))
}

pub fn peer(of: &Session, value: u64, blobs: &SharedBlobs) -> Session {
    let snapshot = of.snapshot().unwrap_or_else(|error| panic!("snapshot: {error}"));
    KitSession::open_snapshot(&snapshot, Box::new(blobs.clone()), actor(value)).unwrap_or_else(|error| panic!("open_snapshot: {error}"))
}

/// Edits `session`'s view with `f` and records what changed, the way the engine bridge does.
pub fn edit(session: &mut Session, blobs: &SharedBlobs, f: impl FnOnce(&mut Document)) {
    edit_as(session, blobs, &person("test"), f);
}

/// [`edit`] recorded as `origin` (an `import` one: a file changed outside, read again).
pub fn edit_as(session: &mut Session, blobs: &SharedBlobs, origin: &Origin, f: impl FnOnce(&mut Document)) {
    let before = session.model().document().clone();
    let mut after = (*before).clone();
    f(&mut after);
    let after = Arc::new(after);
    let mut ids = session.model().ids().clone();
    let mut sink = blobs.clone();
    session
        .record(origin, |tx| write_changes(tx, &before, &after, &mut ids, &mut sink).map(|_| ()).map_err(KitError::from))
        .unwrap_or_else(|error| panic!("record: {error}"));
    session.model_mut().adopt(after, ids);
}

/// A draft of `of` (a fork of its replica, actor `value`) and the version it was made at.
pub fn draft(of: &Session, value: u64, blobs: &SharedBlobs) -> (Session, Version) {
    let draft = of.fork(actor(value), Box::new(blobs.clone())).unwrap_or_else(|error| panic!("fork: {error}"));
    (draft, of.version())
}

/// Merges `draft`'s changes since `base` into `into` as the person accepting them
/// (`merge_changes`, data model 6.6); a refusal is the error.
pub fn merge(draft: &Session, base: &Version, into: &mut Session) -> Result<(), String> {
    let changes = draft.export_changes(base).unwrap_or_else(|error| panic!("export: {error}"));
    into.merge_changes(&changes, &person("test")).map(|_| ()).map_err(|error| error.to_string())
}

/// Sends `from`'s changes that `to` lacks.
pub fn send(from: &Session, to: &mut Session) {
    let bytes = from.export_changes(&to.version()).unwrap_or_else(|error| panic!("export: {error}"));
    to.import_changes(&bytes).unwrap_or_else(|error| panic!("import: {error}"));
}

pub fn sync(a: &mut Session, b: &mut Session) {
    send(a, b);
    send(b, a);
    send(a, b);
    assert_eq!(a.version(), b.version(), "the replicas converged");
}

/// The view as its id-blind JSON, for comparing views of different replicas.
pub fn canonical(d: &Document) -> serde_json::Value {
    kdm_model::normalize(d)
}

/// The paths in which two canonical JSON values differ (the first few), for failure messages.
pub fn differences(a: &serde_json::Value, b: &serde_json::Value) -> Vec<String> {
    fn rec(a: &serde_json::Value, b: &serde_json::Value, at: String, out: &mut Vec<String>) {
        if out.len() >= 8 || a == b {
            return;
        }
        match (a, b) {
            (serde_json::Value::Object(x), serde_json::Value::Object(y)) => {
                let keys: BTreeSet<&String> = x.keys().chain(y.keys()).collect();
                for k in keys {
                    rec(x.get(k).unwrap_or(&serde_json::Value::Null), y.get(k).unwrap_or(&serde_json::Value::Null), format!("{at}.{k}"), out);
                }
            }
            (serde_json::Value::Array(x), serde_json::Value::Array(y)) if x.len() == y.len() => {
                for (i, (p, q)) in x.iter().zip(y).enumerate() {
                    rec(p, q, format!("{at}[{i}]"), out);
                }
            }
            _ => {
                let short = |v: &serde_json::Value| v.to_string().chars().take(160).collect::<String>();
                out.push(format!("{at}: {} != {}", short(a), short(b)));
            }
        }
    }
    let mut out = Vec::new();
    rec(a, b, String::new(), &mut out);
    out
}

pub fn assert_same_content(what: &str, a: &Document, b: &Document) {
    let (x, y) = (canonical(a), canonical(b));
    assert!(x == y, "{what}: the drawings differ:\n{:#?}", differences(&x, &y));
}

/// The object of the node named `name` in `session`'s view.
pub fn node_object(session: &Session, name: &str) -> ObjectId {
    let d = session.model().document();
    session.model().ids().node(named_id(d, name)).unwrap_or_else(|| panic!("no object for {name}")).clone()
}

/// The replica holds no reference to a deleted object (what the repair after a merge clears):
/// list entries, symbol instances, graphic style links, assets and settings.
pub fn assert_replica_clean(replica: &Replica) {
    let gone =
        |v: Option<&serde_json::Value>| matches!(v, Some(serde_json::Value::String(id)) if !replica.exists(&ObjectId::from_string(id.as_str())));
    let root = ObjectId::from_string("drawing");
    for field in [
        "artboards",
        "swatches",
        "swatchGroups",
        "graphicStyles",
        "charStyles",
        "paraStyles",
        "symbols",
        "patterns",
        "slices",
        "assets",
        "images",
        "guides",
        "threads",
    ] {
        for item in replica.list(&root, field).unwrap_or_default() {
            assert!(replica.exists(&item), "drawing.{field} lists deleted {item}");
        }
    }
    for node in replica.objects_of_type("Node") {
        let data = replica.get(&node, "data").unwrap_or_default();
        if data.get("type").and_then(|t| t.as_str()) == Some("symbolinstance") {
            assert!(!gone(data.get("symbol").and_then(|s| s.get("ref"))), "instance {node} names a deleted symbol");
        }
        let props = replica.get(&node, "props").unwrap_or_default();
        assert!(!gone(props.get("graphic_style")), "node {node} links a deleted graphic style");
    }
    for asset in replica.objects_of_type("Asset") {
        let props = replica.get(&asset, "props").unwrap_or_default();
        for n in props.get("nodes").and_then(|n| n.as_array()).into_iter().flatten() {
            assert!(!gone(Some(n)), "asset {asset} names a deleted object");
        }
    }
    for thread in replica.objects_of_type("Thread") {
        let frames = replica.list(&thread, "frames").unwrap_or_default();
        assert!(!frames.is_empty(), "thread {thread} has no frames left");
        for frame in frames {
            assert!(replica.exists(&frame), "thread {thread} names deleted {frame}");
        }
    }
    for guide in replica.objects_of_type("Guide") {
        for field in ["layer", "artboard"] {
            assert!(!gone(Some(&replica.get(&guide, field).unwrap_or_default())), "guide {guide} is on a deleted {field}");
        }
    }
    let saved = replica.map_get(&root, "settings", "saved_selections").unwrap_or_default();
    for o in saved.as_array().into_iter().flatten().flat_map(|s| s.get("objects").and_then(|o| o.as_array()).into_iter().flatten()) {
        assert!(!gone(Some(o)), "a saved selection names a deleted object");
    }
}

/// Every reference in `d` resolves: symbol instances, graphic style links, threads, guides'
/// layers and artboards, saved selections, assets and variable bindings; and node ids are unique.
pub fn assert_no_dangling(d: &Document) {
    let mut ids = std::collections::HashSet::new();
    d.walk(|n| assert!(ids.insert(n.id), "duplicate object id {}", n.id));
    let symbols: std::collections::HashSet<&str> = d.symbols.iter().map(|s| s.name.as_str()).collect();
    let styles: std::collections::HashSet<u32> = d.graphic_styles.iter().map(|g| g.id).collect();
    d.walk(|n| {
        if let NodeKind::SymbolInstance { symbol, .. } = &n.kind {
            assert!(symbols.contains(symbol.as_str()), "instance {} of symbol {symbol}, which is gone", n.id);
        }
        if let Some(style) = n.graphic_style {
            assert!(styles.contains(&style), "object {} links graphic style {style}, which is gone", n.id);
        }
    });
    for thread in &d.text_threads {
        for id in thread {
            assert!(ids.contains(id), "a thread names object {id}, which is gone");
        }
    }
    let artboards: std::collections::HashSet<u32> = d.artboards.iter().map(|a| a.id).collect();
    for g in &d.guides {
        if let Some(layer) = g.layer {
            assert!(ids.contains(&layer), "a guide is on layer {layer}, which is gone");
        }
        if let Some(a) = g.artboard {
            assert!(artboards.contains(&a), "a guide is on artboard {a}, which is gone");
        }
    }
    for s in &d.saved_selections {
        for id in &s.objects {
            assert!(ids.contains(id), "saved selection {} names object {id}, which is gone", s.name);
        }
    }
    for a in &d.assets {
        for id in &a.nodes {
            assert!(ids.contains(id), "asset {} names object {id}, which is gone", a.name);
        }
    }
    for id in d.variables.bindings.keys() {
        assert!(ids.contains(id), "a variable binding names object {id}, which is gone");
    }
}
