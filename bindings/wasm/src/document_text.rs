//! `document.text` (KitText 1.0, kition-sdk `docs/platform/sets/text.md`): the drawing's words
//! as plain text, so a host can index them for search. The words of a drawing
//! live in its type objects, outside the object model a host reads.
//!
//! Each visible story (point type, area type, type on a path; a thread of frames is one story)
//! is one block, blocks separated by a newline. Stories are grouped by the artboard their first
//! frame's centre lies on, artboards in artboard order, and within an artboard read top to
//! bottom, then left to right. Stories on no artboard come last, as the pasteboard. Hidden
//! objects and template layers are left out. Each artboard with words is one part, labelled
//! with its name. Reading changes nothing.

use std::collections::HashSet;

use kition_kit::text::{DocumentText, TextBuilder};
use kition_vector_doc::{Document, Node, NodeId, NodeKind};
use kition_vector_geom::Point;

/// The label of the part with the words on no artboard.
pub const PASTEBOARD: &str = "Pasteboard";

/// One story: its words and where its first frame is.
struct Story {
    words: String,
    at: Point,
    top: f64,
    left: f64,
}

fn collect(doc: &Document, node: &Node, done: &mut HashSet<NodeId>, out: &mut Vec<Story>) {
    if !node.visible || matches!(node.kind, NodeKind::Layer { template: true, .. }) {
        return;
    }
    if let NodeKind::Text(text) = &node.kind {
        if !done.insert(node.id) {
            return;
        }
        let words = match doc.text_threads.iter().find(|thread| thread.contains(&node.id)) {
            Some(thread) => {
                done.extend(thread.iter().copied());
                thread
                    .iter()
                    .filter_map(|id| doc.node(*id))
                    .filter_map(|frame| match &frame.kind {
                        NodeKind::Text(text) => Some(text.plain_text()),
                        _ => None,
                    })
                    .collect()
            }
            None => text.plain_text(),
        };
        let bounds = node.geometric_bounds().unwrap_or_default();
        let (top, left) = if bounds.y0.is_finite() && bounds.x0.is_finite() { (bounds.y0, bounds.x0) } else { (0.0, 0.0) };
        out.push(Story { words, at: bounds.center(), top, left });
        return;
    }
    for child in node.children().into_iter().flatten() {
        collect(doc, child, done, out);
    }
}

/// The drawing's words, artboard by artboard (see the module docs), at most `limit` characters.
pub fn document_text(doc: &Document, limit: usize) -> DocumentText {
    let mut stories = Vec::new();
    let mut done = HashSet::new();
    for layer in &doc.layers {
        collect(doc, layer, &mut done, &mut stories);
    }
    // Each story on the first artboard holding its centre; `None`: the pasteboard.
    let mut boards: Vec<Vec<Story>> = doc.artboards.iter().map(|_| Vec::new()).collect();
    let mut pasteboard = Vec::new();
    for story in stories {
        match doc.artboards.iter().position(|board| board.rect.contains(story.at)) {
            Some(index) => boards.get_mut(index).unwrap_or(&mut pasteboard).push(story),
            None => pasteboard.push(story),
        }
    }
    let mut out = TextBuilder::new(limit);
    let labelled = doc
        .artboards
        .iter()
        .enumerate()
        .map(|(index, board)| if board.name.trim().is_empty() { format!("Artboard {}", index + 1) } else { board.name.clone() });
    for (label, mut stories) in labelled.chain(std::iter::once(PASTEBOARD.to_owned())).zip(boards.into_iter().chain([pasteboard])) {
        // Rows by whole points from the top, then left to right: a total order, so sorting
        // never sees an inconsistent comparison.
        stories.sort_by(|a, b| a.top.floor().total_cmp(&b.top.floor()).then(a.left.total_cmp(&b.left)));
        let words: Vec<String> =
            stories.iter().map(|story| story.words.replace("\r\n", "\n").replace('\r', "\n")).filter(|words| !words.trim().is_empty()).collect();
        if !words.is_empty() && !out.part(label, &words.join("\n")) {
            break;
        }
    }
    out.finish()
}
