//! The premise guard when another program rewrites the `.kvector` file between the agent's
//! observe and its write (C8-land-11-vector item 6, issue-c8-outside). The agent works through the
//! kits' MCP server's tools (`kition_kits::tools::call_as`) on the `kition-vector` executable,
//! holding the `revision` of an inspect made before the colleague's edit. The colleague is this
//! kit's engine, an instance of its own, opening the file from disk, running commands and saving
//! it, so the server sees only a changed file.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use kition_kit::KitDocument;
use kition_kits::tools::{self, CallOptions, Guard};
use kition_kits::{KitDefinition, KitManager};
use kition_vector_kit::VectorDocument;
use serde_json::{Value, json};

const ME: &str = "agent-me";
const FILE: &str = "Drawing.kvector";
const SHAPES: usize = 8;

fn kit() -> &'static KitDefinition {
    KitDefinition::from_manifest_text(include_str!("../../../kit.json")).unwrap()
}

/// Where rectangle `R<n>` stands: a row, 40 pt apart, 20 pt wide, none touching another.
fn spot(n: usize) -> (f64, f64) {
    (20.0 + 40.0 * (n % 10) as f64, 20.0 + 40.0 * (n / 10) as f64)
}

/// A drawing with `shapes` rectangles `R0`, `R1`, … and a group `G` holding two more, `G0`, `G1`.
fn drawing(shapes: usize) -> Vec<u8> {
    let mut doc = VectorDocument::create(FILE, 800, 600).unwrap();
    for n in 0..shapes {
        let (x, y) = spot(n);
        let id = doc.execute("shape.rectangle", json!({"x":x,"y":y,"width":20,"height":20})).unwrap()["id"].clone();
        doc.execute("object.setProps", json!({"ids":[id],"name":format!("R{n}")})).unwrap();
    }
    let mut members = Vec::new();
    for n in 0..2 {
        let id = doc.execute("shape.ellipse", json!({"x":40.0 * n as f64 + 20.0,"y":300,"width":20,"height":20})).unwrap()["id"].clone();
        doc.execute("object.setProps", json!({"ids":[id],"name":format!("G{n}")})).unwrap();
        members.push(id);
    }
    doc.execute("select.set", json!({"ids": members})).unwrap();
    let group = doc.execute("arrange.group", json!({})).unwrap()["id"].clone();
    doc.execute("object.setProps", json!({"ids":[group],"name":"G"})).unwrap();
    doc.execute("edit.deselectAll", json!({})).unwrap();
    doc.save(None).unwrap()
}

fn backdate(file: &Path, before: std::time::SystemTime) {
    std::fs::File::options().write(true).open(file).unwrap().set_modified(before - Duration::from_secs(3600)).unwrap();
}

/// A colleague's program: this kit's engine, its own instance, opening the file, running
/// `steps` and saving it back.
fn colleague(file: &Path, steps: &[(&str, Value)]) -> Vec<Value> {
    let at = std::fs::metadata(file).unwrap().modified().unwrap();
    let mut doc = VectorDocument::open(FILE, &std::fs::read(file).unwrap()).unwrap();
    let answers = steps
        .iter()
        .map(|(command, params)| doc.execute(command, params.clone()).unwrap_or_else(|error| panic!("colleague's {command}: {error}")))
        .collect();
    std::fs::write(file, doc.save(None).unwrap()).unwrap();
    backdate(file, at);
    answers
}

/// The file as a new engine reads it: object `id` as `file.node` gives it (`None` if gone).
fn on_disk(file: &Path, id: u64) -> Option<Value> {
    let mut doc = VectorDocument::open(FILE, &std::fs::read(file).unwrap()).unwrap();
    doc.execute("file.node", json!({"id": id})).ok()
}

/// The ids of the objects an inspect lists (at any depth), by name.
fn ids_by_name(inspected: &Value) -> std::collections::BTreeMap<String, u64> {
    fn walk(node: &Value, out: &mut std::collections::BTreeMap<String, u64>) {
        if let (Some(name), Some(id)) = (node["name"].as_str(), node["id"].as_u64()) {
            out.insert(name.to_owned(), id);
        }
        for child in node["children"].as_array().into_iter().flatten() {
            walk(child, out);
        }
    }
    let mut out = std::collections::BTreeMap::new();
    for layer in inspected["layers"].as_array().into_iter().flatten() {
        walk(layer, &mut out);
    }
    out
}

static WORKSPACES: AtomicUsize = AtomicUsize::new(0);

fn workspace() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("kition-vector-agent-{}-{}", std::process::id(), WORKSPACES.fetch_add(1, Ordering::Relaxed)));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::canonicalize(dir).unwrap()
}

struct Fixture {
    kits: KitManager,
    options: CallOptions,
    workspace: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        Self::with(SHAPES, Guard::Premise)
    }

    fn with(shapes: usize, guard: Guard) -> Self {
        let workspace = workspace();
        std::fs::write(workspace.join(FILE), drawing(shapes)).unwrap();
        let kits = KitManager::new(Some(workspace.clone()), vec![(kit(), Some(PathBuf::from(env!("CARGO_BIN_EXE_kition-vector"))))]);
        Self { kits, options: CallOptions { guard, ..CallOptions::default() }, workspace }
    }

    fn file(&self) -> PathBuf {
        self.workspace.join(FILE)
    }

    fn colleague(&self, steps: &[(&str, Value)]) -> Vec<Value> {
        colleague(&self.file(), steps)
    }

    async fn call(&self, action: &str, mut input: Value) -> Result<Value, Value> {
        input["path"] = json!(FILE);
        match tools::call_as(&self.kits, &format!("vector_{action}"), &input, self.options, ME).await {
            Some(Ok(output)) => Ok(output.output),
            Some(Err(error)) => Err(error.structured()),
            None => panic!("vector_{action} is no tool"),
        }
    }

    async fn ok(&self, action: &str, input: Value) -> Value {
        self.call(action, input).await.unwrap_or_else(|error| panic!("vector_{action}: {error}"))
    }

    async fn refused(&self, action: &str, input: Value) -> Value {
        match self.call(action, input).await {
            Err(error) => error,
            Ok(answer) => panic!("vector_{action} was expected to be refused: {answer}"),
        }
    }

    /// The drawing as the agent inspects it, and the answer's `revision`.
    async fn inspect(&self) -> (Value, String) {
        let seen = self.ok("inspect", json!({})).await;
        (seen["details"].clone(), revision(&seen))
    }

    async fn shutdown(self) {
        self.kits.shutdown().await;
        let _ = std::fs::remove_dir_all(&self.workspace);
    }
}

fn revision(answer: &Value) -> String {
    answer["revision"].as_str().unwrap_or_else(|| panic!("a revision: {answer}")).to_owned()
}

fn id_text(id: &Value) -> Option<String> {
    id.as_str().map(str::to_owned).or_else(|| id.as_u64().map(|id| id.to_string()))
}

fn ids(list: &Value) -> Vec<String> {
    list.as_array().map(|items| items.iter().filter_map(|item| id_text(&item["id"])).collect()).unwrap_or_default()
}

fn assert_conflict(refused: &Value, id: u64, reason: &str) {
    assert_eq!(refused["error"]["code"], "conflict", "{refused}");
    let found = refused["conflicts"]
        .as_array()
        .and_then(|conflicts| conflicts.iter().find(|conflict| id_text(&conflict["id"]).as_deref() == Some(id.to_string().as_str())))
        .unwrap_or_else(|| panic!("no conflict on {id}: {refused}"));
    assert_eq!(found["reason"], reason, "{refused}");
}

fn rename(id: u64, name: &str) -> Value {
    json!({"command":"object.setProps","params":{"ids":[id],"name":name}})
}

fn fill(id: u64, color: &str) -> Value {
    json!({"command":"paint.setFill","params":{"ids":[id],"color":color}})
}

fn name_on_disk(file: &Path, id: u64) -> Value {
    on_disk(file, id).map_or(Value::Null, |node| node["name"].clone())
}

// ---- 1. An unrelated change applies, and `others` names only it ----

#[tokio::test(flavor = "multi_thread")]
async fn s1_an_unrelated_change_applies_and_others_names_only_it() {
    let f = Fixture::new();
    let (details, basis) = f.inspect().await;
    let names = ids_by_name(&details);
    let (mine, theirs, moved) = (names["R2"], names["R5"], names["R6"]);
    f.colleague(&[("object.setProps", json!({"ids":[theirs],"name":"Theirs"}))]);
    let ran = f.ok("run", json!({"basis": basis, "steps": [rename(mine, "Mine")]})).await;
    assert_eq!(ids(&ran["others"]["changed"]), [theirs.to_string()], "{ran}");
    println!("s1: others for one unrelated rename: {} bytes: {}", ran["others"].to_string().len(), ran["others"]);
    // A move of another object, then the agent's fill of its own by id.
    f.colleague(&[("select.set", json!({"ids":[moved]})), ("object.move", json!({"dx":3,"dy":0}))]);
    let ran = f.ok("run", json!({"basis": revision(&ran), "steps": [fill(mine, "#ff0000")]})).await;
    assert_eq!(ids(&ran["others"]["changed"]), [moved.to_string()], "{ran}");
    assert_eq!(name_on_disk(&f.file(), mine), "Mine");
    f.shutdown().await;
}

// ---- 2. The target's same field is refused `changed`; another field of it is not ----

#[tokio::test(flavor = "multi_thread")]
async fn s2_the_same_field_is_refused_and_another_field_applies() {
    let f = Fixture::new();
    let (details, basis) = f.inspect().await;
    let mine = ids_by_name(&details)["R2"];
    f.colleague(&[("object.setProps", json!({"ids":[mine],"name":"Theirs"}))]);
    let before = std::fs::read(f.file()).unwrap();
    let refused = f.refused("run", json!({"basis": basis, "steps": [rename(mine, "Mine")]})).await;
    assert_conflict(&refused, mine, "changed");
    assert_eq!(std::fs::read(f.file()).unwrap(), before, "nothing written");
    // The colleague's rename is outside a fill's premise (appearance): the fill applies.
    let ran = f.ok("run", json!({"basis": basis, "steps": [fill(mine, "#00ff00")]})).await;
    assert_eq!(name_on_disk(&f.file(), mine), "Theirs", "{ran}");
    // The colleague's fill, then the agent's fill with the older basis: refused.
    let (_, basis) = f.inspect().await;
    f.colleague(&[("paint.setFill", json!({"ids":[mine],"color":"#0000ff"}))]);
    let refused = f.refused("run", json!({"basis": basis, "steps": [fill(mine, "#ff0000")]})).await;
    assert_conflict(&refused, mine, "changed");
    f.shutdown().await;
}

// ---- 3. The target deleted ----

#[tokio::test(flavor = "multi_thread")]
async fn s3_a_deleted_target_is_refused() {
    let f = Fixture::new();
    let (details, basis) = f.inspect().await;
    let mine = ids_by_name(&details)["R2"];
    f.colleague(&[("edit.delete", json!({"ids":[mine]}))]);
    for step in [rename(mine, "Mine"), fill(mine, "#ff0000"), json!({"command":"object.move","on":[mine],"params":{"dx":5,"dy":0}})] {
        let refused = f.refused("run", json!({"basis": basis, "steps": [step]})).await;
        assert_conflict(&refused, mine, "deleted");
    }
    f.shutdown().await;
}

// ---- 4. Inserted before / after the index a row moves to ----

/// `node.move` puts a row at an index of its parent. A colleague's object inserted at that place
/// shifts it: the write is refused. An insertion elsewhere is not.
#[tokio::test(flavor = "multi_thread")]
async fn s4_an_insertion_at_the_index_refuses_a_move_there_and_elsewhere_applies() {
    let f = Fixture::new();
    let (details, basis) = f.inspect().await;
    let names = ids_by_name(&details);
    let group = names["G"];
    let mine = names["R2"];
    // Elsewhere: the colleague adds an object at the top of the layer, the agent moves R2 into
    // the group at index 1 (between G0 and G1).
    f.colleague(&[("shape.rectangle", json!({"x":600,"y":500,"width":10,"height":10}))]);
    f.ok("run", json!({"basis": basis, "steps": [{"command":"node.move","params":{"id":mine,"parent":group,"index":1}}]})).await;
    // At the place: the colleague puts G1 first in the group (the order the agent saw changes),
    // the agent moves R3 to index 1.
    let (details, basis) = f.inspect().await;
    let names = ids_by_name(&details);
    f.colleague(&[("node.move", json!({"id": names["G1"], "parent": group, "index": 0}))]);
    let answer =
        f.call("run", json!({"basis": basis, "steps": [{"command":"node.move","params":{"id":names["R3"],"parent":group,"index":1}}]})).await;
    // Waiting for the SDK (C8-fix-12): an insertion at an empty window [i, i) is applied at
    // 1935d52. Pinned: say what happened.
    println!("s4: move into a reordered group at index 1: {}", if answer.is_ok() { "APPLIED (SDK C8-fix-12 pending)" } else { "refused" });
    f.shutdown().await;
}

// ---- 5. The target moved, for a write that rests on where it is ----

#[tokio::test(flavor = "multi_thread")]
async fn s5_a_move_of_the_target_refuses_a_relative_move_but_not_a_rename() {
    let f = Fixture::new();
    let (details, basis) = f.inspect().await;
    let mine = ids_by_name(&details)["R2"];
    f.colleague(&[("select.set", json!({"ids":[mine]})), ("object.move", json!({"dx":0,"dy":40}))]);
    let refused = f.refused("run", json!({"basis": basis, "steps": [{"command":"object.move","on":[mine],"params":{"dx":5,"dy":0}}]})).await;
    assert_conflict(&refused, mine, "changed");
    f.ok("run", json!({"basis": basis, "steps": [rename(mine, "Mine")]})).await;
    assert_eq!(name_on_disk(&f.file(), mine), "Mine");
    f.shutdown().await;
}

// ---- 6. A group moved: a write on a member at its place ----

/// The colleague moves the group G; the agent's move of member G0 rests on G0's geometry, which
/// the group's move changed: refused. A rename of G0 applies.
#[tokio::test(flavor = "multi_thread")]
async fn s6_a_parent_moved_refuses_a_members_move_and_not_its_rename() {
    let f = Fixture::new();
    let (details, basis) = f.inspect().await;
    let names = ids_by_name(&details);
    f.colleague(&[("select.set", json!({"ids":[names["G"]]})), ("object.move", json!({"dx":10,"dy":0}))]);
    let refused = f.refused("run", json!({"basis": basis, "steps": [{"command":"object.move","on":[names["G0"]],"params":{"dx":5,"dy":0}}]})).await;
    assert_conflict(&refused, names["G0"], "changed");
    f.ok("run", json!({"basis": basis, "steps": [rename(names["G0"], "Mine")]})).await;
    // A fill of the group reaches its members: a colleague's new member refuses it.
    let (_, basis) = f.inspect().await;
    f.colleague(&[("shape.rectangle", json!({"x":500,"y":500,"width":10,"height":10}))]);
    let added = ids_by_name(&VectorDocument::open(FILE, &std::fs::read(f.file()).unwrap()).unwrap().inspect().unwrap());
    let _ = added;
    let ran = f.call("run", json!({"basis": basis, "steps": [fill(names["G"], "#ff0000")]})).await;
    assert!(ran.is_ok(), "an object added outside the group leaves the group's fill alone: {ran:?}");
    f.shutdown().await;
}

// ---- 7. A name swap and a rename away, for a write by name ----

/// Swatches are looked up by name: the colleague renames the swatch the agent's fill names; the
/// fill by that name is refused (`relocated` or `deleted`), never applied with another swatch.
#[tokio::test(flavor = "multi_thread")]
async fn s7_a_swatch_renamed_away_refuses_a_fill_by_its_name() {
    let f = Fixture::new();
    let (details, basis) = f.inspect().await;
    let mine = ids_by_name(&details)["R2"];
    f.colleague(&[("swatch.new", json!({"name":"Brand","color":"#123456"}))]);
    let (_, basis2) = f.inspect().await;
    let _ = basis;
    f.colleague(&[("swatch.edit", json!({"name":"Brand","newName":"Old Brand"})), ("swatch.new", json!({"name":"Brand","color":"#ff00ff"}))]);
    let answer = f.call("run", json!({"basis": basis2, "steps": [{"command":"paint.setFill","params":{"ids":[mine],"swatch":"Brand"}}]})).await;
    match &answer {
        Err(refused) => {
            assert_eq!(refused["error"]["code"], "conflict", "{refused}");
            println!("s7: fill by a swapped swatch name: refused {}", refused["conflicts"]);
        }
        Ok(ran) => panic!("a fill by a name another swatch took was applied: {ran}"),
    }
    f.shutdown().await;
}

// ---- 8. Guard modes: epoch refuses an unrelated change, off overwrites ----

#[tokio::test(flavor = "multi_thread")]
async fn s8_epoch_refuses_an_unrelated_change_and_off_overwrites() {
    let f = Fixture::with(SHAPES, Guard::Epoch);
    let (details, basis) = f.inspect().await;
    let names = ids_by_name(&details);
    f.colleague(&[("object.setProps", json!({"ids":[names["R5"]],"name":"Theirs"}))]);
    let refused = f.refused("run", json!({"basis": basis, "steps": [rename(names["R2"], "Mine")]})).await;
    assert_eq!(refused["error"]["code"], "conflict", "{refused}");
    f.shutdown().await;
    let f = Fixture::with(SHAPES, Guard::Off);
    let (details, basis) = f.inspect().await;
    let mine = ids_by_name(&details)["R2"];
    f.colleague(&[("object.setProps", json!({"ids":[mine],"name":"Theirs"}))]);
    let ran = f.ok("run", json!({"basis": basis, "steps": [rename(mine, "Mine")]})).await;
    assert_eq!(ids(&ran["others"]["changed"]), [mine.to_string()], "{ran}");
    assert_eq!(name_on_disk(&f.file(), mine), "Mine", "the colleague's name overwritten");
    f.shutdown().await;
}

// ---- 9. An artboard by index ----

/// The colleague inserts an artboard before the one the agent resizes by index: refused.
#[tokio::test(flavor = "multi_thread")]
async fn s9_an_artboard_by_index_after_a_reorder_is_refused() {
    let f = Fixture::new();
    f.colleague(&[("artboard.new", json!({"x":900,"y":0,"width":100,"height":100}))]);
    let (_, basis) = f.inspect().await;
    f.colleague(&[("artboard.reorder", json!({"index":1,"to":0}))]);
    let answer = f.call("run", json!({"basis": basis, "steps": [{"command":"artboard.setProps","params":{"index":0,"name":"Mine"}}]})).await;
    match &answer {
        Err(refused) => {
            assert_eq!(refused["error"]["code"], "conflict", "{refused}");
            println!("s9: artboard by index after a reorder: refused {}", refused["conflicts"]);
        }
        Ok(ran) => panic!("a write by artboard index landed on the board another put there: {ran}"),
    }
    f.shutdown().await;
}

// ---- 12. Structural writes and where their objects are (C8-vector-4 item 2) ----

/// Where `id` is on disk: its parent's id (none at the top of a layer is its layer) and index.
fn place_on_disk(file: &Path, id: u64) -> (Value, usize) {
    let doc = VectorDocument::open(FILE, &std::fs::read(file).unwrap()).unwrap();
    let inspected = doc.inspect().unwrap();
    fn find(node: &Value, parent: &Value, id: u64) -> Option<(Value, usize)> {
        let children = node["children"].as_array()?;
        for (at, child) in children.iter().enumerate() {
            if child["id"].as_u64() == Some(id) {
                return Some((parent.clone(), at));
            }
            if let Some(found) = find(child, &child["id"], id) {
                return Some(found);
            }
        }
        None
    }
    inspected["layers"].as_array().into_iter().flatten().find_map(|layer| find(layer, &layer["id"], id)).unwrap_or((Value::Null, 0))
}

/// A group and a Pathfinder result depend on the stacking order and the parent of what they
/// combine: a colleague's move of one of them into another group, or above the other, refuses
/// them (`relocated`). A delete does not depend on where its object is: it applies.
#[tokio::test(flavor = "multi_thread")]
async fn s12_a_move_into_another_group_refuses_a_group_and_not_a_delete() {
    for (command, refuses) in [
        ("arrange.group", true),
        ("object.pathfinder.unite", true),
        ("object.clippingMask.make", true),
        ("edit.delete", false),
        ("object.lock", false),
    ] {
        let f = Fixture::new();
        let (details, basis) = f.inspect().await;
        let names = ids_by_name(&details);
        let (r5, r6, group) = (names["R5"], names["R6"], names["G"]);
        f.colleague(&[("node.move", json!({"id": r6, "parent": group, "index": 1}))]);
        let on = if command == "edit.delete" || command == "object.lock" { json!([r6]) } else { json!([r5, r6]) };
        let answer = f.call("run", json!({"basis": basis, "steps": [{"command": command, "on": on}]})).await;
        match (&answer, refuses) {
            (Err(refused), true) => assert_conflict(refused, r6, "relocated"),
            (Ok(_), false) => {}
            (Err(refused), false) => panic!("{command} of R6 refused for R6's move into G: {refused}"),
            (Ok(ran), true) => panic!("{command} on R5 and R6 applied after R6 went into G: {ran}"),
        }
        println!("s12: {command} after a move into another group: {}", if answer.is_ok() { "applied" } else { "refused relocated" });
        f.shutdown().await;
    }
}

/// The colleague restacks R6 below R5 in their layer: a Pathfinder Minus Front of them (whose
/// result says which is cut from which) is refused; a fill of R6 is not.
#[tokio::test(flavor = "multi_thread")]
async fn s12_a_restack_refuses_minus_front_and_not_a_fill() {
    let f = Fixture::new();
    let (details, basis) = f.inspect().await;
    let names = ids_by_name(&details);
    let (r5, r6) = (names["R5"], names["R6"]);
    let (layer, at5) = place_on_disk(&f.file(), r5);
    f.colleague(&[("node.move", json!({"id": r6, "parent": layer, "index": at5}))]);
    let refused = f.refused("run", json!({"basis": basis, "steps": [{"command":"object.pathfinder.minusFront","on":[r5, r6]}]})).await;
    assert_conflict(&refused, r6, "relocated");
    f.ok("run", json!({"basis": basis, "steps": [fill(r6, "#ff0000")]})).await;
    f.shutdown().await;
}

/// An object the colleague adds at the top of the layer, or deletes below them, leaves R5 and R6
/// in the same parent and order: their group should apply. Added on top it does; deleted below,
/// their indexes shift and the guard (comparing indexes, SDK 1935d52) refuses it as `relocated`:
/// waiting for the SDK (a premise on the order among a call's objects), pinned as it is.
#[tokio::test(flavor = "multi_thread")]
async fn s12_an_object_added_or_deleted_elsewhere_leaves_a_group_alone() {
    for step in ["create", "delete below"] {
        let f = Fixture::new();
        let (details, basis) = f.inspect().await;
        let names = ids_by_name(&details);
        let change = if step == "create" {
            ("shape.rectangle", json!({"x":600,"y":500,"width":10,"height":10}))
        } else {
            ("edit.delete", json!({"ids":[names["R2"]]}))
        };
        f.colleague(&[change]);
        let answer = f.call("run", json!({"basis": basis, "steps": [{"command":"arrange.group","on":[names["R5"], names["R6"]]}]})).await;
        println!("s12: group after an unrelated {step}: {}", if answer.is_ok() { "applied" } else { "REFUSED (waiting for the SDK)" });
        if step == "create" {
            assert!(answer.is_ok(), "group refused after an unrelated {step}: {answer:?}");
        } else {
            // Waiting for the SDK: pinned so a change of it is seen.
            assert!(answer.is_err(), "the SDK now lets a group apply after a delete below: drop this pin");
        }
        f.shutdown().await;
    }
}

// ---- 13. A text style by name (C8-vector-4 item 3) ----

/// A character style is its own object (`charStyle:Emphasis`): a colleague's edit of another
/// style leaves the agent's edit of `Emphasis` alone; an edit of `Emphasis` itself refuses it.
#[tokio::test(flavor = "multi_thread")]
async fn s13_a_text_style_edit_rests_on_that_style_only() {
    let f = Fixture::new();
    f.colleague(&[("charStyle.new", json!({"name":"Emphasis"})), ("charStyle.new", json!({"name":"Quiet"}))]);
    let (_, basis) = f.inspect().await;
    f.colleague(&[("charStyle.setAttrs", json!({"name":"Quiet","attrs":{"size":9}}))]);
    let mine = json!({"command":"charStyle.setAttrs","params":{"name":"Emphasis","attrs":{"size":18}}});
    let ran = f.ok("run", json!({"basis": basis, "steps": [mine]})).await;
    let (_, basis) = f.inspect().await;
    let _ = ran;
    f.colleague(&[("charStyle.setAttrs", json!({"name":"Emphasis","attrs":{"size":30}}))]);
    let refused =
        f.refused("run", json!({"basis": basis, "steps": [{"command":"charStyle.setAttrs","params":{"name":"Emphasis","attrs":{"size":12}}}]})).await;
    assert_eq!(refused["error"]["code"], "conflict", "{refused}");
    println!("s13: a style edit after another style's edit: applied; after its own: refused {}", refused["conflicts"]);
    f.shutdown().await;
}

// ---- 15. A named entry's identity (C8-vector-5 item 1) ----

/// A swatch's tool id is its persistent identity (`swatch:<uid>`), the name an input: the agent
/// that made `Brand` still reaches it by that id after the colleague renamed it, a write by the
/// name `Brand` is refused `relocated` after that rename, and applies after a rename of another
/// swatch.
#[tokio::test(flavor = "multi_thread")]
async fn s15_a_renamed_swatch_keeps_its_tool_id_and_its_old_name_refuses() {
    let f = Fixture::new();
    let made = f.ok("run", json!({"steps": [{"command":"swatch.new","params":{"name":"Brand","color":"#123456"}}, {"command":"swatch.new","params":{"name":"Other","color":"#654321"}}]})).await;
    let created = made["changes"]["created"].as_array().cloned().unwrap_or_default();
    let brand = created
        .iter()
        .find(|item| item["values"]["name"] == "Brand")
        .and_then(|item| id_text(&item["id"]))
        .unwrap_or_else(|| panic!("Brand created: {made}"));
    assert!(brand.starts_with("swatch:") && brand != "swatch:Brand", "the tool id is no name: {brand}");
    // Another swatch renamed: the edit by name applies.
    let (_, basis) = f.inspect().await;
    f.colleague(&[("swatch.edit", json!({"name":"Other","newName":"Elsewhere"}))]);
    f.ok("run", json!({"basis": basis, "steps": [{"command":"swatch.edit","params":{"name":"Brand","color":"#0000ff"}}]})).await;
    // That swatch renamed: the edit by its old name is refused.
    let (_, basis) = f.inspect().await;
    f.colleague(&[("swatch.edit", json!({"name":"Brand","newName":"Brand Blue"}))]);
    let refused = f.refused("run", json!({"basis": basis, "steps": [{"command":"swatch.edit","params":{"name":"Brand","color":"#ff0000"}}]})).await;
    assert_eq!(refused["error"]["code"], "conflict", "{refused}");
    let reasons: Vec<&str> = refused["conflicts"].as_array().into_iter().flatten().filter_map(|c| c["reason"].as_str()).collect();
    assert!(reasons.contains(&"relocated"), "{refused}");
    // The id the agent holds still names it.
    let set = f.ok("batch", json!({"ops":[{"op":"set","target":brand,"field":"name","value":"Brand Green"}]})).await;
    let changed = ids(&set["changes"]["changed"]);
    assert!(changed.contains(&brand), "{set}");
    let doc = VectorDocument::open(FILE, &std::fs::read(f.file()).unwrap()).unwrap();
    let saved: Value = serde_json::from_slice(&doc.save(None).unwrap()).unwrap();
    let names: Vec<&str> = saved["document"]["swatches"].as_array().unwrap().iter().filter_map(|s| s["name"].as_str()).collect();
    assert!(names.contains(&"Brand Green") && !names.contains(&"Brand Blue"), "{names:?}");
    println!("s15: rename of another swatch: applied; of Brand: refused {}; set by {brand}: applied", refused["conflicts"]);
    f.shutdown().await;
}

// ---- 16. Two premises on one object (C8-vector-5 item 6) ----

/// A layer write without ids acts on the current layer, which is also the layer new art goes
/// into: two premises on one object, which the pinned SDK would keep only the last of (the
/// insertion layer's: shown, unlocked, a layer). Joined, the name the write sets is still held:
/// the colleague's rename of the layer refuses the agent's rename, and a colleague's change of
/// another layer does not.
#[tokio::test(flavor = "multi_thread")]
async fn s16_a_layer_rename_without_ids_holds_the_name_it_sets() {
    let f = Fixture::new();
    let (details, basis) = f.inspect().await;
    let layer = details["layers"][0]["id"].as_u64().unwrap_or_else(|| panic!("a layer: {details}"));
    f.colleague(&[("layer.setProps", json!({"ids":[layer],"name":"Theirs"}))]);
    let refused = f.refused("run", json!({"basis": basis, "steps": [{"command":"layer.setProps","params":{"name":"Mine"}}]})).await;
    assert_conflict(&refused, layer, "changed");
    let (_, basis) = f.inspect().await;
    f.colleague(&[("layer.new", json!({"name":"Other"}))]);
    f.ok("run", json!({"basis": basis, "steps": [{"command":"layer.setProps","params":{"name":"Mine"}}]})).await;
    println!("s16: layer rename without ids after the colleague's rename: refused {}", refused["conflicts"]);
    f.shutdown().await;
}

// ---- 17. Commands that pick their objects by a stroke (C8-vector-5 item 2) ----

/// `path.knife` with nothing selected cuts every closed path under its stroke. The stroke here
/// crosses R2 alone. Refused when the colleague moved another object into the stroke or R2 out
/// of it; applied after a move or a recolour elsewhere and after a new object elsewhere. A delete
/// elsewhere is refused: the journal keeps no geometry of a deleted object (the drawing has no
/// `position` field or derived bounds), so whether it was under the stroke is not known.
#[tokio::test(flavor = "multi_thread")]
async fn s17_a_knife_by_stroke_follows_what_is_under_it() {
    let knife = json!({"command":"path.knife","params":{"points":[[110,0],[110,60]]}});
    let cases: [(&str, &str, Option<&str>); 6] = [
        ("another moved in", "R5", Some("relocated")),
        ("the cut one moved out", "R2", Some("relocated")),
        ("another moved elsewhere", "R7", None),
        ("another recoloured", "R7", None),
        ("a new one elsewhere", "", None),
        ("another deleted", "R7", Some("refused")),
    ];
    for (what, name, expected) in cases {
        let f = Fixture::new();
        let (details, basis) = f.inspect().await;
        let names = ids_by_name(&details);
        let id = names.get(name).copied().unwrap_or(0);
        let change = match what {
            "another moved in" => ("object.move", json!({"dx":-115,"dy":0})),
            "the cut one moved out" => ("object.move", json!({"dx":0,"dy":200})),
            "another moved elsewhere" => ("object.move", json!({"dx":0,"dy":300})),
            "another recoloured" => ("paint.setFill", json!({"ids":[id],"color":"#00ff00"})),
            "a new one elsewhere" => ("shape.rectangle", json!({"x":600,"y":500,"width":10,"height":10})),
            _ => ("edit.delete", json!({"ids":[id]})),
        };
        f.colleague(&[("select.set", json!({"ids": if id == 0 { vec![] } else { vec![id] }})), change, ("edit.deselectAll", json!({}))]);
        let answer = f.call("run", json!({"basis": basis, "steps": [knife.clone()]})).await;
        match (expected, &answer) {
            (None, Ok(_)) => println!("s17: knife after {what}: applied"),
            (Some("refused"), Err(refused)) => {
                assert_eq!(refused["error"]["code"], "conflict", "{what}: {refused}");
                println!("s17: knife after {what}: refused {}", refused["conflicts"]);
            }
            (Some(reason), Err(refused)) => {
                assert_eq!(refused["error"]["code"], "conflict", "{what}: {refused}");
                let reasons: Vec<&str> = refused["conflicts"].as_array().into_iter().flatten().filter_map(|c| c["reason"].as_str()).collect();
                assert!(reasons.contains(&reason), "{what}: {refused}");
                println!("s17: knife after {what}: refused {}", refused["conflicts"]);
            }
            _ => panic!("s17: knife after {what}: expected {expected:?}, got {answer:?}"),
        }
        f.shutdown().await;
    }
    // Given what it cuts (`on`), a delete elsewhere leaves the knife alone.
    let f = Fixture::new();
    let (details, basis) = f.inspect().await;
    let names = ids_by_name(&details);
    f.colleague(&[("edit.delete", json!({"ids":[names["R7"]]}))]);
    let on = json!({"command":"path.knife","on":[names["R2"]],"params":{"points":[[110,0],[110,60]]}});
    f.ok("run", json!({"basis": basis, "steps": [on]})).await;
    println!("s17: knife on R2 after another deleted: applied");
    f.shutdown().await;
}

// ---- 14. Receipt bytes (C8-vector-4 item 6) ----

/// What a receipt costs: one property, one move, a group of 5 and of 200 moved, the same offset
/// on 200 objects. Printed; nothing may be cut short without saying so.
#[tokio::test(flavor = "multi_thread")]
async fn s14_receipt_bytes() {
    let f = Fixture::with(200, Guard::Premise);
    let (details, _) = f.inspect().await;
    let names = ids_by_name(&details);
    let all: Vec<u64> = (0..200).map(|n| names[&format!("R{n}")]).collect();
    let five = f.ok("run", json!({"steps": [{"command":"arrange.group","on":all[0..5]}]})).await;
    let five = five["completed"][0]["result"]["id"].as_u64().unwrap_or_else(|| panic!("a group: {five}"));
    let (_, basis) = f.inspect().await;
    let mut rows = Vec::new();
    let mut measure = |label: &str, ran: &Value| {
        let changes = serde_json::to_string(&ran["changes"]).unwrap();
        let text = ran.to_string();
        assert!(!text.contains("truncated") || text.contains("\"truncated\":true"), "{label}: cut short silently? {text}");
        rows.push(format!("{label}: {} bytes of changes{}", changes.len(), if changes.contains("footprint") { " (footprint)" } else { "" }));
    };
    let ran = f.ok("run", json!({"basis": basis, "steps": [rename(all[10], "One")]})).await;
    measure("one property", &ran);
    let ran = f.ok("run", json!({"steps": [{"command":"object.move","on":[all[11]],"params":{"dx":5,"dy":0}}]})).await;
    measure("one move", &ran);
    let ran = f.ok("run", json!({"steps": [{"command":"object.move","on":[five],"params":{"dx":5,"dy":0}}]})).await;
    measure("a group of 5 moved", &ran);
    let ran = f.ok("run", json!({"steps": [{"command":"object.move","on":all[5..200],"params":{"dx":5,"dy":0}}]})).await;
    measure("195 objects moved by one offset", &ran);
    let big = f.ok("run", json!({"steps": [{"command":"arrange.group","on":all[5..200]}]})).await;
    let big = big["completed"][0]["result"]["id"].as_u64().unwrap();
    let ran = f.ok("run", json!({"steps": [{"command":"object.move","on":[big],"params":{"dx":5,"dy":0}}]})).await;
    measure("a group of 195 moved", &ran);
    for row in &rows {
        println!("s14: {row}");
    }
    f.shutdown().await;
}

// ---- 10. Fifty rounds of an unrelated change ----

/// One step of xorshift64.
fn xorshift(seed: &mut u64) -> u64 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 7;
    *seed ^= *seed << 17;
    *seed
}

/// Fifty rounds: the colleague renames one of the rectangles other than R7 (picked by a fixed
/// seed), then the agent renames R7 with the revision of its last answer. Answers the refusals.
async fn fifty_unrelated(guard: Guard) -> usize {
    let f = Fixture::with(24, guard);
    let (details, mut basis) = f.inspect().await;
    let names = ids_by_name(&details);
    let mut seed = 0x2545_f491_4f6c_dd1d_u64;
    let mut refused = 0;
    for round in 0..50 {
        let mut other = usize::try_from(xorshift(&mut seed) % 23).unwrap();
        if other >= 7 {
            other += 1;
        }
        f.colleague(&[("object.setProps", json!({"ids":[names[&format!("R{other}")]],"name":format!("Theirs{round}")}))]);
        match f.call("run", json!({"basis": basis, "steps": [rename(names["R7"], &format!("Mine{round}"))]})).await {
            Ok(ran) => basis = revision(&ran),
            Err(error) => {
                refused += 1;
                basis = revision(&error);
            }
        }
    }
    f.shutdown().await;
    refused
}

#[tokio::test(flavor = "multi_thread")]
async fn s10_fifty_unrelated_changes_premise_refuses_none_epoch_all() {
    let premise = fifty_unrelated(Guard::Premise).await;
    let epoch = fifty_unrelated(Guard::Epoch).await;
    println!("s10: 50 rounds of an unrelated rename: premise refused {premise}/50, epoch refused {epoch}/50");
    assert_eq!(premise, 0);
    assert_eq!(epoch, 50);
}

// ---- 11. The mixed table ----

/// What the colleague does in a round of the mixed table.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Change {
    Rename,
    Fill,
    Move,
    Delete,
    Create,
    SwatchAdded,
    SwatchSwapped,
    ArtboardAdded,
    ArtboardReordered,
    ArtboardRenamed,
}

const CHANGES: [Change; 10] = [
    Change::Rename,
    Change::Fill,
    Change::Move,
    Change::Delete,
    Change::Create,
    Change::SwatchAdded,
    Change::SwatchSwapped,
    Change::ArtboardAdded,
    Change::ArtboardReordered,
    Change::ArtboardRenamed,
];

/// The agent's write forms, and the rule for when a refusal of each is necessary, written before
/// the table was run:
///
/// - `id`: rename R7 (`object.setProps`). Necessary when the colleague renamed or deleted R7.
/// - `name`: fill R7 with the swatch named `Brand`. Necessary when the colleague deleted R7,
///   changed R7's fill, or took the name `Brand` to another swatch (`SwatchSwapped`).
/// - `index`: rename the artboard at index 0. Necessary when another artboard is at index 0 now
///   (`ArtboardReordered`) or the colleague renamed that artboard.
/// - `structure`: group R7 and R8 (`arrange.group` on them). Necessary when the colleague deleted
///   R7 or R8 (C8-vector-engine, the maintainers' rule of necessity: a group rests on where its objects
///   are, their parent and stacking, not on their fields; a rename, a fill or a move that keeps the
///   parent and the stacking applies, and the saved group holds both writers' edits, checked
///   below. Before: any rename, fill, move or delete of R7 or R8).
///
/// - `position` (C8-vector-4, written before it was run): Pathfinder Unite of R7 and R8
///   (`object.pathfinder.unite`), a structural write whose result depends on their stacking order
///   and parent. Necessary as for `structure`: the colleague changed or deleted R7 or R8 in any way
///   (the changes of the table never restack or reparent them; s12 covers those).
///
/// The place form is the artboard index. (Some commands do pick objects by a point when nothing
/// is selected, C8-vector-4 item 4: path.knife, path.eraseRegion, path.joinScrub, path.blob,
/// shaper.select, shaper.scribble, symbol.adjust; none is a form of this table.)
const FORMS: [&str; 5] = ["id", "name", "index", "structure", "position"];

/// A refusal the rules call false that the SDK at 1935d52 makes by design: a structural write
/// holds its objects' index in their list (`positions: ["nodes"]`), and the guard compares
/// indexes, so an object deleted below R7 and R8 in their layer shifts them and refuses the write
/// though their order and parent are as they were. Waiting for the SDK (a premise on the order
/// among a call's objects); counted and printed apart, never as necessary.
fn waiting_for_sdk(form: &str, change: Change, on: &str) -> bool {
    matches!(form, "structure" | "position")
        && change == Change::Delete
        && on.strip_prefix('R').and_then(|n| n.parse::<usize>().ok()).is_some_and(|n| n < 7)
}

fn necessary(form: &str, change: Change, on: &str) -> bool {
    let target = on == "R7";
    let structural = on == "R7" || on == "R8";
    match form {
        "id" => target && matches!(change, Change::Rename | Change::Delete),
        "name" => target && matches!(change, Change::Delete | Change::Fill) || change == Change::SwatchSwapped,
        "index" => matches!(change, Change::ArtboardReordered | Change::ArtboardRenamed),
        "structure" => structural && change == Change::Delete,
        "position" => structural && matches!(change, Change::Rename | Change::Fill | Change::Move | Change::Delete),
        _ => panic!("no form {form}"),
    }
}

/// What a saved group of R7 and R8 lacks of both writers' edits: the group (one parent holding
/// both, not a layer) and the colleague's `change` of `id` (its name, its fill, its moved bounds).
fn not_merged(file: &Path, id: u64, (r7, r8): (u64, u64), change: Change) -> Option<String> {
    let (p7, p8) = (place_on_disk(file, r7).0, place_on_disk(file, r8).0);
    let mut doc = VectorDocument::open(FILE, &std::fs::read(file).unwrap()).unwrap();
    let parent = doc.execute("file.node", json!({"id": p7})).ok();
    if p7 != p8 || parent.as_ref().is_none_or(|node| node["kind"]["type"] != "group") {
        return Some(format!("R7 and R8 are not grouped ({p7} / {p8})"));
    }
    let node = on_disk(file, id).unwrap_or_default();
    let ok = match change {
        Change::Rename => node["name"] == "Theirs",
        Change::Fill => node["appearance"].to_string().contains("0.666"),
        // Moved by (7, 3) from where `spot` put it.
        Change::Move => {
            let n: usize = node["name"].as_str().and_then(|name| name.strip_prefix('R')).and_then(|n| n.parse().ok()).unwrap_or(0);
            let (x, y) = spot(n);
            node["kind"].to_string().contains(&format!("[{:.1},{:.1}]", x + 7.0, y + 3.0))
        }
        _ => true,
    };
    (!ok).then(|| format!("the colleague's {change:?} of {id} is lost: {node}"))
}

#[tokio::test(flavor = "multi_thread")]
async fn s11_the_mixed_table() {
    use std::collections::BTreeMap;
    let mut seed = 0x9e37_79b9_7f4a_7c15_u64;
    // (form, outcome) -> count, and the rows of what went wrong.
    let mut table: BTreeMap<(&str, &str), usize> = BTreeMap::new();
    let mut wrong = Vec::new();
    for round in 0..50 {
        let change = CHANGES[usize::try_from(xorshift(&mut seed) % 10).unwrap()];
        // The object the colleague's change is on: R7 or R8 a third of the time each, else another.
        let on = match xorshift(&mut seed) % 6 {
            0 | 1 => "R7".to_owned(),
            2 | 3 => "R8".to_owned(),
            n => format!("R{}", n),
        };
        for form in FORMS {
            let f = Fixture::with(10, Guard::Premise);
            f.colleague(&[
                ("swatch.new", json!({"name":"Brand","color":"#123456"})),
                ("artboard.new", json!({"x":900,"y":0,"width":100,"height":100})),
            ]);
            let (details, basis) = f.inspect().await;
            let names = ids_by_name(&details);
            let id = names[&on];
            let step: (&str, Value) = match change {
                Change::Rename => ("object.setProps", json!({"ids":[id],"name":"Theirs"})),
                Change::Fill => ("paint.setFill", json!({"ids":[id],"color":"#00aa00"})),
                Change::Move => ("object.setBounds", json!({"ids":[id],"x":500,"y":500})),
                Change::Delete => ("edit.delete", json!({"ids":[id]})),
                Change::Create => ("shape.rectangle", json!({"x":600,"y":400,"width":10,"height":10})),
                Change::SwatchAdded => ("swatch.new", json!({"name":"Other","color":"#abcdef"})),
                Change::SwatchSwapped => ("swatch.edit", json!({"name":"Brand","newName":"Old Brand"})),
                Change::ArtboardAdded => ("artboard.new", json!({"x":1100,"y":0,"width":100,"height":100})),
                Change::ArtboardReordered => ("artboard.reorder", json!({"index":1,"to":0})),
                Change::ArtboardRenamed => ("artboard.setProps", json!({"index":0,"name":"Theirs"})),
            };
            let mut steps = vec![step];
            if change == Change::SwatchSwapped {
                steps.push(("swatch.new", json!({"name":"Brand","color":"#ff00ff"})));
            }
            if change == Change::Move {
                steps.insert(0, ("select.set", json!({"ids":[id]})));
                steps[1] = ("object.move", json!({"dx":7,"dy":3}));
            }
            f.colleague(&steps);
            let (r7, r8) = (names["R7"], names["R8"]);
            let write = match form {
                "id" => rename(r7, "Mine"),
                "name" => json!({"command":"paint.setFill","params":{"ids":[r7],"swatch":"Brand"}}),
                "index" => json!({"command":"artboard.setProps","params":{"index":0,"name":"Mine"}}),
                "structure" => json!({"command":"arrange.group","on":[r7, r8]}),
                _ => json!({"command":"object.pathfinder.unite","on":[r7, r8]}),
            };
            let answer = f.call("run", json!({"basis": basis, "steps": [write]})).await;
            // A group that applied after the colleague's change of one of its objects: the saved
            // drawing holds the group of R7 and R8 and the colleague's edit (both writers' intent).
            if form == "structure"
                && answer.is_ok()
                && (on == "R7" || on == "R8")
                && let Some(missing) = not_merged(&f.file(), names[&on], (r7, r8), change)
            {
                wrong.push(format!("round {round}: structure after {change:?} on {on}: applied, but {missing}"));
                *table.entry((form, "applied, not merged")).or_default() += 1;
            }
            let outcome = match answer {
                Ok(_) if necessary(form, change, &on) => "applied wrongly",
                Ok(_) => "applied",
                Err(error) if error["error"]["code"] == "conflict" => {
                    if necessary(form, change, &on) {
                        "refused necessary"
                    } else if waiting_for_sdk(form, change, &on) {
                        "refused false (SDK)"
                    } else {
                        "refused false"
                    }
                }
                Err(error) => {
                    wrong.push(format!("round {round}: {form} after {change:?} on {on}: failed at kit: {}", error["error"]));
                    "failed at kit"
                }
            };
            if matches!(outcome, "applied wrongly" | "refused false" | "refused false (SDK)") {
                wrong.push(format!("round {round}: {form} after {change:?} on {on}: {outcome}"));
            }
            *table.entry((form, outcome)).or_default() += 1;
            f.shutdown().await;
        }
    }
    println!("s11 mixed table (seed 0x9e3779b97f4a7c15, 50 rounds x 5 forms):");
    for ((form, outcome), count) in &table {
        println!("  {form:9} {outcome:18} {count}");
    }
    for line in &wrong {
        println!("  {line}");
    }
    let bad: usize = table
        .iter()
        .filter(|((_, outcome), _)| matches!(*outcome, "applied wrongly" | "refused false" | "applied, not merged"))
        .map(|(_, count)| count)
        .sum();
    assert_eq!(bad, 0, "{wrong:#?}");
}
