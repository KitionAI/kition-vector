//! Command addresses follow the engine's resolvers. Local indexes are places, not IDs.

use kition_kit::KitCommand;
use serde_json::{Value, json};

mod table;

/// The declaration row of `id`, if the command has one.
pub(crate) fn row(id: &str) -> Option<&'static table::D> {
    table::TABLE.binary_search_by(|row| row.id.cmp(id)).ok().and_then(|at| table::TABLE.get(at))
}

/// Commands whose `id` also takes an effect's catalogue name (a string): only a whole number there
/// names an object.
const ID_OR_NAME: &[&str] = &["appearance.addEffect", "effect.apply"];

/// Parameters a schema offered that the handler never reads (its notation's output `{id}`, or a
/// key a scan found in the handler's answer): a number there names nothing, and
/// the command refuses it as it refuses any key it does not take (engine rule E7a).
const UNREAD: &[(&str, &str)] = &[
    ("document.exportPdf", "id"),
    ("eyedropper.setOptions", "id"),
    ("graphicStyle.library", "id"),
    ("graphicStyle.list", "id"),
    ("graphicStyle.merge", "id"),
    ("library.add", "id"),
    ("library.get", "id"),
    ("links.info", "to"),
    ("recolor.apply", "swatch"),
    ("recolor.colors", "swatch"),
    ("recolor.reduce", "swatch"),
    ("text.tabs.set", "to"),
    ("appearance.copyFrom", "id"),
    ("object.blend.info", "id"),
    ("object.blend.make", "id"),
    ("object.clippingMask.make", "id"),
    ("object.distributeSpacing", "id"),
    ("object.envelope.makeWithTopObject", "id"),
    ("object.pattern.make", "id"),
    ("path.join", "id"),
    ("perspective.draw", "id"),
    ("select.anchorsMany", "id"),
    ("swatch.merge", "id"),
    ("text.findFontFiles", "id"),
    ("transparency.makeOpacityMask", "id"),
    ("type.size.decrease", "id"),
    ("type.size.increase", "id"),
];

/// Parameters that take a string the handler looks up as a name or catalogue id, never a number.
const STRINGS: &[(&str, &str, &str)] = &[
    ("appearance.addEffect", "effect", "the effect's catalogue id (`stylize.dropShadow`)"),
    ("effect.apply", "effect", "the effect's catalogue id (`stylize.dropShadow`)"),
    ("plugin.info", "id", "the plug-in's id"),
    ("plugin.remove", "id", "the plug-in's id"),
    ("plugin.run", "id", "the plug-in's id"),
];

/// The schema node at `path` (`a`, `a.b`, `a[].b`) of a kit schema (a map of parameters).
fn node_at<'a>(schema: &'a mut serde_json::Map<String, Value>, path: &str) -> Option<&'a mut Value> {
    let mut segments = path.split('.');
    let first = segments.next()?;
    let (name, items) = first.strip_suffix("[]").map_or((first, false), |name| (name, true));
    let mut at = schema.get_mut(name)?;
    if items {
        at = at.get_mut("items")?;
    }
    for segment in segments {
        let (name, items) = segment.strip_suffix("[]").map_or((segment, false), |name| (name, true));
        at = at.get_mut("properties")?.get_mut(name)?;
        if items {
            at = at.get_mut("items")?;
        }
    }
    Some(at)
}

/// Declares what the command's handler resolves ([`table`]): the parameter it acts on, the others
/// naming objects, their types, its places, the names it looks up and the editor state it reads.
/// Object parameters take whole numbers (a list of them for `ids`), and a parameter the handler
/// resolves that the schema did not list is added.
pub fn apply(command: &mut KitCommand) {
    let Some(row) = row(&command.id) else { return };
    command.targets = row.targets;
    command.objects = row.objects;
    if command.schema.is_none() {
        // A command whose notation has no schema takes any key: its object parameters are
        // declared as objects, and `on` reaches it through the selection it reads.
        command.targets = None;
        command.objects = row.all_objects;
    }
    command.object_types = row.types;
    command.places = row.places;
    command.name_params = row.names;
    command.reads = row.reads;
    let Some(schema) = command.schema.as_mut().and_then(Value::as_object_mut) else { return };
    for (_, key) in UNREAD.iter().filter(|(id, _)| *id == command.id) {
        schema.remove(*key);
    }
    for (_, key, what) in STRINGS.iter().filter(|(id, _, _)| *id == command.id) {
        schema.insert((*key).to_owned(), json!({"type":"string","description":what}));
    }
    // `target` beside `ids` chooses the objects themselves or what their groups and layers hold
    // (`appearance_targets`); an object parameter of that name is declared, never retyped.
    if !row.types.iter().any(|(path, _)| *path == "target")
        && let Some(node) = schema.get_mut("target")
        && node.to_string().contains("\"integer\"")
    {
        *node = json!({"type":"string","description":"\"object\" (default) or \"contents\": the objects themselves, or what their groups and layers hold"});
    }
    for key in row.add {
        schema.insert((*key).to_owned(), json!({}));
    }
    for (path, types) in row.types {
        if row.names.iter().any(|(name, _)| name == path) {
            continue;
        }
        let Some(node) = node_at(schema, path) else { continue };
        let list = *path == "ids" || node.get("type").and_then(Value::as_str) == Some("array");
        let what = if types.contains(&"Slice") {
            "slice"
        } else if types.contains(&"Asset") {
            "export asset"
        } else {
            "object"
        };
        if ID_OR_NAME.contains(&command.id.as_str()) && *path == "id" {
            *node = json!({"oneOf":[{"type":"integer"},{"type":"string"}],"description":"an object ID, or the effect's catalogue name"});
        } else if list {
            *node = json!({"type":"array","items":{"type":"integer"},"description":format!("{what} IDs")});
        } else {
            *node = json!({"type":"integer","description":format!("{what} ID")});
        }
    }
}
