//! The KHP conformance fixtures (`conformance/`) and the check catalogue (`agent/checks.json`,
//! from [`crate::check::CATALOGUE`]): `basic.kvector` (no diagnostics), `edit.json` (steps that
//! change it), one `check-<code>.kvector` per catalogue code, and `expected/` previews of `basic`.
//! Each fixture starts from a new drawing made with the engine's own commands; the broken
//! references a file can hold but the commands never make (an image without its pixels, a symbol or
//! a graphic style the drawing lost, a font that is not installed) are made by editing the
//! document, as a file damaged elsewhere would be.
//!
//! `KITION_WRITE_CONFORMANCE=1 cargo test -p kition-vector-kit conformance` writes them; without
//! the variable the test only checks that every fixture it would write reports its code.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use kition_kit::KitDocument;
use kition_vector_doc::{Document, ImageObject, Node, NodeId, NodeKind};
use serde_json::{Value, json};

use crate::VectorDocument;
use crate::check::{CATALOGUE, PER_CODE, catalogue_json};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn drawing() -> VectorDocument {
    VectorDocument::create("Drawing.kvector", 200, 100).expect("drawing")
}

fn run(drawing: &mut VectorDocument, command: &str, params: Value) -> Value {
    drawing.execute(command, params).unwrap_or_else(|error| panic!("{command}: {error}"))
}

fn rectangle(drawing: &mut VectorDocument, x: f64, y: f64) -> u64 {
    run(drawing, "shape.rectangle", json!({ "x": x, "y": y, "width": 40, "height": 24 }))["id"].as_u64().expect("id")
}

fn doc_mut(drawing: &mut VectorDocument) -> &mut Document {
    Arc::make_mut(&mut drawing.session.active_mut().expect("open").doc)
}

/// A copy of object `id` under a new id with `kind`, added to the first layer.
fn add_like(drawing: &mut VectorDocument, id: u64, offset: u64, edit: impl FnOnce(&mut Node)) {
    let doc = doc_mut(drawing);
    let mut node = doc.node(NodeId(id)).expect("object").clone();
    node.id = NodeId(id + offset);
    edit(&mut node);
    let NodeKind::Layer { children, .. } = &mut Arc::make_mut(&mut doc.layers[0]).kind else { panic!("a layer") };
    children.push(Arc::new(node));
}

/// The drawing every fixture starts from: one rectangle on its artboard.
fn basic() -> VectorDocument {
    let mut drawing = drawing();
    run(&mut drawing, "shape.rectangle", json!({ "x": 20, "y": 20, "width": 80, "height": 50 }));
    run(&mut drawing, "shape.ellipse", json!({ "x": 120, "y": 25, "width": 50, "height": 50 }));
    drawing
}

fn fixture(code: &str) -> VectorDocument {
    let mut drawing = basic();
    match code {
        "vector.missing-image" => {
            let id = rectangle(&mut drawing, 30.0, 30.0);
            add_like(&mut drawing, id, 1000, |node| {
                node.kind = NodeKind::Image(ImageObject {
                    key: "lost-picture".into(),
                    width: 10,
                    height: 10,
                    xf: kition_vector_geom::Affine::IDENTITY,
                    link: None,
                    placement: Default::default(),
                });
            });
        }
        "vector.missing-symbol" => {
            let id = rectangle(&mut drawing, 30.0, 30.0);
            add_like(&mut drawing, id, 1000, |node| {
                node.kind = NodeKind::SymbolInstance { symbol: "Lost Star".into(), xf: kition_vector_geom::Affine::IDENTITY };
            });
        }
        "vector.no-artboard" => doc_mut(&mut drawing).artboards.clear(),
        "vector.missing-link" => {
            // A picture linked to a file that is not in the workspace: the drawing keeps its pixels.
            let picture = image::RgbaImage::from_pixel(8, 8, image::Rgba([40, 120, 200, 255]));
            let mut png = std::io::Cursor::new(Vec::new());
            picture.write_to(&mut png, image::ImageFormat::Png).expect("png");
            let data = kition_vector_format::base64_encode(&png.into_inner());
            let placed = run(&mut drawing, "file.place", json!({ "name": "picture.png", "dataBase64": data, "at": [60, 50] }));
            let id = placed["ids"][0].as_u64().expect("placed");
            let doc = doc_mut(&mut drawing);
            let mut node = doc.node(NodeId(id)).expect("image").clone();
            let NodeKind::Image(image) = &mut node.kind else { panic!("an image") };
            image.link = Some(kition_vector_doc::LinkInfo::new("lost/picture.png".to_owned()));
            let NodeKind::Layer { children, .. } = &mut Arc::make_mut(&mut doc.layers[0]).kind else { panic!("a layer") };
            for child in children.iter_mut() {
                if child.id.0 == id {
                    *child = Arc::new(node.clone());
                }
            }
        }
        "vector.missing-graphic-style" => {
            let id = rectangle(&mut drawing, 30.0, 30.0);
            add_like(&mut drawing, id, 1000, |node| node.graphic_style = Some(77));
        }
        "vector.off-artboard" => {
            rectangle(&mut drawing, 600.0, 20.0);
        }
        "vector.font-substituted" => {
            let id = run(&mut drawing, "text.create", json!({ "x": 20, "y": 90, "text": "Hello" }))["id"].as_u64().expect("id");
            let doc = doc_mut(&mut drawing);
            let mut node = doc.node(NodeId(id)).expect("text").clone();
            let NodeKind::Text(text) = &mut node.kind else { panic!("text") };
            for run in &mut text.runs {
                run.style.font_family = "Kition Missing Sans".into();
            }
            let NodeKind::Layer { children, .. } = &mut Arc::make_mut(&mut doc.layers[0]).kind else { panic!("a layer") };
            for child in children.iter_mut() {
                if child.id.0 == id {
                    *child = Arc::new(node.clone());
                }
            }
        }
        "vector.missing-glyphs" => {
            // Ethiopic: no bundled font has it.
            run(&mut drawing, "text.create", json!({ "x": 20, "y": 90, "text": "ሰላም" }));
        }
        "vector.empty-group" => {
            let id = rectangle(&mut drawing, 30.0, 30.0);
            add_like(&mut drawing, id, 1000, |node| node.kind = NodeKind::Group { children: Vec::new(), clip: false });
        }
        "vector.empty-text" => {
            run(&mut drawing, "text.create", json!({ "x": 20, "y": 90, "text": " " }));
        }
        "vector.stray-point" => {
            let id = rectangle(&mut drawing, 30.0, 30.0);
            add_like(&mut drawing, id, 1000, |node| {
                if let NodeKind::Path { path, .. } = &mut node.kind {
                    let mut bez = kition_vector_geom::BezPath::new();
                    bez.move_to((60.0, 60.0));
                    *path = kition_vector_geom::PathData::from_bezpath(&bez);
                }
            });
        }
        "vector.truncated" => {
            let id = rectangle(&mut drawing, 30.0, 30.0);
            for n in 0..=PER_CODE as u64 {
                add_like(&mut drawing, id, 1000 + n, |node| node.kind = NodeKind::Group { children: Vec::new(), clip: false });
            }
        }
        other => panic!("no fixture for {other}"),
    }
    drawing
}

fn edit() -> Value {
    json!({ "steps": [
        { "command": "shape.rectangle", "params": { "x": 40, "y": 60, "width": 30, "height": 20 } },
    ] })
}

fn png(drawing: &VectorDocument, scale: f64) -> Vec<u8> {
    let (width, height) = ((f64::from(drawing.width()) * scale) as u32, (f64::from(drawing.height()) * scale) as u32);
    let rgba = drawing.render(0.0, 0.0, scale, width, height).expect("render");
    let picture = image::RgbaImage::from_raw(width, height, rgba).expect("pixels");
    let mut out = std::io::Cursor::new(Vec::new());
    picture.write_to(&mut out, image::ImageFormat::Png).expect("png");
    out.into_inner()
}

fn codes(drawing: &VectorDocument) -> Vec<String> {
    drawing.check().into_iter().map(|diagnostic| diagnostic.code).collect()
}

#[test]
fn every_fixture_reports_its_code_and_basic_is_clean() {
    let write = std::env::var_os("KITION_WRITE_CONFORMANCE").is_some();
    let out = repo().join("conformance");
    let basic = basic();
    assert_eq!(codes(&basic), Vec::<String>::new(), "basic.kvector is clean");
    if write {
        std::fs::create_dir_all(out.join("expected")).expect("conformance/expected");
        std::fs::create_dir_all(repo().join("agent")).expect("agent");
        std::fs::write(out.join("basic.kvector"), basic.save(None).expect("save")).expect("basic");
        std::fs::write(out.join("edit.json"), format!("{:#}\n", edit())).expect("edit.json");
        std::fs::write(out.join("expected/basic.png"), png(&basic, 1.0)).expect("expected preview");
        std::fs::write(out.join("expected/basic-scale-2.png"), png(&basic, 2.0)).expect("expected preview scale 2");
        let tolerance = json!({
            "maxChannelDelta": 2,
            "maxDifferentPixels": 100,
            "note": "The engine's own renderer draws the preview; anti-aliased edges may differ by a step between platforms, on at most 0.5% of the 200 x 100 picture's pixels."
        });
        std::fs::write(out.join("expected/tolerance.json"), format!("{tolerance:#}\n")).expect("tolerance");
        std::fs::write(repo().join("agent/checks.json"), format!("{:#}\n", catalogue_json())).expect("checks.json");
    }
    for entry in CATALOGUE {
        let drawing = fixture(entry.code);
        let bytes = drawing.save(None).expect("save");
        let reopened = VectorDocument::open("fixture.kvector", &bytes).expect("reopen");
        assert!(codes(&reopened).iter().any(|code| code == entry.code), "{}: {:?}", entry.code, codes(&reopened));
        if write {
            std::fs::write(out.join(format!("check-{}.kvector", entry.code.trim_start_matches("vector."))), bytes).expect("fixture");
        }
    }
}
