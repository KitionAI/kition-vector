//! Local id bookkeeping. [`Ids`] maps the engine ids of what a view holds to their objects;
//! [`Known`] remembers every object a view ever gave a local id (deleted ones included), so a
//! reload after an undo, redo or merge gives a surviving or restored object the id it had.
//!
//! Node, slice and asset ids share the document's id counter (`Document::alloc_id`); artboard and
//! graphic style ids have their own. Swatches, swatch groups, graphic styles, text styles,
//! symbols, patterns, guides and text threads have no id of their own in the model: their objects
//! are kept in the view's order, and paired between two versions of a list by content
//! ([`crate::list::align`]; a thread by its frames). A symbol's art has node ids of its own, which
//! are the art's (data: they get fresh ones when the art is used), not the drawing's: they are
//! not written, and a read gives each art node the id the view gave its object, else a fresh one
//! unique in that symbol.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;

use kition_kit::kdm::{NodeRef, ObjectId};
use kition_kit::session::IdMap;
use kition_vector_doc::{Document, Node, NodeId};

use crate::{KdmResult, MAX_DEPTH, kdm_err};

/// The objects of one tree of nodes (the drawing's layers, or a symbol's art) by node id, and
/// each one's tree node, so a later write can move it or put children under it (a transaction
/// cannot read the replica's tree back).
#[derive(Clone, Debug, Default)]
pub(crate) struct TreeIds {
    pub nodes: IdMap<NodeId>,
    pub tree: HashMap<ObjectId, NodeRef>,
}

/// The objects behind a view's nodes, artboards, slices, assets, images and named lists.
#[derive(Clone, Debug, Default)]
pub struct Ids {
    pub nodes: IdMap<NodeId>,
    /// Each node object's tree node, so a later write can move it or put children under it (a
    /// transaction cannot read the replica's tree back).
    pub(crate) tree: HashMap<ObjectId, NodeRef>,
    pub artboards: IdMap<u32>,
    pub slices: IdMap<NodeId>,
    pub assets: IdMap<u64>,
    /// By image key.
    pub images: BTreeMap<String, ObjectId>,
    /// Parallel to the document's lists of the same names.
    pub(crate) swatches: Vec<ObjectId>,
    pub(crate) swatch_groups: Vec<(ObjectId, Vec<ObjectId>)>,
    pub(crate) graphic_styles: Vec<ObjectId>,
    pub(crate) char_styles: Vec<ObjectId>,
    pub(crate) para_styles: Vec<ObjectId>,
    pub(crate) symbols: Vec<ObjectId>,
    /// Each symbol's art, parallel to `symbols`.
    pub(crate) symbol_trees: Vec<TreeIds>,
    pub(crate) patterns: Vec<ObjectId>,
    pub(crate) guides: Vec<ObjectId>,
    /// Parallel to the document's `text_threads`.
    pub(crate) threads: Vec<ObjectId>,
}

impl Ids {
    /// The object of node `id`.
    pub fn node(&self, id: NodeId) -> Option<&ObjectId> {
        self.nodes.object(id)
    }

    /// The object of the swatch at `index` of the document's swatch list.
    pub fn swatch(&self, index: usize) -> Option<&ObjectId> {
        self.swatches.get(index)
    }

    /// The object of the symbol at `index` of the document's symbol list.
    pub fn symbol(&self, index: usize) -> Option<&ObjectId> {
        self.symbols.get(index)
    }

    /// The object of the graphic style at `index` of the document's graphic style list.
    pub fn graphic_style(&self, index: usize) -> Option<&ObjectId> {
        self.graphic_styles.get(index)
    }

    /// The object of the pattern at `index` of the document's pattern list.
    pub fn pattern(&self, index: usize) -> Option<&ObjectId> {
        self.patterns.get(index)
    }
}

/// Object ids to the local ids a view has ever given them. See [`Known::remember`].
#[derive(Clone, Debug, Default)]
pub struct Known {
    pub(crate) nodes: HashMap<ObjectId, NodeId>,
    pub(crate) slices: HashMap<ObjectId, NodeId>,
    pub(crate) assets: HashMap<ObjectId, u64>,
    pub(crate) artboards: HashMap<ObjectId, u32>,
    pub(crate) styles: HashMap<ObjectId, u32>,
    /// Symbols' art nodes.
    pub(crate) symbol_nodes: HashMap<ObjectId, NodeId>,
    /// The highest node, slice or asset id ever handed out (or below the counter seen), so fresh
    /// ids never reuse one.
    pub(crate) max: u64,
    pub(crate) max_artboard: u32,
    pub(crate) max_style: u32,
}

impl Known {
    /// Remembers the object each of `doc`'s nodes, slices, assets, artboards and graphic styles
    /// maps to.
    pub fn remember(&mut self, doc: &Document, ids: &Ids) {
        self.max = self.max.max(doc.peek_next_id().saturating_sub(1));
        walk(&doc.layers, 0, &mut |n| {
            self.max = self.max.max(n.id.0);
            if let Some(object) = ids.nodes.object(n.id) {
                self.nodes.insert(object.clone(), n.id);
            }
        });
        for slice in &doc.slices {
            self.max = self.max.max(slice.id.0);
            if let Some(object) = ids.slices.object(slice.id) {
                self.slices.insert(object.clone(), slice.id);
            }
        }
        for asset in &doc.assets {
            self.max = self.max.max(asset.id);
            if let Some(object) = ids.assets.object(asset.id) {
                self.assets.insert(object.clone(), asset.id);
            }
        }
        for artboard in &doc.artboards {
            self.max_artboard = self.max_artboard.max(artboard.id);
            if let Some(object) = ids.artboards.object(artboard.id) {
                self.artboards.insert(object.clone(), artboard.id);
            }
        }
        for (style, object) in doc.graphic_styles.iter().zip(&ids.graphic_styles) {
            self.max_style = self.max_style.max(style.id);
            if style.id != 0 {
                self.styles.insert(object.clone(), style.id);
            }
        }
        for (symbol, tree) in doc.symbols.iter().zip(&ids.symbol_trees) {
            walk(std::slice::from_ref(&symbol.art), 0, &mut |n| {
                if let Some(object) = tree.nodes.object(n.id) {
                    self.symbol_nodes.insert(object.clone(), n.id);
                }
            });
        }
    }
}

/// Visits every node of `nodes` and their descendants (as deep as the bridge goes).
pub(crate) fn walk<'a>(nodes: &'a [Arc<Node>], depth: usize, f: &mut dyn FnMut(&'a Arc<Node>)) {
    if depth > MAX_DEPTH {
        return;
    }
    for n in nodes {
        f(n);
        if let Some(children) = n.children() {
            walk(children, depth + 1, f);
        }
    }
}

/// Every node of `nodes`, by id (first holder wins).
pub(crate) fn index(nodes: &[Arc<Node>]) -> HashMap<NodeId, &Arc<Node>> {
    let mut out = HashMap::new();
    walk(nodes, 0, &mut |n| {
        out.entry(n.id).or_insert(n);
    });
    out
}

fn depth_of(nodes: &[Arc<Node>], depth: usize) -> usize {
    let mut deepest = depth;
    for n in nodes {
        if let Some(children) = n.children() {
            if depth >= MAX_DEPTH {
                return depth + 1;
            }
            deepest = deepest.max(depth_of(children, depth + 1));
        }
    }
    deepest
}

/// Errs if `doc` holds an id twice where a replica needs it unique (see [`crate::decode`]), or
/// nests deeper than the bridge goes: writing it would merge two objects into one, or lose what
/// is too deep.
pub(crate) fn check_unique(doc: &Document) -> KdmResult<()> {
    if depth_of(&doc.layers, 0) > MAX_DEPTH {
        return Err(kdm_err(format!("the drawing nests groups deeper than {MAX_DEPTH}; refusing to write it")));
    }
    let mut seen = HashSet::new();
    let mut duplicate = None;
    walk(&doc.layers, 0, &mut |n| {
        if !seen.insert(n.id) && duplicate.is_none() {
            duplicate = Some(n.id);
        }
    });
    if let Some(id) = duplicate {
        return Err(kdm_err(format!("the drawing holds object {id} more than once; refusing to write it")));
    }
    fn unique<T: std::hash::Hash + Eq>(what: &str, ids: impl Iterator<Item = T>) -> KdmResult<()> {
        let mut seen = HashSet::new();
        for id in ids {
            if !seen.insert(id) {
                return Err(kdm_err(format!("the drawing holds {what} more than once; refusing to write it")));
            }
        }
        Ok(())
    }
    for symbol in &doc.symbols {
        let art = std::slice::from_ref(&symbol.art);
        if depth_of(art, 0) > MAX_DEPTH {
            return Err(kdm_err(format!("symbol {} nests groups deeper than {MAX_DEPTH}; refusing to write it", symbol.name)));
        }
        let mut seen = HashSet::new();
        let mut twice = false;
        walk(art, 0, &mut |n| twice |= !seen.insert(n.id));
        if twice {
            return Err(kdm_err(format!("symbol {} holds an object id twice; refusing to write it", symbol.name)));
        }
    }
    unique("a slice id", doc.slices.iter().map(|s| s.id))?;
    unique("an asset id", doc.assets.iter().map(|a| a.id))?;
    unique("an artboard id", doc.artboards.iter().map(|a| a.id))?;
    unique("a graphic style id", doc.graphic_styles.iter().map(|g| g.id).filter(|id| *id != 0))?;
    Ok(())
}

/// Gives every duplicate id (after its first use) a fresh one, so [`check_unique`] passes.
/// References to a duplicated id keep naming its first holder.
pub(crate) fn dedupe(doc: &mut Document) {
    doc.fix_next_id();
    fn renumber(nodes: &mut [Arc<Node>], seen: &mut HashSet<NodeId>, doc: &mut Document, depth: usize) {
        if depth > MAX_DEPTH {
            return;
        }
        for n in nodes {
            let duplicate = !seen.insert(n.id);
            let has_children = n.children().is_some_and(|c| !c.is_empty());
            if !duplicate && !has_children {
                continue;
            }
            let node = Arc::make_mut(n);
            if duplicate {
                node.id = doc.alloc_id();
                seen.insert(node.id);
            }
            if let Some(children) = node.children_mut() {
                renumber(children, seen, doc, depth + 1);
            }
        }
    }
    let mut seen = HashSet::new();
    let mut any = false;
    walk(&doc.layers, 0, &mut |n| any |= !seen.insert(n.id));
    if any {
        let mut layers = std::mem::take(&mut doc.layers);
        renumber(&mut layers, &mut HashSet::new(), doc, 0);
        doc.layers = layers;
    }
    // A symbol's art ids are its own (see the module docs): unique within it.
    let mut symbols = std::mem::take(&mut doc.symbols);
    for symbol in &mut symbols {
        let mut seen = HashSet::new();
        let mut twice = false;
        walk(std::slice::from_ref(&symbol.art), 0, &mut |n| twice |= !seen.insert(n.id));
        if twice {
            let mut one = vec![symbol.art.clone()];
            renumber(&mut one, &mut HashSet::new(), doc, 0);
            if let Some(art) = one.pop() {
                symbol.art = art;
            }
        }
    }
    doc.symbols = symbols;
    let mut seen = HashSet::new();
    for i in 0..doc.slices.len() {
        if doc.slices.get(i).is_some_and(|s| !seen.insert(s.id)) {
            let id = doc.alloc_id();
            if let Some(slice) = doc.slices.get_mut(i) {
                slice.id = id;
            }
        }
    }
    let mut seen = HashSet::new();
    for i in 0..doc.assets.len() {
        if doc.assets.get(i).is_some_and(|a| !seen.insert(a.id)) {
            let id = doc.alloc_id().0;
            if let Some(asset) = doc.assets.get_mut(i) {
                asset.id = id;
            }
        }
    }
    let mut next = doc.artboards.iter().map(|a| a.id).max().unwrap_or(0);
    let mut seen = HashSet::new();
    for artboard in &mut doc.artboards {
        if !seen.insert(artboard.id) {
            next = next.saturating_add(1);
            artboard.id = next;
            seen.insert(next);
        }
    }
    let mut next = doc.graphic_styles.iter().map(|g| g.id).max().unwrap_or(0);
    let mut seen = HashSet::new();
    for style in &mut doc.graphic_styles {
        if style.id != 0 && !seen.insert(style.id) {
            next = next.saturating_add(1);
            style.id = next;
            seen.insert(next);
        }
    }
}
