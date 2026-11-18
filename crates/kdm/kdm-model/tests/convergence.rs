//! Seeded random multi-session convergence: ~200 random model-level edits (adds, deletes, moves,
//! recolours, reparents, reorders, groups, layers, swatches and their links, symbols and their
//! instances, guides, settings, images) on two or three sessions with random exchanges and
//! undos; after a final exchange every replica has the same state hash and the same (id-blind)
//! view, no view holds a dangling reference, and each view is what a fresh read gives.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use std::collections::HashSet;
use std::sync::Arc;

use common::{Session, SharedBlobs, assert_no_dangling, assert_replica_clean, canonical, drawing, edit, first, peer, sync_all};
use kition_vector_doc::color::{Color, Paint, Swatch};
use kition_vector_doc::geom::{Affine, Rect, shapes};
use kition_vector_doc::{Appearance, Document, Guide, ImageBlob, ImageObject, Node, NodeId, NodeKind, Symbol};

/// xorshift64*: deterministic, no dependency.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn below(&mut self, n: usize) -> usize {
        if n == 0 { 0 } else { (self.next() % n as u64) as usize }
    }
    fn chance(&mut self, percent: u64) -> bool {
        self.next() % 100 < percent
    }
    fn color(&mut self) -> Color {
        Color::rgb(self.below(256) as f32 / 255.0, self.below(256) as f32 / 255.0, self.below(256) as f32 / 255.0)
    }
}

fn path(d: &mut Document, rng: &mut Rng) -> Node {
    let id = d.alloc_id();
    let (x, y) = (rng.below(500) as f64, rng.below(300) as f64);
    let mut n =
        Node::path(id, shapes::rectangle(Rect::new(x, y, x + 20.0, y + 20.0)), Appearance::basic(Paint::solid(rng.color()), Paint::None, 0.0));
    n.name = Some(format!("Path {id}"));
    n
}

/// Every node id with its kind's ability to hold children.
fn all_nodes(d: &Document) -> Vec<(NodeId, bool)> {
    let mut out = Vec::new();
    d.walk(|n| out.push((n.id, matches!(n.kind, NodeKind::Layer { .. } | NodeKind::Group { .. }))));
    out
}

fn non_layers(d: &Document) -> Vec<NodeId> {
    let layers: HashSet<NodeId> = d.layers.iter().map(|l| l.id).collect();
    all_nodes(d).into_iter().map(|(id, _)| id).filter(|id| !layers.contains(id)).collect()
}

/// Drops the references to what `d` no longer holds, as the engine's own commands and edit
/// post-passes do.
fn tidy(d: &mut Document) {
    let live: HashSet<NodeId> = all_nodes(d).into_iter().map(|(id, _)| id).collect();
    let symbols: HashSet<String> = d.symbols.iter().map(|s| s.name.clone()).collect();
    let mut instances = Vec::new();
    d.walk(|n| {
        if let NodeKind::SymbolInstance { symbol, .. } = &n.kind
            && !symbols.contains(symbol)
        {
            instances.push(n.id);
        }
    });
    for id in instances {
        let _ = d.remove(id);
    }
    let live: HashSet<NodeId> = live.into_iter().filter(|id| d.node(*id).is_some()).collect();
    for thread in &mut d.text_threads {
        thread.retain(|id| live.contains(id));
    }
    d.text_threads.retain(|t| !t.is_empty());
    for s in &mut d.saved_selections {
        s.objects.retain(|id| live.contains(id));
    }
    for a in &mut d.assets {
        a.nodes.retain(|id| live.contains(id));
    }
    d.assets.retain(|a| !a.nodes.is_empty());
    d.variables.bindings.retain(|id, _| live.contains(id));
    let artboards: HashSet<u32> = d.artboards.iter().map(|a| a.id).collect();
    d.guides.retain(|g| g.layer.is_none_or(|l| live.contains(&l)) && g.artboard.is_none_or(|a| artboards.contains(&a)));
}

fn random_edit(d: &mut Document, rng: &mut Rng) {
    let nodes = all_nodes(d);
    let containers: Vec<NodeId> = nodes.iter().filter(|(_, c)| *c).map(|(id, _)| *id).collect();
    let objects = non_layers(d);
    match rng.below(20) {
        0..=2 => {
            // A path somewhere.
            let parent = containers[rng.below(containers.len())];
            let n = path(d, rng);
            let len = d.children(Some(parent)).map_or(0, Vec::len);
            let at = rng.below(len + 1);
            let _ = d.insert(Some(parent), at, n);
        }
        3 if !objects.is_empty() => {
            let _ = d.remove(objects[rng.below(objects.len())]);
        }
        4 | 5 if !objects.is_empty() => {
            let id = objects[rng.below(objects.len())];
            let dx = rng.below(40) as f64 - 20.0;
            if let Some(NodeKind::Path { path, .. }) = d.node_mut(id).map(|n| &mut n.kind) {
                path.transform(Affine::translate((dx, 0.0)));
            }
        }
        6 | 7 if !objects.is_empty() => {
            let id = objects[rng.below(objects.len())];
            let c = rng.color();
            if let Some(n) = d.node_mut(id) {
                n.appearance.set_fill(Paint::solid(c));
            }
        }
        8 if !objects.is_empty() => {
            // Reparent (never into itself).
            let id = objects[rng.below(objects.len())];
            let target = containers[rng.below(containers.len())];
            let len = d.children(Some(target)).map_or(0, Vec::len);
            let _ = d.move_node(id, Some(target), rng.below(len + 1));
        }
        9 if !objects.is_empty() => {
            // Reorder within its parent.
            let id = objects[rng.below(objects.len())];
            if let Some((parent, _, count)) = d.position(id) {
                let _ = d.move_node(id, parent, rng.below(count));
            }
        }
        10 => {
            // Group two siblings.
            let parent = containers[rng.below(containers.len())];
            let gid = d.alloc_id();
            if let Ok(children) = d.children_mut(Some(parent))
                && children.len() >= 2
            {
                let at = rng.below(children.len() - 1);
                let pair: Vec<Arc<Node>> = children.drain(at..at + 2).collect();
                let mut g = Node::group(gid, pair);
                g.name = Some(format!("Group {gid}"));
                children.insert(at, Arc::new(g));
            }
        }
        11 => {
            // Ungroup the first group found.
            let group = nodes.iter().find(|(id, c)| *c && !d.layers.iter().any(|l| l.id == *id)).map(|(id, _)| *id);
            if let Some(g) = group
                && let Some((parent, at, _)) = d.position(g)
                && let Ok(node) = d.remove(g)
                && let Ok(children) = d.children_mut(parent)
            {
                let inner = node.children().cloned().unwrap_or_default();
                for (k, c) in inner.into_iter().enumerate() {
                    children.insert((at + k).min(children.len()), c);
                }
            }
        }
        12 => {
            let _ = d.add_layer(None);
        }
        13 if d.layers.len() > 1 => {
            let id = d.layers[rng.below(d.layers.len())].id;
            let _ = d.remove(id);
        }
        14 => {
            let name = format!("Swatch {}", rng.below(1000));
            d.swatches.push(Swatch { uid: Default::default(), name, paint: Paint::solid(rng.color()), global: true, spot: false });
        }
        15 if !d.swatches.is_empty() && !objects.is_empty() => {
            // Link an object's fill to a swatch.
            let s = d.swatches[rng.below(d.swatches.len())].clone();
            let id = objects[rng.below(objects.len())];
            if let (Paint::Solid { color, .. }, Some(n)) = (s.paint.clone(), d.node_mut(id)) {
                n.appearance.set_fill(Paint::Solid { color, swatch: Some(s.name.clone()), tint: 1.0 });
            }
        }
        16 => {
            // A symbol and an instance of it, or delete a symbol and its instances.
            if d.symbols.is_empty() || rng.chance(60) {
                let name = format!("Symbol {}", rng.below(1000));
                let art = path(d, rng);
                d.symbols.push(Symbol { uid: Default::default(), name: name.clone(), art: Arc::new(art) });
                let parent = d.layers[rng.below(d.layers.len())].id;
                let id = d.alloc_id();
                let mut n = Node::new(id, NodeKind::SymbolInstance { symbol: name, xf: Affine::IDENTITY });
                n.name = Some(format!("Instance {id}"));
                let _ = d.insert(Some(parent), usize::MAX, n);
            } else {
                let i = rng.below(d.symbols.len());
                d.symbols.remove(i);
            }
        }
        17 => {
            let layer = d.layers[rng.below(d.layers.len())].id;
            d.guides.push(Guide { vertical: rng.chance(50), pos: rng.below(500) as f64, artboard: None, layer: Some(layer) });
        }
        18 => {
            d.grid.spacing = 10.0 + rng.below(90) as f64;
        }
        _ => {
            // An image (new bytes) and an image object showing it.
            let bytes = vec![0x89, b'P', b'N', b'G', rng.below(256) as u8, rng.below(256) as u8, rng.below(256) as u8];
            let key = format!("img-{}", rng.below(1_000_000));
            d.images.insert(key.clone(), ImageBlob::new("image/png", bytes));
            let parent = d.layers[rng.below(d.layers.len())].id;
            let id = d.alloc_id();
            let mut n = Node::new(
                id,
                NodeKind::Image(ImageObject { key, width: 2, height: 2, xf: Affine::IDENTITY, link: None, placement: Default::default() }),
            );
            n.name = Some(format!("Image {id}"));
            let _ = d.insert(Some(parent), usize::MAX, n);
        }
    }
    if d.layers.is_empty() {
        d.add_layer(None);
    }
    tidy(d);
}

fn run(seed: u64, sessions: usize, steps: usize) {
    let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
    let blobs = SharedBlobs::default();
    let a = first(&drawing(), &blobs);
    let mut all: Vec<Session> = vec![a];
    for k in 1..sessions {
        let next = peer(&all[0], 10 + k as u64, &blobs);
        all.push(next);
    }
    for _ in 0..steps {
        let who = rng.below(all.len());
        let sub = rng.next();
        edit(&mut all[who], &blobs, |d| random_edit(d, &mut Rng(sub | 1)));
        if rng.chance(25) {
            let (from, to) = (rng.below(all.len()), rng.below(all.len()));
            if from != to {
                let bytes = all[from].export_changes(&all[to].version()).unwrap_or_else(|error| panic!("export: {error}"));
                all[to].import_changes(&bytes).unwrap_or_else(|error| panic!("seed {seed}: import: {error}"));
            }
        }
        if rng.chance(3) {
            let who = rng.below(all.len());
            all[who].undo().unwrap_or_else(|error| panic!("seed {seed}: undo: {error}"));
        }
    }
    sync_all(&mut all);
    let hash = all[0].state_hash().unwrap_or_else(|error| panic!("hash: {error}"));
    let view = canonical(all[0].model().document());
    for (k, session) in all.iter().enumerate() {
        assert_eq!(session.state_hash().unwrap_or_else(|error| panic!("hash: {error}")), hash, "seed {seed}: session {k} state hash");
        let mine = canonical(session.model().document());
        assert!(mine == view, "seed {seed}: session {k} view:\n{:#?}", common::differences(&mine, &view));
        assert_no_dangling(session.model().document());
        assert_replica_clean(session.replica());
        // The view is what a fresh read of the replica gives.
        let (fresh, _) = kdm_model::read_document(session.replica(), &blobs).unwrap_or_else(|error| panic!("read: {error}"));
        let fresh = canonical(&fresh);
        assert!(fresh == view, "seed {seed}: session {k} view matches its replica:\n{:#?}", common::differences(&fresh, &view));
    }
}

#[test]
fn two_sessions_converge() {
    for seed in [1, 2, 3] {
        run(seed, 2, 200);
    }
}

#[test]
fn three_sessions_converge() {
    for seed in [11, 12] {
        run(seed, 3, 200);
    }
}
