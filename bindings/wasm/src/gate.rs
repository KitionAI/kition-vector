//! The one check every command an agent runs passes, however it runs (KHP A28, A29): directly,
//! as a `command` operation of the object model's `model.batch`, or as a step of a command that
//! runs others (`command.batch`, Reselect, Draw in Perspective, a tool's action).
//!
//! kition-headless refuses, for `commands.execute`, a command that is not listed for the agent: one
//! that names files (its schema types a parameter `file`), or whose id is not a protocol id. The
//! pinned kition-sdk does not check `model.batch`'s commands, nor can it see the steps a kit's own
//! command runs; [`may_run`] applies the same rules there. Every call the headless kit runs for
//! the agent or the host reaches the engine through `ReplicaPort::model_command`, which asks
//! [`may_run`] first and installs it as the session's step gate
//! ([`kition_vector_engine::Session::step_gate`]) for the commands the call runs in turn.
//!
//! Once the kit pins a kition-sdk with `kition_kit::dispatch::may_run` (one gate for every way an
//! agent runs a command), `Session::may_run` asks that instead, and `model_command` no longer
//! installs this one.

use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

use kition_kit::KitCommand;
use serde_json::Value;

/// Commands that cannot work without a folder of their own on the computer: off the command list
/// while a host serves the kit ([`crate::hosting::files_only`]). Data Recovery keeps its copies in
/// a folder; the host keeps the document.
pub const HOSTED_OFF: &[&str] = &["file.recovery.list", "file.recovery.save"];

/// What the gate knows of a command: its parameter schema, and whether it names files.
struct Known {
    schema: Option<Value>,
    names_files: bool,
}

static KNOWN: OnceLock<HashMap<String, Known>> = OnceLock::new();

/// Whether a command id fits the protocol's pattern (`group.name`, lowercase first letter), as
/// kition-headless lists commands.
fn protocol_id(id: &str) -> bool {
    let ok = |c: char| c.is_ascii_alphanumeric() || c == '_';
    let mut segments = id.split('.');
    segments.next().is_some_and(|first| first.starts_with(|c: char| c.is_ascii_lowercase()) && first.chars().all(ok))
        && segments.all(|s| !s.is_empty() && s.chars().all(ok))
}

/// Learns the commands of `commands` (a document's full list) once.
pub(crate) fn learn(commands: impl FnOnce() -> Vec<KitCommand>) {
    KNOWN.get_or_init(|| {
        commands()
            .into_iter()
            .map(|command| {
                let names_files = match &command.schema {
                    Some(schema) => !kition_kit::schema::file_params(schema).is_empty(),
                    None => crate::files::FILES.iter().any(|file| file.command == command.id),
                };
                (command.id, Known { schema: command.schema, names_files })
            })
            .collect()
    });
}

/// Whether the agent (or the host) may run `command` with `params`: `Err` says why not and what
/// to do instead (KHP A48). A command the engine does not know is left to the engine to refuse.
pub fn may_run(command: &str, params: &Value) -> Result<(), String> {
    if crate::hosting::files_only() && HOSTED_OFF.contains(&command) {
        return Err(format!(
            "`{command}` is not available here: it keeps files in a folder of its own, and the host owns every file (KHP A29); the host keeps the document and its versions"
        ));
    }
    let Some(known) = KNOWN.get().and_then(|known| known.get(command)) else { return Ok(()) };
    if !protocol_id(command) || known.names_files {
        return Err(format!(
            "`{command}` is not available to the agent: it names files by path, and the host opens, saves, places and exports documents (KHP A28, A29). document.exportBytes answers an export's bytes"
        ));
    }
    if let Some(schema) = &known.schema {
        kition_kit::schema::refuse_unknown(command, schema, params)?;
        let given = params.as_object().cloned().unwrap_or_default();
        if let Some(error) = kition_kit::schema::param_errors(schema, &given).first() {
            return Err(format!("`{command}`: {}", error.message));
        }
    }
    Ok(())
}

/// [`may_run`] as the engine's step gate.
pub(crate) fn step_gate() -> kition_vector_engine::CommandGate {
    Arc::new(may_run)
}

#[cfg(test)]
mod tests {
    use kition_kit::KitDocument;
    use serde_json::json;

    use super::*;

    #[test]
    fn steps_that_name_files_are_refused_and_others_run() {
        let doc = crate::VectorDocument::create("Drawing.kvector", 64, 64).unwrap_or_else(|error| panic!("{error}"));
        learn(|| doc.commands());
        let refused = may_run("document.export", &json!({ "path": "/elsewhere/a.svg" })).err().unwrap_or_default();
        assert!(refused.contains("not available to the agent") && refused.contains("document.exportBytes"), "{refused}");
        assert!(may_run("file.saveAs", &json!({})).is_err(), "a command that names files, whatever its parameters");
        assert_eq!(may_run("shape.rectangle", &json!({ "x": 1, "y": 1, "width": 4, "height": 4 })), Ok(()));
        assert!(may_run("shape.rectangle", &json!({ "widht": 4 })).is_err(), "unknown parameters, as directly");
        assert_eq!(may_run("no.such.command", &json!({})), Ok(()), "the engine answers unknown commands");
    }
}
