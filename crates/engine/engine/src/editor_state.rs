//! The editing session around the active document, as an agent's call saves and puts it back
//! (Kit Process Protocol section 7a): what a command may read besides its parameters. None of it is
//! the document: taking or restoring it is no edit (no revision, no dirty state, no undo step).

use kition_vector_doc::NodeId;
use kition_vector_doc::selection::Selection;
use kition_vector_geom::Affine;
use kition_vector_tools::PaintDefaults;
use serde_json::{Value, json};

use crate::{DrawMode, Session, ViewInfo};

/// Everything [`Session::editor_snapshot`] keeps.
#[derive(Clone, Debug)]
pub struct EditorSnapshot {
    pub selection: Selection,
    pub active_layer: Option<NodeId>,
    pub layer_rows: Vec<NodeId>,
    pub variables_highlight: Option<String>,
    pub isolation: Option<NodeId>,
    pub mask_view: Option<NodeId>,
    last_transform: Option<(Affine, bool)>,
    last_selection_cmd: Option<(String, Value, Vec<NodeId>)>,
    last_perspective: Option<Value>,
    pub tool: &'static str,
    paint: PaintDefaults,
    fill_active: bool,
    draw_mode: DrawMode,
    draw_inside: Option<NodeId>,
    gradient_stop: Option<(usize, crate::cmd::gradient::StopOwner)>,
    appearance_item: Option<crate::cmd::appearance::ActiveItem>,
    freeform_point: Option<(usize, crate::cmd::gradient::StopOwner)>,
    /// The preferences that change what a write does to the same objects when its parameters
    /// leave them out (C8-vector-4 item 8): Scale Strokes & Effects, Scale Corners, Transform
    /// Patterns, Use Preview Bounds, Move Locked and Hidden Artwork with Artboard, the Keyboard
    /// Increment; and the interface language, which decides how new type and new text styles
    /// align in Japanese (C8-vector-5 item 5). A command reading one declares `Mode`: an agent's
    /// run sets it with `prefs.set` in an earlier step and the host puts it back after the call.
    preferences: Preferences,
}

/// [`EditorSnapshot::preferences`].
#[derive(Clone, Debug, PartialEq)]
struct Preferences {
    scale_strokes: bool,
    scale_corners: bool,
    transform_pattern_tiles: bool,
    use_preview_bounds: bool,
    move_locked_with_artboard: bool,
    keyboard_increment: f64,
    interface_language: String,
}

impl Preferences {
    fn of(prefs: &crate::Prefs) -> Self {
        Self {
            scale_strokes: prefs.scale_strokes,
            scale_corners: prefs.scale_corners,
            transform_pattern_tiles: prefs.transform_pattern_tiles,
            use_preview_bounds: prefs.use_preview_bounds,
            move_locked_with_artboard: prefs.move_locked_with_artboard,
            keyboard_increment: prefs.keyboard_increment,
            interface_language: prefs.interface_language.clone(),
        }
    }

    fn put(self, prefs: &mut crate::Prefs) {
        prefs.scale_strokes = self.scale_strokes;
        prefs.scale_corners = self.scale_corners;
        prefs.transform_pattern_tiles = self.transform_pattern_tiles;
        prefs.use_preview_bounds = self.use_preview_bounds;
        prefs.move_locked_with_artboard = self.move_locked_with_artboard;
        prefs.keyboard_increment = self.keyboard_increment;
        prefs.interface_language = self.interface_language;
    }
}

impl EditorSnapshot {
    /// What an agent reads of it: JSON naming the same state, with a digest of what JSON does
    /// not show (the current paint, Transform Again, panel items), so equal states answer equal
    /// values.
    pub fn to_json(&self) -> Value {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        format!(
            "{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}",
            self.last_transform,
            self.last_selection_cmd,
            self.last_perspective,
            self.paint,
            self.draw_inside,
            self.gradient_stop,
            self.appearance_item,
            self.freeform_point,
            self.mask_view
        )
        .hash(&mut hasher);
        json!({
            "selection": self.selection,
            "currentLayer": self.active_layer.map(|id| id.0),
            "layerRows": self.layer_rows.iter().map(|id| id.0).collect::<Vec<_>>(),
            "isolation": self.isolation.map(|id| id.0),
            "variable": self.variables_highlight,
            "tool": self.tool,
            "fillActive": self.fill_active,
            "drawMode": self.draw_mode,
            "session": format!("{:016x}", hasher.finish()),
            "preferences": {
                "scaleStrokes": self.preferences.scale_strokes,
                "scaleCorners": self.preferences.scale_corners,
                "transformPatternTiles": self.preferences.transform_pattern_tiles,
                "usePreviewBounds": self.preferences.use_preview_bounds,
                "moveLockedWithArtboard": self.preferences.move_locked_with_artboard,
                "keyboardIncrement": self.preferences.keyboard_increment,
                "interfaceLanguage": self.preferences.interface_language,
            },
        })
    }
}

impl Session {
    /// The editing session of the active document, or `None` when no document is open.
    pub fn editor_snapshot(&self) -> Option<EditorSnapshot> {
        let state = self.active()?;
        Some(EditorSnapshot {
            selection: state.selection.clone(),
            active_layer: state.active_layer,
            layer_rows: state.layer_rows.clone(),
            variables_highlight: state.variables_highlight.clone(),
            isolation: state.isolation,
            mask_view: state.mask_view,
            last_transform: state.last_transform,
            last_selection_cmd: state.last_selection_cmd.clone(),
            last_perspective: state.last_perspective.clone(),
            tool: self.tool_id(),
            paint: self.paint.clone(),
            fill_active: self.fill_active,
            draw_mode: self.draw_mode,
            draw_inside: self.draw_inside,
            gradient_stop: self.gradient_stop,
            appearance_item: self.active_appearance_item.clone(),
            freeform_point: self.freeform_point,
            preferences: Preferences::of(&self.prefs),
        })
    }

    /// Puts `snapshot` back, leaving out the objects the document no longer has (as Undo does).
    /// The tool is chosen again only when another one was chosen meanwhile.
    pub fn restore_editor(&mut self, snapshot: EditorSnapshot) {
        let tool = snapshot.tool;
        if self.tool_id() != tool {
            // A tool that cannot be chosen now (no document) stays as it is: no edit is lost.
            let _ = self.select_tool(tool, ViewInfo::default());
        }
        self.paint = snapshot.paint;
        self.fill_active = snapshot.fill_active;
        self.draw_mode = snapshot.draw_mode;
        self.gradient_stop = snapshot.gradient_stop;
        self.active_appearance_item = snapshot.appearance_item;
        self.freeform_point = snapshot.freeform_point;
        if Preferences::of(&self.prefs) != snapshot.preferences {
            snapshot.preferences.put(&mut self.prefs);
        }
        let Some(state) = self.active_mut() else { return };
        let exists = |id: &NodeId| state.doc.node(*id).is_some();
        let keep = |id: Option<NodeId>| id.filter(exists);
        let mut selection = snapshot.selection;
        selection.objects.retain(exists);
        selection.anchors.retain(|id, _| exists(id));
        selection.key = keep(selection.key);
        selection.target = keep(selection.target);
        selection.guides.retain(|&guide| guide < state.doc.guides.len());
        state.selection = selection;
        state.active_layer = keep(snapshot.active_layer);
        state.layer_rows = snapshot.layer_rows.into_iter().filter(exists).collect();
        state.variables_highlight = snapshot.variables_highlight;
        state.isolation = keep(snapshot.isolation);
        state.mask_view = keep(snapshot.mask_view);
        state.last_transform = snapshot.last_transform;
        state.last_selection_cmd = snapshot.last_selection_cmd;
        state.last_perspective = snapshot.last_perspective;
        let inside = keep(snapshot.draw_inside);
        self.draw_inside = inside;
    }
}

/// What of `saved`, an editor state as [`EditorSnapshot::to_json`] gives it (another session's,
/// before its file was read again), `doc` no longer has: the first selected object, anchor
/// path, key object, target, slice or guide, the current layer, a highlighted Layers row or the
/// isolated group it names that is gone, as a reason (`deleted active selection 12`). `None` when
/// everything resolves. A selection that is no selection is left to its reader to refuse.
pub fn unresolved(doc: &kition_vector_doc::Document, saved: &Value) -> Option<String> {
    let exists = |id: NodeId| doc.node(id).is_some();
    let id = |value: &Value| value.as_u64().map(NodeId);
    if let Some(selection) = saved.get("selection").and_then(|value| serde_json::from_value::<Selection>(value.clone()).ok()) {
        if let Some(gone) = selection.objects.iter().chain(selection.anchors.keys()).find(|id| !exists(**id)) {
            return Some(format!("deleted active selection {}", gone.0));
        }
        if let Some(gone) = selection.key.filter(|id| !exists(*id)) {
            return Some(format!("deleted key object {}", gone.0));
        }
        if let Some(gone) = selection.target.filter(|id| !exists(*id)) {
            return Some(format!("deleted active target {}", gone.0));
        }
        if let Some(gone) = selection.slices.iter().find(|id| !exists(**id) && !doc.slices.iter().any(|slice| slice.id == **id)) {
            return Some(format!("deleted selected slice {}", gone.0));
        }
        if let Some(gone) = selection.guides.iter().find(|guide| **guide >= doc.guides.len()) {
            return Some(format!("deleted selected guide {gone}"));
        }
    }
    if let Some(gone) = saved.get("currentLayer").and_then(id).filter(|id| !exists(*id)) {
        return Some(format!("deleted current layer {}", gone.0));
    }
    if let Some(gone) = saved.get("layerRows").and_then(Value::as_array).into_iter().flatten().filter_map(id).find(|id| !exists(*id)) {
        return Some(format!("deleted highlighted Layers row {}", gone.0));
    }
    if let Some(gone) = saved.get("isolation").and_then(id).filter(|id| !exists(*id)) {
        return Some(format!("deleted isolated group {}", gone.0));
    }
    None
}
