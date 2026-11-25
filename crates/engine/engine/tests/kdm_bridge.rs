//! The command bridge to each drawing's KDM replica: every mutating command (or pointer gesture)
//! replicates as one transaction that reproduces the drawing in a peer, undo after a remote edit
//! keeps the remote edit, a drag is one undo step (and a remote edit in the middle of it is
//! kept), an undo group is one step, a direct write before a command is no part of its step,
//! selection-only changes write nothing, the history limit never loses an edit, a remote deletion
//! of the selection moves it on, and two sessions taking turns at random commands, each taking
//! the other's changes in order, end equal, with the engine's drawing equal to its own replica
//! after every step.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use kition_vector_engine::doc::{Document, NodeId, NodeKind};
use kition_vector_engine::tools::{PointerEvent, PointerKind};
use kition_vector_engine::{Origin, Session, UndoScope, Version, ViewInfo};
use serde_json::{Value, json};

/// A session with one drawing: rectangles A and B on Layer 1, C on a second layer "Notes".
fn new_session() -> Session {
    let mut s = Session::new();
    s.execute("file.new", &json!({"width": 800, "height": 600})).unwrap();
    rect(&mut s, "A", 100.0, 100.0);
    rect(&mut s, "B", 300.0, 100.0);
    s.execute("layer.new", &json!({"name": "Notes"})).unwrap();
    rect(&mut s, "C", 100.0, 400.0);
    s.execute("select.none", &json!({})).unwrap();
    s
}

fn rect(s: &mut Session, name: &str, x: f64, y: f64) -> u64 {
    let r = s.execute("shape.rectangle", &json!({"x": x, "y": y, "width": 100, "height": 80})).unwrap();
    let id = r["id"].as_u64().unwrap();
    s.execute("layer.setProps", &json!({"id": id, "name": name})).unwrap();
    id
}

/// A second session holding a replica of `a`'s active drawing.
fn peer(a: &mut Session) -> Session {
    let mut b = Session::new();
    let blobs = a.blobs_since(&Version::empty()).unwrap();
    b.open_snapshot(&a.snapshot().unwrap(), blobs).unwrap();
    b
}

/// Sends `from`'s changes (and the blobs they need) that `to` lacks.
fn send(from: &mut Session, to: &mut Session) {
    let since = to.version().unwrap();
    let blobs = from.blobs_since(&since).unwrap();
    to.add_blobs(&blobs).unwrap();
    let bytes = from.export_changes(&since).unwrap();
    to.import_changes(&bytes).unwrap();
}

fn sync(a: &mut Session, b: &mut Session) {
    for _ in 0..8 {
        send(a, b);
        send(b, a);
        if a.version().unwrap() == b.version().unwrap() {
            return;
        }
    }
    panic!("the replicas did not converge");
}

fn doc(s: &Session) -> &Document {
    &s.active().unwrap().doc
}

fn canonical(d: &Document) -> Value {
    kdm_model::normalize(&kdm_model::shared_part(d))
}

/// `a` and `b` hold the same replicated state and the same drawing (ids aside).
fn assert_same(a: &mut Session, b: &mut Session, what: &str) {
    assert_eq!(a.state_hash().unwrap(), b.state_hash().unwrap(), "{what}: replicas differ");
    let (x, y) = (canonical(doc(a)), canonical(doc(b)));
    assert!(x == y, "{what}: drawings differ:\n{:#?}", differences(&x, &y));
}

/// The drawing as read back fresh from the session's replica, without flushing anything.
fn replica_doc(s: &Session) -> Document {
    let kdm = s.kdm().expect("an edit made the replica");
    kdm_model::read_document(kdm.replica(), kdm.blobs()).unwrap().0
}

/// The engine's drawing is exactly what its replica holds.
fn assert_holds(s: &Session, what: &str) {
    let (mine, held) = (canonical(doc(s)), canonical(&replica_doc(s)));
    assert!(mine == held, "{what}: the drawing is not what its replica holds:\n{:#?}", differences(&mine, &held));
}

/// The replica's own undo steps (one per undoable transaction).
fn replica_steps(s: &Session) -> usize {
    s.kdm().map_or(0, |k| k.steps(UndoScope::Person).0)
}

fn named(d: &Document, name: &str) -> u64 {
    let mut found = None;
    d.walk(|n| {
        if found.is_none() && n.name.as_deref() == Some(name) {
            found = Some(n.id.0);
        }
    });
    found.unwrap_or_else(|| panic!("no object named {name}"))
}

fn x_of(d: &Document, name: &str) -> f64 {
    d.node(NodeId(named(d, name))).and_then(|n| n.geometric_bounds()).map_or(f64::NAN, |b| b.x0)
}

fn fill_of(d: &Document, name: &str) -> String {
    format!("{:?}", d.node(NodeId(named(d, name))).unwrap().appearance.fill_paint())
}

/// Moves object `id` by (`dx`, `dy`) with `object.move`, which moves the selection.
fn move_object(s: &mut Session, id: u64, dx: f64, dy: f64) {
    s.execute("select.set", &json!({"ids": [id]})).unwrap();
    s.execute("object.move", &json!({"dx": dx, "dy": dy})).unwrap();
}

fn drag(s: &mut Session, from: (f64, f64), to: (f64, f64)) {
    let v = ViewInfo::default();
    s.select_tool("selection", v).unwrap();
    s.pointer(&PointerEvent::new(PointerKind::Down, from.0, from.1), v).unwrap();
    for k in 1..=4 {
        let t = f64::from(k) / 4.0;
        s.pointer(&PointerEvent::new(PointerKind::Drag, from.0 + (to.0 - from.0) * t, from.1 + (to.1 - from.1) * t), v).unwrap();
    }
    s.pointer(&PointerEvent::new(PointerKind::Up, to.0, to.1), v).unwrap();
}

/// A named edit of a session.
type Edit = fn(&mut Session);

#[test]
fn one_command_one_transaction_reproduces_in_a_peer() {
    let cases: Vec<(&str, Edit)> = vec![
        ("move", |s| {
            let a = named(doc(s), "A");
            move_object(s, a, 15.0, 5.0);
        }),
        ("recolour", |s| {
            let b = named(doc(s), "B");
            s.execute("paint.setFill", &json!({"color": "#00ff00", "ids": [b]})).unwrap();
        }),
        ("group", |s| {
            let (a, b) = (named(doc(s), "A"), named(doc(s), "B"));
            s.execute("select.set", &json!({"ids": [a, b]})).unwrap();
            s.execute("object.group", &json!({})).unwrap();
        }),
        ("reparent", |s| {
            let a = named(doc(s), "A");
            let notes = doc(s).layers[1].id.0;
            s.execute("layer.move", &json!({"ids": [a], "target": notes})).unwrap();
        }),
        ("bring to front", |s| {
            let a = named(doc(s), "A");
            s.execute("select.set", &json!({"ids": [a]})).unwrap();
            s.execute("object.arrange.bringToFront", &json!({})).unwrap();
        }),
        ("delete", |s| {
            let b = named(doc(s), "B");
            s.execute("edit.clear", &json!({"ids": [b]})).unwrap();
        }),
        ("drag", |s| drag(s, (150.0, 140.0), (190.0, 160.0))),
        ("new artboard", |s| {
            s.execute("artboard.new", &json!({"x": 900, "y": 0, "width": 200, "height": 100})).unwrap();
        }),
        ("new swatch", |s| {
            s.execute("swatch.new", &json!({"name": "Brand", "color": "#123456", "global": true})).unwrap();
        }),
        ("guide", |s| {
            s.execute("guide.add", &json!({"vertical": false, "pos": 50})).unwrap();
        }),
        ("text", |s| {
            s.execute("text.create", &json!({"x": 10, "y": 500, "text": "point"})).unwrap();
        }),
        ("new layer", |s| {
            s.execute("layer.new", &json!({"name": "Ink"})).unwrap();
        }),
    ];
    for (what, run) in cases {
        let mut a = new_session();
        let mut b = peer(&mut a);
        let steps = replica_steps(&a);
        let history = a.active().unwrap().history.undo.len();
        run(&mut a);
        let pushed = a.active().unwrap().history.undo.len() - history;
        assert_eq!(replica_steps(&a), steps + pushed, "{what}: one replica step per history step");
        assert!(pushed >= 1, "{what}: the command made an undo step");
        assert_holds(&a, what);
        send(&mut a, &mut b);
        assert_same(&mut a, &mut b, what);
    }
}

#[test]
fn a_drag_is_one_undo_step() {
    let mut a = new_session();
    let mut b = peer(&mut a);
    let x = x_of(doc(&a), "A");
    let steps = replica_steps(&a);
    drag(&mut a, (150.0, 140.0), (190.0, 140.0));
    assert_eq!(replica_steps(&a), steps + 1, "the whole drag is one transaction");
    assert_eq!(x_of(doc(&a), "A"), x + 40.0);
    send(&mut a, &mut b);
    assert_eq!(x_of(doc(&b), "A"), x + 40.0);
    a.execute("edit.undo", &json!({})).unwrap();
    assert_eq!(x_of(doc(&a), "A"), x);
    assert_holds(&a, "after undo");
}

#[test]
fn a_remote_edit_in_the_middle_of_a_drag_keeps_both() {
    let mut a = new_session();
    let mut b = peer(&mut a);
    let x = x_of(doc(&a), "A");
    let v = ViewInfo::default();
    a.select_tool("selection", v).unwrap();
    a.pointer(&PointerEvent::new(PointerKind::Down, 150.0, 140.0), v).unwrap();
    a.pointer(&PointerEvent::new(PointerKind::Drag, 160.0, 140.0), v).unwrap();
    let bid = named(doc(&b), "B");
    b.execute("paint.setFill", &json!({"color": "#ff0000", "ids": [bid]})).unwrap();
    send(&mut b, &mut a);
    assert_eq!(fill_of(doc(&a), "B"), fill_of(doc(&b), "B"), "the remote recolour shows during the drag");
    a.pointer(&PointerEvent::new(PointerKind::Drag, 170.0, 140.0), v).unwrap();
    a.pointer(&PointerEvent::new(PointerKind::Up, 170.0, 140.0), v).unwrap();
    assert_eq!(x_of(doc(&a), "A"), x + 20.0);
    assert_eq!(fill_of(doc(&a), "B"), fill_of(doc(&b), "B"), "the drag did not undo the remote recolour");
    sync(&mut a, &mut b);
    assert_same(&mut a, &mut b, "after the drag");
    assert_holds(&a, "after the drag");
}

#[test]
fn undo_after_a_remote_edit_keeps_it() {
    let mut a = new_session();
    let mut b = peer(&mut a);
    let aid = named(doc(&a), "A");
    move_object(&mut a, aid, 50.0, 0.0);
    send(&mut a, &mut b);
    let bid = named(doc(&b), "B");
    b.execute("paint.setFill", &json!({"color": "#0000ff", "ids": [bid]})).unwrap();
    sync(&mut a, &mut b);
    let x = x_of(doc(&a), "A");
    a.execute("edit.undo", &json!({})).unwrap();
    assert_eq!(x_of(doc(&a), "A"), x - 50.0, "the own move is undone");
    assert_eq!(fill_of(doc(&a), "B"), fill_of(doc(&b), "B"), "the remote recolour stays");
    a.execute("edit.redo", &json!({})).unwrap();
    assert_eq!(x_of(doc(&a), "A"), x);
    sync(&mut a, &mut b);
    assert_same(&mut a, &mut b, "after undo and redo");
}

#[test]
fn undo_back_to_the_saved_drawing_is_clean() {
    let mut a = new_session();
    a.version().unwrap();
    a.doc_mut().unwrap().mark_saved();
    let aid = named(doc(&a), "A");
    move_object(&mut a, aid, 5.0, 0.0);
    assert!(a.active().unwrap().is_dirty());
    a.execute("edit.undo", &json!({})).unwrap();
    assert!(!a.active().unwrap().is_dirty(), "undone back to what was saved");
}

#[test]
fn an_undo_group_is_one_step_and_a_cancelled_one_leaves_nothing() {
    let mut a = new_session();
    let mut b = peer(&mut a);
    let aid = named(doc(&a), "A");
    let x = x_of(doc(&a), "A");
    let history = a.active().unwrap().history.undo.len();
    a.begin_undo_group();
    for _ in 0..3 {
        move_object(&mut a, aid, 10.0, 0.0);
    }
    a.end_undo_group(false);
    assert_eq!(a.active().unwrap().history.undo.len(), history + 1, "one undo step");
    assert_holds(&a, "group");
    send(&mut a, &mut b);
    assert_eq!(x_of(doc(&b), "A"), x + 30.0);
    a.execute("edit.undo", &json!({})).unwrap();
    assert_eq!(x_of(doc(&a), "A"), x, "the whole group is undone");
    assert_holds(&a, "group undone");

    a.begin_undo_group();
    for _ in 0..2 {
        move_object(&mut a, aid, 7.0, 0.0);
    }
    a.end_undo_group(true);
    assert_eq!(x_of(doc(&a), "A"), x, "a cancelled group is undone");
    assert_holds(&a, "cancelled group");
    sync(&mut a, &mut b);
    assert_same(&mut a, &mut b, "cancelled group");
    assert_eq!(x_of(doc(&b), "A"), x);
}

#[test]
fn a_direct_write_before_a_command_survives_undoing_the_command() {
    let mut a = new_session();
    let mut b = peer(&mut a);
    let (aid, bid) = (named(doc(&a), "A"), named(doc(&a), "B"));
    a.execute("select.set", &json!({"ids": [aid]})).unwrap();
    // A write outside any command (no undo step of its own), then a command.
    std::sync::Arc::make_mut(&mut a.doc_mut().unwrap().doc).node_mut(NodeId(bid)).unwrap().name = Some("B2".into());
    a.execute("object.move", &json!({"dx": 10, "dy": 0})).unwrap();
    assert_holds(&a, "after the move");
    a.execute("edit.undo", &json!({})).unwrap();
    assert_eq!(named(doc(&a), "B2"), bid, "undoing the move keeps the direct write");
    assert_holds(&a, "after undoing the move");
    send(&mut a, &mut b);
    assert_same(&mut a, &mut b, "after undoing the move");
    assert!(doc(&b).node(NodeId(named(doc(&b), "B2"))).is_some());
}

#[test]
fn selection_only_changes_write_nothing() {
    let mut a = new_session();
    let version = a.version().unwrap();
    let aid = named(doc(&a), "A");
    a.execute("select.set", &json!({"ids": [aid]})).unwrap();
    a.execute("select.all", &json!({})).unwrap();
    a.execute("select.none", &json!({})).unwrap();
    assert_eq!(a.version().unwrap(), version, "selecting is this person's own");
}

#[test]
fn every_edit_replicates_at_the_history_limit() {
    let mut a = new_session();
    let mut b = peer(&mut a);
    a.doc_mut().unwrap().history.limit = 3;
    let aid = named(doc(&a), "A");
    let x = x_of(doc(&a), "A");
    for k in 1..=6 {
        move_object(&mut a, aid, 1.0, 0.0);
        send(&mut a, &mut b);
        assert_eq!(x_of(doc(&b), "A"), x + f64::from(k), "edit {k} reached the peer");
    }
    assert_eq!(replica_steps(&a), 3, "the replica keeps as many steps as the history");
    for _ in 0..3 {
        a.execute("edit.undo", &json!({})).unwrap();
    }
    assert_eq!(x_of(doc(&a), "A"), x + 3.0);
    assert_holds(&a, "after undoing to the limit");
}

#[test]
fn a_remote_deletion_of_the_selection_fixes_it() {
    let mut a = new_session();
    let mut b = peer(&mut a);
    let aid = named(doc(&a), "A");
    a.execute("select.set", &json!({"ids": [aid]})).unwrap();
    let bid = named(doc(&b), "A");
    b.execute("edit.clear", &json!({"ids": [bid]})).unwrap();
    send(&mut b, &mut a);
    assert!(a.active().unwrap().selection.objects.is_empty(), "the deleted object left the selection");
    assert_holds(&a, "after the remote deletion");
}

/// Kition platform KitModel 1.1 M12a: on one replica, the person's undo and an agent's undo each
/// take back only their own steps and keep the other's.
#[test]
fn agent_edits_record_their_origin_and_undo_in_their_own_scope() {
    let mut a = new_session();
    let aid = named(doc(&a), "A");
    let agent = Origin::agent("agent:helper");
    let x = x_of(doc(&a), "A");
    a.execute("select.set", &json!({"ids": [aid]})).unwrap();
    a.execute_as("object.move", &json!({"dx": 3, "dy": 0}), &agent).unwrap();
    assert_eq!(x_of(doc(&a), "A"), x + 3.0);
    a.execute("edit.undo", &json!({})).unwrap();
    assert_eq!(x_of(doc(&a), "A"), x + 3.0, "the person's undo keeps the agent's edit");
    a.execute_as("edit.undo", &json!({}), &agent).unwrap();
    assert_eq!(x_of(doc(&a), "A"), x, "the agent's undo takes the agent's step");
    assert_holds(&a, "agent edit undone");
}

/// M12a: an agent's undo of a step under the person's later edit keeps the person's edit, and
/// `edit.undo`'s `scope` (engine rule E14a) names whose step to undo.
#[test]
fn an_agent_undo_under_the_persons_later_step_keeps_the_persons_edit() {
    let mut a = new_session();
    let (aid, bid) = (named(doc(&a), "A"), named(doc(&a), "B"));
    let agent = Origin::agent("agent:helper");
    let (xa, xb) = (x_of(doc(&a), "A"), x_of(doc(&a), "B"));
    a.execute("select.set", &json!({"ids": [aid]})).unwrap();
    a.execute_as("object.move", &json!({"dx": 3, "dy": 0}), &agent).unwrap();
    a.execute("select.set", &json!({"ids": [bid]})).unwrap();
    a.execute("object.move", &json!({"dx": 5, "dy": 0})).unwrap();
    a.execute("edit.undo", &json!({"scope": "agent"})).unwrap();
    assert_eq!(x_of(doc(&a), "A"), xa, "the agent's step is undone");
    assert_eq!(x_of(doc(&a), "B"), xb + 5.0, "the person's later edit stays");
    a.execute_as("edit.redo", &json!({}), &agent).unwrap();
    assert_eq!(x_of(doc(&a), "A"), xa + 3.0, "the agent's redo brings its step back");
    assert_eq!(x_of(doc(&a), "B"), xb + 5.0);
    assert!(a.execute("edit.undo", &json!({"scope": "other"})).is_err(), "an unknown scope is refused");
    assert_holds(&a, "after scoped undo and redo");
}

#[test]
fn a_file_newer_than_its_stored_replica_wins_on_open() {
    let mut a = new_session();
    let snapshot = a.snapshot().unwrap();
    let blobs = a.blobs().unwrap();
    // The file saved elsewhere has one more object than the replica.
    rect(&mut a, "D", 500.0, 500.0);
    let file = kition_vector_engine::doc::Document::clone(doc(&a));
    let bytes = kition_vector_format::save(&file, false);

    let mut b = Session::new();
    b.open_with_replica("Drawing.kvector", &bytes, &snapshot, blobs.clone()).unwrap();
    named(doc(&b), "D");
    assert!(!b.active().unwrap().is_dirty(), "it opens clean");
    assert_holds(&b, "the file was recorded onto the replica");

    // The same file as the replica opens without writing anything.
    let mut c = Session::new();
    let same = kition_vector_format::save(&replica_doc(&b), false);
    let version = b.version().unwrap();
    c.open_with_replica("Drawing.kvector", &same, &b.snapshot().unwrap(), b.blobs().unwrap()).unwrap();
    assert_eq!(c.version().unwrap(), version);
}

/// A threaded story edited through the engine: after every step the engine's drawing (the
/// session's own view, each frame showing its part of the story) is what a fresh read of the
/// replica gives, which lays the story out through the frames again.
#[test]
fn a_threaded_story_shows_as_a_fresh_read_lays_it_out() {
    let mut s = new_session();
    let frames: Vec<u64> = (0..3).map(|k| rect(&mut s, &format!("F{k}"), 100.0 + 120.0 * f64::from(k), 300.0)).collect();
    s.execute("select.set", &json!({"ids": frames})).unwrap();
    s.execute("text.thread.create", &json!({})).unwrap();
    assert_eq!(doc(&s).text_threads.len(), 1, "one thread of three frames");
    assert_holds(&s, "threaded");
    let first = doc(&s).text_threads[0][0].0;
    let long = "The quick brown fox jumps over the lazy dog. ".repeat(12);
    s.execute("text.editRange", &json!({"id": first, "start": 0, "end": 0, "insert": long})).unwrap();
    assert_holds(&s, "typed into the first frame");
    let last = *doc(&s).text_threads[0].last().unwrap();
    let end = match &doc(&s).node(last).unwrap().kind {
        NodeKind::Text(t) => t.plain_text().len(),
        _ => panic!("the last frame is type"),
    };
    s.execute("text.editRange", &json!({"id": last.0, "start": end, "end": end, "insert": " The end."})).unwrap();
    assert_holds(&s, "typed at the story's end, in the last frame");
    move_object(&mut s, first, 0.0, 10.0);
    assert_holds(&s, "a frame moved");
    s.execute("edit.undo", &json!({})).unwrap();
    s.execute("edit.undo", &json!({})).unwrap();
    assert_holds(&s, "undone");
}

#[test]
fn passthroughs_answer_before_any_edit() {
    let mut s = Session::new();
    s.execute("file.new", &json!({"width": 100, "height": 100})).unwrap();
    assert!(s.kdm().is_none(), "no replica until asked");
    assert!(s.doc_id().is_none());
    let version = s.version().unwrap();
    assert!(s.kdm().is_some());
    assert!(s.doc_id().is_some());
    assert!(!s.snapshot().unwrap().is_empty());
    assert!(!s.state_hash().unwrap().is_empty());
    assert!(!s.export_changes(&Version::empty()).unwrap().is_empty());
    assert_eq!(s.version().unwrap(), version, "asking writes nothing");
}

/// xorshift64*: deterministic, no dependency.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn below(&mut self, n: usize) -> usize {
        if n == 0 { 0 } else { (self.next() % n as u64) as usize }
    }
}

fn objects(d: &Document) -> Vec<u64> {
    let layers: Vec<NodeId> = d.layers.iter().map(|l| l.id).collect();
    let mut out = Vec::new();
    d.walk(|n| {
        if !layers.contains(&n.id) && !matches!(n.kind, NodeKind::Layer { .. }) {
            out.push(n.id.0);
        }
    });
    out
}

/// One random command (errors are fine: a command may not apply).
fn random_command(s: &mut Session, rng: &mut Rng) -> String {
    let d = doc(s).clone();
    let objs = objects(&d);
    let pick = |rng: &mut Rng| objs.get(rng.below(objs.len())).copied();
    let layer = |rng: &mut Rng| d.layers.get(rng.below(d.layers.len())).map(|l| l.id.0);
    let (id, params) = match rng.below(13) {
        0 | 1 => ("shape.rectangle", json!({"x": rng.below(600), "y": rng.below(400), "width": 30, "height": 20})),
        2 | 3 => {
            let _ = s.execute("select.set", &json!({"ids": pick(rng).into_iter().collect::<Vec<_>>()}));
            ("object.move", json!({"dx": rng.below(30) as i64 - 15, "dy": 3}))
        }
        4 | 5 => ("paint.setFill", json!({"color": format!("#{:06x}", rng.next() & 0xff_ffff), "ids": pick(rng).into_iter().collect::<Vec<_>>()})),
        6 => {
            let ids: Vec<u64> = [pick(rng), pick(rng)].into_iter().flatten().collect();
            let _ = s.execute("select.set", &json!({"ids": ids}));
            ("object.group", json!({}))
        }
        7 => {
            let _ = s.execute("select.set", &json!({"ids": pick(rng).into_iter().collect::<Vec<_>>()}));
            ("object.ungroup", json!({}))
        }
        8 => ("layer.move", json!({"ids": pick(rng).into_iter().collect::<Vec<_>>(), "target": layer(rng)})),
        9 => {
            let _ = s.execute("select.set", &json!({"ids": pick(rng).into_iter().collect::<Vec<_>>()}));
            ("object.arrange.bringToFront", json!({}))
        }
        10 => ("edit.clear", json!({"ids": pick(rng).into_iter().collect::<Vec<_>>()})),
        11 => ("edit.undo", json!({})),
        _ => ("layer.new", json!({})),
    };
    let _ = s.execute(id, &params);
    id.to_owned()
}

/// Two sessions take turns at random commands on one line of changes: before a session edits,
/// it takes the changes the other made that it lacks.
#[test]
fn random_commands_keep_drawing_and_replica_equal_and_reach_the_peer() {
    for seed in [5u64, 6, 7] {
        let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
        let mut a = new_session();
        let mut b = peer(&mut a);
        for step in 0..120 {
            let (s, other) = if rng.below(2) == 0 { (&mut a, &mut b) } else { (&mut b, &mut a) };
            send(other, s);
            let what = random_command(s, &mut rng);
            assert_holds(s, &format!("seed {seed} step {step} ({what})"));
            if rng.below(4) == 0 {
                send(s, other);
                assert_holds(other, &format!("seed {seed} step {step} after import"));
            }
        }
        sync(&mut a, &mut b);
        assert_same(&mut a, &mut b, &format!("seed {seed}"));
    }
}

/// The paths in which two canonical JSON values differ (the first few), for failure messages.
fn differences(a: &Value, b: &Value) -> Vec<String> {
    fn rec(a: &Value, b: &Value, at: String, out: &mut Vec<String>) {
        if out.len() >= 8 || a == b {
            return;
        }
        match (a, b) {
            (Value::Object(x), Value::Object(y)) => {
                let keys: std::collections::BTreeSet<&String> = x.keys().chain(y.keys()).collect();
                for k in keys {
                    rec(x.get(k).unwrap_or(&Value::Null), y.get(k).unwrap_or(&Value::Null), format!("{at}.{k}"), out);
                }
            }
            (Value::Array(x), Value::Array(y)) if x.len() == y.len() => {
                for (i, (p, q)) in x.iter().zip(y).enumerate() {
                    rec(p, q, format!("{at}[{i}]"), out);
                }
            }
            _ => {
                let short = |v: &Value| v.to_string().chars().take(160).collect::<String>();
                out.push(format!("{at}: {} != {}", short(a), short(b)));
            }
        }
    }
    let mut out = Vec::new();
    rec(a, b, String::new(), &mut out);
    out
}
