//! Format fidelity scorecard (roadmap phase 7).
//!
//! `KITION_VECTOR_CORPUS=<dir>`: every SVG in that directory is imported, exported and imported
//! again, and the test reports
//!
//! - per feature class (paths, transforms, fills and strokes, gradients, patterns, text, groups and
//!   layers, clip and mask, symbols, effects, images, …), which files keep it through import →
//!   export → import (the two documents' leaves of that class are compared as multisets of
//!   `path=value`, so a moved object is not a loss but a dropped property is), and
//! - what the exported SVG lost against the original SVG: elements and attributes (style
//!   declarations counted as the attributes they stand for) counted per part kind (`defs` for
//!   what definitions hold, `body` for the rest).
//!
//! Native drawings in the corpus (`.kvector`, and drawings of the editor Kition Vector
//! started from, whose files differ only in their format tag) are exported to SVG in memory first
//! and that SVG is scored: they cover what the SVG files of a corpus rarely hold (type, symbols,
//! effects, swatches).
//!
//! `KITION_FIDELITY_OUT=<dir>` writes the full report (`vector-fidelity.txt`, per file and
//! aggregated) and the aggregate scorecard (`vector-fidelity.json`, no file names) there.
//! `KITION_FIDELITY_BASELINE=<file>` fails when the corpus scores below such a scorecard: fewer
//! files keeping a class, more leaves lost, or less markup retained (the non-regression gate).
//! Skipped when the corpus variable is unset. The KDM leg of the product's path (import into a
//! replica, read back) is covered by `kdm-model`'s `tests/roundtrip.rs`.
//!
//! The scorer below the kit-specific part is the same as in Kition Word, Slide and Sheet (the
//! package features are read from SVG markup instead of a zip package, so `features` is
//! kit-specific here).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;

use serde_json::{Value, json};

use kition_vector_doc::Document;

// ---- Kit-specific -------------------------------------------------------------------------------

const KIT: &str = "vector";
const CORPUS_VAR: &str = "KITION_VECTOR_CORPUS";
const EXTENSIONS: &[&str] = &["svg", "svgz", "kvector"];
/// A class the self-test's document must have.
const PRIMARY_CLASS: &str = "paths";

type Model = Document;

fn import(bytes: &[u8]) -> Result<Model, String> {
    let text = kition_vector_svg::text_of(bytes).map_err(|error| error.to_string())?;
    kition_vector_svg::import(&text).map_err(|error| error.to_string())
}

fn export(doc: &Model) -> Result<Vec<u8>, String> {
    Ok(kition_vector_svg::export(doc, &kition_vector_svg::ExportOptions::default()).into_bytes())
}

/// The SVG a corpus file is scored by: an SVG file as it is, a native drawing exported to SVG
/// (`None`: neither).
fn source(bytes: &[u8]) -> Option<Vec<u8>> {
    if kition_vector_svg::is_svgz(bytes) || String::from_utf8_lossy(bytes.get(..bytes.len().min(1024)).unwrap_or(bytes)).contains("<svg") {
        return Some(bytes.to_vec());
    }
    let file: Value = serde_json::from_slice(bytes).ok()?;
    let mut doc: Document = serde_json::from_value(file.get("document")?.clone()).ok()?;
    doc.drop_edit_modes();
    doc.migrate_overprint_black();
    doc.migrate_color_profiles();
    doc.fix_next_id();
    export(&doc).ok()
}

/// The model as JSON, its layer tree flattened into `nodes` (each `{type: node}` with the type of
/// the container it is in, and how many children it has), plus the images by content. A
/// reference to an image names its content, since the importer keys images by content anyway.
fn model_value(doc: &Model) -> Value {
    fn rename(value: &mut Value, names: &BTreeMap<String, String>) {
        match value {
            Value::String(s) => {
                if let Some(content) = names.get(s.as_str()) {
                    *s = content.clone();
                }
            }
            Value::Array(items) => items.iter_mut().for_each(|v| rename(v, names)),
            Value::Object(map) => map.values_mut().for_each(|v| rename(v, names)),
            _ => {}
        }
    }
    fn flatten(node: &Value, parent: &str, out: &mut Vec<Value>) {
        let mut node = node.clone();
        let mut children = Vec::new();
        let mut kind = String::from("node");
        if let Some(k) = node.get_mut("kind").and_then(Value::as_object_mut) {
            if let Some(Value::Array(c)) = k.remove("children") {
                children = c;
            }
            kind = k.get("type").and_then(Value::as_str).unwrap_or("node").to_owned();
        }
        if let Some(map) = node.as_object_mut() {
            map.insert("in".into(), json!(parent));
            map.insert("children".into(), json!(children.len()));
        }
        out.push(json!({ kind.clone(): node }));
        for child in &children {
            flatten(child, &kind, out);
        }
    }
    /// Points and matrices (`[x, y]`, `[a, b, c, d, e, f]`) as one leaf per number, so that each
    /// is compared within [`close`]'s tolerance.
    fn split_numbers(value: &mut Value) {
        match value {
            Value::Array(items) if items.len() > 1 && items.iter().all(Value::is_number) => {
                const KEYS: [&str; 6] = ["a", "b", "c", "d", "e", "f"];
                let names: &[&str] = if items.len() == 2 { &["x", "y"] } else { &KEYS };
                if items.len() <= names.len() {
                    *value = Value::Object(names.iter().zip(items.iter()).map(|(k, v)| ((*k).to_owned(), v.clone())).collect());
                }
            }
            Value::Array(items) => items.iter_mut().for_each(split_numbers),
            Value::Object(map) => map.values_mut().for_each(split_numbers),
            _ => {}
        }
    }
    let mut value = serde_json::to_value(doc).unwrap_or(Value::Null);
    split_numbers(&mut value);
    let names: BTreeMap<String, String> = doc.images.iter().map(|(key, blob)| (key.clone(), format!("image:{}", digest(&blob.bytes)))).collect();
    rename(&mut value, &names);
    if let Value::Object(map) = &mut value {
        let mut images: Vec<Value> = doc.images.values().map(|blob| json!(format!("{}:{}", blob.mime, digest(&blob.bytes)))).collect();
        images.sort_by_key(|v| v.to_string());
        map.insert("images".into(), Value::Array(images));
        let mut nodes = Vec::new();
        if let Some(Value::Array(layers)) = map.remove("layers") {
            for layer in &layers {
                flatten(layer, "document", &mut nodes);
            }
        }
        map.insert("nodes".into(), Value::Array(nodes));
    }
    value
}

/// Keys whose values are ids or caches a re-import may renumber without losing anything.
fn ignored(key: &str) -> bool {
    matches!(key, "id" | "next_id")
}

/// Whether two numbers of `class` are the same after a round trip: the SVG writer rounds to three
/// decimals and colours go through 8-bit channels.
fn close(_class: &str, a: f64, b: f64) -> bool {
    (a - b).abs() <= 0.01_f64.max(a.abs() * 1e-4)
}

/// The feature class of a leaf, by its path (`.key` segments, `[]` for array items, `#` for
/// numeric map keys). Nodes are `.nodes[].<type>.…`.
fn classify(path: &str) -> &'static str {
    const RULES: &[(&str, &str)] = &[
        (".images", "images"),
        (".nodes[].image.", "images"),
        ("mask", "clip and mask"),
        (".clip", "clip and mask"),
        (".symbols", "symbols"),
        (".nodes[].symbolinstance.", "symbols"),
        (".effects", "effects"),
        ("gradient", "gradients"),
        ("pattern", "patterns"),
        (".nodes[].text.", "text"),
        ("_styles", "text"),
        (".swatch", "swatches and styles"),
        (".graphic_style", "swatches and styles"),
        (".appearance", "fills and strokes"),
        (".xf", "transforms"),
        ("transform", "transforms"),
        (".nodes[].path.", "paths"),
        (".nodes[].compound.", "paths"),
        (".nodes[].blend.", "paths"),
        (".nodes[].envelope.", "paths"),
        (".nodes[].mesh.", "paths"),
        (".nodes[].repeat.", "paths"),
        (".nodes[].layer.", "groups and layers"),
        (".nodes[].group.", "groups and layers"),
        (".artboards", "document"),
        (".units", "document"),
        (".title", "document"),
        (".color_mode", "document"),
        (".setup", "document"),
        (".version", "document"),
    ];
    let lower = path.to_ascii_lowercase();
    for (needle, class) in RULES {
        if lower.contains(needle) {
            return class;
        }
    }
    "other"
}

/// Elements and attributes of an SVG by part kind: `defs` for definitions (and what they hold),
/// `body` for the rest. A `style` attribute's declarations count as the attributes they stand for.
fn features(svg: &[u8]) -> Result<Features, String> {
    const DEFINITIONS: &[&str] = &["defs", "clipPath", "mask", "pattern", "symbol", "linearGradient", "radialGradient", "filter", "marker"];
    let text = kition_vector_svg::text_of(svg).map_err(|error| error.to_string())?;
    let opts = usvg::roxmltree::ParsingOptions { allow_dtd: true, ..Default::default() };
    let xml = usvg::roxmltree::Document::parse_with_options(&text, opts).map_err(|error| error.to_string())?;
    let mut out = Features::new();
    for node in xml.descendants().filter(|n| n.is_element()) {
        let element = node.tag_name().name();
        if node.tag_name().namespace().is_some_and(|ns| ns != "http://www.w3.org/2000/svg") {
            continue;
        }
        let kind = if node.ancestors().any(|a| DEFINITIONS.contains(&a.tag_name().name())) { "defs" } else { "body" };
        let counts = out.entry(kind.into()).or_default();
        *counts.entry(element.to_owned()).or_default() += 1;
        for attr in node.attributes() {
            if attr.name() == "style" {
                for declaration in attr.value().split(';') {
                    if let Some((property, _)) = declaration.split_once(':') {
                        *counts.entry(format!("{element}@{}", property.trim())).or_default() += 1;
                    }
                }
                continue;
            }
            *counts.entry(format!("{element}@{}", attr.name())).or_default() += 1;
        }
    }
    Ok(out)
}

/// A small drawing exercising most classes, for the self-test.
fn synthetic() -> Model {
    use kition_vector_color::{Color, Paint};
    use kition_vector_doc::{Appearance, Node};
    use kition_vector_geom::{Rect, shapes};
    let mut doc = Document::new(200.0, 100.0);
    let layer = doc.layers[0].id;
    let solid = |hex: &str| Paint::solid(Color::from_hex(hex).unwrap_or(Color::BLACK));
    let a =
        Node::path(doc.alloc_id(), shapes::rectangle(Rect::new(10.0, 10.0, 90.0, 60.0)), Appearance::basic(solid("#ff8000"), solid("#0000ff"), 2.0));
    let b =
        Node::path(doc.alloc_id(), shapes::ellipse(Rect::new(100.0, 20.0, 180.0, 80.0)), Appearance::basic(solid("#336699"), solid("#000000"), 1.0));
    let _ = doc.insert(Some(layer), usize::MAX, a);
    let _ = doc.insert(Some(layer), usize::MAX, b);
    doc
}

// ---- Shared scorer (as in kition-word, kition-slide, kition-sheet) -------------------------------

/// Leaves of a model by class: `path=value` → count.
type Leaves = BTreeMap<&'static str, BTreeMap<String, u64>>;
/// Features of a package by part kind: feature → count.
type Features = BTreeMap<String, BTreeMap<String, u64>>;

fn digest(bytes: &[u8]) -> String {
    // FNV-1a: enough to tell two media items apart in a report.
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash = (hash ^ u64::from(*byte)).wrapping_mul(0x0100_0000_01b3);
    }
    format!("{:016x}/{}", hash, bytes.len())
}

fn fold_digits(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut in_digits = false;
    for c in name.chars() {
        if c.is_ascii_digit() {
            if !in_digits {
                out.push('#');
            }
            in_digits = true;
        } else {
            out.push(c);
            in_digits = false;
        }
    }
    out
}

fn leaves(value: &Value) -> Leaves {
    fn walk(value: &Value, path: &mut String, out: &mut Leaves) {
        match value {
            Value::Null => {}
            Value::Object(map) => {
                for (key, item) in map {
                    if ignored(key) {
                        continue;
                    }
                    let len = path.len();
                    path.push('.');
                    path.push_str(if key.chars().all(|c| c.is_ascii_digit() || c == '-') { "#" } else { key });
                    walk(item, path, out);
                    path.truncate(len);
                }
            }
            Value::Array(items) if items.iter().any(|v| v.is_object() || v.is_array()) => {
                let len = path.len();
                path.push_str("[]");
                for item in items {
                    walk(item, path, out);
                }
                path.truncate(len);
            }
            scalar => {
                *out.entry(classify(path)).or_default().entry(format!("{path}={scalar}")).or_default() += 1;
            }
        }
    }
    let mut out = Leaves::new();
    walk(value, &mut String::new(), &mut out);
    out
}

#[derive(Default)]
struct ClassScore {
    files: u64,
    files_kept: u64,
    leaves: u64,
    kept: u64,
    added: u64,
}

#[derive(Default)]
struct Scorecard {
    files: u64,
    unreadable: u64,
    unwritable: u64,
    unrereadable: u64,
    classes: BTreeMap<&'static str, ClassScore>,
    /// (class, path with digits folded) → (files, leaves lost, an example: lost leaf → a leaf the
    /// second import has instead on that path, if any)
    lost_leaves: BTreeMap<(&'static str, String), (u64, u64, String)>,
    /// (class, path with digits folded) → (files, leaves added, an example)
    added_leaves: BTreeMap<(&'static str, String), (u64, u64, String)>,
    xml_original: u64,
    xml_kept: u64,
    /// (part kind, feature) → (files, count lost)
    lost_features: BTreeMap<(String, String), (u64, u64)>,
    part_kinds: BTreeMap<String, (u64, u64)>,
    lines: Vec<String>,
}

/// The number a leaf holds, if it holds one.
fn number_of(leaf: &str) -> Option<f64> {
    leaf.split_once('=').and_then(|(_, v)| if v.starts_with('"') { None } else { v.parse().ok() })
}

/// Folded path → (count, an example).
type Paths = BTreeMap<String, (u64, String)>;

/// How many of `left`'s leaves `right` keeps (equal, or a number [`close`] to a spare one on the
/// same path), how many it adds, what it lost and what it added.
fn compare(class: &str, left: &BTreeMap<String, u64>, right: &BTreeMap<String, u64>) -> (u64, u64, Paths, Paths) {
    let mut spare: BTreeMap<String, Vec<(f64, &str)>> = BTreeMap::new();
    let mut added = 0;
    for (leaf, m) in right {
        let extra = m.saturating_sub(left.get(leaf).copied().unwrap_or(0));
        added += extra;
        if let Some(v) = number_of(leaf) {
            for _ in 0..extra {
                spare.entry(path_of(leaf)).or_default().push((v, leaf));
            }
        }
    }
    let mut kept = 0;
    let mut lost = Paths::new();
    for (leaf, n) in left {
        let m = right.get(leaf).copied().unwrap_or(0);
        kept += m.min(*n);
        let path = path_of(leaf);
        for _ in m..*n {
            if let (Some(v), Some(list)) = (number_of(leaf), spare.get_mut(&path))
                && let Some(i) = list.iter().position(|(w, _)| close(class, v, *w))
            {
                list.swap_remove(i);
                kept += 1;
                added -= 1;
                continue;
            }
            let entry = lost.entry(fold_digits(&path)).or_default();
            entry.0 += 1;
            if entry.1.is_empty() {
                let instead = right.keys().find(|r| path_of(r) == path && !left.contains_key(*r)).map(String::as_str).unwrap_or("nothing");
                entry.1 = format!("{} → {}", clip(leaf), clip(instead));
            }
        }
    }
    let mut gained = Paths::new();
    let mut note = |leaf: &str, n: u64| {
        let entry = gained.entry(fold_digits(&path_of(leaf))).or_default();
        entry.0 += n;
        if entry.1.is_empty() {
            entry.1 = clip(leaf);
        }
    };
    for (leaf, m) in right {
        let extra = m.saturating_sub(left.get(leaf).copied().unwrap_or(0));
        if extra > 0 && number_of(leaf).is_none() {
            note(leaf, extra);
        }
    }
    for (_, leaf) in spare.values().flatten() {
        note(leaf, 1);
    }
    (kept, added, lost, gained)
}

/// `leaf`, cut to a readable length.
fn clip(leaf: &str) -> String {
    let mut out: String = leaf.chars().take(160).collect();
    if out.len() < leaf.len() {
        out.push('…');
    }
    out
}

fn path_of(leaf: &str) -> String {
    leaf.split_once('=').map(|(p, _)| p.to_owned()).unwrap_or_default()
}

impl Scorecard {
    fn file(&mut self, name: &str, bytes: &[u8]) {
        self.files += 1;
        let Ok(first) = import(bytes) else {
            self.unreadable += 1;
            self.lines.push(format!("{name}: not importable"));
            return;
        };
        let Ok(written) = export(&first) else {
            self.unwritable += 1;
            self.lines.push(format!("{name}: export failed"));
            return;
        };
        let Ok(second) = import(&written) else {
            self.unrereadable += 1;
            self.lines.push(format!("{name}: export not re-importable"));
            return;
        };
        let (a, b) = (leaves(&model_value(&first)), leaves(&model_value(&second)));
        let mut lost_classes = Vec::new();
        let empty = BTreeMap::new();
        for (class, left) in &a {
            let right = b.get(class).unwrap_or(&empty);
            let score = self.classes.entry(class).or_default();
            let total: u64 = left.values().sum();
            let (kept, added, lost_paths, added_paths) = compare(class, left, right);
            score.files += 1;
            score.leaves += total;
            score.kept += kept;
            score.added += added;
            if kept == total && added == 0 {
                score.files_kept += 1;
            } else {
                lost_classes.push(format!("{class} {kept}/{total}+{added}"));
            }
            for (paths, into) in [(lost_paths, &mut self.lost_leaves), (added_paths, &mut self.added_leaves)] {
                for (path, (n, example)) in paths {
                    let entry = into.entry((class, path)).or_default();
                    entry.0 += 1;
                    entry.1 += n;
                    if entry.2.is_empty() {
                        entry.2 = example;
                    }
                }
            }
        }
        for (class, right) in &b {
            if !a.contains_key(class) {
                let score = self.classes.entry(class).or_default();
                score.files += 1;
                score.added += right.values().sum::<u64>();
                lost_classes.push(format!("{class} 0/0+{}", right.values().sum::<u64>()));
            }
        }
        let (Ok(original), Ok(exported)) = (features(bytes), features(&written)) else {
            self.lines.push(format!("{name}: classes changed: {lost_classes:?}; package not comparable"));
            return;
        };
        let (mut total, mut kept) = (0u64, 0u64);
        for (kind, counts) in &original {
            let theirs = exported.get(kind);
            let kind_total: u64 = counts.values().sum();
            let mut kind_kept = 0;
            for (feature, n) in counts {
                let m = theirs.and_then(|t| t.get(feature)).copied().unwrap_or(0);
                kind_kept += m.min(*n);
                if m < *n {
                    let entry = self.lost_features.entry((kind.clone(), feature.clone())).or_default();
                    entry.0 += 1;
                    entry.1 += n - m;
                }
            }
            let entry = self.part_kinds.entry(kind.clone()).or_default();
            entry.0 += kind_total;
            entry.1 += kind_kept;
            total += kind_total;
            kept += kind_kept;
        }
        self.xml_original += total;
        self.xml_kept += kept;
        self.lines.push(format!("{name}: xml {:.1}%; classes changed: {lost_classes:?}", percent(kept, total)));
    }

    fn json(&self) -> Value {
        let classes: serde_json::Map<String, Value> = self
            .classes
            .iter()
            .map(|(class, s)| {
                ((*class).to_owned(), json!({ "files": s.files, "filesKept": s.files_kept, "leaves": s.leaves, "kept": s.kept, "added": s.added }))
            })
            .collect();
        json!({
            "kit": KIT,
            "files": self.files,
            "unreadable": self.unreadable,
            "unwritable": self.unwritable,
            "unrereadable": self.unrereadable,
            "classes": classes,
            "xml": { "original": self.xml_original, "kept": self.xml_kept },
        })
    }

    fn report(&self) -> String {
        let mut out = String::new();
        let ok = self.files - self.unreadable - self.unwritable - self.unrereadable;
        out.push_str(&format!(
            "{KIT} fidelity: {} files, {ok} round-tripped ({} unreadable, {} export failed, {} export unreadable)\n\n",
            self.files, self.unreadable, self.unwritable, self.unrereadable
        ));
        out.push_str("class                        files kept   leaves kept      added\n");
        for (class, s) in &self.classes {
            out.push_str(&format!(
                "{class:<28} {:>4}/{:<4}  {:>6.2}% of {:<9} {}\n",
                s.files_kept,
                s.files,
                percent(s.kept, s.leaves),
                s.leaves,
                s.added
            ));
        }
        out.push_str(&format!(
            "\nXML retained: {:.2}% of {} elements and attributes\n",
            percent(self.xml_kept, self.xml_original),
            self.xml_original
        ));
        let mut kinds: Vec<_> = self.part_kinds.iter().collect();
        kinds.sort_by_key(|(_, (total, kept))| std::cmp::Reverse(total - kept));
        out.push_str("\npart kinds by features lost:\n");
        for (kind, (total, kept)) in kinds.iter().take(25) {
            out.push_str(&format!("  {kind:<50} {:>9} lost of {:<9} ({:.1}% kept)\n", total - kept, total, percent(*kept, *total)));
        }
        let mut lost: Vec<_> = self.lost_features.iter().collect();
        lost.sort_by_key(|(_, (files, count))| std::cmp::Reverse((*files, *count)));
        out.push_str("\ntop lost XML features by files (files, count):\n");
        for ((kind, feature), (files, count)) in lost.iter().take(150) {
            out.push_str(&format!("  {files:>3} {count:>8}  {kind} {feature}\n"));
        }
        lost.sort_by_key(|(_, (files, count))| std::cmp::Reverse((*count, *files)));
        out.push_str("\ntop lost XML features by count (files, count):\n");
        for ((kind, feature), (files, count)) in lost.iter().take(150) {
            out.push_str(&format!("  {files:>3} {count:>8}  {kind} {feature}\n"));
        }
        let mut lost: Vec<_> = self.lost_leaves.iter().collect();
        lost.sort_by_key(|(_, (files, count, _))| std::cmp::Reverse((*files, *count)));
        out.push_str("\ntop lost model leaves (files, count, example):\n");
        for ((class, path), (files, count, example)) in lost.iter().take(300) {
            out.push_str(&format!("  {files:>3} {count:>8}  [{class}] {path}\n               e.g. {example}\n"));
        }
        let mut added: Vec<_> = self.added_leaves.iter().collect();
        added.sort_by_key(|(_, (files, count, _))| std::cmp::Reverse((*files, *count)));
        out.push_str("\ntop added model leaves (files, count, example):\n");
        for ((class, path), (files, count, example)) in added.iter().take(100) {
            out.push_str(&format!("  {files:>3} {count:>8}  [{class}] {path}\n               e.g. {example}\n"));
        }
        out.push_str("\nper file:\n");
        for line in &self.lines {
            out.push_str(&format!("  {line}\n"));
        }
        out
    }
}

fn percent(part: u64, whole: u64) -> f64 {
    if whole == 0 { 100.0 } else { part as f64 * 100.0 / whole as f64 }
}

/// The regressions of `now` against `baseline`, both [`Scorecard::json`] values.
fn regressions(now: &Value, baseline: &Value) -> Vec<String> {
    let mut out = Vec::new();
    let number = |v: &Value, path: &[&str]| path.iter().fold(v, |v, key| &v[*key]).as_u64().unwrap_or(0);
    for key in ["unreadable", "unwritable", "unrereadable"] {
        if number(now, &[key]) > number(baseline, &[key]) {
            out.push(format!("{key}: {} > {}", number(now, &[key]), number(baseline, &[key])));
        }
    }
    let ratio = |kept: u64, total: u64| if total == 0 { 1.0 } else { kept as f64 / total as f64 };
    if let Some(classes) = baseline["classes"].as_object() {
        for class in classes.keys() {
            let (b, n) = (&baseline["classes"][class], &now["classes"][class]);
            if number(n, &["filesKept"]) < number(b, &["filesKept"]) {
                out.push(format!("{class}: {} files keep it, baseline {}", number(n, &["filesKept"]), number(b, &["filesKept"])));
            }
            // More leaves lost than the class grew by: a model that learns a property may also
            // count it on objects that were lost already, which is not a regression.
            let lost = |v: &Value| number(v, &["leaves"]).saturating_sub(number(v, &["kept"]));
            let grown = number(n, &["leaves"]).saturating_sub(number(b, &["leaves"]));
            if lost(n) > lost(b) + grown {
                out.push(format!("{class}: {} leaves lost, baseline {} (+{grown} leaves)", lost(n), lost(b)));
            }
        }
    }
    let (rb, rn) = (
        ratio(number(baseline, &["xml", "kept"]), number(baseline, &["xml", "original"])),
        ratio(number(now, &["xml", "kept"]), number(now, &["xml", "original"])),
    );
    if rn + 1e-9 < rb {
        out.push(format!("XML retained {:.4}%, baseline {:.4}%", rn * 100.0, rb * 100.0));
    }
    out
}

#[test]
fn corpus_fidelity() {
    let Ok(dir) = std::env::var(CORPUS_VAR) else { return };
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .unwrap_or_else(|error| panic!("{dir}: {error}"))
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| EXTENSIONS.iter().any(|e| ext.eq_ignore_ascii_case(e))))
        // macOS keeps resource forks of copied files as `._<name>` beside them: not documents.
        .filter(|path| !path.file_name().is_some_and(|n| n.to_string_lossy().starts_with("._")))
        .collect();
    files.sort();
    let mut card = Scorecard::default();
    for path in &files {
        let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let bytes = std::fs::read(path).unwrap_or_else(|error| panic!("{name}: {error}"));
        let Some(svg) = source(&bytes) else {
            card.files += 1;
            card.unreadable += 1;
            card.lines.push(format!("{name}: not an SVG or a drawing"));
            continue;
        };
        card.file(&name, &svg);
    }
    let report = card.report();
    let summary: String = report.split("\npart kinds").next().unwrap_or_default().to_owned();
    eprintln!("{summary}");
    let now = card.json();
    if let Ok(out) = std::env::var("KITION_FIDELITY_OUT") {
        let _ = std::fs::create_dir_all(&out);
        std::fs::write(format!("{out}/{KIT}-fidelity.txt"), &report).unwrap_or_else(|error| panic!("{out}: {error}"));
        let pretty = serde_json::to_string_pretty(&now).unwrap_or_default();
        std::fs::write(format!("{out}/{KIT}-fidelity.json"), pretty).unwrap_or_else(|error| panic!("{out}: {error}"));
    }
    if let Ok(path) = std::env::var("KITION_FIDELITY_BASELINE") {
        let text = std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{path}: {error}"));
        let baseline: Value = serde_json::from_str(&text).unwrap_or_else(|error| panic!("{path}: {error}"));
        let regressed = regressions(&now, &baseline);
        assert!(regressed.is_empty(), "fidelity regressed against {path}:\n{}", regressed.join("\n"));
    }
}

#[test]
fn scorer_passes_a_lossless_round_trip_and_flags_a_regression() {
    let doc = synthetic();
    let bytes = export(&doc).unwrap_or_else(|error| panic!("export: {error}"));
    let mut card = Scorecard::default();
    card.file("synthetic", &bytes);
    assert_eq!(card.unreadable + card.unwritable + card.unrereadable, 0, "{}", card.report());
    assert!(card.classes.values().all(|s| s.files_kept == s.files), "{}", card.report());
    assert!(card.classes.contains_key(PRIMARY_CLASS), "{}", card.report());
    assert!(card.xml_original > 0 && card.xml_kept == card.xml_original, "{}", card.report());
    // A baseline of this run accepts it; one that kept more flags it.
    let now = card.json();
    assert!(regressions(&now, &now).is_empty());
    let mut better = now.clone();
    better["xml"]["kept"] = json!(card.xml_kept + 1);
    better["xml"]["original"] = json!(card.xml_original);
    assert_eq!(regressions(&now, &better).len(), 1);
}
