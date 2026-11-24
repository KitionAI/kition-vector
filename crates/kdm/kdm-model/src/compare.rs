//! [`normalize`]: an id-blind form of a [`Document`], for comparing views. Local ids are minted
//! by every read (never replicated), so two views of the same content — a round trip, or two
//! replicas that converged — almost never share them.

use std::collections::HashMap;
use std::sync::Arc;

use kition_vector_doc::Document;
use serde_json::{Map, Value};

use crate::refs::{KEYS_WITH_IDS, Mapper, map_asset_nodes, map_setting};
use crate::write::{children_key_of, safe_ids};

/// The id-blind JSON of `d`: every node id renumbered from 1 in tree order (slices and asset ids
/// after them), each symbol's art numbered from 1 in its own tree order, artboards and graphic
/// styles numbered by position, every reference to one rewritten to match and a reference to
/// nothing left out (a replica does not keep those), the id counter and the editor's view state
/// (saved view, open Layers rows) left out, each type object's runs as a read gives them back
/// (neighbours with one look merged, see [`crate::text`]), and each image's bytes as a digest.
/// Two drawings with the same content normalize to the same value whatever ids either holds. Ids
/// inside art that is data (opacity masks, patterns) are kept as they are: they are that art's
/// own. Numbers are compared as KDM keeps them: a double holding a whole number is that integer
/// (raw JSON such as `unknown` reads back so), and ids inside art that is data as a replica takes
/// them (see [`safe_ids`]).
pub fn normalize(d: &Document) -> Value {
    let Ok(mut whole) = serde_json::to_value(d) else { return Value::Null };
    whole_numbers(&mut whole, 0);
    text_runs(&mut whole, 0);
    if let Some(Value::Array(symbols)) = whole.get_mut("symbols") {
        for symbol in symbols {
            if let Some(art) = symbol.get_mut("art") {
                let mut list = Value::Array(vec![std::mem::take(art)]);
                number_nodes(&mut list, &mut HashMap::new(), &mut 0, 0);
                if let Value::Array(mut one) = list
                    && let Some(back) = one.pop()
                {
                    *art = back;
                }
            }
        }
    }
    let Value::Object(mut v) = whole else { return Value::Null };
    let mut nodes: HashMap<u64, u64> = HashMap::new();
    let mut next = 0u64;
    if let Some(layers) = v.get_mut("layers") {
        number_nodes(layers, &mut nodes, &mut next, 0);
    }
    let mut artboards: HashMap<u64, u64> = HashMap::new();
    if let Some(Value::Array(list)) = v.get_mut("artboards") {
        for (i, a) in list.iter_mut().enumerate() {
            let new = i as u64 + 1;
            if let Some(id) = a.get("id").and_then(Value::as_u64) {
                artboards.entry(id).or_insert(new);
            }
            if let Some(map) = a.as_object_mut() {
                map.insert("id".into(), Value::from(new));
            }
        }
    }
    let mut styles: HashMap<u64, u64> = HashMap::new();
    if let Some(Value::Array(list)) = v.get_mut("graphic_styles") {
        for (i, g) in list.iter_mut().enumerate() {
            let new = i as u64 + 1;
            if let Some(id) = g.get("id").and_then(Value::as_u64).filter(|id| *id != 0) {
                styles.entry(id).or_insert(new);
            }
            if let Some(map) = g.as_object_mut() {
                map.insert("id".into(), Value::from(new));
            }
        }
    }
    if let Some(layers) = v.get_mut("layers") {
        restyle(layers, &styles, 0);
    }
    if let Some(Value::Array(symbols)) = v.get_mut("symbols") {
        for symbol in symbols {
            if let Some(art) = symbol.get_mut("art") {
                let mut list = Value::Array(vec![std::mem::take(art)]);
                restyle(&mut list, &styles, 0);
                if let Value::Array(mut one) = list
                    && let Some(back) = one.pop()
                {
                    *art = back;
                }
            }
        }
    }
    for key in ["slices", "assets"] {
        if let Some(Value::Array(list)) = v.get_mut(key) {
            for item in list {
                next += 1;
                if let Some(map) = item.as_object_mut() {
                    map.insert("id".into(), Value::from(next));
                }
            }
        }
    }
    let node = |x: &Value| x.as_u64().and_then(|n| nodes.get(&n)).map(|n| Value::from(*n));
    let artboard = |x: &Value| x.as_u64().and_then(|n| artboards.get(&n)).map(|n| Value::from(*n));
    let m = Mapper { node: &node, artboard: &artboard };
    if let Some(Value::Array(list)) = v.get_mut("assets") {
        for asset in list {
            map_asset_nodes(asset, &node);
        }
    }
    for key in KEYS_WITH_IDS {
        if let Some(x) = v.get_mut(*key) {
            map_setting(key, x, &m);
            if x.is_null() {
                v.remove(*key);
            }
        }
    }
    v.remove("next_id");
    let mut images = Map::new();
    for (key, img) in &d.images {
        let mut entry = Map::new();
        entry.insert("mime".into(), Value::String(img.mime.clone()));
        entry.insert("bytes".into(), Value::String(digest(&img.bytes)));
        if let Some(p) = &img.proxy {
            entry.insert("proxy".into(), Value::String(digest(p)));
        }
        images.insert(key.clone(), Value::Object(entry));
    }
    v.insert("images".into(), Value::Object(images));
    let mut v = Value::Object(v);
    safe_ids(&mut v, 0);
    v
}

/// Puts every type object's runs inside `v` in the form a read gives them
/// ([`crate::text::canonical_runs`]).
fn text_runs(v: &mut Value, depth: usize) {
    if depth > crate::MAX_DEPTH * 4 {
        return;
    }
    match v {
        Value::Object(map) => {
            if map.get("type").and_then(Value::as_str) == Some("text")
                && let Some(Value::Array(runs)) = map.get_mut("runs")
            {
                crate::text::canonical_runs(runs);
            }
            for item in map.values_mut() {
                text_runs(item, depth + 1);
            }
        }
        Value::Array(items) => items.iter_mut().for_each(|item| text_runs(item, depth + 1)),
        _ => {}
    }
}

/// Renumbers the nodes of a serialized child list (depth first), answering old id -> new id.
fn number_nodes(list: &mut Value, map: &mut HashMap<u64, u64>, next: &mut u64, depth: usize) {
    let Some(items) = list.as_array_mut() else { return };
    if depth > crate::MAX_DEPTH {
        return;
    }
    for n in items {
        if let Some(id) = n.get("id").and_then(Value::as_u64) {
            *next += 1;
            map.entry(id).or_insert(*next);
            if let Some(obj) = n.as_object_mut() {
                obj.insert("id".into(), Value::from(*next));
            }
        }
        if let Some(children) = children_of(n) {
            number_nodes(children, map, next, depth + 1);
        }
    }
}

fn children_of(n: &mut Value) -> Option<&mut Value> {
    let kind = n.get_mut("kind")?;
    let key = children_key_of(kind.get("type")?.as_str()?)?;
    kind.get_mut(key)
}

/// Rewrites each node's graphic style link by `styles`, leaving out links to no style.
fn restyle(list: &mut Value, styles: &HashMap<u64, u64>, depth: usize) {
    let Some(items) = list.as_array_mut() else { return };
    if depth > crate::MAX_DEPTH {
        return;
    }
    for n in items {
        if let Some(obj) = n.as_object_mut()
            && let Some(style) = obj.remove("graphic_style")
            && let Some(new) = style.as_u64().and_then(|s| styles.get(&s))
        {
            obj.insert("graphic_style".into(), Value::from(*new));
        }
        if let Some(children) = children_of(n) {
            restyle(children, styles, depth + 1);
        }
    }
}

/// Rewrites every double holding a whole number within ±2^53 as that integer, as a KDM replica
/// keeps numbers (kition-kdm's canonical form: JavaScript has one number type), so a value read
/// back from a replica compares equal to the one written. Typed fields read back as they were;
/// only raw JSON (`unknown`, keys a newer version wrote) can differ this way.
fn whole_numbers(v: &mut Value, depth: usize) {
    const SAFE: f64 = 9_007_199_254_740_992.0;
    if depth > crate::MAX_DEPTH * 4 {
        return;
    }
    match v {
        Value::Number(n) if !n.is_i64() && !n.is_u64() => {
            if let Some(f) = n.as_f64().filter(|f| f.is_finite() && f.fract() == 0.0 && f.abs() <= SAFE) {
                // Exact: |f| <= 2^53 and whole.
                #[allow(clippy::cast_possible_truncation)]
                let i = f as i64;
                *v = Value::from(i);
            }
        }
        Value::Array(items) => items.iter_mut().for_each(|x| whole_numbers(x, depth + 1)),
        Value::Object(map) => map.values_mut().for_each(|x| whole_numbers(x, depth + 1)),
        _ => {}
    }
}

/// FNV-1a of `bytes`, with their length: a digest for comparing image bytes in tests.
fn digest(bytes: &[u8]) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{}:{h:016x}", bytes.len())
}

/// Whether `a` and `b` hold the same content with the same local ids (their id counters and
/// caches aside), numbers and ids compared as a replica keeps them (see [`whole_numbers`] and
/// [`safe_ids`]).
pub(crate) fn same_content(a: &Document, b: &Document) -> bool {
    let json = |d: &Document| -> Option<Value> {
        let Ok(mut whole) = serde_json::to_value(d) else { return None };
        whole_numbers(&mut whole, 0);
        safe_ids(&mut whole, 0);
        let Value::Object(mut v) = whole else { return None };
        v.remove("next_id");
        Some(Value::Object(v))
    };
    let images_same = a.images.len() == b.images.len()
        && a.images.iter().zip(&b.images).all(|((ka, x), (kb, y))| {
            ka == kb
                && (Arc::ptr_eq(&x.bytes, &y.bytes) || x.bytes == y.bytes)
                && match (&x.proxy, &y.proxy) {
                    (None, None) => true,
                    (Some(p), Some(q)) => Arc::ptr_eq(p, q) || p == q,
                    _ => false,
                }
        });
    images_same && json(a).is_some_and(|x| json(b).is_some_and(|y| x == y))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A replica keeps a whole double as an integer: raw JSON read back from one holds `45` where
    /// the file held `45.0`, and that is the same content.
    #[test]
    fn whole_doubles_compare_as_integers() {
        let mut file = Document::new(400.0, 300.0);
        file.unknown.insert("perspectiveGrid".into(), serde_json::json!({ "angle": 45.0, "origin": [800.0, 0.5] }));
        let mut replica = file.clone();
        replica.unknown.insert("perspectiveGrid".into(), serde_json::json!({ "angle": 45, "origin": [800, 0.5] }));
        assert!(same_content(&file, &replica));
        assert_eq!(normalize(&file), normalize(&replica));
        replica.unknown.insert("perspectiveGrid".into(), serde_json::json!({ "angle": 45, "origin": [800, 0.25] }));
        assert!(!same_content(&file, &replica));
    }
}
