//! What [`KitDocument::check`](kition_kit::KitDocument::check) reports on a drawing (KHP
//! A21): images whose pixels the drawing does not hold, linked files that were not read (outside
//! the workspace, or missing), symbol instances and graphic styles
//! that name nothing, art that lies off every artboard, empty groups and text, stray points,
//! characters no font here can draw, and fonts drawn with a substitute. Objects are named as `object:<id>` (the ids the engine's
//! commands take in `ids`).
//!
//! Codes: `vector.no-artboard`, `vector.missing-image`, `vector.missing-symbol`,
//! `vector.missing-graphic-style`, `vector.missing-link`, `vector.off-artboard`, `vector.missing-glyphs`, `vector.font-substituted`,
//! `vector.empty-group`, `vector.empty-text`, `vector.stray-point`, and `vector.truncated` when a
//! code is found more often than [`PER_CODE`].

use std::collections::{BTreeMap, BTreeSet};

use kition_kit::diagnostics::{Diagnostic, Severity, Target};
use kition_vector_doc::{Document, Node, NodeKind};
use kition_vector_geom::Rect;
use serde_json::json;

/// At most this many diagnostics of one code; the rest are counted in one `vector.truncated`.
pub const PER_CODE: usize = 100;

/// One code the check reports: its severity and a one-sentence summary for the catalogue.
pub struct Entry {
    pub code: &'static str,
    pub severity: Severity,
    pub summary: &'static str,
}

const fn entry(code: &'static str, severity: Severity, summary: &'static str) -> Entry {
    Entry { code, severity, summary }
}

/// Every code [`check`] reports, errors first: the catalogue shipped as `agent/checks.json` (KHP
/// A38), with one conformance fixture per code (`conformance/check-<name>.kvector`).
pub const CATALOGUE: &[Entry] = &[
    entry("vector.missing-image", Severity::Error, "An image shows pixels the drawing does not hold, so it draws nothing."),
    entry("vector.missing-symbol", Severity::Error, "A symbol instance names a symbol the drawing does not define, so it draws nothing."),
    entry("vector.no-artboard", Severity::Warning, "The drawing has no artboard, so nothing is placed on a page for export or print."),
    entry("vector.missing-graphic-style", Severity::Warning, "An object is linked to a graphic style the drawing no longer has."),
    entry(
        "vector.missing-link",
        Severity::Warning,
        "A linked file was not read, because it is missing or lies outside the workspace, so the drawing shows its saved preview or a placeholder.",
    ),
    entry("vector.off-artboard", Severity::Warning, "Art lies outside every artboard, so exports and prints leave it out."),
    entry(
        "vector.missing-glyphs",
        Severity::Warning,
        "Text has characters no font here can draw, so they are left out until a font that has them is added.",
    ),
    entry("vector.font-substituted", Severity::Info, "A text font is not available here, so its text is drawn with a substitute."),
    entry("vector.empty-group", Severity::Info, "A group holds nothing; removing it changes nothing on the artboard."),
    entry("vector.empty-text", Severity::Info, "A text object has no text; removing it changes nothing on the artboard."),
    entry("vector.stray-point", Severity::Info, "A path with a single point paints nothing; removing it changes nothing on the artboard."),
    entry("vector.truncated", Severity::Info, "One code was found more often than the check lists; the rest are counted in one diagnostic."),
];

/// The catalogue as `agent/checks.json` holds it: code, severity, summary, and its fixture.
pub fn catalogue_json() -> serde_json::Value {
    serde_json::Value::Array(
        CATALOGUE
            .iter()
            .map(|entry| {
                json!({
                    "code": entry.code,
                    "severity": entry.severity,
                    "summary": entry.summary,
                    "fixture": format!("check-{}.kvector", entry.code.trim_start_matches("vector.")),
                })
            })
            .collect(),
    )
}

/// Everything worth fixing in the drawing, errors first. `has_font` says whether a family can be
/// drawn here and `can_draw` whether some font here has a character (the engine's font database).
pub fn check(doc: &Document, has_font: &mut dyn FnMut(&str) -> bool, can_draw: &mut dyn FnMut(char) -> bool) -> Vec<Diagnostic> {
    let mut found = Found::default();
    if doc.artboards.is_empty() {
        found.push(Diagnostic::new(
            "vector.no-artboard",
            Severity::Warning,
            "The drawing has no artboard, so nothing is placed on a page for export or print.",
        ));
    }
    let symbols: BTreeSet<&str> = doc.symbols.iter().map(|symbol| symbol.name.as_str()).collect();
    let styles: BTreeSet<u32> = doc.graphic_styles.iter().map(|style| style.id).filter(|id| *id != 0).collect();
    let mut fonts: BTreeMap<String, u64> = BTreeMap::new();
    // The characters of each text object, for the glyphs no font here has.
    let mut texts: Vec<(u64, String, String)> = Vec::new();
    doc.walk(|node| {
        let id = node.id.0;
        let name = label(node);
        match &node.kind {
            NodeKind::Image(image) => {
                let held = doc
                    .images
                    .get(&image.key)
                    .is_some_and(|blob| !blob.bytes.is_empty() || blob.proxy.as_ref().is_some_and(|proxy| !proxy.is_empty()));
                if !held {
                    found.push(
                        Diagnostic::new(
                            "vector.missing-image",
                            Severity::Error,
                            format!("{name} shows an image the drawing does not hold, so it draws nothing."),
                        )
                        .on(on(id)),
                    );
                }
            }
            NodeKind::SymbolInstance { symbol, .. } if !symbols.contains(symbol.as_str()) => found.push(
                Diagnostic::new(
                    "vector.missing-symbol",
                    Severity::Error,
                    format!("{name} is an instance of the symbol \"{symbol}\", which the drawing does not define."),
                )
                .on(on(id)),
            ),
            NodeKind::Group { children, .. } if children.is_empty() => found.push(
                Diagnostic::new("vector.empty-group", Severity::Info, format!("{name} is an empty group."))
                    .on(on(id))
                    .fix("edit.clear", Some(json!({ "ids": [id] }))),
            ),
            NodeKind::Text(text) => {
                if text.runs.iter().all(|run| run.text.trim().is_empty() && run.inline.is_none()) {
                    found.push(
                        Diagnostic::new("vector.empty-text", Severity::Info, format!("{name} is a text object without text."))
                            .on(on(id))
                            .fix("edit.clear", Some(json!({ "ids": [id] }))),
                    );
                }
                // One diagnostic per family, on the first object that uses it.
                for run in &text.runs {
                    fonts.entry(run.style.font_family.clone()).or_insert(id);
                }
                texts.push((id, name.clone(), text.runs.iter().map(|run| run.text.as_str()).collect()));
            }
            NodeKind::Path { guide: false, .. } if node.geometric_bounds().is_some_and(|bounds| bounds.width() == 0.0 && bounds.height() == 0.0) => {
                found.push(
                    Diagnostic::new("vector.stray-point", Severity::Info, format!("{name} is a path with a single point: it paints nothing."))
                        .on(on(id))
                        .fix("edit.clear", Some(json!({ "ids": [id] }))),
                )
            }
            _ => {}
        }
        if let Some(style) = node.graphic_style
            && style != 0
            && !styles.contains(&style)
        {
            found.push(
                Diagnostic::new(
                    "vector.missing-graphic-style",
                    Severity::Warning,
                    format!("{name} is linked to a graphic style the drawing no longer has."),
                )
                .on(on(id)),
            );
        }
    });
    // Links the host did not lend (KHP A42): the link stays as data, its file unread.
    for link in kition_vector_engine::cmd::links::report(doc) {
        if link["exists"] == json!(false) {
            let target = link["target"].as_str().unwrap_or_default();
            let id = link["id"].as_str().and_then(|id| id.strip_prefix("object:")).and_then(|id| id.parse::<u64>().ok());
            let name = id.and_then(|id| doc.node(kition_vector_doc::NodeId(id))).map_or_else(|| "A linked object".to_owned(), label);
            let mut diagnostic = Diagnostic::new(
                "vector.missing-link",
                Severity::Warning,
                format!(
                    "{name} links to \"{target}\", which was not read: it is missing, or not a path inside the workspace relative to the drawing. With the file in the workspace beside the drawing, links.update reads it; links.list lists every link."
                ),
            );
            if let Some(id) = id {
                diagnostic = diagnostic.on(on(id));
            }
            found.push(diagnostic);
        }
    }
    off_artboard(doc, &mut found);
    for (id, name, characters) in texts {
        let mut missing: Vec<char> = characters.chars().filter(|c| !can_draw(*c)).collect();
        missing.sort_unstable();
        missing.dedup();
        if !missing.is_empty() {
            let shown: String = missing.iter().take(8).collect();
            found.push(
                Diagnostic::new(
                    "vector.missing-glyphs",
                    Severity::Warning,
                    format!("{name} has {} characters no font here can draw ({shown}); add a font that has them.", missing.len()),
                )
                .on(on(id)),
            );
        }
    }
    for (family, id) in fonts {
        if !family.is_empty() && !has_font(&family) {
            found.push(
                Diagnostic::new(
                    "vector.font-substituted",
                    Severity::Info,
                    format!("The font \"{family}\" is not available here, so its text is drawn with a substitute."),
                )
                .on(on(id)),
            );
        }
    }
    found.finish()
}

#[derive(Default)]
struct Found {
    list: Vec<Diagnostic>,
    counts: BTreeMap<String, usize>,
}

impl Found {
    fn push(&mut self, diagnostic: Diagnostic) {
        let count = self.counts.entry(diagnostic.code.clone()).or_default();
        *count += 1;
        if *count <= PER_CODE {
            self.list.push(diagnostic);
        }
    }

    fn finish(mut self) -> Vec<Diagnostic> {
        for (code, count) in &self.counts {
            if *count > PER_CODE {
                self.list.push(Diagnostic::new(
                    "vector.truncated",
                    Severity::Info,
                    format!("{} more `{code}` diagnostics are not listed.", count - PER_CODE),
                ));
            }
        }
        self.list.sort_by_key(|diagnostic| diagnostic.severity);
        self.list
    }
}

fn on(id: u64) -> Target {
    Target { path: Some(format!("object:{id}")), ..Target::default() }
}

fn label(node: &Node) -> String {
    match node.name.as_deref().filter(|name| !name.trim().is_empty()) {
        Some(name) => format!("\"{name}\""),
        None => format!("Object {}", node.id.0),
    }
}

/// Whether `a` and `b` share some area (touching edges do not count).
fn overlaps(a: Rect, b: Rect) -> bool {
    a.x0 < b.x1 && b.x0 < a.x1 && a.y0 < b.y1 && b.y0 < a.y1
}

/// Visible art directly inside a layer (or a sublayer) that lies off every artboard. Objects
/// inside groups move with their group, so only the outermost one is reported.
fn off_artboard(doc: &Document, found: &mut Found) {
    if doc.artboards.is_empty() {
        return;
    }
    fn visit(nodes: &[std::sync::Arc<Node>], doc: &Document, found: &mut Found) {
        for node in nodes {
            if !node.visible {
                continue;
            }
            match &node.kind {
                NodeKind::Layer { children, .. } => visit(children, doc, found),
                NodeKind::Path { guide: true, .. } => {}
                _ => {
                    let Some(bounds) = node.visual_bounds() else { continue };
                    if !doc.artboards.iter().any(|board| overlaps(bounds, board.rect)) {
                        found.push(
                            Diagnostic::new(
                                "vector.off-artboard",
                                Severity::Warning,
                                format!("{} lies outside every artboard, so exports and prints leave it out.", label(node)),
                            )
                            .on(on(node.id.0)),
                        );
                    }
                }
            }
        }
    }
    visit(&doc.layers, doc, found);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::VectorDocument;
    use kition_kit::KitDocument;

    fn drawing() -> VectorDocument {
        VectorDocument::create("Drawing.kvector", 200, 100).expect("drawing")
    }

    fn doc_of(drawing: &VectorDocument) -> &Document {
        &drawing.session.active().expect("open").doc
    }

    fn doc_mut(drawing: &mut VectorDocument) -> &mut Document {
        std::sync::Arc::make_mut(&mut drawing.session.active_mut().expect("open").doc)
    }

    fn codes(drawing: &VectorDocument) -> Vec<String> {
        check(doc_of(drawing), &mut |_| true, &mut |_| true).into_iter().map(|d| d.code).collect()
    }

    fn rect(drawing: &mut VectorDocument, x: f64, y: f64) -> u64 {
        let made = drawing.execute("shape.rectangle", json!({ "x": x, "y": y, "width": 20, "height": 10 })).expect("rectangle");
        made["id"].as_u64().expect("id")
    }

    #[test]
    fn a_drawing_on_its_artboard_is_clean() {
        let mut drawing = drawing();
        rect(&mut drawing, 20.0, 10.0);
        assert_eq!(codes(&drawing), Vec::<String>::new());
        assert_eq!(drawing.check(), Vec::new());
    }

    #[test]
    fn reports_art_off_every_artboard() {
        let mut drawing = drawing();
        let id = rect(&mut drawing, 500.0, 10.0);
        let found = check(doc_of(&drawing), &mut |_| true, &mut |_| true);
        assert_eq!(found.iter().map(|d| d.code.as_str()).collect::<Vec<_>>(), ["vector.off-artboard"]);
        assert_eq!(found[0].target.path.as_deref(), Some(format!("object:{id}").as_str()));
        doc_mut(&mut drawing).artboards.clear();
        assert_eq!(codes(&drawing), ["vector.no-artboard"]);
    }

    #[test]
    fn reports_missing_images_symbols_and_styles_errors_first() {
        let mut drawing = drawing();
        let id = rect(&mut drawing, 20.0, 10.0);
        let doc = doc_mut(&mut drawing);
        let node = doc.node(kition_vector_doc::NodeId(id)).expect("rectangle").clone();
        let mut styled = node.clone();
        styled.id = kition_vector_doc::NodeId(id + 1000);
        styled.graphic_style = Some(77);
        let mut instance = node.clone();
        instance.id = kition_vector_doc::NodeId(id + 1001);
        instance.kind = NodeKind::SymbolInstance { symbol: "Star".into(), xf: kition_vector_geom::Affine::IDENTITY };
        let mut image = node.clone();
        image.id = kition_vector_doc::NodeId(id + 1002);
        image.kind = NodeKind::Image(kition_vector_doc::ImageObject {
            key: "gone".into(),
            width: 10,
            height: 10,
            xf: kition_vector_geom::Affine::IDENTITY,
            link: None,
            placement: Default::default(),
        });
        let NodeKind::Layer { children, .. } = &mut std::sync::Arc::make_mut(&mut doc.layers[0]).kind else { panic!("a layer") };
        children.extend([styled, instance, image].map(std::sync::Arc::new));
        let found = check(doc_of(&drawing), &mut |_| true, &mut |_| true);
        let codes: Vec<_> = found.iter().map(|d| (d.code.as_str(), d.severity)).collect();
        assert_eq!(
            codes,
            [
                ("vector.missing-symbol", Severity::Error),
                ("vector.missing-image", Severity::Error),
                ("vector.missing-graphic-style", Severity::Warning)
            ]
        );
        kition_kit::diagnostics::check_diagnostics(&found).expect("valid diagnostics");
    }

    #[test]
    fn reports_empty_groups_stray_points_and_missing_fonts() {
        let mut drawing = drawing();
        let id = rect(&mut drawing, 20.0, 10.0);
        let doc = doc_mut(&mut drawing);
        let node = doc.node(kition_vector_doc::NodeId(id)).expect("rectangle").clone();
        let mut group = node.clone();
        group.id = kition_vector_doc::NodeId(id + 1000);
        group.kind = NodeKind::Group { children: Vec::new(), clip: false };
        let mut point = node.clone();
        point.id = kition_vector_doc::NodeId(id + 1001);
        if let NodeKind::Path { path, .. } = &mut point.kind {
            let mut bez = kition_vector_geom::BezPath::new();
            bez.move_to((30.0, 30.0));
            *path = kition_vector_geom::PathData::from_bezpath(&bez);
        }
        let NodeKind::Layer { children, .. } = &mut std::sync::Arc::make_mut(&mut doc.layers[0]).kind else { panic!("a layer") };
        children.extend([group, point].map(std::sync::Arc::new));
        let mut found = codes(&drawing);
        found.sort();
        assert_eq!(found, ["vector.empty-group", "vector.stray-point"]);

        let mut drawing = self::drawing();
        drawing.execute("text.create", json!({ "x": 20, "y": 40, "text": "Hello" })).expect("text");
        assert_eq!(codes(&drawing), Vec::<String>::new());
        let missing: Vec<_> = check(doc_of(&drawing), &mut |_| false, &mut |_| true).into_iter().map(|d| d.code).collect();
        assert_eq!(missing, ["vector.font-substituted"]);
    }

    #[test]
    fn caps_each_code_and_says_how_many_more() {
        let mut drawing = drawing();
        let id = rect(&mut drawing, 20.0, 10.0);
        let doc = doc_mut(&mut drawing);
        let node = doc.node(kition_vector_doc::NodeId(id)).expect("rectangle").clone();
        let groups: Vec<_> = (0..PER_CODE as u64 + 3)
            .map(|n| {
                let mut group = node.clone();
                group.id = kition_vector_doc::NodeId(id + 1000 + n);
                group.kind = NodeKind::Group { children: Vec::new(), clip: false };
                std::sync::Arc::new(group)
            })
            .collect();
        let NodeKind::Layer { children, .. } = &mut std::sync::Arc::make_mut(&mut doc.layers[0]).kind else { panic!("a layer") };
        children.extend(groups);
        let found = check(doc_of(&drawing), &mut |_| true, &mut |_| true);
        assert_eq!(found.iter().filter(|d| d.code == "vector.empty-group").count(), PER_CODE);
        let truncated: Vec<_> = found.iter().filter(|d| d.code == "vector.truncated").collect();
        assert_eq!(truncated.len(), 1);
        assert!(truncated[0].message.starts_with("3 more `vector.empty-group`"));
    }
}
