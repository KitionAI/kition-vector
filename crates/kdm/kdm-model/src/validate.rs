//! Deterministic repair after a merge, an undo or a redo ([`kition_kit::session::KitModel::validate`]):
//! state only, so every replica writes the same repair for the same merged state.
//!
//! - A symbol instance of a deleted symbol (one peer deleted the symbol while another placed an
//!   instance) is deleted, as `symbol.delete` without expanding deletes instances.
//! - A node's graphic style link to a deleted style is dropped (the node keeps its look).
//! - Saved selections, variable bindings and export assets let go of deleted objects (an asset
//!   left with none goes); a guide on a deleted layer or artboard goes; a thread lets go of
//!   deleted frames, and goes with the last (its story was theirs).
//! - List entries naming deleted objects are removed. An object listed twice is left alone: the
//!   reader reads it once, at its first place.
//!
//! - A named entry (swatch, graphic style, character or paragraph style, symbol, pattern) holding
//!   the persistent identity (`key`) an earlier entry of its table holds (two peers made one at
//!   the same place with the same name) gets another, as `Document::settle_uids` gives it
//!   ([`settle_keys`]). One without (a model batch created it) gets one in the batch itself.
//!
//! A swatch or pattern reference to a deleted object needs no repair: it reads back as the name it
//! had, which names nothing (a paint with no swatch link keeps its colour).

use std::collections::BTreeSet;

use kition_kit::kdm::{ObjectId, Tx};
use serde_json::{Map, Value};

use crate::refs::{Mapper, SETTINGS_WITH_IDS, map_asset_nodes, map_setting};
use crate::{KdmResult, kdm_err, root_id};

/// Whether `value` is an object id (a string) naming an object that no longer exists.
fn gone(tx: &Tx<'_>, value: Option<&Value>) -> bool {
    matches!(value, Some(Value::String(id)) if !tx.exists(&ObjectId::from_string(id.as_str())))
}

pub(crate) fn validate(tx: &mut Tx<'_>, _changed: &BTreeSet<ObjectId>) -> KdmResult<bool> {
    let root = root_id();
    if !tx.exists(&root) {
        return Ok(false);
    }
    let mut wrote = false;

    // Nodes: instances of deleted symbols, links to deleted graphic styles.
    for node in tx.objects_of_type("Node") {
        let data = tx.get(&node, "data")?;
        let instance = data.get("type").and_then(Value::as_str) == Some("symbolinstance");
        if instance && gone(tx, data.get("symbol").and_then(|s| s.get("ref"))) {
            tx.delete(&node)?;
            wrote = true;
            continue;
        }
        let mut props = tx.get(&node, "props")?;
        if gone(tx, props.get("graphic_style"))
            && let Some(map) = props.as_object_mut()
        {
            map.remove("graphic_style");
            tx.set(&node, "props", props)?;
            wrote = true;
        }
    }

    // Guides on what is gone.
    for guide in tx.objects_of_type("Guide") {
        if gone(tx, Some(&tx.get(&guide, "layer")?)) || gone(tx, Some(&tx.get(&guide, "artboard")?)) {
            tx.delete(&guide)?;
            wrote = true;
        }
    }

    // Threads.
    for thread in tx.objects_of_type("Thread") {
        wrote |= clean_dangling(tx, &thread, "frames")?;
        if tx.list_items(&thread, "frames")?.is_empty() {
            tx.delete(&thread)?;
            wrote = true;
        }
    }

    // Export assets.
    for asset in tx.objects_of_type("Asset") {
        let props = tx.get(&asset, "props")?;
        let mut fixed = props.clone();
        {
            let live = |v: &Value| (!gone(tx, Some(v))).then(|| v.clone());
            map_asset_nodes(&mut fixed, &live);
        }
        if fixed != props {
            if fixed.get("nodes").and_then(Value::as_array).is_some_and(Vec::is_empty) {
                tx.delete(&asset)?;
            } else {
                tx.set(&asset, "props", fixed)?;
            }
            wrote = true;
        }
    }

    // Settings naming objects.
    let mut fixes = Vec::new();
    {
        let live = |v: &Value| (!gone(tx, Some(v))).then(|| v.clone());
        let m = Mapper { node: &live, artboard: &live };
        for key in SETTINGS_WITH_IDS {
            let v = tx.map_get(&root, "settings", key)?;
            if v.is_null() {
                continue;
            }
            let mut fixed = v.clone();
            map_setting(key, &mut fixed, &m);
            if fixed != v {
                fixes.push((*key, fixed));
            }
        }
    }
    for (key, fixed) in fixes {
        if fixed.is_null() {
            tx.map_delete(&root, "settings", key)?;
        } else {
            tx.map_set(&root, "settings", key, fixed)?;
        }
        wrote = true;
    }

    wrote |= settle_keys(tx, false)?;

    // Dangling list entries.
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
        wrote |= clean_dangling(tx, &root, field)?;
    }
    for group in tx.objects_of_type("SwatchGroup") {
        wrote |= clean_dangling(tx, &group, "swatches")?;
    }
    Ok(wrote)
}

/// Removes list entries naming an object that no longer exists.
fn clean_dangling(tx: &mut Tx<'_>, owner: &ObjectId, field: &str) -> KdmResult<bool> {
    let items = tx.list_items(owner, field)?;
    let mut wrote = false;
    for (index, object) in items.iter().enumerate().rev() {
        if !tx.exists(object) {
            tx.list_remove(owner, field, index)?;
            wrote = true;
        }
    }
    Ok(wrote)
}

/// Gives every named entry with a `key` (its uid, the field of its tool id) an earlier entry of
/// its table holds, and with `missing` every one without, a new one: [`kition_vector_doc::derive_uid`] of its table, its
/// place in the table as a read lists it, and its name, as `Document::settle_uids` gives it to a
/// document read from this replica. Answers whether it wrote.
pub(crate) fn settle_keys(tx: &mut Tx<'_>, missing: bool) -> KdmResult<bool> {
    let root = root_id();
    if !tx.exists(&root) {
        return Ok(false);
    }
    let mut seen_objects = std::collections::HashSet::new();
    let mut live = |tx: &Tx<'_>, owner: &ObjectId, field: &str| -> KdmResult<Vec<ObjectId>> {
        Ok(tx.list_items(owner, field)?.into_iter().filter(|o| tx.exists(o) && seen_objects.insert(o.clone())).collect())
    };
    let mut swatches = live(tx, &root, "swatches")?;
    let groups = live(tx, &root, "swatchGroups")?;
    for group in &groups {
        swatches.extend(live(tx, group, "swatches")?);
    }
    let mut tables = vec![("swatch", swatches), ("swatchGroup", groups)];
    for (table, field) in
        [("graphicStyle", "graphicStyles"), ("charStyle", "charStyles"), ("paraStyle", "paraStyles"), ("symbol", "symbols"), ("pattern", "patterns")]
    {
        tables.push((table, live(tx, &root, field)?));
    }
    let mut wrote = false;
    for (table, objects) in tables {
        let mut keys = Vec::with_capacity(objects.len());
        for object in &objects {
            let key = tx.get(object, "key")?.as_str().unwrap_or_default().to_owned();
            keys.push(key);
        }
        let mut used: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut fresh = vec![false; keys.len()];
        for (i, key) in keys.iter().enumerate() {
            if key.is_empty() {
                fresh[i] = missing;
            } else if !used.insert(key.clone()) {
                fresh[i] = true;
            }
        }
        for (i, object) in objects.iter().enumerate() {
            if !fresh[i] {
                continue;
            }
            let name = tx.get(object, "name")?.as_str().unwrap_or_default().to_owned();
            let key = (0..).map(|salt| kition_vector_doc::derive_uid(table, i, &name, salt)).find(|k| !used.contains(k)).unwrap_or_default();
            used.insert(key.clone());
            tx.set(object, "key", Value::String(key))?;
            wrote = true;
        }
    }
    Ok(wrote)
}

/// Makes every name reference (`{ref, name}`, [`crate::refs`]) to a swatch, symbol or pattern
/// carry that object's current name. A read gives a reference its object's current name anyway,
/// so a saved file holds the new one; a model batch that renames the object does not touch the
/// references, and its receipt would tell less than a read of the saved file (C8-vector-5).
/// Answers whether it wrote.
pub(crate) fn follow_renames(tx: &mut Tx<'_>) -> KdmResult<bool> {
    let mut names = std::collections::HashMap::new();
    for ty in ["Swatch", "Symbol", "Pattern"] {
        for object in tx.objects_of_type(ty) {
            if let Some(name) = tx.get(&object, "name")?.as_str() {
                names.insert(object.as_str().to_owned(), name.to_owned());
            }
        }
    }
    if names.is_empty() {
        return Ok(false);
    }
    fn walk(v: &mut Value, names: &std::collections::HashMap<String, String>, depth: usize) -> bool {
        if depth > 4096 {
            return false;
        }
        match v {
            Value::Object(map) => {
                if map.len() == 2
                    && let (Some(Value::String(object)), Some(Value::String(name))) = (map.get("ref"), map.get("name"))
                    && let Some(now) = names.get(object)
                    && now != name
                {
                    map.insert("name".into(), Value::String(now.clone()));
                    return true;
                }
                let mut changed = false;
                for value in map.values_mut() {
                    changed |= walk(value, names, depth + 1);
                }
                changed
            }
            Value::Array(items) => {
                let mut changed = false;
                for item in items {
                    changed |= walk(item, names, depth + 1);
                }
                changed
            }
            _ => false,
        }
    }
    let mut wrote = false;
    for (ty, fields) in [
        ("Node", &["data", "appearance", "props"][..]),
        ("Swatch", &["props"][..]),
        ("GraphicStyle", &["props"][..]),
        ("CharStyle", &["props"][..]),
        ("ParaStyle", &["props"][..]),
        ("Pattern", &["props"][..]),
    ] {
        for object in tx.objects_of_type(ty) {
            for field in fields {
                let mut value = tx.get(&object, field)?;
                if walk(&mut value, &names, 0) {
                    tx.set(&object, field, value)?;
                    wrote = true;
                }
            }
        }
    }
    Ok(wrote)
}

/// Gives a named entry that is no complete entry (a model batch created it with a name alone:
/// a swatch has no paint, a symbol no art) the content a new one of its kind has: a swatch of
/// black, a graphic style of no appearance, a text style that sets nothing, a symbol an empty
/// group as its art, a pattern of no art. A read could not read it otherwise, and the drawing
/// would refuse it (C8-vector-5). Answers whether it wrote.
pub(crate) fn complete_entries(tx: &mut Tx<'_>) -> KdmResult<bool> {
    use kition_vector_doc::color::{Color, Paint, Swatch};
    use kition_vector_doc::pattern::PatternDef;
    use kition_vector_doc::text::TextStyleDef;
    use kition_vector_doc::{Appearance, GraphicStyle, Node, NodeId, NodeKind};
    fn props_of<T: serde::Serialize>(value: &T) -> Value {
        let mut v = serde_json::to_value(value).unwrap_or(Value::Null);
        if let Some(map) = v.as_object_mut() {
            map.remove("name");
            map.remove("uid");
            map.remove("id");
        }
        v
    }
    fn reads<T: serde::de::DeserializeOwned>(props: &Value) -> bool {
        let mut map = props.as_object().cloned().unwrap_or_default();
        map.insert("name".into(), Value::String(String::new()));
        serde_json::from_value::<T>(Value::Object(map)).is_ok()
    }
    let mut wrote = false;
    for symbol in tx.objects_of_type("Symbol") {
        if tx.tree(&symbol, "nodes")?.iter().any(|entry| tx.exists(&entry.object)) {
            continue;
        }
        let group = Node::new(NodeId(0), NodeKind::Group { children: Vec::new(), clip: false });
        let fields: Map<String, Value> = crate::write::node_fields(&group, &crate::refs::Names::default())?
            .into_iter()
            .filter(|(field, v)| *field != "id" && !v.is_null())
            .map(|(k, v)| (k.to_owned(), v))
            .collect();
        let art = tx.create("Node", fields)?;
        tx.tree_create(&symbol, "nodes", None, &art)?;
        wrote = true;
    }
    for ty in ["Swatch", "GraphicStyle", "CharStyle", "ParaStyle", "Pattern"] {
        for object in tx.objects_of_type(ty) {
            let field = "props";
            let value = tx.get(&object, field)?;
            let fine = match ty {
                "Swatch" => reads::<Swatch>(&value),
                "GraphicStyle" => reads::<GraphicStyle>(&value),
                "CharStyle" | "ParaStyle" => reads::<TextStyleDef>(&value),
                _ => reads::<PatternDef>(&value),
            };
            // A text style reads without its props, but the file stores them (`attrs: {}`).
            if fine && !value.is_null() {
                continue;
            }
            let default = match ty {
                "Swatch" => {
                    props_of(&Swatch { uid: Default::default(), name: String::new(), paint: Paint::solid(Color::BLACK), global: false, spot: false })
                }
                "GraphicStyle" => props_of(&GraphicStyle::new("", Appearance::default())),
                "CharStyle" | "ParaStyle" => props_of(&TextStyleDef { uid: Default::default(), name: String::new(), attrs: Default::default() }),
                _ => props_of(&PatternDef::new("", Vec::new())),
            };
            // What the batch gave stays where it reads; the rest is the default's.
            let mut merged = default;
            if let (Some(into), Some(given)) = (merged.as_object_mut(), value.as_object()) {
                for (k, v) in given {
                    into.insert(k.clone(), v.clone());
                }
            }
            tx.set(&object, field, merged)?;
            wrote = true;
        }
    }
    Ok(wrote)
}

/// Gives a node, an artboard, a slice or an export asset a model batch created with its scalar
/// fields alone what a new one has, as the writer would store it: an id after every one the
/// drawing has (its tool id), a node the kind and look of an empty group, an artboard a name and
/// the size of the first artboard to the right of the others, a slice a rectangle. An export
/// asset of no objects is refused: the drawing drops one (C8-vector-5). Answers whether it wrote.
pub(crate) fn complete_objects(tx: &mut Tx<'_>) -> KdmResult<bool> {
    use kition_vector_doc::geom::Rect;
    use kition_vector_doc::{Artboard, Node, NodeId, NodeKind, Slice};
    let number = |tx: &Tx<'_>, object: &ObjectId| tx.get(object, "id").ok().and_then(|v| v.as_u64());
    let mut next = 0u64;
    for ty in ["Node", "Slice", "Asset"] {
        for object in tx.objects_of_type(ty) {
            next = next.max(number(tx, &object).unwrap_or(0));
        }
    }
    let mut next_artboard = tx.objects_of_type("Artboard").iter().filter_map(|o| number(tx, o)).max().unwrap_or(0);
    let mut wrote = false;
    let fill = |tx: &mut Tx<'_>, object: &ObjectId, fields: crate::write::Fields| -> KdmResult<bool> {
        let mut wrote = false;
        for (field, value) in fields {
            let now = tx.get(object, field)?;
            let merged = match (&now, &value) {
                (Value::Null, _) => value.clone(),
                (Value::Object(given), Value::Object(default)) if field == "props" => {
                    let mut out = default.clone();
                    for (k, v) in given {
                        out.insert(k.clone(), v.clone());
                    }
                    Value::Object(out)
                }
                _ => continue,
            };
            if merged != now && !merged.is_null() {
                tx.set(object, field, merged)?;
                wrote = true;
            }
        }
        Ok(wrote)
    };
    // Only the drawing's nodes have ids: a symbol's art nodes have none (`crate::ids`).
    let root = root_id();
    let in_drawing: std::collections::HashSet<ObjectId> =
        if tx.exists(&root) { tx.tree(&root, "nodes")?.into_iter().map(|entry| entry.object).collect() } else { Default::default() };
    for object in tx.objects_of_type("Node") {
        let art = !in_drawing.contains(&object);
        if !tx.get(&object, "data")?.is_null() && (art || number(tx, &object).is_some()) {
            continue;
        }
        let id = match number(tx, &object) {
            Some(id) => id,
            None if art => 0,
            None => {
                next = next.saturating_add(1);
                next
            }
        };
        let node = Node::new(NodeId(id), NodeKind::Group { children: Vec::new(), clip: false });
        let mut fields = crate::write::node_fields(&node, &crate::refs::Names::default())?;
        if art && let Some(slot) = fields.iter_mut().find(|(field, _)| *field == "id") {
            slot.1 = Value::Null;
        }
        wrote |= fill(tx, &object, fields)?;
    }
    let boards = tx.objects_of_type("Artboard");
    let rects: Vec<Rect> = boards.iter().filter_map(|o| tx.get(o, "rect").ok().and_then(|v| serde_json::from_value::<Rect>(v).ok())).collect();
    for object in boards {
        let rect = tx.get(&object, "rect")?;
        let name = tx.get(&object, "props")?.get("name").cloned();
        if !rect.is_null() && name.is_some() && number(tx, &object).is_some() {
            continue;
        }
        let id = match number(tx, &object) {
            Some(id) => id,
            None => {
                next_artboard = next_artboard.saturating_add(1);
                next_artboard
            }
        };
        let size = rects.first().map_or((612.0, 792.0), |r| (r.width(), r.height()));
        let right = rects.iter().map(|r| r.x1).fold(f64::NEG_INFINITY, f64::max);
        let x = if right.is_finite() { right + 20.0 } else { 0.0 };
        let board = Artboard {
            id: u32::try_from(id).unwrap_or(u32::MAX),
            name: format!("Artboard {id}"),
            rect: Rect::new(x, 0.0, x + size.0, size.1),
            show_center_mark: false,
            show_cross_hairs: false,
        };
        wrote |= fill(tx, &object, crate::write::artboard_fields(&board)?)?;
    }
    for object in tx.objects_of_type("Slice") {
        if !tx.get(&object, "props")?.get("rect").is_none_or(Value::is_null) && number(tx, &object).is_some() {
            continue;
        }
        let id = match number(tx, &object) {
            Some(id) => id,
            None => {
                next = next.saturating_add(1);
                next
            }
        };
        let slice = Slice { id: NodeId(id), rect: Rect::new(0.0, 0.0, 100.0, 100.0), options: Default::default() };
        wrote |= fill(tx, &object, crate::write::slice_fields(&slice)?)?;
    }
    // A node goes only into one that holds others (a layer, group, compound path, blend, envelope or
    // repeat): the drawing has nowhere to keep one inside a path, type or mesh, and dropped it on
    // the next read (C8-vector-engine-2, kit-env's create).
    if tx.exists(&root) {
        let entries = tx.tree(&root, "nodes")?;
        for entry in &entries {
            let Some(parent) = &entry.parent else { continue };
            let Some(holder) = entries.iter().find(|e| &e.node == parent) else { continue };
            let data = tx.get(&holder.object, "data")?;
            let kind = data.get("type").and_then(Value::as_str).unwrap_or_default();
            if crate::write::children_key_of(kind).is_none() {
                return Err(kdm_err(format!(
                    "object {} is a {kind} and holds no objects: put the new object into a layer or a group",
                    tx.get(&holder.object, "id").ok().and_then(|v| v.as_u64()).map_or_else(|| holder.object.as_str().to_owned(), |id| id.to_string())
                )));
            }
        }
    }
    for object in tx.objects_of_type("Asset") {
        let props = tx.get(&object, "props")?;
        if number(tx, &object).is_none() && props.get("nodes").and_then(Value::as_array).is_none_or(Vec::is_empty) {
            return Err(kdm_err("an export asset needs the objects it exports (props.nodes)"));
        }
        if number(tx, &object).is_none() {
            next = next.saturating_add(1);
            tx.set(&object, "id", Value::from(next))?;
            wrote = true;
        }
        if props.get("name").is_none_or(Value::is_null) {
            let mut props = props.clone();
            if let Some(map) = props.as_object_mut() {
                map.insert("name".into(), Value::String(format!("Asset {next}")));
            }
            tx.set(&object, "props", props)?;
            wrote = true;
        }
    }
    Ok(wrote)
}
