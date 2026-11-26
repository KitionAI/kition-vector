//! # kition-vector-kit
//!
//! The Vector engine for Kition's in-page editor, on the Kition SDK. [`VectorDocument`]
//! implements [`KitDocument`] for a Kition Vector drawing (`.kvector`, and the formats
//! Kition Vector opens: SVG, PDF, AI, EPS), and [`KitTools`] for Kition Vector's own tools, so the
//! host runs the engine's selection, pen, shape, and type tools unchanged.
//!
//! Document pixels are points, measured from the top-left of the drawing's extent: the artboards'
//! union, grown to take in art that lies off them (the pasteboard, [`pasteboard`]).
#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

use std::cell::{Cell, RefCell};

mod address;
pub mod agent_text;
pub mod check;
mod collab;
#[cfg(test)]
mod conformance;
mod cursors;
pub mod document_text;
mod files;
pub mod gate;
pub mod hosting;
mod params;
mod pass;
mod place;
mod rests;
mod selection;

use kition_kit::raster::{self, Frame};
use kition_kit::tools::{KitTools, PointerAction, ToolInfo, ToolMods, ToolOverlay, ToolPointer, ToolRequest, ToolResponse};
use kition_kit::{KitCommand, KitDocument, MAX_RENDER_SIDE};
use kition_vector_engine::{Session, command_specs};
use kition_vector_engine::{UiRequest, ViewInfo};
use kition_vector_geom::{Affine, Point, Rect};
use kition_vector_render::{RenderOptions, Renderer};
use kition_vector_tools::{Mods, Overlay, PointerEvent, PointerKind, ToolKey};
use serde_json::{Value, json};

/// One open Kition Vector drawing.
pub struct VectorDocument {
    session: Session,
    name: String,
    renderer: RefCell<Renderer>,
    /// Where the pointer hovers, for the hover highlight.
    hover: Option<Point>,
    /// The editor states [`KitDocument::editor_state`] answered lately, by their `session`
    /// digest: what the JSON cannot carry (the current paint, Transform Again) comes back from here.
    snapshots: RefCell<Vec<(Value, kition_vector_engine::editor_state::EditorSnapshot)>>,
    /// What the steps of a run the guard is asking about select ([`pass`]).
    pass: pass::Pass,
    /// The extent while a tool's gesture lasts: its top-left stays put, so the document pixels a
    /// drag goes on sending keep meaning the same points ([`VectorDocument::bounds`]).
    held: Cell<Option<Rect>>,
    /// The modifier keys of the last pointer event: the cursor shows what they make a click do.
    mods: Mods,
    /// The keys the active tool takes now ahead of their shortcuts (`toolKeys` in `inspect()`).
    claims: Vec<&'static str>,
    /// The area the last tool event changed, when only the moved selection's ([`KitTools::take_damage`]).
    damage: Option<kition_kit::tools::ToolDamage>,
    /// The view's zoom the last tool event came at.
    zoom: f64,
}

/// The places in `inspect()` that are the editing session, not the drawing (Kit Process Protocol
/// section 7a): what an agent's call saves and puts back, and kition-headless's volatile keys.
pub const EDITOR_STATE: &[&str] =
    &["/currentLayer", "/layerRows", "/isolation", "/selection", "/selectionBounds", "/selectionRotation", "/target", "/tool", "/toolKeys", "/paint"];

/// The keys a tool may take ahead of their shortcuts (`Tool::claims_key`), as the page names them.
const CLAIMABLE: &[(&str, ToolKey)] = &[
    ("Enter", ToolKey::Enter),
    ("Backspace", ToolKey::Backspace),
    ("Delete", ToolKey::Delete),
    ("ArrowUp", ToolKey::Up),
    ("ArrowDown", ToolKey::Down),
    ("ArrowLeft", ToolKey::Left),
    ("ArrowRight", ToolKey::Right),
    ("Tab", ToolKey::Tab),
    ("Home", ToolKey::Home),
    ("End", ToolKey::End),
    ("[", ToolKey::BracketLeft),
    ("]", ToolKey::BracketRight),
];

/// How far past the artboards art off them may take the extent (points, each side): the
/// pasteboard's size.
const PASTEBOARD_REACH: f64 = 16_384.0;
/// The extent grows past the artboards in steps of this many points, so art nudged about off them
/// seldom moves the document's origin.
const PASTEBOARD_STEP: f64 = 72.0;
/// The margin kept around art off the artboards (points), for its anti-aliased edges.
const PASTEBOARD_MARGIN: f64 = 18.0;

/// The area the canvas shows: the artboards `boards`, grown on each side where `art` lies past
/// them, by whole [`PASTEBOARD_STEP`]s taking in a [`PASTEBOARD_MARGIN`], at most
/// [`PASTEBOARD_REACH`]. Art on the artboards leaves them as they are.
fn pasteboard(boards: Rect, art: Option<Rect>) -> Rect {
    let Some(art) = art.filter(|art| [art.x0, art.y0, art.x1, art.y1].iter().all(|v| v.is_finite())) else { return boards };
    let grow = |past: f64| {
        if past <= 0.0 { 0.0 } else { (((past + PASTEBOARD_MARGIN) / PASTEBOARD_STEP).ceil() * PASTEBOARD_STEP).min(PASTEBOARD_REACH) }
    };
    Rect::new(
        boards.x0 - grow(boards.x0 - art.x0),
        boards.y0 - grow(boards.y0 - art.y0),
        boards.x1 + grow(art.x1 - boards.x1),
        boards.y1 + grow(art.y1 - boards.y1),
    )
}

fn engine_error(error: impl std::fmt::Display) -> String {
    error.to_string()
}

impl VectorDocument {
    /// Refuses a parameter `command` does not take (a typo or a guessed name) before it runs,
    /// naming the one it resembles, instead of running without it (engine rule E7a). The schemas
    /// are the ones the catalogue offers agents, read once.
    pub(crate) fn refuse_unknown(&self, command: &str, params: &Value) -> Result<(), String> {
        static SCHEMAS: std::sync::OnceLock<std::collections::HashMap<String, Value>> = std::sync::OnceLock::new();
        let schemas = SCHEMAS.get_or_init(|| self.commands().into_iter().filter_map(|command| Some((command.id, command.schema?))).collect());
        schemas.get(command).map_or(Ok(()), |schema| kition_kit::schema::refuse_unknown(command, schema, params))
    }

    /// A document over a session the caller set up (its active drawing is the one shown).
    fn from_session(session: Session, name: &str) -> Self {
        Self {
            session,
            name: name.to_owned(),
            renderer: RefCell::new(Renderer::new()),
            hover: None,
            snapshots: RefCell::new(Vec::new()),
            pass: pass::Pass::default(),
            held: Cell::new(None),
            mods: Mods::default(),
            claims: Vec::new(),
            damage: None,
            zoom: 1.0,
        }
    }

    fn active(&self) -> Result<&kition_vector_engine::DocState, String> {
        self.session.active().ok_or_else(|| "no document is open".to_owned())
    }

    /// The artboards' union in points.
    fn artboards(&self) -> Rect {
        let Ok(state) = self.active() else { return Rect::new(0.0, 0.0, 612.0, 792.0) };
        let mut boards = state.doc.artboards.iter().map(|board| board.rect);
        let first = boards.next().unwrap_or(Rect::new(0.0, 0.0, 612.0, 792.0));
        boards.fold(first, |all, board| all.union(board))
    }

    /// The area document pixels cover, in points: the artboards and the art off them
    /// ([`pasteboard`]). While a gesture lasts its top-left holds where the gesture began (the
    /// page goes on sending the drag in those pixels); it may still grow right and down.
    fn bounds(&self) -> Rect {
        let live = pasteboard(self.artboards(), self.active().ok().and_then(|state| state.doc.art_bounds()));
        match self.held.get() {
            Some(held) => Rect::new(held.x0, held.y0, live.x1.max(held.x0 + 1.0), live.y1.max(held.y0 + 1.0)),
            None => live,
        }
    }

    /// The keys the active tool takes ahead of their shortcuts now, asked after every event and
    /// command, for `inspect()`.
    fn ask_claims(&mut self, zoom: f64) {
        let view = Self::view(zoom);
        self.zoom = view.zoom;
        let session = &mut self.session;
        self.claims = CLAIMABLE.iter().filter(|(_, key)| session.tool_claims_key(*key, view)).map(|(name, _)| *name).collect();
    }

    /// After a command: it may change the keys the active tool takes (the artboards a Delete
    /// removes, a gradient stop selected).
    pub(crate) fn after_command(&mut self) {
        self.ask_claims(self.zoom);
    }

    /// The visual bounds of the selected objects, in points.
    fn selected_bounds(&self) -> Option<Rect> {
        let state = self.active().ok()?;
        state.doc.bounds_of(&state.selection.objects, true)
    }

    fn origin(&self) -> Point {
        let bounds = self.bounds();
        Point::new(bounds.x0, bounds.y0)
    }

    /// `width × height` straight-alpha pixels of the area at document pixel (`x`, `y`), `scale`
    /// screen pixels per point, artboards white as on screen.
    fn draw(&self, x: f64, y: f64, scale: f64, width: u32, height: u32) -> Result<Vec<u8>, String> {
        let state = self.active()?;
        let origin = self.origin();
        let view = Affine::scale(scale) * Affine::translate((-(origin.x + x), -(origin.y + y)));
        // In isolation mode everything around the isolated group draws dimmed.
        let options = RenderOptions { artboards: true, isolated: state.isolation, ..Default::default() };
        let rendered =
            self.renderer.try_borrow_mut().map_err(|_| "the renderer is busy".to_owned())?.render(&state.doc, width, height, view, &options);
        Ok(rendered.to_straight())
    }

    fn view(zoom: f64) -> ViewInfo {
        ViewInfo { zoom: if zoom.is_finite() && zoom > 0.0 { zoom } else { 1.0 }, ..ViewInfo::default() }
    }

    fn to_engine(&self, x: f64, y: f64) -> Point {
        let origin = self.origin();
        Point::new(origin.x + x, origin.y + y)
    }

    fn respond(&mut self, requests: Vec<UiRequest>, at: Option<Point>, zoom: f64) -> ToolResponse {
        let view = Self::view(zoom);
        let origin = self.origin();
        let mut engine = selection::overlays(&self.session);
        engine.extend(self.session.overlays(view));
        engine.extend(self.hover.and_then(|at| selection::hover(&self.session, at, view.zoom)));
        let overlays = engine.into_iter().filter_map(|overlay| overlay_out(overlay, origin)).collect();
        let mods = self.mods;
        let cursor = at.map_or_else(|| "default".to_owned(), |point| cursors::css(self.session.cursor(point, mods, view)));
        self.ask_claims(zoom);
        ToolResponse {
            requests: requests.into_iter().map(request_out).collect(),
            overlays,
            cursor,
            busy: self.session.tool_busy(),
            wants_text: self.session.tool_wants_text(),
        }
    }
}

/// Journaled commands that change only what the person sees or the session's own state
/// (selection, isolation, the current layer, tool and panel options, the clipboard) or produce
/// bytes: they never commit to the drawing or its replica.
const VIEWING: &[&str] = &[
    "file.activate",
    "file.export",
    "file.exportSelection",
    "file.exportForOffice",
    "file.exportPdf",
    "file.exportEps",
    "file.exportDxf",
    "file.exportForWeb",
    "file.saveCopy",
    "edit.copy",
    "edit.findNext",
    "edit.colorSettings",
    "view.drawMode",
    "object.isolate",
    "object.exitIsolation",
    "object.clippingMask.editContents",
    "object.clippingMask.editMask",
    "object.blend.options",
    "gradient.selectStop",
    "appearance.setActiveItem",
    "appearance.targetContents",
    "layer.setCurrent",
    "layer.highlight",
    "layer.target",
    "layer.selectAll",
    "layer.locate",
    "eyedropper.setOptions",
    "paint.toggleActive",
    "transparency.toggleNewMasksClipping",
    "transparency.toggleNewMasksInverted",
];

/// Whether running `spec` changes the drawing (the catalog's `mutates`, which read-only documents
/// refuse): the engine journals selection and session commands too.
fn mutates(spec: &kition_vector_engine::CommandSpec) -> bool {
    let selects = spec.id.starts_with("select.") && spec.id != "select.save" || matches!(spec.id, "edit.selectAll" | "edit.deselectAll");
    spec.journal && !selects && !VIEWING.contains(&spec.id)
}

fn mods_in(mods: ToolMods) -> Mods {
    Mods { shift: mods.shift, alt: mods.alt, cmd: mods.command, ctrl: mods.control, space: mods.space }
}

fn request_out(request: UiRequest) -> ToolRequest {
    match request {
        UiRequest::Dialog(name, params) => ToolRequest::Dialog { name, params },
        UiRequest::SwitchTool(id) => ToolRequest::SwitchTool { id },
        UiRequest::Status(text) => ToolRequest::Status { text },
    }
}

fn rgba([r, g, b]: [u8; 3]) -> [u8; 4] {
    [r, g, b, 255]
}

/// An engine overlay in document pixels.
fn overlay_out(overlay: Overlay, origin: Point) -> Option<ToolOverlay> {
    let at = |point: Point| (point.x - origin.x, point.y - origin.y);
    Some(match overlay {
        Overlay::Marquee(rect) => {
            ToolOverlay::Marquee { x0: rect.x0 - origin.x, y0: rect.y0 - origin.y, x1: rect.x1 - origin.x, y1: rect.y1 - origin.y }
        }
        Overlay::Path { path, color, width, dashed } => {
            let moved = Affine::translate((-origin.x, -origin.y)) * path;
            ToolOverlay::Path { d: moved.to_svg(), color: rgba(color), width, dashed }
        }
        Overlay::Line { a, b, color, dashed } => {
            let ((ax, ay), (bx, by)) = (at(a), at(b));
            ToolOverlay::Line { ax, ay, bx, by, color: rgba(color), dashed }
        }
        Overlay::Anchor { p, color, filled, size } => {
            let (x, y) = at(p);
            ToolOverlay::Anchor { x, y, color: rgba(color), filled, size }
        }
        Overlay::Handle { p, color } => {
            let (x, y) = at(p);
            ToolOverlay::Handle { x, y, color: rgba(color) }
        }
        Overlay::Label { p, text, color } => {
            let (x, y) = at(p);
            ToolOverlay::Label { x, y, text, color: rgba(color) }
        }
        Overlay::Measure { p, text } => {
            let (x, y) = at(p);
            ToolOverlay::Measure { x, y, text }
        }
        Overlay::Highlight { quad, color } => {
            let corner = |point: Point| {
                let (x, y) = at(point);
                [x, y]
            };
            ToolOverlay::Highlight { quad: [corner(quad[0]), corner(quad[1]), corner(quad[2]), corner(quad[3])], color }
        }
        Overlay::Swatch { p, color, selected } => {
            let (x, y) = at(p);
            ToolOverlay::Swatch { x, y, color, selected }
        }
        Overlay::GridLine { a, b, color } => {
            let ((ax, ay), (bx, by)) = (at(a), at(b));
            ToolOverlay::Line { ax, ay, bx, by, color, dashed: false }
        }
    })
}

fn key_in(key: &str) -> Option<ToolKey> {
    Some(match key {
        "Enter" => ToolKey::Enter,
        "Escape" => ToolKey::Escape,
        "Backspace" => ToolKey::Backspace,
        "Delete" => ToolKey::Delete,
        "ArrowUp" | "Up" => ToolKey::Up,
        "ArrowDown" | "Down" => ToolKey::Down,
        "ArrowLeft" | "Left" => ToolKey::Left,
        "ArrowRight" | "Right" => ToolKey::Right,
        "[" => ToolKey::BracketLeft,
        "]" => ToolKey::BracketRight,
        "Tab" => ToolKey::Tab,
        "Home" => ToolKey::Home,
        "End" => ToolKey::End,
        digit if digit.len() == 1 && digit.as_bytes()[0].is_ascii_digit() => ToolKey::Digit(digit.as_bytes()[0] - b'0'),
        _ => return None,
    })
}

/// A session for a kit document: it draws with the bundled fonts and those the host adds, never the
/// machine's, so a drawing renders the same everywhere, and it reaches
/// no file by path (KHP A29, section 10.6): the host opens, saves and exports the document and
/// lends the files it links to, in the page and headless. The engine's host-only mode is on for
/// the session and, since rendering and the engine's worker threads read linked files too, for
/// the whole process ([`kition_vector_engine::file_access`]).
pub(crate) fn kit_session() -> Session {
    kition_vector_text::FontDb::global().set_system_fonts(false);
    kition_vector_engine::file_access::host_files_only_process();
    let mut session = Session::new();
    session.host_files_only = true;
    session
}

impl KitDocument for VectorDocument {
    /// A named entry a model batch creates gets its persistent identity in the batch's own
    /// transaction ([`kdm_model::settle_batch`]).
    fn batch_settler(&self) -> Option<kition_kit::BatchSettler> {
        Some(Box::new(kdm_model::settle_batch))
    }

    /// Another program saved the file of this clean drawing: only what it changed is written, as
    /// an import outside both undo histories, so the session's own steps stay undoable and keep
    /// the file's edits ([`Session::reread_file`]). The ids, the editor state and the images stay.
    fn reconcile_reread(&mut self, fresh: &Self) -> Result<bool, String> {
        let reread = self.session.reread_file(&fresh.active()?.doc).map_err(engine_error)?;
        if reread {
            // A run's scratch copy is of the drawing before the file's edits.
            self.pass.reset();
        }
        Ok(reread)
    }

    /// What is wrong with the drawing (KHP A21): see [`check`].
    fn check(&self) -> Vec<kition_kit::diagnostics::Diagnostic> {
        let fonts = kition_vector_text::FontDb::global();
        self.active().map(|state| check::check(&state.doc, &mut |family| fonts.has_family(family), &mut |c| fonts.can_draw(c))).unwrap_or_default()
    }

    /// Opens a `.kvector` drawing, or SVG, PDF, AI, or EPS: `name`'s extension picks the format.
    fn open(name: &str, bytes: &[u8]) -> Result<Self, String> {
        let mut session = kit_session();
        session.execute("file.open", &json!({ "name": name, "dataBase64": kition_vector_format::base64_encode(bytes) })).map_err(engine_error)?;
        Ok(Self::from_session(session, name))
    }

    /// A new drawing with one artboard of `width × height` points.
    fn create(name: &str, width: u32, height: u32) -> Result<Self, String> {
        let mut session = kit_session();
        let title = std::path::Path::new(name).file_stem().map_or_else(|| "Untitled".to_owned(), |stem| stem.to_string_lossy().into_owned());
        session.execute("file.new", &json!({ "name": title, "width": width, "height": height })).map_err(engine_error)?;
        if let Some(state) = session.active_mut() {
            state.mark_saved();
        }
        Ok(Self::from_session(session, name))
    }

    fn width(&self) -> u32 {
        self.bounds().width().round().max(1.0) as u32
    }

    fn height(&self) -> u32 {
        self.bounds().height().round().max(1.0) as u32
    }

    fn revision(&self) -> u64 {
        self.active().map_or(0, |state| state.revision)
    }

    fn dirty(&self) -> bool {
        self.active().is_ok_and(kition_vector_engine::DocState::is_dirty)
    }

    /// Above 1:1 the art is drawn at the zoom, so it stays sharp; below, it is drawn at 1:1 and
    /// reduced exactly, as the contract requires.
    fn render(&self, x: f64, y: f64, scale: f64, width: u32, height: u32) -> Result<Vec<u8>, String> {
        let (width, height) = (width.clamp(1, MAX_RENDER_SIDE), height.clamp(1, MAX_RENDER_SIDE));
        let scale = if scale.is_finite() && scale > 0.0 { scale } else { 1.0 };
        if scale >= 1.0 {
            return self.draw(x, y, scale, width, height);
        }
        let x0 = x.floor() as i32;
        let y0 = y.floor() as i32;
        let span_x = ((f64::from(width) / scale).ceil() as i32).max(1);
        let span_y = ((f64::from(height) / scale).ceil() as i32).max(1);
        self.render_area(x0, y0, x0.saturating_add(span_x), y0.saturating_add(span_y), width, height)
    }

    fn render_area(&self, x0: i32, y0: i32, x1: i32, y1: i32, width: u32, height: u32) -> Result<Vec<u8>, String> {
        let (w, h) = (
            u32::try_from(x1.saturating_sub(x0).max(1)).unwrap_or(1).min(MAX_RENDER_SIDE),
            u32::try_from(y1.saturating_sub(y0).max(1)).unwrap_or(1).min(MAX_RENDER_SIDE),
        );
        let full = self.draw(f64::from(x0), f64::from(y0), 1.0, w, h)?;
        Ok(raster::render_area(Frame { rgba: &full, width: w, height: h }, 0, 0, w as i32, h as i32, width, height))
    }

    fn execute(&mut self, command: &str, params: Value) -> Result<Value, String> {
        self.pass.reset();
        self.refuse_unknown(command, &params)?;
        let answer = self.session.execute(command, &params).map_err(engine_error);
        self.after_command();
        answer
    }

    /// The drawing as JSON: artboards, layer tree with bounds, selection, history.
    fn inspect(&self) -> Result<Value, String> {
        let mut inspected = self.peek("file.inspect")?;
        let can_undo = inspected.get("history").and_then(Value::as_array).is_some_and(|history| !history.is_empty());
        let can_redo = inspected.get("redo").and_then(Value::as_array).is_some_and(|redo| !redo.is_empty());
        let origin = self.origin();
        // The files the drawing links to and whether they were read (KHP A42), when it links any.
        let links = kition_vector_engine::cmd::links::report(&self.active()?.doc);
        if let Some(object) = inspected.as_object_mut() {
            object.insert("canUndo".into(), Value::Bool(can_undo));
            object.insert("canRedo".into(), Value::Bool(can_redo));
            object.insert("origin".into(), json!([origin.x, origin.y]));
            // The keys the active tool takes now ahead of their shortcuts (the editor gives them to it).
            object.insert("toolKeys".into(), json!(self.claims));
            if !links.is_empty() {
                object.insert("links".into(), Value::Array(links));
            }
        }
        Ok(inspected)
    }

    fn commands(&self) -> Vec<KitCommand> {
        let mut commands: Vec<kition_kit::KitCommand> = {
            command_specs()
                .iter()
                .map(|spec| {
                    let info = spec.info(&self.session);
                    KitCommand {
                        id: info.id.to_owned(),
                        // The label as the menus show it: a trailing "…" marks a command that asks
                        // for values first, which the editor opens a dialog for.
                        title: info.label.to_owned(),
                        menu: (!info.menu.is_empty()).then(|| info.menu.iter().map(|part| (*part).to_owned()).collect()),
                        shortcut: info.shortcut.map(str::to_owned),
                        params: Some(info.params.to_owned()),
                        mutates: mutates(spec),
                        enabled: info.enabled,
                        ..Default::default()
                    }
                    .with_kit_schema()
                })
                .collect()
        };
        agent_text::apply(&mut commands);
        for command in &mut commands {
            if let Some(schema) = command.schema.as_mut() {
                params::complete(&command.id, schema);
            }
        }
        params::export_bytes(&mut commands);
        for command in &mut commands {
            address::apply(command);
        }
        kition_kit::files::declare(&mut commands, files::FILES);
        // Served to an agent by a host that owns every file: what needs a folder of its own goes.
        if hosting::files_only() {
            commands.retain(|command| !gate::HOSTED_OFF.contains(&command.id.as_str()));
        }
        commands
    }

    /// The commands that cannot run now, without building the full list and its schemas: what
    /// Kition asks after every change for menu and shortcut states.
    fn disabled_commands(&self) -> Vec<String> {
        command_specs().iter().filter(|spec| (spec.enabled)(&self.session).is_err()).map(|spec| spec.id.to_owned()).collect()
    }

    /// The drawing as file bytes in the format of `name`'s extension (`.kvector` by default).
    fn save(&self, name: Option<&str>) -> Result<Vec<u8>, String> {
        let target = name.unwrap_or(&self.name);
        let extension =
            std::path::Path::new(target).extension().map(|extension| extension.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
        let state = self.active()?;
        if extension.is_empty() || kition_vector_format::is_native_ext(&extension) {
            return Ok(kition_vector_format::save_file(&state.doc));
        }
        Err(format!("saving as .{extension} is an export; save the drawing as .kvector"))
    }

    /// `document.text` (KitText 1.0): the drawing's words, a part per artboard in reading order
    /// (see [`document_text`]). It changes nothing.
    fn text(&self, limit: usize) -> Result<kition_kit::text::DocumentText, String> {
        Ok(document_text::document_text(&self.active()?.doc, limit))
    }

    fn mark_saved(&mut self) {
        if let Some(state) = self.session.active_mut() {
            state.mark_saved();
        }
    }

    fn editor_state(&self) -> Value {
        let Some(snapshot) = self.session.editor_snapshot() else { return Value::Null };
        let value = snapshot.to_json();
        if let Ok(mut snapshots) = self.snapshots.try_borrow_mut() {
            snapshots.retain(|(kept, _)| *kept != value);
            snapshots.push((value.clone(), snapshot));
            let excess = snapshots.len().saturating_sub(32);
            snapshots.drain(..excess);
        }
        value
    }

    fn restore_editor_state(&mut self, saved: &Value) -> Result<(), String> {
        if saved.is_null() {
            return Ok(());
        }
        let kept = self.snapshots.borrow().iter().rev().find(|(value, _)| value == saved).map(|(_, snapshot)| snapshot.clone());
        let mut snapshot = match kept {
            Some(snapshot) => snapshot,
            // A state this document did not answer (the session's before its file was read
            // again): what its JSON says, over the session as it is, once everything it names is
            // in this drawing; none is silently left out.
            None => {
                if let Some(reason) = kition_vector_engine::editor_state::unresolved(&self.active()?.doc, saved) {
                    return Err(reason);
                }
                self.session.editor_snapshot().ok_or_else(|| "no document is open".to_owned())?
            }
        };
        let ids = |key: &str| -> Vec<kition_vector_doc::NodeId> {
            saved.get(key).and_then(Value::as_array).into_iter().flatten().filter_map(Value::as_u64).map(kition_vector_doc::NodeId).collect()
        };
        let id = |key: &str| saved.get(key).and_then(Value::as_u64).map(kition_vector_doc::NodeId);
        if let Some(selection) = saved.get("selection") {
            snapshot.selection =
                serde_json::from_value(selection.clone()).map_err(|error| format!("invalid-params: the saved selection: {error}"))?;
        }
        snapshot.active_layer = id("currentLayer");
        snapshot.layer_rows = ids("layerRows");
        snapshot.isolation = id("isolation");
        snapshot.variables_highlight = saved.get("variable").and_then(Value::as_str).map(str::to_owned);
        self.session.restore_editor(snapshot);
        Ok(())
    }

    /// `on`, object ids as the inspect lists them, becomes the selection. The drawing has no
    /// current part or time an agent sets: every command that acts on an artboard takes it.
    fn set_context(&mut self, context: &kition_kit::CallContext) -> Result<(), String> {
        if context.part.is_some() {
            return Err("invalid-params: Kition Vector has no current part; give the command its `artboard`".to_owned());
        }
        if context.time.is_some() {
            return Err("invalid-params: a drawing has no time".to_owned());
        }
        if context.on.is_empty() {
            return Ok(());
        }
        let state = self.session.active_mut().ok_or_else(|| "no document is open".to_owned())?;
        let mut ids = Vec::new();
        for value in &context.on {
            let id = value
                .as_u64()
                .or_else(|| value.as_str().and_then(|text| text.parse().ok()))
                .map(kition_vector_doc::NodeId)
                .ok_or_else(|| format!("invalid-params: {value} is not an object id"))?;
            let node = state.doc.node(id).ok_or_else(|| format!("invalid-params: the drawing has no object {value}"))?;
            if node.is_layer() {
                return Err(format!("invalid-params: {value} is a layer; `on` takes objects (a layer command takes its `ids`)"));
            }
            ids.push(id);
        }
        state.selection = kition_vector_doc::Selection::default();
        state.selection.objects = ids;
        Ok(())
    }

    fn derived(&self, ids: &[Value], names: &[String]) -> Result<Vec<kition_kit::DerivedValues>, String> {
        Ok(place::derived(&self.active()?.doc, ids, names))
    }

    fn under(&self, command: &str, params: &Value) -> Result<Option<kition_kit::Under>, String> {
        let state = self.active()?;
        // The selection an earlier step of the run made, with the one before the run ([`pass`]).
        let mut selection = state.selection.objects.clone();
        for id in self.pass.selected().unwrap_or_default() {
            if !selection.contains(&id) {
                selection.push(id);
            }
        }
        let clipboard: Vec<String> = self.session.clipboard.swatches.iter().map(|swatch| swatch.name.clone()).collect();
        let mut answer = place::under(&self.editor(state, &selection, clipboard.clone()), command, params);
        // And what it rests on in the drawing the earlier steps of the run leave.
        let later = self.pass.with_state(|copy| {
            let selection = copy.selection.objects.clone();
            place::under(&self.editor(copy, &selection, clipboard.clone()), command, params)
        });
        if let (Some(answer), Some(Some(later))) = (answer.as_mut(), later) {
            place::join(answer, later);
        }
        self.pass.ask(state, command, params);
        Ok(answer)
    }

    /// What a picking command's point or stroke named at `basis` ([`place::covered`]).
    fn covered(&self, command: &str, params: &Value, objects: &[kition_kit::CoveredAsk]) -> Result<Option<kition_kit::CoveredAt>, String> {
        let state = self.active()?;
        let mut selection = state.selection.objects.clone();
        for id in self.pass.asked_selection().unwrap_or_default() {
            if !selection.contains(&id) {
                selection.push(id);
            }
        }
        let clipboard: Vec<String> = self.session.clipboard.swatches.iter().map(|swatch| swatch.name.clone()).collect();
        Ok(place::covered(&self.editor(state, &selection, clipboard), command, params, objects))
    }
}

impl VectorDocument {
    /// The engine state a call's premises are read in ([`place::Editor`]). The guard asks only for
    /// an agent's writes: new art goes where an agent's would ([`kition_vector_engine::DocState::target_parent`]),
    /// nowhere when the agent's call would be refused (the current layer locked or hidden).
    fn editor<'a>(
        &self,
        state: &'a kition_vector_engine::DocState,
        selection: &'a [kition_vector_doc::NodeId],
        clipboard: Vec<String>,
    ) -> place::Editor<'a> {
        let insertion = if state.passed_over_layer().is_some() { None } else { state.target_parent().ok().flatten() };
        let mut rows = state.highlighted_rows();
        if rows.is_empty() {
            rows.extend(state.current_layer());
        }
        place::Editor { doc: &state.doc, selection, subjects: state.selection.subjects(), rows, insertion, clipboard }
    }

    /// Runs a read-only engine query. Queries don't change the document; the engine's API takes
    /// `&mut self` for all commands, so this clones the session's active state cheaply (an
    /// `Arc` document) into a scratch session.
    fn peek(&self, command: &str) -> Result<Value, String> {
        let state = self.active()?;
        let mut scratch = Session::new();
        scratch.host_files_only = self.session.host_files_only;
        // The paint new art gets and the proxy in front: the inspect's `paint` reports them.
        scratch.paint = self.session.paint.clone();
        scratch.fill_active = self.session.fill_active;
        scratch.add_document((*state.doc).clone(), None);
        if let Some(copy) = scratch.active_mut() {
            copy.selection = state.selection.clone();
            // The isolated group too: the inspect reports it (isolation mode on the canvas).
            copy.isolation = state.isolation;
            // The current layer and the Layers panel's highlighted rows: the inspect reports them
            // (`currentLayer`, `layerRows`), and the panel draws them.
            copy.active_layer = state.active_layer;
            copy.layer_rows = state.layer_rows.clone();
            copy.revision = state.revision;
            copy.history = state.history.clone();
        }
        scratch.execute(command, &json!({})).map_err(engine_error)
    }
}

impl KitTools for VectorDocument {
    fn tools(&self) -> Vec<ToolInfo> {
        kition_vector_tools::catalog::TOOL_GROUPS
            .iter()
            .enumerate()
            .flat_map(|(group, tools)| {
                tools.iter().map(move |tool| ToolInfo {
                    id: tool.id.to_owned(),
                    label: tool.label.to_owned(),
                    shortcut: tool.shortcut.map(str::to_owned),
                    icon: tool.icon.to_owned(),
                    group,
                })
            })
            .collect()
    }

    fn tool(&self) -> String {
        self.session.tool_id().to_owned()
    }

    fn select_tool(&mut self, id: &str, zoom: f64) -> Result<ToolResponse, String> {
        if kition_vector_tools::tool_info(id).is_none() {
            return Err(format!("no tool {id}"));
        }
        self.session.select_tool(id, Self::view(zoom)).map_err(engine_error)?;
        self.held.set(None);
        Ok(self.respond(Vec::new(), None, zoom))
    }

    fn pointer(&mut self, event: &ToolPointer) -> Result<ToolResponse, String> {
        let at = self.to_engine(event.x, event.y);
        // A gesture keeps the origin it began with (see `bounds`).
        if event.action == PointerAction::Down && self.held.get().is_none() {
            self.held.set(Some(self.bounds()));
        }
        self.mods = mods_in(event.mods);
        let kind = match event.action {
            PointerAction::Down => PointerKind::Down,
            PointerAction::Drag => PointerKind::Drag,
            PointerAction::Up => PointerKind::Up,
            PointerAction::Move => PointerKind::Move,
            PointerAction::DoubleClick => PointerKind::DoubleClick,
        };
        self.hover = (event.action == PointerAction::Move).then_some(at);
        let engine_event = PointerEvent { kind, pos: at, mods: self.mods, pressure: event.pressure.clamp(0.0, 1.0) };
        // A drag that moves, scales or rotates the selection changes only the pixels under it,
        // where it was and where it is now: the page repaints just that ([`Self::damage_of`]).
        let before = (kind == PointerKind::Drag && self.session.tool_transforming()).then(|| self.selected_bounds()).flatten();
        let (revision, extent) = (self.revision(), self.bounds());
        let answer = self.session.pointer(&engine_event, Self::view(event.zoom)).map_err(engine_error);
        self.damage = before.and_then(|before| self.damage_of(before, revision, extent));
        if !self.session.tool_busy() {
            self.held.set(None);
        }
        Ok(self.respond(answer?, Some(at), event.zoom))
    }

    fn key(&mut self, key: &str, mods: ToolMods, zoom: f64) -> Result<ToolResponse, String> {
        let Some(engine_key) = key_in(key) else { return Ok(self.respond(Vec::new(), None, zoom)) };
        let requests = self.session.tool_key(engine_key, mods_in(mods), Self::view(zoom)).map_err(engine_error);
        // An Escape that dropped the gesture ends what it held.
        if !self.session.tool_busy() {
            self.held.set(None);
        }
        Ok(self.respond(requests?, None, zoom))
    }

    fn text(&mut self, text: &str, zoom: f64) -> Result<ToolResponse, String> {
        let requests = self.session.tool_text(text, Self::view(zoom)).map_err(engine_error)?;
        Ok(self.respond(requests, None, zoom))
    }

    fn state(&mut self, x: f64, y: f64, _mods: ToolMods, zoom: f64) -> ToolResponse {
        let at = self.to_engine(x, y);
        self.respond(Vec::new(), Some(at), zoom)
    }

    fn take_damage(&mut self) -> Option<kition_kit::tools::ToolDamage> {
        self.damage.take()
    }
}

/// Points of margin around a moved selection's repaint: anti-aliased edges and hairlines.
const DAMAGE_MARGIN: f64 = 4.0;

impl VectorDocument {
    /// The document pixels a drag of the selection changed, from its visual bounds `before` the
    /// event to now; `None` (repaint everything) unless the drawing changed and everything that
    /// changed is the moved art: no blend, text wrap, envelope or symbol that other art follows,
    /// and no growth of the extent (its pixels moved).
    fn damage_of(&self, before: Rect, revision: u64, extent: Rect) -> Option<kition_kit::tools::ToolDamage> {
        if self.revision() == revision || !self.session.tool_transforming() || self.bounds() != extent {
            return None;
        }
        let state = self.active().ok()?;
        if !local_edits(&state.doc) {
            return None;
        }
        let after = self.selected_bounds()?;
        let origin = self.origin();
        let area = before.union(after).inflate(DAMAGE_MARGIN, DAMAGE_MARGIN);
        let bounds = self.bounds();
        if !(area.x0.is_finite() && area.y0.is_finite() && area.x1.is_finite() && area.y1.is_finite()) {
            return None;
        }
        Some(kition_kit::tools::ToolDamage {
            x0: (area.x0.max(bounds.x0) - origin.x).floor(),
            y0: (area.y0.max(bounds.y0) - origin.y).floor(),
            x1: (area.x1.min(bounds.x1) - origin.x).ceil(),
            y1: (area.y1.min(bounds.y1) - origin.y).ceil(),
        })
    }
}

/// Whether moving art in `doc` changes the pixels of that art alone: none of its art is drawn from
/// other art (a blend's steps, type wrapping round art, an envelope, a symbol instance, a live
/// paint group, an opacity mask), which a moved object would redraw elsewhere.
fn local_edits(doc: &kition_vector_doc::Document) -> bool {
    fn plain(node: &kition_vector_doc::Node) -> bool {
        use kition_vector_doc::NodeKind;
        let own = matches!(
            node.kind,
            NodeKind::Layer { .. }
                | NodeKind::Group { .. }
                | NodeKind::Path { .. }
                | NodeKind::Compound { .. }
                | NodeKind::Text(_)
                | NodeKind::Image(_)
        ) && node.mask.is_none()
            && node.wrap.is_none()
            && !node.region_paint;
        own && node.children().into_iter().flatten().all(|child| plain(child))
    }
    doc.layers.iter().all(|layer| plain(layer))
}

kition_kit::export_kit_document!(VectorDocument as "VectorDocument");
kition_kit::export_kit_tools!(VectorDocument as "VectorDocument");

/// Vector's own additions to the exported class: the collaboration members (engine contract 2).
/// Shapes mirror `kition-sdk`'s `session::web` (`export_kit_session!` does not apply here; see
/// `crate::collab`).
#[cfg(target_arch = "wasm32")]
pub mod web {
    use kition_kit::wasm_bindgen::prelude::*;
    use serde_json::json;

    use crate::__kition_kit_export::WebDocument;

    fn error(message: String) -> JsError {
        JsError::new(&message)
    }

    /// An origin in the KDM's commit form, the JSON array `[kind, principal, label]`.
    fn origin_of(origin: &str) -> Result<kition_vector_engine::Origin, JsError> {
        kition_vector_engine::Origin::decode(origin).ok_or_else(|| error(format!("origin is not [kind, principal, label]: {origin}")))
    }

    #[wasm_bindgen(js_class = "VectorDocument", wasm_bindgen = kition_kit::wasm_bindgen)]
    impl WebDocument {
        pub fn version(&mut self) -> Result<Vec<u8>, JsError> {
            self.0.version().map_err(error)
        }

        #[wasm_bindgen(js_name = exportChanges)]
        pub fn export_changes(&mut self, since: &[u8]) -> Result<Vec<u8>, JsError> {
            self.0.export_changes(since).map_err(error)
        }

        #[wasm_bindgen(js_name = importChanges)]
        pub fn import_changes(&mut self, bytes: &[u8]) -> Result<String, JsError> {
            self.0.import_changes(bytes).map(|damage| serde_json::to_string(&damage).unwrap_or_else(|_| "null".to_owned())).map_err(error)
        }

        pub fn snapshot(&mut self) -> Result<Vec<u8>, JsError> {
            self.0.snapshot().map_err(error)
        }

        pub fn undo(&mut self) -> Result<String, JsError> {
            self.0.undo().map(|(done, damage)| json!({ "done": done, "damage": damage }).to_string()).map_err(error)
        }

        pub fn redo(&mut self) -> Result<String, JsError> {
            self.0.redo().map(|(done, damage)| json!({ "done": done, "damage": damage }).to_string()).map_err(error)
        }

        #[wasm_bindgen(js_name = canUndo)]
        pub fn can_undo(&self) -> bool {
            self.0.can_undo()
        }

        #[wasm_bindgen(js_name = canRedo)]
        pub fn can_redo(&self) -> bool {
            self.0.can_redo()
        }

        /// `origin` is the KDM's commit form, the JSON array `[kind, principal, label]`.
        #[wasm_bindgen(js_name = executeAs)]
        pub fn execute_as(&mut self, command: &str, params: &str, origin: &str) -> Result<String, JsError> {
            let params = kition_kit::parse_params(params).map_err(error)?;
            let origin = origin_of(origin)?;
            self.0.execute_as(command, params, &origin).map(|value| value.to_string()).map_err(error)
        }

        /// Every blob the replica references or can still bring back, for a peer's `addBlobs`.
        pub fn blobs(&mut self) -> Result<Vec<u8>, JsError> {
            self.0.blobs().map_err(error)
        }

        /// Adds a peer's blobs (from their `blobs()`): kept until the next `importChanges`.
        #[wasm_bindgen(js_name = addBlobs)]
        pub fn add_blobs(&mut self, bytes: &[u8]) -> Result<(), JsError> {
            self.0.add_blobs(bytes).map_err(error)
        }

        /// The blobs newly needed since `since` (empty bytes: everything the current state
        /// references), encoded like `blobs()`: for live sync, so a change ships only the blobs
        /// it needs.
        #[wasm_bindgen(js_name = blobsSince)]
        pub fn blobs_since(&mut self, since: &[u8]) -> Result<Vec<u8>, JsError> {
            self.0.blobs_since(since).map_err(error)
        }

        #[wasm_bindgen(js_name = kdmResets)]
        pub fn kdm_resets(&self) -> u32 {
            self.0.kdm_resets()
        }

        /// `model/describe` (KitObjectModel 1.0) as JSON text; a failure's message is the error's
        /// JSON (`{"code", "index", "message"}`).
        #[wasm_bindgen(js_name = modelDescribe)]
        pub fn model_describe(&mut self) -> Result<String, JsError> {
            self.0.model_describe().map(|answer| answer.to_string()).map_err(|failure| error(failure.to_string()))
        }

        /// `model/batch`: the request and the answer as JSON text, one undo step; a failure's
        /// message is the error's JSON (`{"code", "index", "message"}`).
        #[wasm_bindgen(js_name = modelBatch)]
        pub fn model_batch(&mut self, request: &str) -> Result<String, JsError> {
            self.0.model_batch(request).map(|answer| answer.to_string()).map_err(|failure| error(failure.to_string()))
        }

        /// A draft's changes taken in as one undo step, recorded with `origin` (`[kind, principal,
        /// label]`): `{"damage", "summary"}`. A refusal fails with the KDM's message.
        #[wasm_bindgen(js_name = mergeChanges)]
        pub fn merge_changes(&mut self, changes: &[u8], origin: &str) -> Result<String, JsError> {
            let origin = origin_of(origin)?;
            self.0.merge_changes(changes, &origin).map(|answer| answer.to_string()).map_err(error)
        }

        /// What merging a draft's changes would change now, as `StateDiff` JSON; changes nothing.
        #[wasm_bindgen(js_name = compareChanges)]
        pub fn compare_changes(&self, changes: &[u8]) -> Result<String, JsError> {
            self.0.compare_changes(changes).map(|answer| answer.to_string()).map_err(error)
        }

        /// Writes the state a snapshot holds over the drawing as one change of `origin`; answers
        /// the damage JSON.
        #[wasm_bindgen(js_name = restoreState)]
        pub fn restore_state(&mut self, snapshot: &[u8], origin: &str) -> Result<String, JsError> {
            let origin = origin_of(origin)?;
            self.0.restore_state(snapshot, &origin).map(|damage| serde_json::to_string(&damage).unwrap_or_else(|_| "null".to_owned())).map_err(error)
        }

        /// An agent's reply (JSON text) with its object ids named as the tool interface names them.
        #[wasm_bindgen(js_name = modelReply)]
        pub fn model_reply(&self, value: &str) -> Result<String, JsError> {
            self.0.model_reply(value).map_err(error)
        }

        /// What changed since the version `since` (a receipt): `{"changes": StateDiff}`, or
        /// `{"changes": null, "unavailable"}` when the replica's log does not hold it.
        #[wasm_bindgen(js_name = changesSince)]
        pub fn changes_since(&self, since: &[u8]) -> String {
            self.0.changes_since(since).to_string()
        }

        /// Opens a file with the replica a host stored for it (the Kit UI protocol's `replica`
        /// capability); a replica that does not open is ignored and the file opens plainly.
        #[wasm_bindgen(js_name = openWithReplica)]
        pub fn open_with_replica(name: &str, bytes: &[u8], snapshot: &[u8], blobs: &[u8]) -> Result<WebDocument, JsError> {
            crate::VectorDocument::open_with_replica(name, bytes, snapshot, blobs).map(Self).map_err(error)
        }

        /// Whether the drawing has made its replica yet (it makes one on the first shared edit).
        #[wasm_bindgen(getter, js_name = hasReplica)]
        pub fn has_replica(&self) -> bool {
            self.0.has_replica()
        }

        /// The replica's own document id, for the Kit UI protocol's `replica/info` message.
        /// `undefined` before the drawing has made a replica yet.
        #[wasm_bindgen(getter, js_name = docId)]
        pub fn doc_id(&self) -> Option<String> {
            self.0.doc_id()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kition_kit::testing::check_contract;

    /// A 200 × 100 pt drawing with one rectangle.
    fn fixture() -> Vec<u8> {
        let mut doc = VectorDocument::create("Drawing.kvector", 200, 100).unwrap();
        doc.execute("shape.rectangle", json!({ "x": 20, "y": 10, "width": 50, "height": 30 })).unwrap();
        doc.save(None).unwrap()
    }

    #[test]
    fn keeps_the_kit_contract() {
        check_contract::<VectorDocument>(&fixture(), "Drawing.kvector", |doc| {
            doc.execute("shape.ellipse", json!({ "x": 100, "y": 20, "width": 60, "height": 60 })).map(|_| ())
        })
        .unwrap();
    }

    #[test]
    fn renders_the_artboard_and_its_art() {
        let doc = VectorDocument::open("Drawing.kvector", &fixture()).unwrap();
        assert_eq!((doc.width(), doc.height()), (200, 100));
        let pixels = doc.render(0.0, 0.0, 1.0, 200, 100).unwrap();
        let at = |x: usize, y: usize| &pixels[(y * 200 + x) * 4..(y * 200 + x) * 4 + 4];
        assert_eq!(at(5, 5), &[255, 255, 255, 255], "the artboard is white");
        assert!(at(20, 25)[..3].iter().all(|&channel| channel < 160), "the rectangle's dark stroke: {:?}", at(20, 25));
        let sharp = doc.render(18.0, 8.0, 8.0, 64, 64).unwrap();
        assert_eq!(sharp.len(), 64 * 64 * 4, "zoomed in, drawn at the zoom");
        assert!(doc.inspect().unwrap()["canUndo"].is_boolean());
    }

    #[test]
    fn rejects_foreign_native_format_tags_even_under_the_native_extension() {
        let ours = fixture();
        let text = String::from_utf8(ours.clone()).unwrap();
        // Saved pretty-printed (`"format": "kvector"`).
        let foreign = text.replacen("\"format\": \"kvector\"", "\"format\": \"otherapp\"", 1);
        assert_ne!(foreign, text);
        for name in ["Drawing.otherapp", "Drawing.kvector"] {
            assert!(VectorDocument::open(name, foreign.as_bytes()).is_err(), "{name}");
        }
        let doc = VectorDocument::open("Drawing.kvector", &ours).unwrap();
        let saved = doc.save(Some("Drawing.kvector")).unwrap();
        assert!(kition_vector_format::load_file(&saved).is_ok());
    }

    #[test]
    fn the_engines_rectangle_tool_draws_through_pointer_events() {
        let mut doc = VectorDocument::open("Drawing.kvector", &fixture()).unwrap();
        assert!(doc.tools().iter().any(|tool| tool.id == "rectangle"), "{:?}", doc.tools().iter().map(|tool| tool.id.clone()).collect::<Vec<_>>());
        doc.select_tool("rectangle", 1.0).unwrap();
        let before = doc.revision();
        let event = |action, x, y| ToolPointer { action, x, y, mods: ToolMods::default(), pressure: 1.0, zoom: 1.0 };
        doc.pointer(&event(PointerAction::Down, 100.0, 50.0)).unwrap();
        let dragging = doc.pointer(&event(PointerAction::Drag, 150.0, 80.0)).unwrap();
        assert!(dragging.busy, "the tool is mid-drag");
        doc.pointer(&event(PointerAction::Up, 150.0, 80.0)).unwrap();
        assert!(doc.revision() > before);
        let objects = doc.inspect().unwrap()["objects"].as_u64().unwrap_or(0);
        assert!(objects >= 3, "a second rectangle was drawn ({objects} objects)");
    }

    #[test]
    fn the_selection_shows_as_overlays_in_document_pixels() {
        let mut doc = VectorDocument::open("Drawing.kvector", &fixture()).unwrap();
        doc.select_tool("selection", 1.0).unwrap();
        doc.execute("edit.selectAll", json!({})).unwrap();
        let shown = doc.state(0.0, 0.0, ToolMods::default(), 1.0);
        let paths = shown.overlays.iter().filter(|overlay| matches!(overlay, ToolOverlay::Path { .. })).count();
        let anchors = shown.overlays.iter().filter(|overlay| matches!(overlay, ToolOverlay::Anchor { .. })).count();
        assert!(paths >= 2, "the outline and the bounding box: {:?}", shown.overlays);
        assert_eq!(anchors, 4 + 1 + 8, "four anchors, the centre, and eight box handles");
        assert!(
            shown
                .overlays
                .iter()
                .any(|overlay| matches!(overlay, ToolOverlay::Anchor { x, y, .. } if (*x - 20.0).abs() < 1e-6 && (*y - 10.0).abs() < 1e-6))
        );
        doc.execute("edit.deselectAll", json!({})).unwrap();
        assert!(doc.state(0.0, 0.0, ToolMods::default(), 1.0).overlays.is_empty());
        let hovering =
            doc.pointer(&ToolPointer { action: PointerAction::Move, x: 20.0, y: 25.0, mods: ToolMods::default(), pressure: 1.0, zoom: 1.0 }).unwrap();
        assert_eq!(hovering.overlays.len(), 1, "the hover highlight");
    }

    #[test]
    fn mutates_means_the_command_changes_the_drawing() {
        let doc = VectorDocument::open("Drawing.kvector", &fixture()).unwrap();
        let commands = doc.commands();
        let flag = |id: &str| commands.iter().find(|command| command.id == id).map(|command| command.mutates);
        for id in [
            "shape.rectangle",
            "arrange.group",
            "edit.undo",
            "edit.redo",
            "edit.cut",
            "file.new",
            "file.save",
            "command.batch",
            "select.save",
            // Export for Screens remembers its settings in the drawing.
            "file.exportForScreens",
        ] {
            assert_eq!(flag(id), Some(true), "{id} changes the drawing");
        }
        for id in [
            "edit.selectAll",
            "edit.deselectAll",
            "view.drawMode",
            "edit.copy",
            "file.export",
            "file.exportBytes",
            "file.exportPdf",
            "file.exportEps",
            "file.exportDxf",
            "file.exportForWeb",
            "file.saveCopy",
            "layer.setCurrent",
            "file.inspect",
        ] {
            assert_eq!(flag(id), Some(false), "{id} does not change the drawing");
        }
        for id in VIEWING {
            assert!(flag(id).is_some(), "{id} is a command");
        }
    }

    #[test]
    fn commands_that_do_not_mutate_leave_the_replica_alone() {
        let mut doc = VectorDocument::open("Drawing.kvector", &fixture()).unwrap();
        doc.execute("shape.ellipse", json!({ "x": 100, "y": 20, "width": 60, "height": 60 })).unwrap();
        let before = doc.version().unwrap();
        for (id, params) in [
            ("edit.selectAll", json!({})),
            ("select.inverse", json!({})),
            ("edit.deselectAll", json!({})),
            ("edit.selectAll", json!({})),
            ("edit.copy", json!({})),
            ("view.drawMode", json!({ "mode": "behind" })),
            ("paint.toggleActive", json!({})),
            ("file.export", json!({ "format": "svg" })),
            ("file.exportBytes", json!({ "format": "png", "scale": 2, "background": "white", "artboard": 0 })),
            ("file.exportPdf", json!({})),
            ("file.exportEps", json!({})),
            ("file.exportDxf", json!({})),
            ("file.exportForWeb", json!({ "format": "png24" })),
        ] {
            assert!(!doc.commands().iter().any(|command| command.id == id && command.mutates), "{id}");
            let _ = doc.execute(id, params);
            assert_eq!(doc.version().unwrap(), before, "{id} committed to the replica");
        }
        doc.execute("arrange.group", json!({})).unwrap();
        assert_ne!(doc.version().unwrap(), before, "an edit commits");
    }

    #[test]
    fn export_bytes_takes_the_export_options_but_a_path() {
        let mut doc = VectorDocument::open("Drawing.kvector", &fixture()).unwrap();
        let answer = doc.execute("file.exportBytes", json!({ "format": "png", "scale": 2, "background": "white", "artboard": 0 })).unwrap();
        assert!(answer["data"].as_str().is_some_and(|data| data.starts_with("iVBORw0KGgo")), "{answer}");
        let refused = doc.execute("file.exportBytes", json!({ "format": "svg", "path": "/tmp/x.svg" })).err().unwrap_or_default();
        assert!(refused.contains("path"), "{refused}");
    }

    fn press(doc: &mut VectorDocument, action: PointerAction, x: f64, y: f64) -> ToolResponse {
        doc.pointer(&ToolPointer { action, x, y, mods: ToolMods::default(), pressure: 1.0, zoom: 1.0 }).unwrap()
    }

    /// Art off the artboards stays drawn (plan 2026-11-25, stage 2): the extent takes it in, the
    /// origin moving by whole steps, while the drawing's own coordinates, the ones commands and
    /// `inspect()` give, stay what they were.
    #[test]
    fn art_off_the_artboards_is_on_the_canvas_in_its_own_coordinates() {
        let mut doc = VectorDocument::open("Drawing.kvector", &fixture()).unwrap();
        assert_eq!((doc.width(), doc.height()), (200, 100), "art on the artboard leaves the extent as it was");
        doc.execute("shape.rectangle", json!({ "x": -150, "y": 20, "width": 40, "height": 40 })).unwrap();
        let inspected = doc.inspect().unwrap();
        let x0 = inspected["origin"][0].as_f64().unwrap();
        assert_eq!(inspected["origin"][1].as_f64(), Some(0.0), "nothing above the artboard: the top stays");
        assert!(x0 <= -150.0 - PASTEBOARD_MARGIN && (x0 / PASTEBOARD_STEP).fract() == 0.0, "the origin moved left by whole steps: {x0}");
        assert_eq!(doc.width(), (200.0 - x0) as u32);
        assert_eq!(inspected["selectionBounds"]["x"].as_f64(), Some(-150.0), "the drawing's coordinates are its own");
        // The rectangle is drawn on the pasteboard; around it nothing is (the backdrop shows).
        let (cx, cy) = (-130.0 - x0, 40.0);
        let pixels = doc.render(cx - 1.0, cy - 1.0, 1.0, 2, 2).unwrap();
        assert!(pixels[3] > 0, "the rectangle off the artboard is drawn: {:?}", &pixels[..4]);
        let empty = doc.render(cx + 40.0, cy, 1.0, 2, 2).unwrap();
        assert_eq!(empty[3], 0, "the pasteboard itself is clear");
        let board = doc.render(-x0 + 5.0, 5.0, 1.0, 1, 1).unwrap();
        assert_eq!(board, vec![255, 255, 255, 255], "the artboard is white where it was");
    }

    /// A drag keeps the origin it began with, so its pixels go on meaning the same points; the
    /// release lets the extent take in where the art went.
    #[test]
    fn a_drag_off_the_artboard_keeps_its_origin_until_the_release() {
        let mut doc = VectorDocument::open("Drawing.kvector", &fixture()).unwrap();
        doc.select_tool("selection", 1.0).unwrap();
        press(&mut doc, PointerAction::Down, 45.0, 25.0);
        press(&mut doc, PointerAction::Drag, 0.0, 25.0);
        press(&mut doc, PointerAction::Drag, -100.0, 25.0);
        assert_eq!(doc.origin(), Point::new(0.0, 0.0), "the origin holds while the drag lasts");
        press(&mut doc, PointerAction::Up, -100.0, 25.0);
        let bounds = doc.inspect().unwrap()["selectionBounds"].clone();
        assert_eq!(bounds["x"].as_f64().map(f64::round), Some(-125.0), "moved by the drag: {bounds}");
        assert!(doc.origin().x < -125.0, "after the release the extent takes the rectangle in: {:?}", doc.origin());
    }

    /// A drag that moves the selection names the area it changed (where the art was and is), and
    /// the page repaints only that.
    #[test]
    fn a_drag_of_the_selection_repaints_only_where_it_was_and_is() {
        let mut doc = VectorDocument::open("Drawing.kvector", &fixture()).unwrap();
        doc.select_tool("selection", 1.0).unwrap();
        press(&mut doc, PointerAction::Down, 45.0, 25.0);
        press(&mut doc, PointerAction::Drag, 55.0, 30.0);
        let _ = doc.take_damage();
        press(&mut doc, PointerAction::Drag, 75.0, 35.0);
        let damage = doc.take_damage().expect("the moved rectangle's area");
        assert!(damage.x0 <= 30.0 && damage.y0 <= 15.0 && damage.x1 >= 100.0 && damage.y1 >= 50.0, "{damage:?}");
        assert!(damage.x1 - damage.x0 < 120.0, "not the whole drawing: {damage:?}");
        assert!(doc.take_damage().is_none(), "taken once");
        press(&mut doc, PointerAction::Up, 75.0, 35.0);
        assert!(doc.take_damage().is_none(), "the release repaints everything");
    }

    /// While the Pen draws a path it takes Enter and Backspace ahead of their shortcuts, and says
    /// so in `inspect()`; once Enter ended the path the keys are the shortcuts' again.
    #[test]
    fn the_keys_a_tool_takes_are_in_the_inspect() {
        let mut doc = VectorDocument::open("Drawing.kvector", &fixture()).unwrap();
        doc.select_tool("pen", 1.0).unwrap();
        let keys = |doc: &VectorDocument| doc.inspect().unwrap()["toolKeys"].clone();
        assert_eq!(keys(&doc), json!([]));
        for (x, y) in [(120.0, 60.0), (160.0, 80.0)] {
            press(&mut doc, PointerAction::Down, x, y);
            press(&mut doc, PointerAction::Up, x, y);
        }
        let taken = keys(&doc);
        assert!(taken.as_array().is_some_and(|keys| keys.contains(&json!("Enter")) && keys.contains(&json!("Backspace"))), "{taken}");
        doc.key("Enter", ToolMods::default(), 1.0).unwrap();
        assert_eq!(keys(&doc), json!([]), "the path ended");
    }

    #[test]
    fn bad_input_is_an_error_not_a_panic() {
        assert!(VectorDocument::open("Broken.kvector", b"not a drawing").is_err());
        let mut doc = VectorDocument::open("Drawing.kvector", &fixture()).unwrap();
        assert!(doc.execute("no.such.command", json!({})).is_err());
        assert!(doc.select_tool("no-such-tool", 1.0).is_err());
        assert!(doc.save(Some("Drawing.svg")).is_err());
    }
}
