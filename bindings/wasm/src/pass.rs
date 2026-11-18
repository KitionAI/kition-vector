//! The run's own steps (C8-vector-engine). The guard asks `under` for every step of a run, in
//! order, before the run (`kition-kits` `guard::conflicts`), while the drawing is still the one
//! before the run: a step that selects (`select.set`) or that writes and selects what it made (a
//! group, a blend) changes what a later step without `on` acts on. So each step asked about is
//! also run on a scratch copy of the drawing (one copy for the run, each step on the copy the steps
//! before it left), and a later step rests on what it acts on there as well as on what it acts on in
//! the drawing as it is (both: a pass a refused run left behind is only cleared by the next command
//! run, so the drawing now is never left out). A command that reads or writes files, or runs other
//! commands, is not run on the copy: the steps after it rest on the drawing alone, as before.
//! Cleared whenever a command runs.

use std::cell::RefCell;

use kition_vector_doc::NodeId;
use kition_vector_engine::{DocState, Session};
use serde_json::Value;

#[derive(Default)]
pub(crate) struct Pass(RefCell<Option<Session>>, RefCell<Option<Vec<NodeId>>>);

/// Commands that touch files, the host's libraries or run other commands: not run on the copy.
fn outside(command: &str) -> bool {
    const PREFIXES: &[&str] = &[
        "file.",
        "document.",
        "links.",
        "print.",
        "library.",
        "plugin.",
        "css.",
        "webExport.",
        "swatch.library",
        "graphicStyle.library",
        "brush.library",
    ];
    PREFIXES.iter().any(|prefix| command.starts_with(prefix))
        || matches!(command, "command.batch" | "assets.export" | "graphicStyle.saveLibrary" | "swatch.saveLibrary" | "edit.undo" | "edit.redo")
}

impl Pass {
    pub(crate) fn reset(&self) {
        if let Ok(mut held) = self.0.try_borrow_mut() {
            *held = None;
        }
        if let Ok(mut asked) = self.1.try_borrow_mut() {
            *asked = None;
        }
    }

    /// The selection the step asked about last acts on in the copy (before it ran there).
    pub(crate) fn asked_selection(&self) -> Option<Vec<NodeId>> {
        self.1.try_borrow().ok().and_then(|asked| asked.clone())
    }

    /// The drawing as the steps asked about so far would leave it, with its selection, if any.
    pub(crate) fn with_state<T>(&self, read: impl FnOnce(&DocState) -> T) -> Option<T> {
        let held = self.0.try_borrow().ok()?;
        held.as_ref().and_then(Session::active).map(read)
    }

    /// The selection the steps asked about so far would leave.
    pub(crate) fn selected(&self) -> Option<Vec<NodeId>> {
        self.with_state(|state| state.selection.objects.clone())
    }

    /// Answering for `command`: run it on the copy (made from `state` for a run's first step).
    pub(crate) fn ask(&self, state: &DocState, command: &str, params: &Value) {
        let Ok(mut held) = self.0.try_borrow_mut() else { return };
        if outside(command) {
            *held = None;
            if let Ok(mut asked) = self.1.try_borrow_mut() {
                *asked = None;
            }
            return;
        }
        if held.is_none() {
            let mut scratch = Session::new();
            // The copy reaches no folder.
            scratch.host_files_only = true;
            scratch.add_document((*state.doc).clone(), None);
            if let Some(copy) = scratch.active_mut() {
                copy.selection = state.selection.clone();
                copy.active_layer = state.active_layer;
                copy.layer_rows = state.layer_rows.clone();
                copy.isolation = state.isolation;
            }
            *held = Some(scratch);
        }
        // What the step acts on is the selection before it: `covered`, asked after `under`, reads it.
        if let Ok(mut asked) = self.1.try_borrow_mut() {
            *asked = held.as_ref().and_then(Session::active).map(|copy| copy.selection.objects.clone());
        }
        // A step the copy cannot run leaves it as it was (the run would stop there).
        if let Some(scratch) = held.as_mut() {
            let _ = scratch.execute(command, params);
        }
    }
}
