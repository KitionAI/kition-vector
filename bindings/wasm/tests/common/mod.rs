//! The Kition platform kit suite's adapter for Kition Vector (its performance mode): drives
//! `VectorDocument` the way a host does (contract 1 `execute`, the contract 2 replica members),
//! for the `perf` test binary.
#![allow(dead_code, clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;

use kition_convergence::{Blobs, KitAdapter, Result, Rng, Script};
use kition_kit::KitDocument;
use kition_kit::kdm::{ActorId, Replica};
use kition_kit::session::KitModel;
use kition_vector_kit::VectorDocument;
use serde_json::{Value, json};

pub const NAME: &str = "Drawing.kvector";
const WIDTH: u32 = 400;
const HEIGHT: u32 = 300;

/// CJK ideographs ("middle", "text") and an emoji, by code point, for typed text.
const CJK_MIDDLE: char = '\u{4E2D}';
const CJK_TEXT: char = '\u{6587}';
const EMOJI_GRIN: char = '\u{1F600}';

pub struct VectorKit;

/// `(64-char hex SHA-256, u64 LE length, bytes) * N`, as `blobs`/`add_blobs` move them.
pub fn decode_blobs(bytes: &[u8]) -> Result<Blobs> {
    let mut out = Blobs::new();
    let mut at = 0;
    while at < bytes.len() {
        let hash = std::str::from_utf8(bytes.get(at..at + 64).ok_or("truncated hash")?).map_err(|e| e.to_string())?.to_owned();
        let len = u64::from_le_bytes(bytes.get(at + 64..at + 72).ok_or("truncated length")?.try_into().map_err(|_| "length")?) as usize;
        let data = bytes.get(at + 72..at + 72 + len).ok_or("truncated data")?;
        out.insert(hash, data.to_vec());
        at += 72 + len;
    }
    Ok(out)
}

pub fn encode_blobs(blobs: &Blobs) -> Vec<u8> {
    let mut out = Vec::new();
    for (hash, data) in blobs {
        out.extend_from_slice(hash.as_bytes());
        out.extend_from_slice(&(data.len() as u64).to_le_bytes());
        out.extend_from_slice(data);
    }
    out
}

fn exec(doc: &mut VectorDocument, command: &str, params: Value) -> Result<Value> {
    doc.execute(command, params).map_err(|error| format!("{command}: {error}"))
}

/// The layer tree as `document.inspect` reports it, depth first, top of the stack first.
fn nodes(doc: &VectorDocument) -> Vec<Value> {
    fn walk(node: &Value, out: &mut Vec<Value>) {
        out.push(node.clone());
        for child in node["children"].as_array().into_iter().flatten() {
            walk(child, out);
        }
    }
    let inspected = doc.inspect().unwrap_or(Value::Null);
    let mut out = Vec::new();
    for layer in inspected["layers"].as_array().into_iter().flatten() {
        walk(layer, &mut out);
    }
    out
}

fn ids_where(doc: &VectorDocument, keep: impl Fn(&Value) -> bool) -> Vec<u64> {
    nodes(doc).iter().filter(|node| keep(node)).filter_map(|node| node["id"].as_u64()).collect()
}

fn is_leaf(node: &Value) -> bool {
    !matches!(node["kind"].as_str(), Some("Layer" | "Group" | "Sublayer")) && node.get("children").is_none()
}

/// Objects that are not layers or groups, in document order: the same on every replica that
/// has the same state, since the view follows the replica.
fn leaves(doc: &VectorDocument) -> Vec<u64> {
    ids_where(doc, |node| is_leaf(node) && !node["locked"].as_bool().unwrap_or(false))
}

fn groups(doc: &VectorDocument) -> Vec<u64> {
    ids_where(doc, |node| node["kind"].as_str() == Some("Group"))
}

fn layers(doc: &VectorDocument) -> Vec<u64> {
    let inspected = doc.inspect().unwrap_or(Value::Null);
    inspected["layers"].as_array().into_iter().flatten().filter_map(|layer| layer["id"].as_u64()).collect()
}

fn probe(doc: &VectorDocument) -> Result<u64> {
    leaves(doc).first().copied().ok_or_else(|| "the drawing has no object".to_owned())
}

/// The first text object and its text.
fn text_object(doc: &VectorDocument) -> Option<(u64, String)> {
    nodes(doc).iter().find(|node| node["kind"].as_str() == Some("Type") || node.get("text").is_some()).and_then(|node| {
        let text = node["text"].as_str()?.to_owned();
        Some((node["id"].as_u64()?, text))
    })
}

/// The byte offset of character `at` of `text` (its end past the last).
fn byte_at(text: &str, at: usize) -> usize {
    text.char_indices().nth(at).map_or(text.len(), |(byte, _)| byte)
}

fn color(value: u64) -> String {
    format!("#{:06x}", (value.wrapping_mul(0x9E37_79B9)) & 0xff_ffff)
}

/// A small PNG whose pixel derives from `seed`.
fn png(seed: u32) -> Vec<u8> {
    let mut out = Vec::new();
    let [a, b, c, _] = seed.to_le_bytes();
    image::RgbaImage::from_pixel(2, 2, image::Rgba([a, b, c, 255])).write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png).unwrap();
    out
}

fn select(doc: &mut VectorDocument, ids: &[u64]) -> Result<()> {
    exec(doc, "select.set", json!({ "ids": ids })).map(|_| ())
}

/// Runs a command; a refusal because of the drawing's state is no edit (`Ok(false)`).
fn attempt(doc: &mut VectorDocument, command: &str, params: Value) -> Result<bool> {
    Ok(doc.execute(command, params).is_ok())
}

impl VectorKit {
    fn rect(&self, doc: &mut VectorDocument, x: u64, y: u64) -> Result<u64> {
        let made = exec(doc, "shape.rectangle", json!({ "x": x % 360, "y": y % 260, "width": 30, "height": 20 }))?;
        made["id"].as_u64().ok_or_else(|| format!("shape.rectangle answered {made}"))
    }
}

impl KitAdapter for VectorKit {
    type Doc = VectorDocument;

    fn name(&self) -> &str {
        "kit.vector"
    }

    fn schema(&self) -> &str {
        include_str!("../../../../crates/kdm/kdm-model/model/schema.json")
    }

    /// Two layers; on the first, four rectangles in two groups and a loose one, a text object;
    /// on the second, an ellipse.
    fn create(&self) -> Result<VectorDocument> {
        let mut doc = VectorDocument::create(NAME, WIDTH, HEIGHT)?;
        let loose = self.rect(&mut doc, 10, 10)?;
        let a = [self.rect(&mut doc, 60, 10)?, self.rect(&mut doc, 100, 10)?];
        let b = [self.rect(&mut doc, 60, 60)?, self.rect(&mut doc, 100, 60)?];
        for pair in [a, b] {
            select(&mut doc, &pair)?;
            exec(&mut doc, "object.group", json!({}))?;
        }
        exec(&mut doc, "paint.setFill", json!({ "ids": [loose], "color": "#3366cc" }))?;
        exec(&mut doc, "text.create", json!({ "x": 20, "y": 200, "text": "Hello vector" }))?;
        exec(&mut doc, "layer.new", json!({ "name": "Top" }))?;
        exec(&mut doc, "shape.ellipse", json!({ "x": 200, "y": 100, "width": 40, "height": 40 }))?;
        exec(&mut doc, "select.none", json!({}))?;
        Ok(doc)
    }

    fn join(&self, snapshot: &[u8], blobs: &Blobs) -> Result<VectorDocument> {
        let doc = VectorDocument::open_with_replica(NAME, &[], snapshot, &encode_blobs(blobs))?;
        // `open_with_replica` falls back to the plain file when the replica does not open.
        if !doc.has_replica() {
            return Err("the snapshot did not open as a replica".into());
        }
        Ok(doc)
    }

    fn version(&self, doc: &mut VectorDocument) -> Result<Vec<u8>> {
        doc.version()
    }

    fn export_changes(&self, doc: &mut VectorDocument, since: &[u8]) -> Result<Vec<u8>> {
        doc.export_changes(since)
    }

    fn import_changes(&self, doc: &mut VectorDocument, bytes: &[u8]) -> Result<()> {
        doc.import_changes(bytes).map(|_| ())
    }

    fn snapshot(&self, doc: &mut VectorDocument) -> Result<Vec<u8>> {
        doc.snapshot()
    }

    fn state(&self, doc: &mut VectorDocument) -> Result<Value> {
        let snapshot = doc.snapshot()?;
        let replica = Replica::load(kdm_model::VectorModel::schema(), &snapshot, ActorId::random()).map_err(|e| e.to_string())?;
        replica.state_json().map_err(|e| e.to_string())
    }

    fn undo(&self, doc: &mut VectorDocument) -> Result<bool> {
        doc.undo().map(|(done, _)| done)
    }

    fn redo(&self, doc: &mut VectorDocument) -> Result<bool> {
        doc.redo().map(|(done, _)| done)
    }

    fn random_edit(&self, doc: &mut VectorDocument, rng: &mut Rng) -> Result<Option<String>> {
        let leaves = leaves(doc);
        let target = rng.pick(&leaves).copied();
        let value = rng.next_u64() % 10_000;
        let (label, done) = match (rng.below(16), target) {
            (0 | 1, _) => ("rectangle", self.rect(doc, value, value / 7).is_ok()),
            (2, _) => ("ellipse", attempt(doc, "shape.ellipse", json!({ "x": value % 360, "y": value % 260, "width": 20, "height": 20 }))?),
            (3, Some(id)) => ("fill", attempt(doc, "paint.setFill", json!({ "ids": [id], "color": color(value) }))?),
            (4, Some(id)) if leaves.len() > 2 => ("delete", attempt(doc, "edit.clear", json!({ "ids": [id] }))?),
            (5 | 6, Some(id)) => {
                select(doc, &[id])?;
                ("move", attempt(doc, "object.move", json!({ "dx": (value % 21) as i64 - 10, "dy": (value % 13) as i64 - 6 }))?)
            }
            (7, Some(id)) => {
                select(doc, &[id])?;
                ("duplicate", attempt(doc, "edit.duplicate", json!({ "dx": 5, "dy": 5 }))?)
            }
            (8, Some(id)) => {
                select(doc, &[id])?;
                let order =
                    ["object.arrange.bringToFront", "object.arrange.sendToBack", "object.arrange.bringForward", "object.arrange.sendBackward"];
                ("arrange", attempt(doc, order[rng.below(order.len())], json!({}))?)
            }
            (9, Some(id)) if leaves.len() > 1 => {
                let other = leaves[rng.below(leaves.len())];
                if other == id {
                    return Ok(None);
                }
                select(doc, &[id, other])?;
                ("group", attempt(doc, "object.group", json!({}))?)
            }
            (10, _) => {
                let groups = groups(doc);
                let Some(&group) = rng.pick(&groups) else { return Ok(None) };
                select(doc, &[group])?;
                ("ungroup", attempt(doc, "object.ungroup", json!({}))?)
            }
            (11, Some(id)) => {
                let layers = layers(doc);
                let Some(&layer) = rng.pick(&layers) else { return Ok(None) };
                ("to layer", attempt(doc, "layer.move", json!({ "ids": [id], "target": layer, "place": "inside" }))?)
            }
            (12, _) => {
                let text: String = [CJK_MIDDLE, CJK_TEXT, EMOJI_GRIN].iter().collect();
                ("text", attempt(doc, "text.create", json!({ "x": value % 300, "y": 40 + value % 200, "text": format!("t{value}{text}") }))?)
            }
            (13, _) => {
                let Some((id, text)) = text_object(doc) else { return Ok(None) };
                ("retype", attempt(doc, "text.setText", json!({ "ids": [id], "text": format!("{text}{}", value % 10) }))?)
            }
            (14, _) => ("place image", self.script(doc, &Script::InsertBlob { seed: value as u32 })?),
            (15, _) => ("new layer", attempt(doc, "layer.new", json!({ "name": format!("Layer {value}") }))?),
            _ => return Ok(None),
        };
        Ok(done.then(|| label.to_owned()))
    }

    fn script(&self, doc: &mut VectorDocument, step: &Script) -> Result<bool> {
        match step {
            Script::SetField { slot: 0, value } => {
                // The probe's geometry (its `data`).
                let id = probe(doc)?;
                select(doc, &[id])?;
                exec(doc, "object.move", json!({ "dx": 1 + (*value % 40), "dy": 1 + (*value % 30) }))?;
            }
            Script::SetField { value, .. } => {
                // The probe's paint (its `appearance`).
                let id = probe(doc)?;
                exec(doc, "paint.setFill", json!({ "ids": [id], "color": color(u64::from(*value) + 1) }))?;
            }
            Script::Create => {
                self.rect(doc, 300, 200)?;
            }
            Script::Delete => {
                let id = probe(doc)?;
                exec(doc, "edit.clear", json!({ "ids": [id] }))?;
            }
            Script::Duplicate => {
                let id = probe(doc)?;
                select(doc, &[id])?;
                exec(doc, "edit.duplicate", json!({ "dx": 10, "dy": 10 }))?;
            }
            Script::Reorder { which } => {
                // The bottom-most objects, brought to the front of their parents: the loose
                // rectangle in its layer, and a rectangle in its group.
                let leaves = leaves(doc);
                let id = *leaves.iter().rev().nth(usize::from(*which)).ok_or("no object")?;
                select(doc, &[id])?;
                exec(doc, "object.arrange.bringToFront", json!({}))?;
            }
            Script::Reparent { which } => {
                // Each group into the other: the tree must refuse the cycle one of them makes.
                let groups = groups(doc);
                let (Some(&a), Some(&b)) = (groups.first(), groups.get(1)) else { return Ok(false) };
                let (moved, into) = if *which == 0 { (a, b) } else { (b, a) };
                exec(doc, "layer.move", json!({ "ids": [moved], "target": into, "place": "inside" }))?;
            }
            Script::InsertBlob { seed } => {
                let data = kition_vector_format::base64_encode(&png(*seed));
                exec(doc, "file.place", json!({ "name": format!("picture-{seed}.png"), "dataBase64": data }))?;
            }
            // Typing in the first text object, as the Type tool edits it (`text.editRange`, in
            // bytes of its plain text): its characters are its `Node.text`.
            Script::InsertText { at, text } => {
                let Some((id, now)) = text_object(doc) else { return Ok(false) };
                let at = byte_at(&now, *at);
                exec(doc, "text.editRange", json!({ "id": id, "start": at, "end": at, "insert": text }))?;
            }
            Script::DeleteText { at, len } => {
                let Some((id, now)) = text_object(doc) else { return Ok(false) };
                exec(doc, "text.editRange", json!({ "id": id, "start": byte_at(&now, *at), "end": byte_at(&now, at + len), "insert": "" }))?;
            }
            Script::SplitParagraph { at } => {
                let Some((id, now)) = text_object(doc) else { return Ok(false) };
                let at = byte_at(&now, *at);
                exec(doc, "text.editRange", json!({ "id": id, "start": at, "end": at, "insert": "\n" }))?;
            }
            Script::MergeParagraph { at } => {
                let Some((id, now)) = text_object(doc) else { return Ok(false) };
                exec(doc, "text.editRange", json!({ "id": id, "start": byte_at(&now, *at), "end": byte_at(&now, at + 1), "insert": "" }))?;
            }
            _ => return Ok(false),
        }
        exec(doc, "select.none", json!({}))?;
        Ok(true)
    }

    /// The first text object's text (its paragraphs joined by `\n`).
    fn text(&self, doc: &mut VectorDocument) -> Result<Option<String>> {
        Ok(text_object(doc).map(|(_, text)| text))
    }

    /// The view matches a fresh replica of the same state: the same layer tree (ids aside, as
    /// local ids are per replica) and the same pixels.
    fn check_view(&self, doc: &mut VectorDocument) -> Result<()> {
        let snapshot = doc.snapshot()?;
        let blobs = self.blobs(doc)?;
        let fresh = self.join(&snapshot, &blobs)?;
        let tree = |doc: &VectorDocument| strip_ids(&doc.inspect().unwrap_or(Value::Null)["layers"]);
        if tree(doc) != tree(&fresh) {
            return Err("the view's layer tree differs from a fresh replica's".into());
        }
        let (w, h) = (doc.width(), doc.height());
        if doc.render(0.0, 0.0, 1.0, w, h)? != fresh.render(0.0, 0.0, 1.0, w, h)? {
            return Err("the view renders differently from a fresh replica".into());
        }
        Ok(())
    }

    fn blobs(&self, doc: &mut VectorDocument) -> Result<Blobs> {
        decode_blobs(&doc.blobs()?)
    }

    fn add_blobs(&self, doc: &mut VectorDocument, blobs: &Blobs) -> Result<()> {
        doc.add_blobs(&encode_blobs(blobs))
    }

    fn export_file(&self, doc: &mut VectorDocument) -> Result<Option<Vec<u8>>> {
        doc.save(None).map(Some)
    }

    /// A nudge of one object and the repaint of the visible canvas.
    fn keystroke(&self, doc: &mut VectorDocument, rng: &mut Rng) -> Result<()> {
        if let Some(&id) = rng.pick(&leaves(doc)) {
            select(doc, &[id])?;
            exec(doc, "object.move", json!({ "dx": 1, "dy": 0 }))?;
        }
        doc.render_area(0, 0, WIDTH as i32, HEIGHT as i32, WIDTH, HEIGHT).map(|_| ())
    }
}

/// `value` without `id` keys (local ids differ between replicas) or `selection`-like state.
fn strip_ids(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let kept: BTreeMap<String, Value> = map.iter().filter(|(key, _)| key.as_str() != "id").map(|(k, v)| (k.clone(), strip_ids(v))).collect();
            serde_json::to_value(kept).unwrap_or(Value::Null)
        }
        Value::Array(items) => Value::Array(items.iter().map(strip_ids).collect()),
        other => other.clone(),
    }
}
