//! `kition-vector` reads and writes only inside its workspace, and only through its host (KHP A29,
//! A37, A42, A45, section 10.6). A workspace escape audit
//! confirmed by running that an agent could delete a file outside the
//! workspace (`prefs.set recoveryFolder`, then `file.recovery.discard` inside `command.batch`), copy
//! one out (`<image href="file:///...">` pasted, then `file.package`), write exports, saves and
//! templates anywhere through `command.batch` and `model.batch`, list folders (plug-ins, fonts),
//! and that opening a drawing read the files it links to wherever they were.
//!
//! Each test runs itself again as a child process whose current folder, `HOME` and `TMPDIR` are
//! fresh empty folders, so every by-path write lands where it can be seen. Beside them,
//! `outside/` holds files the agent names by path or that documents link to: a sentinel text, a
//! picture and drawings of a colour nothing else uses, a recovery area with an unrelated file, a
//! plug-in and a font file. They must not change, be listed, be read into any answer, render or
//! export, or be deleted; the workspace holds only what was planted and what the host saved.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use kition_headless::{Headless, HostFolder, KitInfo};
use kition_kit::KitDocument;
use kition_kit_server::protocol::{Origin, ScreenshotScope};
use kition_kit_server::{KitBackend, OpenRequest, ScreenshotRequest};
use kition_vector_kit::VectorDocument;
use serde_json::{Map, Value, json};

/// Set in the child: the folder holding `workspace/` and `outside/`.
const CHILD: &str = "KITION_VECTOR_TEST_SANDBOX";

/// The colour of every picture and drawing outside the workspace: seen anywhere, the kit read it.
const OUTSIDE: [u8; 3] = [0x13, 0x57, 0x9b];
/// The colour of a drawing inside the workspace that a document links to: it must show.
const INSIDE: [u8; 3] = [0x24, 0x68, 0xac];
/// The colour of a drawing in a hidden folder of the workspace: hosts refuse hidden paths.
const HIDDEN: [u8; 3] = [0x9b, 0x57, 0x13];

fn hex([r, g, b]: [u8; 3]) -> String {
    format!("{r:02x}{g:02x}{b:02x}")
}

/// Bytes nothing compresses (a zip stores them as they are), so a copy shows in any answer.
fn sentinel() -> Vec<u8> {
    let mut state = 0x9e37_79b9_7f4a_7c15_u64;
    (0..96)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state >> 24) as u8
        })
        .collect()
}

fn png(rgb: [u8; 3]) -> Vec<u8> {
    let picture = image::RgbaImage::from_pixel(16, 16, image::Rgba([rgb[0], rgb[1], rgb[2], 255]));
    let mut out = std::io::Cursor::new(Vec::new());
    picture.write_to(&mut out, image::ImageFormat::Png).unwrap();
    out.into_inner()
}

fn svg_of(rgb: [u8; 3]) -> String {
    format!(r##"<svg xmlns="http://www.w3.org/2000/svg" width="40" height="40"><rect id="r" width="40" height="40" fill="#{}"/></svg>"##, hex(rgb))
}

/// A drawing of one square filled with `fill`, written at `path` by the engine itself: the test's
/// own files are made outside the kit (a kit document turns host-only mode on for the process).
fn square(path: &Path, fill: &str) {
    let mut engine = kition_vector_engine::Session::new();
    engine.execute("file.new", &json!({ "width": 40, "height": 40 })).unwrap();
    engine.execute("shape.rectangle", &json!({ "x": 0, "y": 0, "width": 40, "height": 40 })).unwrap();
    engine.execute("paint.setFill", &json!({ "color": fill })).unwrap();
    engine.execute("file.saveAs", &json!({ "path": path.to_string_lossy() })).unwrap();
}

/// Whether a PNG shows a pixel of `rgb`.
fn shows(png: &[u8], rgb: [u8; 3]) -> bool {
    let Ok(picture) = image::load_from_memory(png) else { return false };
    picture.to_rgba8().pixels().any(|pixel| pixel[3] > 200 && (0..3).all(|c| pixel[c].abs_diff(rgb[c]) <= 3))
}

/// Every file under `dir`, relative to it, with its bytes and modification time (folders: empty).
fn tree(dir: &Path) -> BTreeMap<String, (Vec<u8>, Option<std::time::SystemTime>)> {
    let mut out = BTreeMap::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(next) = stack.pop() {
        for entry in std::fs::read_dir(&next).into_iter().flatten().flatten() {
            let path = entry.path();
            let meta = std::fs::symlink_metadata(&path).unwrap();
            if meta.is_dir() {
                stack.push(path.clone());
            }
            let bytes = if meta.is_file() { std::fs::read(&path).unwrap_or_default() } else { Vec::new() };
            out.insert(path.strip_prefix(dir).unwrap_or(&path).to_string_lossy().into_owned(), (bytes, meta.modified().ok()));
        }
    }
    out
}

fn path(dir: &Path, name: &str) -> String {
    dir.join(name).to_string_lossy().into_owned()
}

fn params(value: Value) -> Map<String, Value> {
    value.as_object().cloned().unwrap_or_default()
}

/// `text` decoded as base64, when it is.
fn base64(text: &str) -> Option<Vec<u8>> {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    if text.len() < 16 || !text.len().is_multiple_of(4) {
        return None;
    }
    let (mut out, mut buffer, mut bits) = (Vec::new(), 0u32, 0u32);
    for byte in text.trim_end_matches('=').bytes() {
        let value = TABLE.iter().position(|known| *known == byte)? as u32;
        buffer = (buffer << 6) | value;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push(((buffer >> bits) & 0xff) as u8);
        }
    }
    Some(out)
}

/// The agent's session: the kit as kition-headless serves it, on a host folder (the workspace).
struct Agent {
    kit: Headless<VectorDocument>,
    folder: HostFolder,
    /// Everything the kit answered: strings, the bytes they encode, pictures.
    seen: Vec<Vec<u8>>,
    problems: Vec<String>,
}

impl Agent {
    fn new(workspace: &Path) -> Self {
        // What the headless kit's `main` does before it serves.
        kition_vector_kit::hosting::set_files_only();
        let mut kit = Headless::<VectorDocument>::with_replica_port(
            KitInfo::from_manifest(include_str!("../../../kit.json"), &[], kition_vector_kit::EDITOR_STATE).unwrap(),
        );
        let folder = HostFolder::new(workspace).unwrap();
        kit.set_host(folder.host());
        Self { kit, folder, seen: Vec::new(), problems: Vec::new() }
    }

    fn open(&mut self, name: &str) -> Option<String> {
        let file = self.folder.file(name).unwrap();
        match self.kit.open(OpenRequest { file: &file, read_only: false, create: None }) {
            Ok(info) => Some(info.id),
            Err(error) => {
                self.record(&json!(error.to_string()));
                None
            }
        }
    }

    fn record(&mut self, value: &Value) {
        match value {
            Value::String(text) => {
                self.seen.push(text.as_bytes().to_vec());
                if let Some(bytes) = base64(text) {
                    self.seen.push(bytes);
                }
            }
            Value::Array(items) => items.iter().for_each(|item| self.record(item)),
            Value::Object(fields) => fields.values().for_each(|item| self.record(item)),
            other => self.seen.push(other.to_string().into_bytes()),
        }
    }

    fn answer(&mut self, answer: Result<Value, String>) -> Result<Value, String> {
        match &answer {
            Ok(value) => self.record(value),
            Err(message) => self.record(&json!(message)),
        }
        answer
    }

    /// `command` run directly, as `commands.execute` runs it for the agent.
    fn run(&mut self, id: &str, command: &str, value: Value) -> Result<Value, String> {
        let answer =
            self.kit.execute(id, command, params(value), Origin::Agent).map(|done| done.result.unwrap_or(Value::Null)).map_err(|e| e.to_string());
        self.answer(answer)
    }

    /// `command` as the one step of the kit's own `command.batch`.
    fn batch(&mut self, id: &str, command: &str, value: Value) -> Result<Value, String> {
        self.run(id, "command.batch", json!({ "commands": [{ "command": command, "params": value }] }))
    }

    /// `command` as a `command` operation of the object model's `model.batch`.
    fn model_batch(&mut self, id: &str, command: &str, value: Value) -> Result<Value, String> {
        let answer = self
            .kit
            .model("model.batch", json!({ "documentId": id, "ops": [{ "op": "command", "id": command, "params": value }] }))
            .map_err(|e| e.to_string());
        self.answer(answer)
    }

    /// `command` must be refused directly, through `command.batch` and through `model.batch`.
    fn refused(&mut self, id: &str, command: &str, value: &Value) {
        for (route, answer) in [
            ("directly", self.run(id, command, value.clone())),
            ("in command.batch", self.batch(id, command, value.clone())),
            ("in model.batch", self.model_batch(id, command, value.clone())),
        ] {
            if let Ok(done) = answer {
                self.problems.push(format!("{command} {value} ran {route}: {done}"));
            }
        }
    }

    fn screenshot(&mut self, id: &str) -> Vec<u8> {
        let request = ScreenshotRequest { scope: ScreenshotScope::Canvas, max_size: 400, scale: 1.0, target: None };
        let shot = self.kit.screenshot(id, &request).map(|shot| shot.png).unwrap_or_default();
        self.seen.push(shot.clone());
        shot
    }

    /// Nothing outside the workspace in anything the kit answered.
    fn assert_nothing_leaked(&mut self, outside: &Path) {
        let mut needles: Vec<(String, Vec<u8>)> = ["secret.txt", "sentinel.png", "linked.png", "plugins/bait.wasm", "fonts/bait.ttf"]
            .iter()
            .map(|name| ((*name).to_owned(), std::fs::read(outside.join(name)).unwrap()))
            .collect();
        for colour in [OUTSIDE, HIDDEN] {
            needles.push((format!("the colour #{}", hex(colour)), hex(colour).into_bytes()));
            needles.push((format!("the colour #{}", hex(colour)), hex(colour).to_uppercase().into_bytes()));
        }
        for listed in ["bait.wasm", "bait.ttf", "notes.txt"] {
            needles.push((format!("the name {listed} (a folder outside was listed)"), listed.as_bytes().to_vec()));
        }
        for seen in &self.seen {
            for (what, needle) in &needles {
                if !needle.is_empty() && seen.windows(needle.len()).any(|window| window == needle.as_slice()) {
                    self.problems.push(format!("an answer holds {what} from outside the workspace"));
                }
            }
            for colour in [OUTSIDE, HIDDEN] {
                if seen.starts_with(b"\x89PNG") && shows(seen, colour) {
                    self.problems.push(format!("a picture the kit answered shows #{} from outside the workspace", hex(colour)));
                }
            }
        }
        self.problems.sort();
        self.problems.dedup();
        assert!(self.problems.is_empty(), "the kit reached outside its workspace:\n{}", self.problems.join("\n"));
    }
}

/// The folders' children a test plants, and what the parent checks afterwards.
struct Sandbox {
    root: PathBuf,
}

impl Sandbox {
    fn new(name: &str) -> Self {
        let root = std::env::temp_dir().join(format!("kition-vector-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for dir in ["workspace", "cwd", "home", "tmp", "outside/rec/proj", "outside/plugins", "outside/fonts", "outside/tpl"] {
            std::fs::create_dir_all(root.join(dir)).unwrap();
        }
        let root = std::fs::canonicalize(root).unwrap();
        let outside = root.join("outside");
        std::fs::write(outside.join("secret.txt"), sentinel()).unwrap();
        std::fs::write(outside.join("sentinel.png"), png(OUTSIDE)).unwrap();
        std::fs::write(outside.join("sentinel.svg"), svg_of(OUTSIDE)).unwrap();
        std::fs::write(outside.join("linked.png"), png(OUTSIDE)).unwrap();
        std::fs::write(outside.join("rec/proj/notes.txt"), b"notes nobody asked the kit about").unwrap();
        std::fs::write(outside.join("plugins/bait.wasm"), b"\0asm\x01\0\0\0").unwrap();
        std::fs::write(outside.join("fonts/bait.ttf"), sentinel().iter().rev().copied().collect::<Vec<u8>>()).unwrap();
        square(&outside.join("planted.kvector"), &format!("#{}", hex(OUTSIDE)));
        std::fs::copy(outside.join("planted.kvector"), outside.join("rec/proj/a.kvector")).unwrap();
        Self { root }
    }

    fn workspace(&self) -> PathBuf {
        self.root.join("workspace")
    }

    fn outside(&self) -> PathBuf {
        self.root.join("outside")
    }

    /// Runs `test` again in a child process in the sandbox, then checks the folders.
    fn run_child(&self, test: &str) {
        let planted = tree(&self.outside());
        let workspace: Vec<String> = tree(&self.workspace()).into_keys().collect();
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", test, "--nocapture", "--test-threads=1"])
            .current_dir(self.root.join("cwd"))
            .env(CHILD, &self.root)
            .env("HOME", self.root.join("home"))
            .env("TMPDIR", self.root.join("tmp"))
            .env("XDG_CONFIG_HOME", self.root.join("home/.config"))
            .env("XDG_DATA_HOME", self.root.join("home/.local/share"))
            .env("XDG_CACHE_HOME", self.root.join("home/.cache"))
            .status()
            .unwrap();
        assert!(status.success(), "the agent's session in the child: {status} (its problems are printed above)");
        let written: Vec<(&str, Vec<String>)> = ["cwd", "home", "tmp"]
            .into_iter()
            .map(|dir| (dir, tree(&self.root.join(dir)).into_keys().collect::<Vec<_>>()))
            .filter(|(_, found)| !found.is_empty())
            .collect();
        assert!(written.is_empty(), "the kit wrote outside its workspace: {written:?}");
        let now = tree(&self.outside());
        let changed: Vec<&String> = planted.keys().chain(now.keys()).filter(|name| planted.get(*name) != now.get(*name)).collect();
        assert!(changed.is_empty(), "the folder outside the workspace changed (written, deleted or touched): {changed:?}");
        assert_eq!(tree(&self.workspace()).into_keys().collect::<Vec<_>>(), workspace, "the workspace holds only what was planted and saved");
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// What the agent does to write, delete or list outside the workspace: the audit's routes V1, V2,
/// V5 and V6, every file parameter of every command through both batch routes, then every command
/// the kit offers with no parameters. Errors are fine; reaching files outside is not.
fn act_writes(root: &Path) {
    let outside = root.join("outside");
    let mut agent = Agent::new(&root.join("workspace"));
    let id = agent.open("drawing.kvector").expect("the drawing opens");

    // Preferences that name folders (V2, V5, V6, and the fonts folder): refused for the agent.
    for (key, folder) in [("recoveryFolder", "rec"), ("templatesFolder", "tpl/a/b"), ("pluginsFolder", "plugins"), ("fontsFolder", "fonts")] {
        agent.refused(&id, "prefs.set", &json!({ "key": key, "value": path(&outside, folder) }));
        agent.refused(&id, "prefs.set", &json!({ "values": { key: path(&outside, folder) } }));
    }
    // V2: Data Recovery lists, locks, saves and (inside a batch) deletes in its folder.
    let _ = agent.run(&id, "file.recovery.list", json!({}));
    let _ = agent.run(&id, "file.recovery.save", json!({}));
    agent.refused(&id, "file.recovery.discard", &json!({ "file": "proj/a.kvector" }));
    agent.refused(&id, "file.recovery.restore", &json!({ "file": "proj/a.kvector" }));
    // V5: Save as Template made a folder in HOME, or anywhere.
    let _ = agent.batch(&id, "file.saveAsTemplate", json!({}));
    agent.refused(&id, "file.saveAsTemplate", &json!({ "path": path(&outside, "tpl/a/b/t.ait") }));
    // V6: the plug-ins folder was listed and read.
    let _ = agent.run(&id, "plugin.list", json!({}));
    let _ = agent.run(&id, "plugin.reload", json!({}));
    agent.refused(&id, "plugin.reload", &json!({ "path": path(&outside, "plugins") }));
    // V1: what the audit wrote outside, and the font search that walked folders.
    agent.refused(&id, "file.export", &json!({ "path": path(&outside, "via-batch.svg") }));
    agent.refused(&id, "file.saveAs", &json!({ "path": path(&outside, "saved-as.kvector") }));
    agent.refused(&id, "file.package", &json!({ "folder": path(&outside, "package") }));
    agent.refused(&id, "text.findFontFiles", &json!({ "folder": path(&outside, "fonts") }));
    // Every file parameter of every command, each pointed outside, through every route.
    let probe = VectorDocument::create("probe.kvector", 16, 16).unwrap();
    for command in probe.commands() {
        let Some(schema) = command.schema.as_ref() else { continue };
        for name in kition_kit::schema::file_params(schema) {
            let node = &schema[&name];
            let node = if node["type"] == "array" { &node["items"] } else { node };
            let target = if node["kind"] == "folder" {
                path(&outside, &format!("dir-{}", command.id))
            } else if node["access"] == "read" {
                path(&outside, "secret.txt")
            } else {
                path(&outside, &format!("out-{}.svg", command.id))
            };
            let value = if schema[&name]["type"] == "array" { json!([target]) } else { json!(target) };
            agent.refused(&id, &command.id, &json!({ name: value }));
        }
    }
    // Every command offered to the agent, with no parameters. Those that need a folder of their
    // own are not offered.
    let offered: Vec<String> = agent.kit.commands(Some(&id)).unwrap().into_iter().map(|command| command.id).collect();
    for gone in kition_vector_kit::gate::HOSTED_OFF {
        if offered.iter().any(|command| command == gone) {
            agent.problems.push(format!("{gone} needs a folder of its own, yet the agent is offered it"));
        }
    }
    for command in offered {
        // Closing ends the session the rest of the sweep runs in.
        if matches!(command.as_str(), "file.close" | "file.closeAll" | "document.close" | "file.revert") {
            continue;
        }
        let _ = agent.run(&id, &command, json!({}));
    }
    if let Err(error) = agent.kit.save(&id, None, None) {
        agent.problems.push(format!("the host could not save the drawing: {error}"));
    }
    agent.assert_nothing_leaked(&outside);
}

#[test]
fn agent_commands_write_delete_and_list_nothing_outside_the_workspace() {
    if let Some(root) = std::env::var_os(CHILD) {
        act_writes(&PathBuf::from(root));
        return;
    }
    let sandbox = Sandbox::new("writes");
    square(&sandbox.workspace().join("drawing.kvector"), "#808080");
    sandbox.run_child("agent_commands_write_delete_and_list_nothing_outside_the_workspace");
}

/// What the agent does to read outside the workspace: open drawings whose links point outside
/// (V4), paste one and package it (V3), XML that names files, and the link commands; then look
/// at, check, export and package the drawings. A link inside the workspace still resolves, through
/// the host.
fn act_reads(root: &Path) {
    let outside = root.join("outside");
    let mut agent = Agent::new(&root.join("workspace"));

    // V4: a drawing whose images link outside, by every spelling, and one inside.
    let svg = agent.open("hostile.svg").expect("the SVG opens");
    let shot = agent.screenshot(&svg);
    if !shows(&shot, INSIDE) {
        agent.problems.push("the image the SVG links inside the workspace (inside.svg) does not show: links resolve through the host".into());
    }
    let described = agent.kit.describe(&svg).map(|described| Value::Object(described.state)).unwrap_or_default();
    agent.record(&described);
    match described.get("links").and_then(Value::as_array) {
        None => agent.problems.push(format!("inspect reports no links (KHP A42): {}", described.get("links").unwrap_or(&Value::Null))),
        Some(links) => {
            for link in links {
                let target = link["target"].as_str().unwrap_or_default();
                let resolves = target == "inside.svg";
                if link["exists"] != json!(resolves) {
                    agent.problems.push(format!("inspect says the link {link} exists: {}, expected {resolves}", link["exists"]));
                }
            }
            if links.len() < 7 {
                agent.problems.push(format!("inspect reports {} of the SVG's 8 links: {links:?}", links.len()));
            }
        }
    }
    let checked = agent.kit.check(&svg).unwrap_or_default();
    agent.record(&checked);
    if !checked.to_string().contains("vector.missing-link") {
        agent.problems.push(format!("the check reports no vector.missing-link: {checked}"));
    }
    for format in ["svg", "png", "pdf"] {
        let _ = agent.run(&svg, "file.exportBytes", json!({ "format": format }));
    }
    for command in ["linkedFile.list", "linkedFile.check", "linkedFile.update", "linkedFile.updateChanged", "file.package"] {
        let _ = agent.run(&svg, command, json!({}));
    }

    // V4: a native drawing linking a picture and a drawing outside by their absolute paths.
    let native = agent.open("hostile.kvector").expect("the drawing opens");
    agent.screenshot(&native);
    for command in ["linkedFile.list", "linkedFile.check", "linkedFile.update", "linkedFile.updateChanged", "linkedFile.embed"] {
        let _ = agent.run(&native, command, json!({}));
    }
    agent.screenshot(&native);
    let described = agent.kit.describe(&native).map(|described| Value::Object(described.state)).unwrap_or_default();
    agent.record(&described);
    let links = described.get("links").and_then(Value::as_array).cloned().unwrap_or_default();
    if links.len() != 2 || links.iter().any(|link| link["exists"] != json!(false)) {
        agent.problems.push(format!("inspect should report the drawing's 2 links outside the workspace as not existing: {links:?}"));
    }
    for format in ["svg", "png", "pdf"] {
        let _ = agent.run(&native, "file.exportBytes", json!({ "format": format }));
    }
    let _ = agent.run(&native, "file.package", json!({}));
    let _ = agent.batch(&native, "file.package", json!({}));

    // V3: a pasted SVG linking files outside, then packaged.
    let pasted = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="40"><image width="40" height="40" href="file://{}"/><image x="50" width="40" height="40" href="file://{}"/></svg>"#,
        path(&outside, "secret.txt"),
        path(&outside, "sentinel.png")
    );
    let _ = agent.run(&native, "clipboard.importSvg", json!({ "svg": pasted }));
    let _ = agent.run(&native, "edit.pasteInPlace", json!({}));
    let _ = agent.run(&native, "linkedFile.list", json!({}));
    let _ = agent.run(&native, "file.package", json!({}));
    agent.refused(&native, "file.saveAs", &json!({ "path": path(&outside, "saved.kvector") }));
    let _ = agent.batch(&native, "file.package", json!({}));
    agent.screenshot(&native);

    // XML that names files: an external entity, an external DTD, a stylesheet import, a `use`.
    let xxe = std::fs::read_to_string(root.join("workspace/xxe.svg")).unwrap();
    let _ = agent.run(&native, "clipboard.importSvg", json!({ "svg": xxe }));
    let _ = agent.run(&native, "edit.pasteInPlace", json!({}));
    if let Some(opened) = agent.open("xxe.svg") {
        agent.screenshot(&opened);
        let text = agent.kit.text(&opened, 10_000).unwrap_or_default();
        agent.record(&text);
        let _ = agent.run(&opened, "file.exportBytes", json!({ "format": "svg" }));
    }
    let text = agent.kit.text(&native, 10_000).unwrap_or_default();
    agent.record(&text);
    for format in ["svg", "png"] {
        let _ = agent.run(&native, "file.exportBytes", json!({ "format": format }));
    }
    agent.assert_nothing_leaked(&outside);
}

#[test]
fn documents_and_links_read_nothing_outside_the_workspace() {
    if let Some(root) = std::env::var_os(CHILD) {
        act_reads(&PathBuf::from(root));
        return;
    }
    let sandbox = Sandbox::new("reads");
    let (workspace, outside) = (sandbox.workspace(), sandbox.outside());
    // A server that must never be asked: an image the SVG links to on it.
    let server = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    server.set_nonblocking(true).unwrap();
    let port = server.local_addr().unwrap().port();

    std::fs::write(workspace.join("inside.svg"), svg_of(INSIDE)).unwrap();
    std::fs::create_dir_all(workspace.join(".hidden")).unwrap();
    std::fs::write(workspace.join(".hidden/inside.svg"), svg_of(HIDDEN)).unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(outside.join("sentinel.svg"), workspace.join("escape.svg")).unwrap();
    let image = |x: u32, href: &str| format!(r#"<image x="{x}" y="0" width="20" height="20" href="{href}"/>"#);
    let hostile = [
        image(0, &format!("file://{}", path(&outside, "sentinel.png"))),
        image(20, &path(&outside, "sentinel.svg")),
        image(40, "../outside/sentinel.png"),
        image(60, "escape.svg"),
        image(80, &format!("http://127.0.0.1:{port}/sentinel.png")),
        image(100, ".hidden/inside.svg"),
        image(120, &format!("//127.0.0.1:{port}/share/sentinel.png")),
        format!(
            r#"<image xmlns:xlink="http://www.w3.org/1999/xlink" x="140" y="0" width="20" height="20" xlink:href="{}"/>"#,
            path(&outside, "linked.png")
        ),
        image(160, "inside.svg"),
    ]
    .concat();
    let hostile = format!(r#"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="40" viewBox="0 0 200 40">{hostile}</svg>"#);
    std::fs::write(workspace.join("hostile.svg"), hostile).unwrap();
    let secret = path(&outside, "secret.txt");
    let sentinel_svg = path(&outside, "sentinel.svg");
    let xxe = format!(
        r#"<?xml version="1.0"?>
<!DOCTYPE svg SYSTEM "file://{sentinel_svg}" [<!ENTITY leak SYSTEM "file://{secret}">]>
<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="200" height="40">
<style>@import url("file://{sentinel_svg}"); @import url("http://127.0.0.1:{port}/x.css");</style>
<use xlink:href="file://{sentinel_svg}#r"/>
<text x="10" y="30">entity: &leak;</text>
</svg>"#
    );
    std::fs::write(workspace.join("xxe.svg"), xxe).unwrap();

    // A native drawing whose picture and placed drawing are linked by absolute paths outside. The
    // files are drawn in near-white when linked, then replaced by the outside colour: only reading
    // them again shows it.
    std::fs::write(outside.join("linked.png"), png([0xfe, 0xfe, 0xfe])).unwrap();
    let coloured = std::fs::read(outside.join("planted.kvector")).unwrap();
    square(&outside.join("planted.kvector"), "#fefefe");
    let mut engine = kition_vector_engine::Session::new();
    engine.execute("file.new", &json!({ "width": 200, "height": 100 })).unwrap();
    engine.execute("file.place", &json!({ "path": path(&outside, "linked.png"), "link": true, "at": [30, 50] })).unwrap();
    engine.execute("file.place", &json!({ "path": path(&outside, "planted.kvector"), "link": true, "at": [120, 50] })).unwrap();
    engine.execute("file.saveAs", &json!({ "path": path(&workspace, "hostile.kvector") })).unwrap();
    std::fs::write(outside.join("linked.png"), png(OUTSIDE)).unwrap();
    std::fs::write(outside.join("planted.kvector"), coloured).unwrap();

    sandbox.run_child("documents_and_links_read_nothing_outside_the_workspace");
    let asked = server.accept();
    assert!(
        matches!(&asked, Err(error) if error.kind() == std::io::ErrorKind::WouldBlock),
        "the kit connected to a server a document named (KHP A37): {asked:?}"
    );
}
