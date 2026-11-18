//! Regression: the Arc warp bends a shape into an arc of its own
//! proportions (a 400 × 100 band stays about as wide, its height the band's thickness) and a
//! positive bend arches it upward, as Arch and Arc Upper do.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

use kition_kit::KitDocument;
use kition_vector_kit::VectorDocument;
use serde_json::{Value, json};

fn run(doc: &mut VectorDocument, command: &str, params: Value) -> Value {
    doc.execute(command, params).unwrap_or_else(|error| panic!("{command}: {error}"))
}

/// The topmost inked row of each column `x` (document pixels), and the inked bounds.
fn ink(doc: &VectorDocument) -> (Vec<Option<usize>>, [usize; 4]) {
    let (w, h) = (doc.width() as usize, doc.height() as usize);
    let px = doc.render(0.0, 0.0, 1.0, w as u32, h as u32).unwrap();
    let dark = |x: usize, y: usize| px[(y * w + x) * 4 + 3] > 128 && px[(y * w + x) * 4] < 100;
    let tops = (0..w).map(|x| (0..h).find(|y| dark(x, *y))).collect();
    let mut b = [usize::MAX, usize::MAX, 0, 0];
    for y in 0..h {
        for x in 0..w {
            if dark(x, y) {
                b = [b[0].min(x), b[1].min(y), b[2].max(x), b[3].max(y)];
            }
        }
    }
    (tops, b)
}

fn warped(style: &str, bend: i32) -> VectorDocument {
    let mut doc = VectorDocument::create("band.kvector", 800, 600).unwrap();
    run(&mut doc, "shape.rectangle", json!({ "x": 200, "y": 250, "width": 400, "height": 100 }));
    run(&mut doc, "paint.setFill", json!({ "color": "#1d3557" }));
    run(&mut doc, "paint.setStroke", json!({ "color": "#1d3557" }));
    run(&mut doc, "object.envelope.makeWithWarp", json!({ "style": style, "bend": bend }));
    doc
}

#[test]
fn an_arc_keeps_the_band_and_bends_up() {
    let (tops, b) = ink(&warped("arc", 20));
    let width = b[2] - b[0];
    assert!((380..=440).contains(&width), "the arc is about as wide as the band ({width}): {b:?}");
    // Positive bend: the middle is higher than the ends, as Arch does.
    let (middle, left, right) = (tops[400].unwrap(), tops[240].unwrap(), tops[560].unwrap());
    assert!(middle + 10 < left && middle + 10 < right, "the middle ({middle}) is above the ends ({left}, {right})");
    let (arch, _) = ink(&warped("arch", 20));
    assert!(arch[400].unwrap() < arch[210].unwrap(), "Arch bends up too");
    // A negative bend arches down.
    let (tops, _) = ink(&warped("arc", -20));
    assert!(tops[400].unwrap() > tops[240].unwrap() + 10 && tops[400].unwrap() > tops[560].unwrap() + 10, "a negative bend sags");
}
