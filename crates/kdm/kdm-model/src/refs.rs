//! References stored as data, and how they are written and read back.
//!
//! - **By name**, to a swatch (`"swatch"`: solid paints, gradient stops, tints, gradient paints),
//!   a symbol (`"symbol"`: symbol instances, inline graphics in text) or a pattern (`"pattern"` of
//!   a `{"type": "pattern"}` paint): written as `{"ref": <object id>, "name": <name>}` when the
//!   name names one of the drawing's, so they follow a rename, and read back as the object's
//!   current name (or the name they had, if the object is gone). A name that names nothing stays
//!   the plain string it was. Text marks hold them the same way ([`crate::text`]).
//! - **By local id**, in the document's settings ([`map_setting`]): saved selections and variable
//!   bindings; a node's graphic style and an asset's objects (in their own registers); and a
//!   guide's layer and artboard and a thread's frames, which are `ref` fields and list entries of
//!   their own objects. Written as object ids; a reference to nothing is not written.

use std::collections::HashMap;

use kition_kit::kdm::ObjectId;
use kition_vector_doc::{Document, NodeId};
use serde_json::{Value, json};

use crate::Ids;

/// JSON nesting the walkers follow (art inside symbols, masks and patterns nests deep).
const MAX_JSON_DEPTH: usize = 4096;

/// Names (and graphic style ids) to objects, for writing.
#[derive(Default)]
pub(crate) struct Names {
    pub swatches: HashMap<String, ObjectId>,
    pub patterns: HashMap<String, ObjectId>,
    pub symbols: HashMap<String, ObjectId>,
    pub styles: HashMap<u32, ObjectId>,
}

impl Names {
    /// `doc`'s named things and the objects `ids` holds for them (first holder of a name wins).
    pub fn of(doc: &Document, ids: &Ids) -> Self {
        let mut names = Self::default();
        for (s, o) in doc.swatches.iter().zip(&ids.swatches) {
            names.swatches.entry(s.name.clone()).or_insert_with(|| o.clone());
        }
        for (group, (_, objects)) in doc.swatch_groups.iter().zip(&ids.swatch_groups) {
            for (s, o) in group.swatches.iter().zip(objects) {
                names.swatches.entry(s.name.clone()).or_insert_with(|| o.clone());
            }
        }
        for (p, o) in doc.patterns.iter().zip(&ids.patterns) {
            names.patterns.entry(p.name.clone()).or_insert_with(|| o.clone());
        }
        for (s, o) in doc.symbols.iter().zip(&ids.symbols) {
            names.symbols.entry(s.name.clone()).or_insert_with(|| o.clone());
        }
        for (g, o) in doc.graphic_styles.iter().zip(&ids.graphic_styles) {
            if g.id != 0 {
                names.styles.entry(g.id).or_insert_with(|| o.clone());
            }
        }
        names
    }
}

/// Objects to local ids and names, for reading.
#[derive(Default)]
pub(crate) struct Locals {
    pub nodes: HashMap<ObjectId, NodeId>,
    pub artboards: HashMap<ObjectId, u32>,
    pub styles: HashMap<ObjectId, u32>,
    pub swatches: HashMap<ObjectId, String>,
    pub patterns: HashMap<ObjectId, String>,
    pub symbols: HashMap<ObjectId, String>,
}

fn reference(object: &ObjectId, name: &str) -> Value {
    json!({ "ref": object.as_str(), "name": name })
}

/// Rewrites the name references inside `v` to `{ref, name}` (see the module docs).
pub(crate) fn names_out(v: &mut Value, names: &Names) {
    out_rec(v, names, 0);
}

fn out_rec(v: &mut Value, names: &Names, depth: usize) {
    if depth > MAX_JSON_DEPTH {
        return;
    }
    match v {
        Value::Object(map) => {
            for (key, table) in [("swatch", &names.swatches), ("symbol", &names.symbols)] {
                if let Some(Value::String(name)) = map.get(key)
                    && let Some(object) = table.get(name.as_str())
                {
                    let r = reference(object, name);
                    map.insert(key.to_owned(), r);
                }
            }
            if map.get("type").and_then(Value::as_str) == Some("pattern")
                && let Some(Value::String(name)) = map.get("pattern")
                && let Some(object) = names.patterns.get(name.as_str())
            {
                let r = reference(object, name);
                map.insert("pattern".into(), r);
            }
            for item in map.values_mut() {
                out_rec(item, names, depth + 1);
            }
        }
        Value::Array(items) => items.iter_mut().for_each(|item| out_rec(item, names, depth + 1)),
        _ => {}
    }
}

/// Reverses [`names_out`]: every `{ref, name}` under a reference key becomes the object's
/// current name, or `name` if it is gone.
pub(crate) fn names_in(v: &mut Value, locals: &Locals) {
    in_rec(v, locals, 0);
}

fn in_rec(v: &mut Value, locals: &Locals, depth: usize) {
    if depth > MAX_JSON_DEPTH {
        return;
    }
    match v {
        Value::Object(map) => {
            for (key, table) in [("swatch", &locals.swatches), ("symbol", &locals.symbols), ("pattern", &locals.patterns)] {
                if let Some(name) = map.get(key).and_then(|r| resolve(r, table)) {
                    map.insert(key.to_owned(), Value::String(name));
                }
            }
            for item in map.values_mut() {
                in_rec(item, locals, depth + 1);
            }
        }
        Value::Array(items) => items.iter_mut().for_each(|item| in_rec(item, locals, depth + 1)),
        _ => {}
    }
}

/// The name a `{ref, name}` stands for now.
fn resolve(r: &Value, table: &HashMap<ObjectId, String>) -> Option<String> {
    let object = r.get("ref")?.as_str()?;
    let name = r.get("name")?.as_str()?;
    Some(table.get(&ObjectId::from_string(object)).cloned().unwrap_or_else(|| name.to_owned()))
}

/// How references to local ids are rewritten: each answers the stored value's replacement, or
/// `None` to drop the reference (it names nothing).
pub(crate) struct Mapper<'a> {
    pub node: &'a dyn Fn(&Value) -> Option<Value>,
    pub artboard: &'a dyn Fn(&Value) -> Option<Value>,
}

/// The settings keys [`map_setting`] rewrites.
pub(crate) const SETTINGS_WITH_IDS: &[&str] = &["saved_selections", "variables"];

/// The serialized document's keys [`map_setting`] rewrites when comparing views
/// ([`crate::normalize`]): the settings', and the guides and threads, which have objects of their
/// own in a replica.
pub(crate) const KEYS_WITH_IDS: &[&str] = &["text_threads", "guides", "saved_selections", "variables"];

/// Rewrites the local ids inside the serialized document's `key` with `m`. A guide on a layer or
/// artboard that is gone goes (as it would with the layer or artboard).
pub(crate) fn map_setting(key: &str, v: &mut Value, m: &Mapper<'_>) {
    match key {
        "text_threads" => {
            if let Value::Array(threads) = v {
                for thread in threads.iter_mut() {
                    if let Value::Array(frames) = thread {
                        *frames = frames.iter().filter_map(|f| (m.node)(f)).collect();
                    }
                }
                threads.retain(|t| t.as_array().is_some_and(|f| !f.is_empty()));
            }
        }
        "guides" => {
            if let Value::Array(guides) = v {
                guides.retain_mut(|g| {
                    let Value::Object(guide) = g else { return true };
                    for (field, f) in [("layer", m.node), ("artboard", m.artboard)] {
                        match guide.get(field) {
                            None | Some(Value::Null) => {}
                            Some(id) => match f(id) {
                                Some(mapped) => {
                                    guide.insert(field.into(), mapped);
                                }
                                None => return false,
                            },
                        }
                    }
                    true
                });
            }
        }
        "saved_selections" => {
            if let Value::Array(saved) = v {
                for s in saved {
                    if let Some(Value::Array(objects)) = s.get_mut("objects") {
                        *objects = objects.iter().filter_map(|o| (m.node)(o)).collect();
                    }
                }
            }
        }
        "variables" => {
            if let Some(Value::Object(bindings)) = v.get_mut("bindings") {
                let old = std::mem::take(bindings);
                for (k, names) in old {
                    let id = k.parse::<u64>().map_or_else(|_| Value::String(k.clone()), Value::from);
                    let mapped = match (m.node)(&id) {
                        Some(Value::String(s)) => s,
                        Some(Value::Number(n)) => n.to_string(),
                        _ => continue,
                    };
                    bindings.insert(mapped, names);
                }
            }
        }
        _ => {}
    }
}

/// An asset's `nodes` rewritten with `node` (dropping what names nothing).
pub(crate) fn map_asset_nodes(v: &mut Value, node: &dyn Fn(&Value) -> Option<Value>) {
    if let Some(Value::Array(nodes)) = v.get_mut("nodes") {
        *nodes = nodes.iter().filter_map(node).collect();
    }
}
