//! What each write rests on, per call (C8-land-11-vector item 4): the objects it acts on, the
//! fields of theirs its result reads or replaces, and the places it names. A command not narrowed
//! here holds every field of every object it names (the SDK's default for a premise that says
//! nothing), which refuses more than needed but never lets a write land on a changed object.
//!
//! The drawing has no field marked `position`: where an object is drawn is its geometry (`data`),
//! and where it sits in the layer tree is its tree place (`nodes`). So a premise that leaves out
//! `data` is not touched by a move, and one with no positions is not touched by a restack.

use kition_kit::{Covered, CoveredAsk, CoveredAt, Covers, EditorState, Premise, Under, Within};
use kition_vector_doc::{Document, Node, NodeId, NodeKind, Selection};
use kition_vector_engine::cmd::picks;
use serde_json::{Value, json};

use crate::address;
use crate::rests::{self, At, Fields, Rest};

/// The engine state a call's place is read in.
pub(crate) struct Editor<'a> {
    pub doc: &'a Document,
    pub selection: &'a [NodeId],
    /// What appearance and transparency commands act on without ids: the Layers-panel target,
    /// else the selected objects.
    pub subjects: &'a [NodeId],
    pub rows: Vec<NodeId>,
    /// Where new art goes (the isolated group, else the current editable layer).
    pub insertion: Option<NodeId>,
    /// The names of the swatches the clipboard's objects link to (a paste relinks them to the
    /// drawing's swatches of those names).
    pub clipboard: Vec<String>,
}

fn numbers(value: Option<&Value>) -> Vec<NodeId> {
    match value {
        Some(Value::Array(values)) => values.iter().filter_map(Value::as_u64).map(NodeId).collect(),
        Some(value) => value.as_u64().map(NodeId).into_iter().collect(),
        None => Vec::new(),
    }
}

fn strings(fields: &[&str]) -> Vec<String> {
    fields.iter().map(|field| (*field).to_owned()).collect()
}

/// The fields of one object a property write (`object.setProps`, `layer.setProps`,
/// `transparency.set`) sets, from the parameters given.
fn property_fields(command: &str, params: &Value) -> Vec<String> {
    let mut fields = Vec::new();
    for (parameter, field) in [
        ("name", "name"),
        ("visible", "visible"),
        ("locked", "locked"),
        ("opacity", "props.opacity"),
        ("blend", "props.blend"),
        ("isolate", "props.isolate"),
        ("knockout", "props.knockout"),
        ("knockoutShape", "props.knockout_shape"),
        ("data", "props.attrs"),
        ("template", "data.template"),
        ("printable", "data.printable"),
        ("preview", "data.preview"),
        ("dimImages", "data.dim_images"),
        ("color", "data.color"),
    ] {
        if params.get(parameter).is_some() {
            fields.push(field.to_owned());
        }
    }
    if params.get("template").is_some() {
        fields.extend(strings(&["locked", "data.dim_images"]));
    }
    // An opacity or blend set on a fill or stroke (`item`) is that item's, in the appearance.
    if command == "transparency.set" && params.get("item").is_some_and(|item| !item.is_null()) {
        fields.retain(|field| !matches!(field.as_str(), "props.opacity" | "props.blend"));
        fields.push("appearance".to_owned());
    }
    // The graphic style link is not touched (C8-vector-read: only `graphicStyle.apply` and
    // `graphicStyle.breakLink` write it).
    fields
}

/// `id` and everything it holds.
fn subtree(doc: &Document, id: NodeId, out: &mut Vec<NodeId>) {
    let Some(node) = doc.node(id) else { return };
    out.push(id);
    for child in node.children().into_iter().flatten() {
        subtree(doc, child.id, out);
    }
}

/// The window of `id`'s children (all of them): a group a write reaches through holds them as
/// they were, so another writer's object put into it or taken out refuses the write.
fn children_window(doc: &Document, id: NodeId) -> Option<Within> {
    let count = doc.node(id)?.children()?.len();
    Some(Within { parent: json!(id.0), field: "nodes".into(), from: 0, to: count, ..Default::default() })
}

/// The strings at `path` (`a`, `a.b`, `a[].b`, a list of strings) in `params`.
fn collect(params: &Value, path: &str, out: &mut Vec<String>) {
    let (head, rest) = path.split_once('.').map_or((path, None), |(head, rest)| (head, Some(rest)));
    let (key, items) = head.strip_suffix("[]").map_or((head, false), |key| (key, true));
    let Some(value) = params.get(key) else { return };
    let values: Vec<&Value> = if items { value.as_array().map(|all| all.iter().collect()).unwrap_or_default() } else { vec![value] };
    for value in values {
        match rest {
            Some(rest) => collect(value, rest, out),
            None => match value {
                Value::String(text) => out.push(text.clone()),
                Value::Array(all) => out.extend(all.iter().filter_map(Value::as_str).map(str::to_owned)),
                _ => {}
            },
        }
    }
}

/// What the call rests on.
pub(crate) fn under(editor: &Editor<'_>, command: &str, params: &Value) -> Option<Under> {
    if command == "command.batch" {
        return Some(batch(editor, params));
    }
    let row = address::row(command)?;
    let doc = editor.doc;
    let mut under = Under::default();
    // The objects the call acts on: its target parameter (`ids`, else `id`, as `targets()` reads
    // them), else the editor state it reads.
    let mut ids = row.targets.map(|name| numbers(params.get(name))).unwrap_or_default();
    if ids.is_empty() && row.targets == Some("ids") {
        ids = numbers(params.get("id"));
    }
    let explicit = !ids.is_empty();
    if !explicit && row.reads.contains(&EditorState::Selection) {
        ids = if command.starts_with("layer.") {
            editor.rows.to_vec()
        } else if command.starts_with("appearance.") || command.starts_with("transparency.") || command.starts_with("effect.") {
            editor.subjects.to_vec()
        } else {
            editor.selection.to_vec()
        };
    }
    let no_position = Vec::<String>::new;
    let kinds: Vec<&str> = ids.iter().filter_map(|id| kind_of(doc, *id)).collect();
    let rest = rests::rest_for(command, params, &kinds);
    match rest {
        Some(rest) => acted_on(editor, rest, command, params, &ids, &mut under),
        // Not a write command: every field of each object, not where it is.
        None => {
            for id in &ids {
                under.premises.push(Premise::on(id.0).positions(no_position()));
            }
        }
    }
    // A command that picks its objects by a point or a stroke (the selection's, else every
    // object under it): the objects under it now, every field of each, and the place told by
    // being asked (`covered`): the guard asks which objects others moved, reshaped, hid, locked,
    // created or deleted were under it at `basis` (C8-vector-5 item 2).
    if picks::PICKING.contains(&command) {
        let selection = selection_of(editor.selection);
        let picked = picks::picked(doc, &selection, command, params).unwrap_or_default();
        for id in &picked {
            under.objects.push(json!(id.0));
            if !ids.contains(id) {
                under.premises.push(Premise::on(id.0));
            }
        }
        under.covers = Some(Covers { fields: strings(COVERS) });
    }
    // Objects named by other parameters (`parent`, `source`, `target`, `items[].id`): held whole,
    // except a parent a row goes into, which only has to exist and stay a container.
    for name in row.objects {
        if row.targets == Some("ids") && *name == "id" {
            continue;
        }
        let read = rest.and_then(|rest| rest.params.iter().find(|(param, ..)| param == name));
        for id in numbers(params.get(*name)) {
            match (rest, read) {
                (Some(_), Some((_, fields, at, inside))) => {
                    under.premises.push(premise(id, *fields, *at));
                    if *inside && let Some(rest) = rest {
                        held_inside(doc, rest, id, &mut under);
                    }
                }
                // Named, and nothing of it read: only that it is there.
                (Some(_), None) => under.premises.push(Premise::on(id.0).fields(Vec::<String>::new()).positions(no_position())),
                (None, _) => under.premises.push(Premise::on(id.0)),
            }
        }
    }
    if command == "node.move" {
        let parent = params.get("parent").and_then(Value::as_u64).map(NodeId);
        let count = doc.children(parent).map_or(0, Vec::len);
        let index = params.get("index").and_then(Value::as_u64).and_then(|n| usize::try_from(n).ok()).unwrap_or(count).min(count);
        // The place it goes, first among the windows it is held by.
        under.within.insert(
            0,
            Within { parent: parent.map_or(json!("drawing"), |id| json!(id.0)), field: "nodes".into(), from: index, to: index, ..Default::default() },
        );
    }
    // An artboard named by its index: that place in the drawing's list.
    for place in row.places.iter().filter(|place| **place != "index" || command.starts_with("artboard.")) {
        let indexes: Vec<usize> = match params.get(*place) {
            Some(Value::Array(items)) => items.iter().filter_map(Value::as_u64).filter_map(|n| usize::try_from(n).ok()).collect(),
            Some(value) => value.as_u64().and_then(|n| usize::try_from(n).ok()).into_iter().collect(),
            None => Vec::new(),
        };
        for index in indexes {
            under.within.push(Within {
                parent: json!("drawing"),
                field: "artboards".into(),
                from: index,
                to: index.saturating_add(1),
                ..Default::default()
            });
        }
    }
    // A name looked up in one of the drawing's named tables. Swatches, colour groups, graphic
    // styles, character and paragraph styles, symbols and patterns have a persistent identity as
    // tool id (`swatch:<uid>`, C8-vector-5; a name is once in its table,
    // `Document::unique_names`): the write rests on the entry the name names now, so another
    // writer's edit of it refuses it and an unrelated entry's does not. The name itself is a name
    // parameter (`x-khp-name`): the guard refuses a rename of that entry away from it, or another
    // entry taking it, as `relocated`. (Text styles have no based-on chain here: a style is its
    // own attributes.)
    for (name, _) in row.names {
        let mut found = Vec::new();
        collect(params, name, &mut found);
        for (path, types) in row.types.iter().filter(|(path, _)| path == name) {
            let _ = path;
            for ty in *types {
                let prefix = match *ty {
                    "Swatch" => "swatch:",
                    "GraphicStyle" => "graphicStyle:",
                    "Symbol" => "symbol:",
                    "Pattern" => "pattern:",
                    "SwatchGroup" => "swatchGroup:",
                    "CharStyle" => "charStyle:",
                    "ParaStyle" => "paraStyle:",
                    _ => continue,
                };
                for text in &found {
                    if let Some(uid) = uid_named(doc, ty, text) {
                        // The entry; where it is in its table only for a command that counts from it
                        // (a reorder of the table leaves the others).
                        let premise = Premise::on(format!("{prefix}{uid}"));
                        under.premises.push(if rest.is_some_and(|rest| rest.name_at) { premise } else { premise.positions(Vec::<String>::new()) });
                    }
                }
            }
        }
    }
    // An artboard by its index: the artboard there now (its tool id), at that place in the list.
    let boards: Vec<usize> = under.within.iter().filter(|within| within.field == "artboards").map(|within| within.from).collect();
    for index in boards {
        if let Some(board) = doc.artboards.get(index) {
            under.objects.push(json!(format!("artboard:{}", board.id)));
            let premise = Premise::on(format!("artboard:{}", board.id)).positions(["artboards"]);
            // A copy of the artboard carries every field of it.
            let copy = rest.is_some_and(|rest| rest.copy) && params.get("copy").and_then(Value::as_bool) == Some(true);
            under.premises.push(match rest.and_then(|rest| rest.board).filter(|_| !copy) {
                Some(fields) => premise.fields(strings(fields)),
                None => premise,
            });
        }
    }
    if let Some(rest) = rest {
        document_wide(editor, rest, params, &ids, &mut under);
    } else if !explicit
        && row.reads.contains(&EditorState::Active)
        && let Some(layer) = editor.insertion
    {
        // New art goes into the insertion layer, which has to stay there, shown and unlocked.
        under.premises.push(Premise::on(layer.0).fields(["visible", "locked", "data.type"]).positions(no_position()));
    }
    join_premises(&mut under);
    Some(under)
}

/// A premise on node `id`: `fields` of it (`None`: every field), and where it is when `at`.
fn premise(id: NodeId, fields: Fields, at: bool) -> Premise {
    let premise = match fields {
        Some(fields) => Premise::on(id.0).fields(strings(fields)),
        None => Premise::on(id.0),
    };
    if at { premise.positions(["nodes"]) } else { premise.positions(Vec::<String>::new()) }
}

/// What kind of object `id` is, as the reading's paths tell them apart.
fn kind_of(doc: &Document, id: NodeId) -> Option<&'static str> {
    let node = doc.node(id)?;
    if doc.parent_of(id).and_then(|parent| doc.node(parent)).is_some_and(is_root) {
        return Some("member");
    }
    Some(match node.kind {
        NodeKind::Layer { .. } => "layer",
        NodeKind::Text(_) => "text",
        NodeKind::SymbolInstance { .. } => "instance",
        _ if is_root(node) => "root",
        _ if node.children().is_some() => "group",
        _ => "leaf",
    })
}

/// Whether `node` is the root of a compound path, blend or clip group, whose members a command
/// acting on one of them reads (or acts on) as one.
fn is_root(node: &Node) -> bool {
    matches!(node.kind, NodeKind::Compound { .. } | NodeKind::Blend { .. } | NodeKind::Group { clip: true, .. }) || node.graph.is_some()
}

/// What a call rests on of the objects it acts on (`ids`), by the command's reading ([`Rest`]).
fn acted_on(editor: &Editor<'_>, rest: &Rest, command: &str, params: &Value, ids: &[NodeId], under: &mut Under) {
    let doc = editor.doc;
    let copy = rest.copy && params.get("copy").and_then(Value::as_bool) == Some(true);
    // The frontmost of them: what its result is put where it was rests on its place.
    let front = ids.iter().copied().filter(|id| doc.node(*id).is_some()).max_by_key(|id| doc.index_path(*id).unwrap_or_default());
    let property = matches!(command, "object.setProps" | "layer.setProps" | "transparency.set");
    for &id in ids {
        let Some(node) = doc.node(id) else {
            // A slice or an export asset (no node): what of it the command reads, not where it is
            // in the drawing's list.
            if doc.slices.iter().any(|slice| slice.id == id) || doc.assets.iter().any(|asset| asset.id == id.0) {
                // Read as the parameter naming it says (`slices`, `assets`), else as an object.
                let fields = rest
                    .params
                    .iter()
                    .find(|(name, ..)| matches!(*name, "slices" | "slice" | "assets" | "asset" | "ids" | "id"))
                    .map_or(rest.leaf, |(_, fields, ..)| *fields);
                under.premises.push(premise(id, fields, false));
            }
            continue;
        };
        let group = node.children().is_some();
        if copy {
            // A copy carries every field of each, and goes where each is.
            under.premises.push(Premise::on(id.0).positions(["nodes"]));
        } else if property {
            under.premises.push(Premise::on(id.0).fields(property_fields(command, params)).positions(Vec::<String>::new()));
        } else {
            let text = matches!(node.kind, NodeKind::Text(_));
            let (fields, at) = if group {
                (rest.group, rest.group_at)
            } else if text {
                (rest.text, rest.leaf_at)
            } else {
                (rest.leaf, rest.leaf_at)
            };
            let at = match at {
                At::No => false,
                At::All => true,
                At::Top => Some(id) == front,
            };
            under.premises.push(premise(id, fields, at));
        }
        if group {
            held_inside(doc, rest, id, under);
        }
        if let Some((fields, above)) = rest.neighbours
            && let Some(siblings) = doc.children(doc.parent_of(id))
            && let Some(at) = siblings.iter().position(|sibling| sibling.id == id)
        {
            // Stored bottom first: above it are those after it.
            let range = if above { siblings.get(at + 1..) } else { siblings.get(..at) };
            for sibling in range.into_iter().flatten().filter(|sibling| !ids.contains(&sibling.id)) {
                let mut held = Vec::new();
                subtree(doc, sibling.id, &mut held);
                for object in held {
                    under.premises.push(premise(object, fields, false));
                    if doc.node(object).is_some_and(|n| n.children().is_some()) {
                        under.within.extend(children_window(doc, object));
                    }
                }
            }
        }
        let Some(parent) = doc.parent_of(id) else { continue };
        let Some(holder) = doc.node(parent) else { continue };
        if is_root(holder) {
            // A member of a compound path, blend or clip group: the command reads (or acts on) the
            // whole, unless the root is acted on itself.
            if let Some((root, members)) = rest.root.filter(|_| !ids.contains(&parent)) {
                under.premises.push(premise(parent, root, false));
                under.within.extend(children_window(doc, parent));
                for member in holder.children().into_iter().flatten().filter(|member| member.id != id && !ids.contains(&member.id)) {
                    under.premises.push(premise(member.id, members, false));
                }
            }
        } else {
            // A row moved into another parent (`node.move`) is put among that parent's children
            // only: where it stood among its old siblings is not read, and another writer's
            // object added beside it there is no reason to refuse the move.
            let leaves = command == "node.move" && params.get("parent").and_then(Value::as_u64).map(NodeId) != Some(parent);
            if rest.stack && !leaves {
                under.within.extend(children_window(doc, parent));
            }
            if let Some(fields) = rest.parent.filter(|_| !ids.contains(&parent)) {
                under.premises.push(premise(parent, fields, false));
            }
        }
    }
}

/// An object of a document-wide read (`all`, `art`): by its kind, with what it holds.
fn every(doc: &Document, all: &rests::All, id: NodeId, at: bool, under: &mut Under) {
    let mut held = Vec::new();
    subtree(doc, id, &mut held);
    for object in held {
        let Some(node) = doc.node(object) else { continue };
        let fields = match node.kind {
            NodeKind::Layer { .. } => all.layers,
            NodeKind::Text(_) => Some(all.text),
            NodeKind::SymbolInstance { .. } => Some(all.instances),
            _ => Some(all.objects),
        };
        if let Some(fields) = fields {
            under.premises.push(premise(object, fields, at && object == id));
        }
        if all.windows && node.children().is_some() {
            under.within.extend(children_window(doc, object));
        }
    }
}

/// What a group `id` holds, as the command reads it: each object inside it (type by the fields
/// read of type), and which objects it and each group inside it hold.
fn held_inside(doc: &Document, rest: &Rest, id: NodeId, under: &mut Under) {
    let mut held = Vec::new();
    subtree(doc, id, &mut held);
    if let Some(fields) = rest.inside {
        for object in held.iter().copied().filter(|object| *object != id) {
            let text = doc.node(object).is_some_and(|n| matches!(n.kind, NodeKind::Text(_)));
            under.premises.push(premise(object, if text && fields == rest.leaf { rest.text } else { fields }, rest.inside_at));
        }
    }
    if rest.windows {
        for container in held.into_iter().filter(|held| doc.node(*held).is_some_and(|n| n.children().is_some())) {
            under.within.extend(children_window(doc, container));
        }
    }
}

/// What a call rests on besides the objects it acts on: the layer new art goes into, the
/// drawing's own fields and tables, every object of a document-wide command, the named entries
/// its objects refer to, and the artboards it reads.
fn document_wide(editor: &Editor<'_>, rest: &Rest, params: &Value, ids: &[NodeId], under: &mut Under) {
    let doc = editor.doc;
    if let Some(fields) = rest.layers {
        // Every top-level layer but those holding what it acts on (acting on nothing: but the
        // layer new art goes into).
        let own: Vec<NodeId> = if ids.is_empty() {
            editor.insertion.and_then(|layer| doc.layer_of(layer)).into_iter().collect()
        } else {
            ids.iter().filter_map(|id| doc.layer_of(*id)).collect()
        };
        for layer in doc.layers.iter().filter(|layer| !own.contains(&layer.id)) {
            under.premises.push(premise(layer.id, fields, false));
        }
    }
    if let Some(art) = &rest.art {
        let index = params.get("index").and_then(Value::as_u64).and_then(|n| usize::try_from(n).ok());
        let boards: Vec<usize> = match index {
            Some(index) => vec![index],
            None => (0..doc.artboards.len()).collect(),
        };
        // The art goes with the artboard: a move or resize with `moveArt`, a duplicate unless `art`
        // is off, a rearrange unless `moveArtwork` is off.
        let switch = |name: &str| params.get(name).and_then(Value::as_bool);
        let moves = match rest.id {
            "artboard.move" | "artboard.setProps" => switch("moveArt") == Some(true),
            "artboard.duplicate" => switch("art") != Some(false),
            _ => switch("moveArtwork") != Some(false),
        };
        if moves {
            for board in boards.iter().filter_map(|index| doc.artboards.get(*index)) {
                for id in doc.art_on_artboard(board.rect, true) {
                    // Copied with the artboard: every field of each, put where each is.
                    every(doc, art, id, art.objects.is_none(), under);
                }
            }
            if let Some(fields) = art.layers {
                for layer in &doc.layers {
                    under.premises.push(premise(layer.id, fields, false));
                    if art.windows {
                        under.within.extend(children_window(doc, layer.id));
                    }
                }
            }
        }
    }
    let named = |ty: &str| -> Vec<String> {
        let Some(row) = address::row(rest.id) else { return Vec::new() };
        let mut found = Vec::new();
        for (name, _) in row.names {
            if row.types.iter().any(|(path, types)| path == name && types.contains(&ty)) {
                collect(params, name, &mut found);
            }
        }
        found
    };
    // The tint swatches of a swatch the command edits by name: they follow its colour.
    if rest.refs.contains(&"tints") {
        for base in named("Swatch") {
            for swatch in doc.swatches_iter().filter(|swatch| swatch.tint_of().is_some_and(|(of, _)| of == base)) {
                if !swatch.uid.is_empty() {
                    under.premises.push(Premise::on(format!("swatch:{}", swatch.uid.as_str())).positions(Vec::<String>::new()));
                }
            }
        }
    }
    if let Some((fields, at)) = rest.users {
        let styles: Vec<u32> =
            named("GraphicStyle").iter().filter_map(|name| doc.graphic_style(name)).map(|style| style.id).filter(|id| *id != 0).collect();
        let symbols = named("Symbol");
        doc.walk(|node| {
            let uses = node.graphic_style.is_some_and(|style| styles.contains(&style))
                || matches!(&node.kind, NodeKind::SymbolInstance { symbol, .. } if symbols.contains(symbol));
            if uses && !ids.contains(&node.id) {
                under.premises.push(premise(node.id, fields, at));
            }
        });
    }
    if let Some(fields) = rest.texts {
        doc.walk(|node| {
            if matches!(node.kind, NodeKind::Text(_)) && !ids.contains(&node.id) {
                under.premises.push(premise(node.id, fields, false));
            }
        });
    }
    if let Some(fields) = rest.bound {
        for id in doc.variables.bindings.keys() {
            under.premises.push(premise(*id, fields, false));
        }
    }
    if let Some((fields, art)) = rest.assets {
        for asset in &doc.assets {
            let premise = Premise::on(asset.id).positions(Vec::<String>::new());
            under.premises.push(match fields {
                Some(fields) => premise.fields(strings(fields)),
                None => premise,
            });
            if let Some(art) = art {
                for node in &asset.nodes {
                    under.premises.push(self::premise(*node, art, false));
                }
            }
        }
    }
    if let (Some(fields), Some(layer)) = (rest.insertion, editor.insertion) {
        under.premises.push(premise(layer, fields, rest.insertion_at));
    }
    if let Some(fields) = setting_fields(rest.id, params) {
        // One setting of a settings group: that setting (C8-vector-engine-2).
        under.premises.push(Premise::on("drawing").fields(fields).positions(Vec::<String>::new()));
    } else if rest.drawing != Some(&[]) {
        let premise = Premise::on("drawing").positions(Vec::<String>::new());
        under.premises.push(match rest.drawing {
            Some(fields) => premise.fields(strings(fields)),
            None => premise,
        });
    }
    if let Some(all) = &rest.all {
        doc.walk(|node| {
            if ids.contains(&node.id) {
                return;
            }
            let fields = match node.kind {
                NodeKind::Layer { .. } => all.layers,
                NodeKind::Text(_) => Some(all.text),
                NodeKind::SymbolInstance { .. } => Some(all.instances),
                _ => Some(all.objects),
            };
            if let Some(fields) = fields {
                under.premises.push(premise(node.id, fields, false));
            }
            if all.windows && node.children().is_some() {
                under.within.extend(children_window(doc, node.id));
            }
        });
        if let Some(fields) = all.slices {
            let slices = doc.slices.iter().map(|slice| slice.id.0);
            for id in slices {
                let premise = Premise::on(id).positions(Vec::<String>::new());
                under.premises.push(match fields {
                    Some(fields) => premise.fields(strings(fields)),
                    None => premise,
                });
            }
        }
    }
    // The objects whose references are read: those it acts on (a member with its root), else
    // every object of a document-wide read, and the art of the export assets it reads.
    let mut referring: Vec<NodeId> =
        ids.iter().map(|id| doc.parent_of(*id).filter(|parent| doc.node(*parent).is_some_and(is_root)).unwrap_or(*id)).collect();
    if ids.is_empty() && rest.all.is_some() {
        referring.extend(doc.layers.iter().map(|layer| layer.id));
    }
    if let Some((_, Some(_))) = rest.assets {
        referring.extend(doc.assets.iter().flat_map(|asset| asset.nodes.iter().copied()));
    }
    for kind in rest.refs {
        for uid in referred(doc, kind, &referring, &editor.clipboard) {
            under.premises.push(Premise::on(uid).positions(Vec::<String>::new()));
        }
    }
    let board = |index: usize, fields: Fields, at: bool| {
        doc.artboards.get(index).map(|board| {
            let premise = Premise::on(format!("artboard:{}", board.id));
            let premise = if at { premise.positions(["artboards"]) } else { premise.positions(Vec::<String>::new()) };
            match fields {
                Some(fields) => premise.fields(strings(fields)),
                None => premise,
            }
        })
    };
    if let Some(fields) = rest.first {
        under.premises.extend(board(0, fields, true));
    }
    if let Some(fields) = rest.last {
        under.premises.extend(board(doc.artboards.len().saturating_sub(1), fields, true));
    }
    if let Some(fields) = rest.boards {
        for index in 0..doc.artboards.len() {
            under.premises.extend(board(index, fields, false));
        }
    }
}

/// The print settings that decide the printed regions (kition_vector_pdf `print_regions`, the layout).
const LAYOUT: &[&str] = &[
    "artboards",
    "range",
    "skipBlank",
    "media",
    "width",
    "height",
    "orientation",
    "autoRotate",
    "transverse",
    "printLayers",
    "placement",
    "scaling",
    "scale",
    "overlap",
    "tileRange",
    "margin",
    "marks",
    "bleed",
];

/// The settings a command that edits a settings group reads and writes, from the keys it is given
/// (docsetup.rs, fileinfo.rs, rasterfx.rs, print.rs, printtiling.rs: only the given keys are written,
/// C8-vector-engine-2); `None` for another command or no keys (a query of the group).
fn setting_fields(command: &str, params: &Value) -> Option<Vec<String>> {
    let keys = |value: &Value| value.as_object().map(|keys| keys.keys().cloned().collect::<Vec<_>>()).unwrap_or_default();
    let mut fields = Vec::new();
    match command {
        "document.setup" => {
            for key in keys(params) {
                match key.as_str() {
                    "units" => fields.push("settings.units".to_owned()),
                    "backgroundContents" => fields.push("settings.setup.background".to_owned()),
                    // A language picks its quotes; quotes beside a language are compared with them.
                    "language" | "quotes" => fields.extend(strings(&["settings.setup.language", "settings.setup.quotes"])),
                    // Script and small caps sizes restamp the type that uses them.
                    "superscript" | "subscript" | "smallCapsSize" => return None,
                    _ => fields.push(format!("settings.setup.{key}")),
                }
            }
        }
        "file.info" => {
            for key in keys(params) {
                fields.push(if key == "title" { "settings.title".to_owned() } else { format!("settings.metadata.{key}") });
            }
        }
        "document.rasterEffectsSettings" => {
            for key in keys(params) {
                match key.as_str() {
                    "resolution" => fields.push("settings.raster_effects_ppi".to_owned()),
                    // A colour model is checked against the document's colour mode.
                    "colorModel" => fields.extend(strings(&["settings.raster_effects.colorModel", "settings.color_mode"])),
                    _ => fields.push(format!("settings.raster_effects.{key}")),
                }
            }
        }
        "print.setup" => {
            for key in keys(params.get("settings").unwrap_or(&Value::Null)) {
                fields.push(format!("settings.print_setup.{key}"));
            }
        }
        "print.tiling.set" => {
            fields.push("settings.print_setup.tileOrigin".to_owned());
            // Placed at a point: relative to the corner of the printed region under it, which the
            // print layout settings, the document setup's bleed and the artboards decide.
            if params.get("origin").is_some_and(|origin| !origin.is_null()) {
                for key in LAYOUT {
                    fields.push(format!("settings.print_setup.{key}"));
                }
                fields.push("settings.setup".to_owned());
                if params.get("artboard").is_none_or(Value::is_null) {
                    fields.push("~artboards".to_owned());
                }
            }
        }
        _ => return None,
    }
    (!fields.is_empty()).then_some(fields)
}

/// The tool ids of the named entries of `kind` the objects `ids` (and what they hold) refer to:
/// the swatches their paint links, an instance's symbol, the graphic style an object is linked
/// to; for `pattern`, the pattern being edited.
fn referred(doc: &Document, kind: &str, ids: &[NodeId], clipboard: &[String]) -> Vec<String> {
    let mut held = Vec::new();
    for id in ids {
        subtree(doc, *id, &mut held);
    }
    let mut out = Vec::new();
    let mut add = |ty: &str, prefix: &str, name: &str| {
        if let Some(uid) = uid_named(doc, ty, name) {
            let id = format!("{prefix}{uid}");
            if !out.contains(&id) {
                out.push(id);
            }
        }
    };
    match kind {
        "swatch" => {
            for node in held.iter().filter_map(|id| doc.node(*id)) {
                let mut names = Vec::new();
                swatch_names(&serde_json::to_value(&node.appearance).unwrap_or_default(), &mut names);
                if let NodeKind::Text(_) = node.kind {
                    swatch_names(&serde_json::to_value(&node.kind).unwrap_or_default(), &mut names);
                }
                for name in names {
                    add("Swatch", "swatch:", &name);
                }
            }
        }
        "symbol" => {
            for node in held.iter().filter_map(|id| doc.node(*id)) {
                if let NodeKind::SymbolInstance { symbol, .. } = &node.kind {
                    add("Symbol", "symbol:", symbol);
                }
            }
        }
        "graphicStyle" => {
            for node in held.iter().filter_map(|id| doc.node(*id)) {
                if let Some(style) = node.graphic_style.and_then(|style| doc.graphic_style_by_id(style)) {
                    let name = style.name.clone();
                    add("GraphicStyle", "graphicStyle:", &name);
                }
            }
        }
        "clipboard" => {
            for name in clipboard {
                add("Swatch", "swatch:", name);
            }
        }
        "pattern" => {
            if let Some(edit) = &doc.pattern_edit {
                let name = edit.pattern.clone();
                add("Pattern", "pattern:", &name);
            }
        }
        _ => {}
    }
    out
}

/// The swatch names a serialized paint holds (`"swatch": "Brand"`).
fn swatch_names(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::Object(fields) => {
            for (key, item) in fields {
                match (key.as_str(), item) {
                    ("swatch", Value::String(name)) if !out.contains(name) => out.push(name.clone()),
                    _ => swatch_names(item, out),
                }
            }
        }
        Value::Array(items) => items.iter().for_each(|item| swatch_names(item, out)),
        _ => {}
    }
}

/// A batch of commands rests on what each of its steps rests on (asked of each with its own
/// command and parameters; the steps run one after another, so a later step's place is read as
/// the drawing is now, before the batch).
fn batch(editor: &Editor<'_>, params: &Value) -> Under {
    let mut under = Under::default();
    for step in params.get("commands").and_then(Value::as_array).into_iter().flatten() {
        let Some(command) = step.get("command").and_then(Value::as_str) else { continue };
        let empty = json!({});
        let Some(one) = self::under(editor, command, step.get("params").unwrap_or(&empty)) else { continue };
        under.objects.extend(one.objects);
        under.within.extend(one.within);
        under.premises.extend(one.premises);
        if one.covers.is_some() {
            under.covers = one.covers;
        }
    }
    join_premises(&mut under);
    under
}

/// `more` joined into `under`: what either rests on.
pub(crate) fn join(under: &mut Under, more: Under) {
    for object in more.objects {
        if !under.objects.contains(&object) {
            under.objects.push(object);
        }
    }
    for within in more.within {
        if !under.within.contains(&within) {
            under.within.push(within);
        }
    }
    under.premises.extend(more.premises);
    if under.covers.is_none() {
        under.covers = more.covers;
    }
    join_premises(under);
}

/// The pinned SDK keeps one premise per object id, the last one a call states (`guard.rs`): a
/// later premise on the same object would replace an earlier one (a layer written without ids is
/// also the insertion layer: its name, then only that it stays shown, unlocked and a layer). So
/// premises on one object are joined: every field and position either needs, every one when
/// either leaves it open (C8-vector-5 item 6).
fn join_premises(under: &mut Under) {
    fn union(into: &mut Option<Vec<String>>, more: Option<Vec<String>>) {
        match (into.as_mut(), more) {
            (Some(known), Some(more)) => {
                for value in more {
                    if !known.contains(&value) {
                        known.push(value);
                    }
                }
            }
            _ => *into = None,
        }
    }
    let mut joined: Vec<Premise> = Vec::with_capacity(under.premises.len());
    for premise in std::mem::take(&mut under.premises) {
        if let Some(known) = joined.iter_mut().find(|known| known.id == premise.id) {
            union(&mut known.fields, premise.fields);
            union(&mut known.positions, premise.positions);
        } else {
            joined.push(premise);
        }
    }
    under.premises = joined;
}

/// What besides the tree decides whether an object is under a picking command's point or stroke:
/// its geometry and kind (`data`), and whether it is shown and unlocked.
const COVERS: &[&str] = &["data", "visible", "locked"];

fn selection_of(ids: &[NodeId]) -> Selection {
    let mut selection = Selection::default();
    selection.set(ids.iter().copied());
    selection
}

/// What a picking command's point or stroke named at `basis` ([`kition_kit::KitDocument::covered`]):
/// the drawing with every object asked about as it was then (one others created left out, one
/// they changed given its `data`, `visible` and `locked` of then), queried as the command queries
/// it. A deleted object cannot be put back (the journal keeps no geometry of it: the drawing has
/// no `position` fields or derived values): when the call names what it acts on (the selection,
/// `on`, a scribble's `ids`) it was under the place if it was one of those, else whether it was
/// there is not known (`None`, which refuses), nor is an object whose needed values are
/// `unknown`.
pub(crate) fn covered(editor: &Editor<'_>, command: &str, params: &Value, objects: &[CoveredAsk]) -> Option<CoveredAt> {
    if !picks::PICKING.contains(&command) {
        return None;
    }
    let mut doc = editor.doc.clone();
    // Given what to act on (the selection, `on`, or a scribble's `ids`), the place names those:
    // a deleted object was under it when it was one of them.
    let given: Vec<NodeId> = match params.get("ids").and_then(Value::as_array).filter(|_| command == "shaper.scribble") {
        Some(ids) => ids.iter().filter_map(Value::as_u64).map(NodeId).collect(),
        None => editor.selection.to_vec(),
    };
    let mut known = Vec::with_capacity(objects.len());
    for ask in objects {
        let id = ask.id.as_u64().or_else(|| ask.id.as_str().and_then(|text| text.parse().ok())).map(NodeId);
        let Some(id) = id else {
            known.push(false);
            continue;
        };
        if ask.created {
            let _ = doc.remove(id);
            known.push(true);
            continue;
        }
        if ask.deleted && !given.is_empty() {
            known.push(true);
            continue;
        }
        if ask.deleted || ask.unknown.iter().any(|field| COVERS.iter().any(|wanted| field == wanted || field.starts_with(&format!("{wanted}.")))) {
            known.push(false);
            continue;
        }
        let ok = match doc.node(id) {
            Some(node) => match then(node, &ask.before) {
                Some(node) => doc.node_mut(id).map(|slot| *slot = node).is_some(),
                None => false,
            },
            None => false,
        };
        known.push(ok);
    }
    let selection = selection_of(editor.selection);
    let under = picks::picked(&doc, &selection, command, params).unwrap_or_default();
    let covered = objects
        .iter()
        .zip(known)
        .map(|(ask, known)| {
            let id = ask.id.as_u64().or_else(|| ask.id.as_str().and_then(|text| text.parse().ok())).map(NodeId);
            let covered = match id {
                Some(id) if known && ask.deleted => Some(given.contains(&id)),
                Some(id) if known => {
                    let mut held = Vec::new();
                    subtree(&doc, id, &mut held);
                    Some(held.iter().any(|object| under.contains(object)))
                }
                _ => None,
            };
            Covered { id: ask.id.clone(), covered, ..Default::default() }
        })
        .collect();
    Some(CoveredAt { under: under.iter().map(|id| json!(id.0)).collect(), covered })
}

/// `node` with the `data`, `visible` and `locked` it had (`before`), its children as they are.
fn then(node: &Node, before: &serde_json::Map<String, Value>) -> Option<Node> {
    let mut value = serde_json::to_value(node).ok()?;
    for (field, was) in before {
        match field.as_str() {
            "data" => {
                let mut kind = was.clone();
                // The register holds the kind without its children: keep the node's.
                if let (Some(kind), Some(now)) = (kind.as_object_mut(), value.get("kind").and_then(Value::as_object)) {
                    for key in ["children", "content", "source"] {
                        if let Some(children) = now.get(key) {
                            kind.insert(key.to_owned(), children.clone());
                        }
                    }
                }
                value["kind"] = kind;
            }
            "visible" | "locked" => value[field.as_str()] = was.clone(),
            _ => {}
        }
    }
    serde_json::from_value(value).ok()
}

/// The uid of the entry of type `ty` named `name` now (swatches and colour groups' swatches are
/// one table).
fn uid_named(doc: &Document, ty: &str, name: &str) -> Option<String> {
    let uid = match ty {
        "Swatch" => doc.swatch(name).map(|s| &s.uid),
        "SwatchGroup" => doc.swatch_groups.iter().find(|g| g.name == name).map(|g| &g.uid),
        "GraphicStyle" => doc.graphic_styles.iter().find(|s| s.name == name).map(|s| &s.uid),
        "CharStyle" => doc.char_styles.iter().find(|s| s.name == name).map(|s| &s.uid),
        "ParaStyle" => doc.para_styles.iter().find(|s| s.name == name).map(|s| &s.uid),
        "Symbol" => doc.symbols.iter().find(|s| s.name == name).map(|s| &s.uid),
        "Pattern" => doc.patterns.iter().find(|s| s.name == name).map(|s| &s.uid),
        _ => None,
    }?;
    (!uid.is_empty()).then(|| uid.as_str().to_owned())
}

/// The drawing's derived values (the root, `drawing`): a digest of each named table, and the
/// order of its artboards. What a write by name or by artboard index rests on.
pub(crate) fn derived(doc: &Document, ids: &[Value], names: &[String]) -> Vec<kition_kit::DerivedValues> {
    use std::hash::{Hash, Hasher};
    if !(ids.is_empty() || ids.iter().any(|id| id == "drawing")) {
        return Vec::new();
    }
    fn digest(value: &impl serde::Serialize) -> Value {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        serde_json::to_string(value).unwrap_or_default().hash(&mut hasher);
        json!(format!("{:016x}", hasher.finish()))
    }
    let mut values = serde_json::Map::new();
    let wanted = |name: &str| names.is_empty() || names.iter().any(|asked| asked.trim_start_matches('~') == name);
    for (name, value) in [
        ("swatches", digest(&doc.swatches)),
        ("swatchGroups", digest(&doc.swatch_groups)),
        // Without the local ids a read mints for styles no object links (C8-vector-5): those change
        // with every reload of the view, not with the table.
        (
            "graphicStyles",
            digest(&doc.graphic_styles.iter().map(|style| kition_vector_doc::GraphicStyle { id: 0, ..style.clone() }).collect::<Vec<_>>()),
        ),
        ("charStyles", digest(&doc.char_styles)),
        ("paraStyles", digest(&doc.para_styles)),
        ("symbols", digest(&doc.symbols)),
        ("patterns", digest(&doc.patterns)),
        ("artboards", json!(doc.artboards.iter().map(|board| board.id).collect::<Vec<_>>())),
    ] {
        if wanted(name) {
            values.insert(name.to_owned(), value);
        }
    }
    vec![kition_kit::DerivedValues { id: json!("drawing"), values }]
}
