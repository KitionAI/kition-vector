//! The schema's containers (task schema-freeze): a type object's characters (`Node.text`), a
//! threaded story (`Thread.text` with its `frames`), a symbol's art (`Symbol.nodes`) and the
//! guides (`Drawing.guides`). For each, two replicas edit it concurrently, the edits merge, both
//! survive, and the replicas end identical.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use std::sync::Arc;

use common::{SharedBlobs, assert_no_dangling, assert_replica_clean, canonical, edit, first, named, named_id, peer, sync};
use kition_kit::kdm::ObjectId;
use kition_vector_doc::color::{Color, Paint};
use kition_vector_doc::geom::{Affine, Point, Rect, shapes};
use kition_vector_doc::{Appearance, CharStyle, Document, Guide, Justify, Node, NodeId, NodeKind, ParaStyle, Symbol, TextKind, TextObject, TextRun};

fn assert_converged(a: &common::Session, b: &common::Session) {
    assert_eq!(a.state_hash().ok(), b.state_hash().ok(), "same replica state");
    let (x, y) = (canonical(a.model().document()), canonical(b.model().document()));
    assert!(x == y, "same view:\n{:#?}", common::differences(&x, &y));
    assert_no_dangling(a.model().document());
    assert_replica_clean(a.replica());
    // Each view is what a fresh read of its replica gives.
    let blobs = SharedBlobs::default();
    for session in [a, b] {
        let (fresh, _) = kdm_model::read_document(session.replica(), &blobs).unwrap();
        let (x, y) = (canonical(&fresh), canonical(session.model().document()));
        assert!(x == y, "the view is the replica's:\n{:#?}", common::differences(&x, &y));
    }
}

fn text_of<'a>(d: &'a Document, name: &str) -> &'a TextObject {
    match &named(d, name).kind {
        NodeKind::Text(t) => t,
        other => panic!("{name} is not type: {other:?}"),
    }
}

fn text_mut<'a>(d: &'a mut Document, name: &str) -> &'a mut TextObject {
    let id = named_id(d, name);
    match &mut d.node_mut(id).unwrap().kind {
        NodeKind::Text(t) => t,
        other => panic!("{name} is not type: {other:?}"),
    }
}

fn point_type(d: &mut Document, name: &str, text: &str) -> Node {
    let id = d.alloc_id();
    let mut n = Node::new(id, NodeKind::Text(Box::new(TextObject::point(Point::new(20.0, 40.0), text, CharStyle::default()))));
    n.name = Some(name.into());
    n
}

fn area_type(d: &mut Document, name: &str, x: f64, text: &str) -> Node {
    let id = d.alloc_id();
    let mut t = TextObject::point(Point::ZERO, text, CharStyle::default());
    t.kind = TextKind::Area { frame: shapes::rectangle(Rect::new(x, 100.0, x + 120.0, 200.0)) };
    t.xf = Affine::IDENTITY;
    let mut n = Node::new(id, NodeKind::Text(Box::new(t)));
    n.name = Some(name.into());
    n
}

fn red() -> CharStyle {
    CharStyle { fill: Paint::solid(Color::rgb(0.9, 0.1, 0.1)), ..CharStyle::default() }
}

fn bold() -> CharStyle {
    CharStyle { font_style: "Bold".into(), ..CharStyle::default() }
}

/// A drawing with point type "Title" ("Hello world") and a second paragraph.
fn with_type() -> Document {
    let mut d = Document::new(400.0, 300.0);
    let layer = d.layers[0].id;
    let t = point_type(&mut d, "Title", "Hello world\nSecond");
    d.insert(Some(layer), usize::MAX, t).unwrap();
    d
}

fn pair(d: &Document) -> (SharedBlobs, common::Session, common::Session) {
    let blobs = SharedBlobs::default();
    let a = first(d, &blobs);
    let b = peer(&a, 2, &blobs);
    (blobs, a, b)
}

#[test]
fn typing_in_two_places_of_one_text_keeps_both() {
    let (blobs, mut a, mut b) = pair(&with_type());
    edit(&mut a, &blobs, |d| text_mut(d, "Title").runs = vec![TextRun::new("Hello big world\nSecond", CharStyle::default())]);
    edit(&mut b, &blobs, |d| text_mut(d, "Title").runs = vec![TextRun::new("Hello world\nSecond line", CharStyle::default())]);
    sync(&mut a, &mut b);
    assert_converged(&a, &b);
    assert_eq!(text_of(a.model().document(), "Title").plain_text(), "Hello big world\nSecond line");
}

#[test]
fn formatting_two_ranges_of_one_text_keeps_both() {
    let (blobs, mut a, mut b) = pair(&with_type());
    edit(&mut a, &blobs, |d| {
        text_mut(d, "Title").runs = vec![TextRun::new("Hello", bold()), TextRun::new(" world\nSecond", CharStyle::default())];
    });
    edit(&mut b, &blobs, |d| {
        text_mut(d, "Title").runs =
            vec![TextRun::new("Hello ", CharStyle::default()), TextRun::new("world", red()), TextRun::new("\nSecond", CharStyle::default())];
    });
    sync(&mut a, &mut b);
    assert_converged(&a, &b);
    let t = text_of(a.model().document(), "Title");
    assert_eq!(
        t.runs,
        vec![
            TextRun::new("Hello", bold()),
            TextRun::new(" ", CharStyle::default()),
            TextRun::new("world", red()),
            TextRun::new("\nSecond", CharStyle::default())
        ]
    );
}

#[test]
fn typing_while_the_other_aligns_a_paragraph_keeps_both() {
    let (blobs, mut a, mut b) = pair(&with_type());
    edit(&mut a, &blobs, |d| text_mut(d, "Title").runs = vec![TextRun::new("Hello world!\nSecond", CharStyle::default())]);
    let centred = ParaStyle { justify: Justify::Center, ..ParaStyle::default() };
    edit(&mut b, &blobs, |d| text_mut(d, "Title").set_paragraph_styles(vec![ParaStyle::default(), centred.clone()]));
    sync(&mut a, &mut b);
    assert_converged(&a, &b);
    let t = text_of(a.model().document(), "Title");
    assert_eq!(t.plain_text(), "Hello world!\nSecond");
    assert_eq!(t.paragraph_styles(), vec![ParaStyle::default(), centred]);
}

#[test]
fn a_type_objects_characters_are_its_text_not_its_data() {
    let blobs = SharedBlobs::default();
    let d = with_type();
    let (replica, ids) = common::new_replica(&d, &blobs);
    let object = ids.node(named_id(&d, "Title")).cloned().unwrap();
    assert_eq!(replica.text(&object, "text").unwrap(), "Hello world\nSecond\n", "the text ends every paragraph with a break");
    let data = replica.get(&object, "data").unwrap();
    for key in ["runs", "para", "paras"] {
        assert!(data.get(key).is_none(), "data holds no {key}");
    }
    assert_eq!(replica.get(&object, "name").unwrap(), serde_json::json!("Title"), "a name is a string");
}

/// A drawing whose story "One two three" flows through two area-type frames "F1" and "F2".
fn with_thread() -> Document {
    let mut d = Document::new(600.0, 400.0);
    let layer = d.layers[0].id;
    let f1 = area_type(&mut d, "F1", 0.0, "One two ");
    let f2 = area_type(&mut d, "F2", 200.0, "three");
    let (i1, i2) = (f1.id, f2.id);
    d.insert(Some(layer), usize::MAX, f1).unwrap();
    d.insert(Some(layer), usize::MAX, f2).unwrap();
    d.text_threads.push(vec![i1, i2]);
    d
}

/// The story the frames of the drawing's first thread show, in order.
fn story(d: &Document) -> String {
    d.text_threads[0]
        .iter()
        .filter_map(|id| d.node(*id))
        .filter_map(|n| if let NodeKind::Text(t) = &n.kind { Some(t.plain_text()) } else { None })
        .collect()
}

#[test]
fn a_threaded_story_is_one_text_and_edits_in_two_frames_merge() {
    let (blobs, mut a, mut b) = pair(&with_thread());
    assert_eq!(story(a.model().document()), "One two three");
    let root = ObjectId::from_string("drawing");
    let thread = a.replica().list(&root, "threads").unwrap()[0].clone();
    assert_eq!(a.replica().text(&thread, "text").unwrap(), "One two three\n", "the story is the thread's");
    for frame in a.replica().list(&thread, "frames").unwrap() {
        assert_eq!(a.replica().text(&frame, "text").unwrap(), "", "a frame's own text is empty");
    }
    // A types at the story's start (in the first frame), B at its end (in the last one).
    edit(&mut a, &blobs, |d| {
        let first = d.text_threads[0][0];
        if let Some(NodeKind::Text(t)) = d.node_mut(first).map(|n| &mut n.kind) {
            let text = t.plain_text();
            t.runs = vec![TextRun::new(format!("Zero {text}"), CharStyle::default())];
        }
    });
    edit(&mut b, &blobs, |d| {
        let last = *d.text_threads[0].last().unwrap();
        if let Some(NodeKind::Text(t)) = d.node_mut(last).map(|n| &mut n.kind) {
            let text = t.plain_text();
            t.runs = vec![TextRun::new(format!("{text} four"), CharStyle::default())];
        }
    });
    sync(&mut a, &mut b);
    assert_converged(&a, &b);
    assert_eq!(story(a.model().document()), "Zero One two three four");
    assert_eq!(a.model().document().text_threads.len(), 1);
}

#[test]
fn unthreading_gives_each_frame_its_own_text_again() {
    let (blobs, mut a, mut b) = pair(&with_thread());
    let shown: Vec<String> = a.model().document().text_threads[0].iter().map(|id| text_of_id(a.model().document(), *id)).collect();
    edit(&mut a, &blobs, |d| d.text_threads.clear());
    sync(&mut a, &mut b);
    assert_converged(&a, &b);
    let d = b.model().document();
    assert!(d.text_threads.is_empty());
    let now: Vec<String> = ["F1", "F2"].iter().map(|name| text_of(d, name).plain_text()).collect();
    assert_eq!(now, shown, "each frame keeps the text it showed");
}

fn text_of_id(d: &Document, id: NodeId) -> String {
    match &d.node(id).unwrap().kind {
        NodeKind::Text(t) => t.plain_text(),
        _ => String::new(),
    }
}

/// A drawing with symbol "Badge": a group of the paths "Disc" and "Star".
fn with_symbol() -> Document {
    let mut d = Document::new(400.0, 300.0);
    let disc =
        Node::path(NodeId(1), shapes::ellipse(Rect::new(0.0, 0.0, 40.0, 40.0)), Appearance::basic(Paint::solid(Color::BLACK), Paint::None, 0.0));
    let star =
        Node::path(NodeId(2), shapes::rectangle(Rect::new(10.0, 10.0, 30.0, 30.0)), Appearance::basic(Paint::solid(Color::WHITE), Paint::None, 0.0));
    let (mut disc, mut star) = (disc, star);
    disc.name = Some("Disc".into());
    star.name = Some("Star".into());
    let mut art = Node::group(NodeId(3), vec![Arc::new(disc), Arc::new(star)]);
    art.name = Some("Badge art".into());
    d.symbols.push(Symbol { uid: Default::default(), name: "Badge".into(), art: Arc::new(art) });
    d.settle_uids(None);
    d
}

fn art_node<'a>(d: &'a Document, name: &str) -> &'a Node {
    let art = &d.symbols[0].art;
    art.children().unwrap().iter().find(|n| n.name.as_deref() == Some(name)).unwrap_or_else(|| panic!("no {name} in the art"))
}

fn art_node_mut<'a>(d: &'a mut Document, name: &str) -> &'a mut Node {
    let art = Arc::make_mut(&mut d.symbols[0].art);
    let children = art.children_mut().unwrap();
    let at = children.iter().position(|n| n.name.as_deref() == Some(name)).unwrap();
    Arc::make_mut(&mut children[at])
}

#[test]
fn editing_two_objects_of_one_symbol_keeps_both() {
    let (blobs, mut a, mut b) = pair(&with_symbol());
    let root = ObjectId::from_string("drawing");
    let symbol = a.replica().list(&root, "symbols").unwrap()[0].clone();
    assert_eq!(a.replica().tree(&symbol, "nodes").unwrap().len(), 3, "the art is the symbol's tree: the group and its two paths");
    edit(&mut a, &blobs, |d| {
        if let NodeKind::Path { path, .. } = &mut art_node_mut(d, "Disc").kind {
            path.transform(Affine::translate((5.0, 0.0)));
        }
    });
    edit(&mut b, &blobs, |d| art_node_mut(d, "Star").appearance.set_fill(Paint::solid(Color::rgb(0.0, 0.5, 1.0))));
    sync(&mut a, &mut b);
    assert_converged(&a, &b);
    let d = a.model().document();
    let NodeKind::Path { path, .. } = &art_node(d, "Disc").kind else { panic!("Disc is a path") };
    assert_eq!(path.bounds().map(|r| r.x0), Some(5.0), "A's move is kept");
    assert_eq!(art_node(d, "Star").appearance.fill_paint(), Paint::solid(Color::rgb(0.0, 0.5, 1.0)), "B's recolour is kept");
}

#[test]
fn deleting_a_symbol_deletes_its_art() {
    let blobs = SharedBlobs::default();
    let mut a = first(&with_symbol(), &blobs);
    let root = ObjectId::from_string("drawing");
    let symbol = a.replica().list(&root, "symbols").unwrap()[0].clone();
    let art: Vec<ObjectId> = a.replica().tree(&symbol, "nodes").unwrap().into_iter().map(|e| e.object).collect();
    edit(&mut a, &blobs, |d| d.symbols.clear());
    assert!(!a.replica().exists(&symbol));
    for object in art {
        assert!(!a.replica().exists(&object), "{object} went with its symbol");
    }
}

#[test]
fn adding_and_moving_different_guides_keeps_both() {
    let mut d = Document::new(400.0, 300.0);
    d.guides.push(Guide::new(true, 10.0));
    d.guides.push(Guide::new(false, 20.0));
    let (blobs, mut a, mut b) = pair(&d);
    edit(&mut a, &blobs, |d| {
        d.guides[0].pos = 15.0;
        d.guides.push(Guide::new(true, 100.0));
    });
    edit(&mut b, &blobs, |d| {
        d.guides[1].pos = 25.0;
        d.guides.push(Guide::new(false, 200.0));
    });
    sync(&mut a, &mut b);
    assert_converged(&a, &b);
    let mut guides: Vec<(bool, f64)> = a.model().document().guides.iter().map(|g| (g.vertical, g.pos)).collect();
    guides.sort_by(|x, y| x.1.total_cmp(&y.1));
    assert_eq!(guides, vec![(true, 15.0), (false, 25.0), (true, 100.0), (false, 200.0)]);
}

#[test]
fn pattern_editing_and_the_active_data_set_are_the_persons_own() {
    let mut d = common::drawing();
    d.variables.datasets.push(kition_vector_doc::DataSet { name: "D1".into(), values: Default::default() });
    d.variables.active_dataset = Some("D1".into());
    let layer = d.add_layer(Some(kition_vector_doc::pattern::PATTERN_EDIT_LAYER));
    d.pattern_edit = Some(kition_vector_doc::PatternEdit { pattern: "Dots".into(), layer, original: None });
    let shared = kdm_model::shared_part(&d);
    assert!(shared.pattern_edit.is_none() && shared.variables.active_dataset.is_none());
    assert!(shared.node(layer).is_none(), "the tile layer stays with the person");
    let blobs = SharedBlobs::default();
    let (replica, _) = common::new_replica(&shared, &blobs);
    let (read, _) = kdm_model::read_document(&replica, &blobs).unwrap();
    assert!(read.pattern_edit.is_none() && read.variables.active_dataset.is_none());
    assert_eq!(read.variables.datasets.len(), 1, "the data sets are shared");
    let root = ObjectId::from_string("drawing");
    assert!(replica.map_get(&root, "settings", "pattern_edit").unwrap().is_null());
}
