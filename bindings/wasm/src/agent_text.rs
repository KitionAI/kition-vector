//! What each command tells people and agents beyond its title and notation: a description of
//! what it does and when to use it, what a read command answers, and types for the parameters the
//! notation leaves loose (KHP A1, A5). The text lives in `agent-commands.json`; the tests check
//! that every command has an entry and every entry names a command and its real parameters.

use std::collections::HashMap;
use std::sync::OnceLock;

use kition_kit::KitCommand;
use serde_json::{Map, Value};

#[derive(Default, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub output: Option<Value>,
    #[serde(default)]
    pub params: Map<String, Value>,
}

pub const SOURCE: &str = include_str!("../agent-commands.json");

fn table() -> &'static HashMap<String, Entry> {
    static TABLE: OnceLock<HashMap<String, Entry>> = OnceLock::new();
    TABLE.get_or_init(|| serde_json::from_str(SOURCE).unwrap_or_default())
}

/// Fills in each command's description and output schema, and replaces the loosely typed
/// parameters with their typed nodes (keeping `required`).
pub fn apply(commands: &mut [KitCommand]) {
    for command in commands {
        let Some(entry) = table().get(&command.id) else { continue };
        if !entry.description.is_empty() {
            command.description.clone_from(&entry.description);
        }
        if command.output_schema.is_none() {
            command.output_schema.clone_from(&entry.output);
        }
        if let Some(Value::Object(schema)) = command.schema.as_mut() {
            for (key, node) in &entry.params {
                let required = schema.get(key).and_then(|old| old.get("required")).cloned();
                let mut node = node.clone();
                if let (Some(required), Some(object)) = (required, node.as_object_mut()) {
                    object.insert("required".into(), required);
                }
                schema.insert(key.clone(), node);
            }
        }
    }
}
