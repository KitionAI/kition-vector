//! C8-vector-engine-2 (issue-c8-engine2-settings-keys): a command that changes one setting of a
//! settings group (Document Setup's bleed, File Info's author, one raster effects option, one
//! print option, the tiling origin) rests on that setting only. Another program rewrites the
//! `.kvector` file, changing another setting of the same group, between the agent's observe and its
//! write: the agent's write applies and the saved file keeps both changes.
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

fn kit() -> &'static KitDefinition {
    KitDefinition::from_manifest_text(include_str!("../../../kit.json")).unwrap()
}

fn drawing() -> Vec<u8> {
    let mut doc = VectorDocument::create(FILE, 800, 600).unwrap();
    doc.execute("shape.rectangle", json!({"x": 20, "y": 20, "width": 20, "height": 20})).unwrap();
    doc.save(None).unwrap()
}

/// Another program: this kit's engine, its own instance, opening the file, running `steps` and
/// saving it back.
fn colleague(file: &Path, steps: &[(&str, Value)]) {
    let at = std::fs::metadata(file).unwrap().modified().unwrap();
    let mut doc = VectorDocument::open(FILE, &std::fs::read(file).unwrap()).unwrap();
    for (command, params) in steps {
        doc.execute(command, params.clone()).unwrap_or_else(|error| panic!("colleague's {command}: {error}"));
    }
    std::fs::write(file, doc.save(None).unwrap()).unwrap();
    std::fs::File::options().write(true).open(file).unwrap().set_modified(at - Duration::from_secs(3600)).unwrap();
}

/// What query `command` answers on the file as saved.
fn on_disk(file: &Path, command: &str) -> Value {
    let mut doc = VectorDocument::open(FILE, &std::fs::read(file).unwrap()).unwrap();
    doc.execute(command, json!({})).unwrap()
}

static WORKSPACES: AtomicUsize = AtomicUsize::new(0);

struct Fixture {
    kits: KitManager,
    workspace: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!("kition-vector-keys-{}-{}", std::process::id(), WORKSPACES.fetch_add(1, Ordering::Relaxed)));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let workspace = std::fs::canonicalize(dir).unwrap();
        std::fs::write(workspace.join(FILE), drawing()).unwrap();
        let kits = KitManager::new(Some(workspace.clone()), vec![(kit(), Some(PathBuf::from(env!("CARGO_BIN_EXE_kition-vector"))))]);
        Self { kits, workspace }
    }

    fn file(&self) -> PathBuf {
        self.workspace.join(FILE)
    }

    async fn call(&self, action: &str, mut input: Value) -> Result<Value, Value> {
        input["path"] = json!(FILE);
        let options = CallOptions { guard: Guard::Premise, ..CallOptions::default() };
        match tools::call_as(&self.kits, &format!("vector_{action}"), &input, options, ME).await {
            Some(Ok(output)) => Ok(output.output),
            Some(Err(error)) => Err(error.structured()),
            None => panic!("vector_{action} is no tool"),
        }
    }

    async fn shutdown(self) {
        self.kits.shutdown().await;
        let _ = std::fs::remove_dir_all(&self.workspace);
    }
}

/// The colleague runs `theirs`, then the agent's `mine` with the basis from before it: it
/// applies, and `query` on the saved file answers what `check` expects of both.
async fn both_kept(what: &str, theirs: (&str, Value), mine: (&str, Value), query: &str, check: impl Fn(&Value) -> bool) {
    let f = Fixture::new();
    let seen = f.call("inspect", json!({})).await.unwrap_or_else(|e| panic!("inspect: {e}"));
    let basis = seen["revision"].as_str().unwrap().to_owned();
    colleague(&f.file(), &[theirs]);
    let step = json!({"command": mine.0, "params": mine.1});
    if let Err(refused) = f.call("run", json!({"basis": basis, "steps": [step]})).await {
        panic!("{what}: refused for another program's change of another setting: {refused}");
    }
    let saved = on_disk(&f.file(), query);
    assert!(check(&saved), "{what}: the saved file lost a change: {saved}");
    f.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn document_setup_bleed_beside_their_language() {
    both_kept(
        "file.setup bleed after their language",
        ("file.setup", json!({"language": "German"})),
        ("file.setup", json!({"bleed": 9})),
        "file.setup",
        |v| v["language"] == "German" && v["bleed"][0] == 9.0,
    )
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn document_setup_bleed_beside_their_units() {
    both_kept(
        "file.setup bleed after their units",
        ("file.setup", json!({"units": "Millimeters"})),
        ("file.setup", json!({"bleed": 9})),
        "file.setup",
        |v| v["units"] == "Millimeters" && v["bleed"][0] == 9.0,
    )
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn file_info_author_beside_their_title() {
    both_kept(
        "file.setInfo author after their title",
        ("file.setInfo", json!({"title": "Theirs"})),
        ("file.setInfo", json!({"author": "Me"})),
        "file.setInfo",
        |v| v["title"] == "Theirs" && v["author"] == "Me",
    )
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn raster_effects_anti_alias_beside_their_clipping_mask() {
    both_kept(
        "file.rasterEffectsSettings antiAlias after their clippingMask",
        ("file.rasterEffectsSettings", json!({"clippingMask": true})),
        ("file.rasterEffectsSettings", json!({"antiAlias": false})),
        "file.rasterEffectsSettings",
        |v| v["clippingMask"] == true && v["antiAlias"] == false,
    )
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn print_setup_collate_beside_their_copies() {
    both_kept(
        "print.setup collate after their copies",
        ("print.setup", json!({"settings": {"copies": 3}})),
        ("print.setup", json!({"settings": {"collate": false}})),
        "print.setup",
        |v| v["settings"]["copies"] == 3 && v["settings"]["collate"] == false,
    )
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn print_tiling_origin_beside_their_copies() {
    both_kept(
        "print.tiling.set after their copies",
        ("print.setup", json!({"settings": {"copies": 3}})),
        ("print.tiling.set", json!({"origin": [30, 30]})),
        "print.setup",
        |v| v["settings"]["copies"] == 3 && v["settings"]["tileOrigin"]["placed"] == true,
    )
    .await;
}
