//! C8-vector-read: one behaviour test per class the independent reading of every write command
//! found (`bindings/wasm/tests/declarations_independent.rs`). Each: an agent observes, another
//! program rewrites the `.kvector` file, and the agent's write with the older `basis` is refused
//! after the change it rests on and applied after an unrelated one. Wrong-write classes: a write
//! that rested on what the guard did not hold landed on what the agent never saw. Held-more
//! classes: a write the guard refused for a change it does not rest on now applies.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use kition_kit::{EditorState, KitDocument};
use kition_kits::tools::{self, CallOptions, Guard};
use kition_kits::{KitDefinition, KitManager};
use kition_vector_kit::VectorDocument;
use serde_json::{Value, json};

const ME: &str = "agent-me";
const FILE: &str = "Drawing.kvector";

fn kit() -> &'static KitDefinition {
    KitDefinition::from_manifest_text(include_str!("../../../kit.json")).unwrap()
}

fn id(answer: &Value) -> u64 {
    answer["id"].as_u64().unwrap_or_else(|| panic!("no id in {answer}"))
}

/// Rectangles `R0`…`R7` in a row 40 pt apart; group `G` of ellipses `G0`, `G1`; compound path `CP`
/// of two squares `K0`, `K1`; `S`, an instance of symbol "Sym"; `R4` linked to graphic style "Style 1";
/// swatches "Brand" (global), its tint swatch "Brand 40%" and "Other"; a vertical guide at 500; and a layer "Top" over "Layer 1"
/// holding `T0`.
fn drawing() -> Vec<u8> {
    let mut doc = VectorDocument::create(FILE, 800, 600).unwrap();
    let mut ex = |command: &str, params: Value| doc.execute(command, params.clone()).unwrap_or_else(|e| panic!("{command} {params}: {e}"));
    ex("swatch.new", json!({"name": "Brand", "color": "#123456", "global": true}));
    ex("swatch.new", json!({"swatch": "Brand", "tint": 40}));
    ex("swatch.new", json!({"name": "Other", "color": "#654321"}));
    let mut r4 = 0;
    for n in 0..8 {
        let r = id(&ex("shape.rectangle", json!({"x": 20 + 40 * n, "y": 20, "width": 20, "height": 20})));
        ex("object.setProps", json!({"ids": [r], "name": format!("R{n}")}));
        if n == 4 {
            r4 = r;
        }
    }
    let g0 = id(&ex("shape.ellipse", json!({"x": 20, "y": 300, "width": 20, "height": 20})));
    let g1 = id(&ex("shape.ellipse", json!({"x": 60, "y": 300, "width": 20, "height": 20})));
    ex("object.setProps", json!({"ids": [g0], "name": "G0"}));
    ex("object.setProps", json!({"ids": [g1], "name": "G1"}));
    ex("select.set", json!({"ids": [g0, g1]}));
    let g = id(&ex("arrange.group", json!({})));
    ex("object.setProps", json!({"ids": [g], "name": "G"}));
    let k0 = id(&ex("shape.rectangle", json!({"x": 200, "y": 300, "width": 40, "height": 40})));
    let k1 = id(&ex("shape.rectangle", json!({"x": 210, "y": 310, "width": 10, "height": 10})));
    ex("select.set", json!({"ids": [k0, k1]}));
    let cp = id(&ex("object.compoundPath.make", json!({})));
    ex("object.setProps", json!({"ids": [cp], "name": "CP"}));
    let members = ex("file.node", json!({"id": cp}))["kind"]["children"].clone();
    for (n, member) in members.as_array().unwrap().iter().enumerate() {
        ex("object.setProps", json!({"ids": [member["id"]], "name": format!("K{n}")}));
    }
    let s0 = id(&ex("shape.rectangle", json!({"x": 400, "y": 300, "width": 20, "height": 20})));
    let s = id(&ex("symbol.new", json!({"name": "Sym", "ids": [s0]})));
    ex("object.setProps", json!({"ids": [s], "name": "S"}));
    ex("graphicStyle.new", json!({"name": "Style 1", "id": r4}));
    ex("graphicStyle.apply", json!({"name": "Style 1", "ids": [r4]}));
    ex("guide.add", json!({"vertical": true, "pos": 500}));
    let top = id(&ex("layer.new", json!({"name": "Top"})));
    let t0 = id(&ex("shape.rectangle", json!({"x": 600, "y": 500, "width": 20, "height": 20})));
    ex("object.setProps", json!({"ids": [t0], "name": "T0"}));
    let _ = top;
    ex("edit.deselectAll", json!({}));
    doc.save(None).unwrap()
}

fn backdate(file: &Path, before: std::time::SystemTime) {
    std::fs::File::options().write(true).open(file).unwrap().set_modified(before - Duration::from_secs(3600)).unwrap();
}

/// A colleague's program: this kit's engine, its own instance, opening the file, running
/// `steps` and saving it back.
fn colleague(file: &Path, steps: &[(&str, Value)]) {
    let at = std::fs::metadata(file).unwrap().modified().unwrap();
    let mut doc = VectorDocument::open(FILE, &std::fs::read(file).unwrap()).unwrap();
    for (command, params) in steps {
        doc.execute(command, params.clone()).unwrap_or_else(|error| panic!("colleague's {command}: {error}"));
    }
    std::fs::write(file, doc.save(None).unwrap()).unwrap();
    backdate(file, at);
}

/// The ids of the objects (and layers) an inspect lists, by name.
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

struct Fixture {
    kits: KitManager,
    options: CallOptions,
    workspace: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!("kition-vector-read-{}-{}", std::process::id(), WORKSPACES.fetch_add(1, Ordering::Relaxed)));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let workspace = std::fs::canonicalize(dir).unwrap();
        std::fs::write(workspace.join(FILE), drawing()).unwrap();
        let kits = KitManager::new(Some(workspace.clone()), vec![(kit(), Some(PathBuf::from(env!("CARGO_BIN_EXE_kition-vector"))))]);
        Self { kits, options: CallOptions { guard: Guard::Premise, ..CallOptions::default() }, workspace }
    }

    fn colleague(&self, steps: &[(&str, Value)]) {
        colleague(&self.workspace.join(FILE), steps);
    }

    async fn call(&self, action: &str, mut input: Value) -> Result<Value, Value> {
        input["path"] = json!(FILE);
        match tools::call_as(&self.kits, &format!("vector_{action}"), &input, self.options, ME).await {
            Some(Ok(output)) => Ok(output.output),
            Some(Err(error)) => Err(error.structured()),
            None => panic!("vector_{action} is no tool"),
        }
    }

    /// The drawing's objects by name, and the inspect's `revision`.
    async fn inspect(&self) -> (std::collections::BTreeMap<String, u64>, String) {
        let seen = self.call("inspect", json!({})).await.unwrap_or_else(|e| panic!("inspect: {e}"));
        (ids_by_name(&seen["details"]), seen["revision"].as_str().unwrap().to_owned())
    }

    /// The agent's `step` (or list of steps) with `basis`: refused as a conflict, or applied.
    async fn write(&self, basis: &str, step: Value) -> Result<Value, Value> {
        let steps = if step.is_array() { step } else { json!([step]) };
        self.call("run", json!({"basis": basis, "steps": steps})).await
    }

    async fn shutdown(self) {
        self.kits.shutdown().await;
        let _ = std::fs::remove_dir_all(&self.workspace);
    }
}

/// After `theirs` (a change the step rests on) the agent's `step` is refused as a conflict; on a
/// fresh drawing, after `unrelated`, it applies. `step` gets the drawing's ids by name.
async fn refused_then_applied(
    what: &str,
    theirs: impl Fn(&std::collections::BTreeMap<String, u64>) -> Vec<(&'static str, Value)>,
    unrelated: impl Fn(&std::collections::BTreeMap<String, u64>) -> Vec<(&'static str, Value)>,
    step: impl Fn(&std::collections::BTreeMap<String, u64>) -> Value,
) {
    let f = Fixture::new();
    let (names, basis) = f.inspect().await;
    f.colleague(&theirs(&names));
    match f.write(&basis, step(&names)).await {
        Err(refused) => assert_eq!(refused["error"]["code"], "conflict", "{what}: {refused}"),
        Ok(ran) => panic!("{what}: the write landed on a change it rests on: {ran}"),
    }
    f.shutdown().await;
    let f = Fixture::new();
    let (names, basis) = f.inspect().await;
    f.colleague(&unrelated(&names));
    if let Err(refused) = f.write(&basis, step(&names)).await {
        panic!("{what}: refused for a change it does not rest on: {refused}");
    }
    f.shutdown().await;
}

/// After `unrelated` (a change the step does not rest on, which the guard held before) the
/// agent's `step` applies.
async fn applies_after(
    what: &str,
    unrelated: impl Fn(&std::collections::BTreeMap<String, u64>) -> Vec<(&'static str, Value)>,
    step: impl Fn(&std::collections::BTreeMap<String, u64>) -> Value,
) {
    let f = Fixture::new();
    let (names, basis) = f.inspect().await;
    f.colleague(&unrelated(&names));
    if let Err(refused) = f.write(&basis, step(&names)).await {
        panic!("{what}: refused for a change it does not rest on: {refused}");
    }
    f.shutdown().await;
}

fn fill(id: u64, color: &str) -> (&'static str, Value) {
    ("paint.setFill", json!({"ids": [id], "color": color}))
}

// ---- Wrong writes: a read that was not held ----

/// What a group holds: a compound path made of a group reads its members' geometry and paint.
#[tokio::test(flavor = "multi_thread")]
async fn w1_a_write_on_a_group_rests_on_what_the_group_holds() {
    refused_then_applied(
        "compound of a group after a member's fill",
        |n| vec![fill(n["G0"], "#ff0000")],
        |n| vec![fill(n["R5"], "#ff0000")],
        |n| json!({"command": "object.compoundPath.make", "on": [n["G"], n["R1"]], "params": {}}),
    )
    .await;
}

/// The root of a member: releasing a compound path by one of its members reads the whole.
#[tokio::test(flavor = "multi_thread")]
async fn w2_a_write_on_a_member_rests_on_its_root() {
    refused_then_applied(
        "compound release by a member after the compound's rename",
        |n| vec![("object.setProps", json!({"ids": [n["CP"]], "name": "Theirs"}))],
        |n| vec![fill(n["R5"], "#ff0000")],
        |n| json!({"command": "object.compoundPath.release", "on": [n["K0"]], "params": {}}),
    )
    .await;
}

/// The drawing's own fields: slices made from the guides read the guides.
#[tokio::test(flavor = "multi_thread")]
async fn w3_a_write_from_the_drawings_settings_rests_on_them() {
    refused_then_applied(
        "slices from guides after a guide added",
        |_| vec![("guide.add", json!({"vertical": true, "pos": 250}))],
        |n| vec![("object.setProps", json!({"ids": [n["R5"]], "name": "Theirs"}))],
        |_| json!({"command": "object.slice.fromGuides", "params": {}}),
    )
    .await;
}

/// Where an object is: bring forward moves it above its neighbour, which another's restack
/// changes.
#[tokio::test(flavor = "multi_thread")]
async fn w4_a_restack_rests_on_where_its_object_is_among_its_siblings() {
    refused_then_applied(
        "bring forward after a sibling restacked",
        |n| vec![("select.set", json!({"ids": [n["R3"]]})), ("arrange.sendToBack", json!({}))],
        |n| vec![fill(n["R5"], "#ff0000")],
        |n| json!({"command": "arrange.bringForward", "on": [n["R2"]], "params": {}}),
    )
    .await;
}

/// The other layers: Hide Others writes their visibility.
#[tokio::test(flavor = "multi_thread")]
async fn w5_hide_others_rests_on_the_other_layers() {
    refused_then_applied(
        "hide others after another showed and hid a layer",
        |n| vec![("layer.setProps", json!({"ids": [n["Top"]], "visible": false}))],
        |n| vec![("layer.setProps", json!({"ids": [n["Top"]], "name": "Top 2"}))],
        |n| json!({"command": "layer.hideOthers", "params": {"ids": [n["R2"]]}}),
    )
    .await;
}

/// A value naming another object: rasterizing a symbol instance draws its symbol's art.
#[tokio::test(flavor = "multi_thread")]
async fn w6_an_instance_rests_on_its_symbol() {
    refused_then_applied(
        "rasterize an instance after its symbol was redefined",
        |n| vec![("select.set", json!({"ids": [n["R6"]]})), ("symbol.update", json!({"name": "Sym"}))],
        |n| vec![fill(n["R5"], "#ff0000")],
        |n| json!({"command": "object.rasterize", "on": [n["S"]], "params": {"ppi": 72}}),
    )
    .await;
}

/// What lies under a brush: Liquify with nothing selected distorts the paths under it.
#[tokio::test(flavor = "multi_thread")]
async fn w7_a_liquify_stroke_rests_on_what_is_under_it() {
    refused_then_applied(
        "liquify after another moved a path under the brush",
        |n| vec![("select.set", json!({"ids": [n["R5"]]})), ("object.move", json!({"dx": -110, "dy": 0}))],
        |n| vec![("select.set", json!({"ids": [n["R7"]]})), ("object.move", json!({"dx": 0, "dy": 200}))],
        |_| json!({"command": "object.liquify", "params": {"tool": "bloat", "points": [[110, 30]]}}),
    )
    .await;
}

/// A parameter resolved elsewhere: a batch's steps name objects in their own parameters.
#[tokio::test(flavor = "multi_thread")]
async fn w8_a_batch_rests_on_what_its_steps_rest_on() {
    refused_then_applied(
        "a batch's rename after another renamed the object",
        |n| vec![("object.setProps", json!({"ids": [n["R2"]], "name": "Theirs"}))],
        |n| vec![("object.setProps", json!({"ids": [n["R5"]], "name": "Theirs"}))],
        |n| json!({"command": "command.batch", "params": {"commands": [{"command": "object.setProps", "params": {"ids": [n["R2"]], "name": "Mine"}}]}}),
    )
    .await;
}

/// A field of the object acted on: a copy carries every field of what it copies.
#[tokio::test(flavor = "multi_thread")]
async fn w9_a_copying_move_rests_on_every_field_of_what_it_copies() {
    refused_then_applied(
        "move with copy after a rename",
        |n| vec![("object.setProps", json!({"ids": [n["R2"]], "name": "Theirs"}))],
        |n| vec![("object.setProps", json!({"ids": [n["R5"]], "name": "Theirs"}))],
        |n| json!({"command": "object.move", "on": [n["R2"]], "params": {"dx": 0, "dy": 100, "copy": true}}),
    )
    .await;
}

/// A default taken from elsewhere: aligning to the artboard reads the artboards.
#[tokio::test(flavor = "multi_thread")]
async fn w10_aligning_to_the_artboard_rests_on_the_artboards() {
    refused_then_applied(
        "align to the artboard after it was resized",
        |_| vec![("artboard.setProps", json!({"index": 0, "width": 700}))],
        |n| vec![fill(n["R5"], "#ff0000")],
        |n| json!({"command": "object.align", "on": [n["R2"]], "params": {"horizontal": "right", "to": "artboard"}}),
    )
    .await;
}

/// Editor state a handler reads is declared (the host puts it back after the call, and the
/// commands tool tells the model to set it earlier in the run).
#[test]
fn w11_editor_state_the_handlers_read_is_declared() {
    let commands = VectorDocument::create(FILE, 10, 10).unwrap().commands();
    let reads = |id: &str| commands.iter().find(|command| command.id == id).unwrap_or_else(|| panic!("no {id}")).reads;
    // New art's paint and the defaults it resets, the undo of what the last step wrote, the
    // current layer a closed drawing leaves.
    for (id, state) in [("appearance.setNewArtBasic", EditorState::Mode), ("edit.undo", EditorState::Mode), ("file.close", EditorState::Active)] {
        assert!(reads(id).contains(&state), "{id} reads {state:?} without declaring it");
    }
}

/// The run's own selection: a step that acts on the selection an earlier step of the same run made
/// rests on what that step selects, not on the selection before the run.
#[tokio::test(flavor = "multi_thread")]
async fn w12_a_step_on_the_selection_an_earlier_step_made_rests_on_it() {
    refused_then_applied(
        "break link of what the run selected after the symbol was redefined",
        |n| vec![("select.set", json!({"ids": [n["R6"]]})), ("symbol.update", json!({"name": "Sym"}))],
        |n| vec![fill(n["R5"], "#ff0000")],
        |n| json!([{"command": "select.set", "params": {"ids": [n["S"]]}}, {"command": "symbol.breakLink", "params": {}}]),
    )
    .await;
}

/// A step that acts on what an earlier write of the same run selected (the group `arrange.group`
/// just made) rests on what that write would leave: a fill of the new group paints its members.
#[tokio::test(flavor = "multi_thread")]
async fn w14_a_step_on_what_an_earlier_write_selected_rests_on_it() {
    refused_then_applied(
        "fill of the group the run made after a member's fill",
        |n| vec![fill(n["R2"], "#00ff00")],
        |n| vec![fill(n["R5"], "#00ff00")],
        |n| json!([{"command": "arrange.group", "on": [n["R2"], n["R3"]], "params": {}}, {"command": "paint.setFill", "params": {"color": "#ff0000"}}]),
    )
    .await;
}

/// A swatch edit recolours the tint swatches made from it: it rests on them, not on the whole
/// swatch table.
#[tokio::test(flavor = "multi_thread")]
async fn w13_a_swatch_edit_rests_on_its_tint_swatches() {
    refused_then_applied(
        "edit of Brand after its tint swatch was renamed",
        |_| vec![("swatch.edit", json!({"name": "Brand 40%", "newName": "Light"}))],
        |_| vec![("swatch.edit", json!({"name": "Other", "newName": "Elsewhere"}))],
        |_| json!({"command": "swatch.edit", "params": {"name": "Brand", "color": "#ff0000"}}),
    )
    .await;
}

// ---- Refusals no change needs: held more than the handler reads ----

/// Bring Forward moves its object above the next one: it rests on where it is among its
/// siblings, not on its paint.
#[tokio::test(flavor = "multi_thread")]
async fn m1_a_restack_does_not_rest_on_its_objects_paint() {
    applies_after(
        "bring forward after the object's fill",
        |n| vec![fill(n["R2"], "#ff0000")],
        |n| json!({"command": "arrange.bringForward", "on": [n["R2"]], "params": {}}),
    )
    .await;
}

/// A group rests on where its objects are (their parent and stacking), not on their fields: after
/// another's rename, refill or move of a member the group applies, and the saved drawing holds
/// the group of both with the other's edit; a member moved into another group refuses it.
#[tokio::test(flavor = "multi_thread")]
async fn m5_a_group_rests_on_where_its_objects_are_and_merges_their_edits() {
    for (what, theirs) in [
        ("rename", ("object.setProps", json!({"name": "Theirs"}))),
        ("fill", ("paint.setFill", json!({"color": "#00aa00"}))),
        ("move", ("object.move", json!({"dx": 7, "dy": 3}))),
    ] {
        let f = Fixture::new();
        let (n, basis) = f.inspect().await;
        let (r2, r3) = (n["R2"], n["R3"]);
        let mut params = theirs.1.clone();
        if what == "move" {
            f.colleague(&[("select.set", json!({"ids": [r2]})), (theirs.0, params)]);
        } else {
            params["ids"] = json!([r2]);
            f.colleague(&[(theirs.0, params)]);
        }
        let ran = f
            .write(&basis, json!({"command": "arrange.group", "on": [r2, r3], "params": {}}))
            .await
            .unwrap_or_else(|e| panic!("group after a {what}: {e}"));
        let group = ran["changes"]["created"].as_array().and_then(|c| c.first()).and_then(|c| c["id"].as_u64()).unwrap_or_else(|| panic!("{ran}"));
        let mut doc = VectorDocument::open(FILE, &std::fs::read(f.workspace.join(FILE)).unwrap()).unwrap();
        let saved = doc.execute("file.node", json!({"id": group})).unwrap();
        let members: Vec<Value> = saved["kind"]["children"].as_array().unwrap().clone();
        assert_eq!(members.len(), 2, "{saved}");
        let mine = members.iter().find(|m| m["id"] == json!(r2)).unwrap_or_else(|| panic!("R2 not in the group: {saved}"));
        match what {
            "rename" => assert_eq!(mine["name"], "Theirs"),
            "fill" => assert!(mine["appearance"].to_string().contains("0.666"), "{mine}"),
            // R2 was drawn at (100, 20): moved by (7, 3).
            _ => assert!(mine["kind"].to_string().contains("[107.0,23.0]"), "{mine}"),
        }
        f.shutdown().await;
    }
    let f = Fixture::new();
    let (n, basis) = f.inspect().await;
    f.colleague(&[("node.move", json!({"id": n["R2"], "parent": n["G"], "index": 0}))]);
    match f.write(&basis, json!({"command": "arrange.group", "on": [n["R2"], n["R3"]], "params": {}})).await {
        Err(refused) => assert_eq!(refused["error"]["code"], "conflict", "{refused}"),
        Ok(ran) => panic!("group after a member was moved into another group: {ran}"),
    }
    f.shutdown().await;
}

/// A write by swatch name rests on that swatch, not on where it is in the table.
#[tokio::test(flavor = "multi_thread")]
async fn m2_a_fill_by_name_does_not_rest_on_the_swatch_order() {
    applies_after(
        "fill by swatch name after the swatches were reordered",
        |_| vec![("swatch.move", json!({"name": "Other", "to": 0}))],
        |n| json!({"command": "paint.setFill", "params": {"ids": [n["R2"]], "swatch": "Brand"}}),
    )
    .await;
}

/// A Pathfinder result goes where the frontmost object was: where the others are is no premise.
#[tokio::test(flavor = "multi_thread")]
async fn m3_pathfinder_rests_on_where_its_frontmost_object_is_only() {
    applies_after(
        "unite after the back object was sent to the back",
        |n| vec![("select.set", json!({"ids": [n["R2"]]})), ("arrange.sendToBack", json!({}))],
        |n| json!({"command": "object.pathfinder.unite", "on": [n["R2"], n["R3"]], "params": {}}),
    )
    .await;
}

/// A fill does not read or break the graphic style link.
#[tokio::test(flavor = "multi_thread")]
async fn m4_a_fill_does_not_rest_on_the_graphic_style_link() {
    applies_after(
        "fill after another broke the object's style link",
        |n| vec![("graphicStyle.breakLink", json!({"ids": [n["R4"]]}))],
        |n| json!({"command": "paint.setFill", "params": {"ids": [n["R4"]], "color": "#ff0000"}}),
    )
    .await;
}
