//! The independent reading of every write command (task card C8-vector-read). For each of the
//! write commands the kit offers, its handler and the engine operation behind it were read —
//! before the declarations (`src/address/table.rs`, `src/place.rs`) or their tests were looked
//! at — for which parameters it resolves and how, what it reads of the drawing and of the editor,
//! and what it writes. Each row below is one code path of a command, on [`fixture`], with what a
//! write with `basis` must hold by that reading, written before it was compared with the
//! declarations; the rows marked `pinned` were not written that way (see [`p`]): 53 of 895, each
//! with the reason in the comment before it. Eight readers read the 508 handlers, each a family
//! group, opening only `crates/**/src`; three saw part of an inline `#[cfg(test)]` module on the
//! way (`cmd/textstyles.rs`, `cmd/cut.rs`, `cmd/docinfo.rs`) and used none of it.
//!
//! Before the declarations were changed to the reading (C8-vector-read), 480 of the 508 commands
//! differed: 353 held less than their handlers read, 347 held more, 220 both. Now none holds less;
//! the rows still holding more are [`HELD_MORE`].
//!
//! A row: the command, its parameters (objects by role), `on` (the step's objects, the selection
//! for the step), the editor state it reads (`selection`, `active`, `mode`), and for each object
//! what the write rests on: the fields it reads or replaces (`*` every field, `-` none: only that
//! it is there; `name`, `visible`, `locked`, `appearance`, `data`, `data.<key>`, `props.<key>` of a
//! node, where `data` of a type object holds its `text` too (its characters, kdm-model's `text`);
//! `settings.<key>`, `guides` and `threads` of the drawing) and whether where it is matters (its place among its
//! parent's children, or an artboard's index). Besides objects: `children:$G` (which objects `$G`
//! holds), `slot:$G[a..b]` (a place in `$G`'s children), `picks` (the objects under a point or a
//! stroke, told by asking the kit), `$ALL` (every object that is not a layer). What is held is
//! computed as the guard does: the step's `on` and the command's target parameter (every field,
//! and where), the objects its other declared parameters name (the same), replaced object by
//! object by `under`'s premises, and `under`'s places.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

use std::collections::{BTreeMap, BTreeSet};

use kition_kit::{CallContext, EditorState, KitCommand, KitDocument};
use kition_vector_kit::VectorDocument;
use serde_json::{Value, json};

fn id(answer: &Value) -> u64 {
    answer["id"].as_u64().unwrap_or_else(|| panic!("no id in {answer}"))
}

/// The drawing every row is read on, saved and opened again, with nothing selected and
/// `Layer 1` current. Top first: `$L2` "Layer 2" holding `$R2` (rectangle); `$L1` "Layer 1"
/// holding `$B` (blend of `$B2` over `$B1`), `$CP` (compound path of `$K2` over `$K1`), `$CG`
/// (clip group: `$CA` ellipse clipped by `$CK` rectangle), `$TA` (area type "Area text"), `$PR`
/// (the rectangle the pattern was made from), `$S` (instance of symbol "Sym"), `$T` (point type
/// "Hello"), `$P` (open path of three anchors), `$G` (group of `$C2` ellipse over `$C1`
/// rectangle), `$E` (ellipse), `$R` (rectangle). Swatches "Brand" (global, every object's fill),
/// its tint swatch "Brand 40%", and "Other"; graphic style "Style 1" (from `$E`); character style "Char 1", paragraph style
/// "Para 1"; symbol "Sym"; pattern "Pat"; artboards 0 "Artboard 1" and 1 "Board 2"; a vertical
/// guide at 100; slice `$SL`; export asset `$AS` (of `$E`); text variable "V" bound to `$T`, data
/// set "D1". In each row's session a copy of `$R` is on the clipboard.
pub fn fixture() -> (Vec<u8>, Vec<(&'static str, u64)>) {
    let mut d = VectorDocument::create("fx.kvector", 400, 300).unwrap();
    let mut ex = |c: &str, p: Value| d.execute(c, p.clone()).unwrap_or_else(|e| panic!("{c} {p}: {e}"));
    ex("swatch.new", json!({"name": "Brand", "color": "#123456", "global": true}));
    ex("swatch.new", json!({"name": "Other", "color": "#654321"}));
    ex("swatch.new", json!({"swatch": "Brand", "tint": 40}));
    let r = id(&ex("shape.rectangle", json!({"x": 10, "y": 10, "width": 50, "height": 40})));
    ex("paint.setFill", json!({"ids": [r], "swatch": "Brand"}));
    let e = id(&ex("shape.ellipse", json!({"x": 80, "y": 10, "width": 40, "height": 40})));
    let c1 = id(&ex("shape.rectangle", json!({"x": 10, "y": 80, "width": 30, "height": 30})));
    let c2 = id(&ex("shape.ellipse", json!({"x": 50, "y": 80, "width": 30, "height": 30})));
    ex("select.set", json!({"ids": [c1, c2]}));
    let g = id(&ex("arrange.group", json!({})));
    let p = id(&ex("path.add", json!({"anchors": [{"x": 150, "y": 20}, {"x": 200, "y": 60}, {"x": 250, "y": 20}]})));
    let t = id(&ex("text.add", json!({"x": 20, "y": 200, "text": "Hello"})));
    ex("graphicStyle.new", json!({"name": "Style 1", "id": e}));
    ex("charStyle.new", json!({"name": "Char 1"}));
    ex("paraStyle.new", json!({"name": "Para 1"}));
    let sr = id(&ex("shape.rectangle", json!({"x": 300, "y": 10, "width": 20, "height": 20})));
    let s = id(&ex("symbol.new", json!({"name": "Sym", "ids": [sr]})));
    let pr = id(&ex("shape.rectangle", json!({"x": 300, "y": 100, "width": 20, "height": 20})));
    ex("select.set", json!({"ids": [pr]}));
    ex("object.pattern.make", json!({"name": "Pat", "edit": false}));
    let ta = id(&ex("text.add", json!({"x": 150, "y": 150, "text": "Area text", "area": {"width": 100, "height": 60}})));
    let ca = id(&ex("shape.ellipse", json!({"x": 20, "y": 240, "width": 40, "height": 40})));
    let ck = id(&ex("shape.rectangle", json!({"x": 10, "y": 230, "width": 60, "height": 30})));
    ex("select.set", json!({"ids": [ck, ca]}));
    let cg = id(&ex("object.clippingMask.make", json!({})));
    let k1 = id(&ex("shape.rectangle", json!({"x": 100, "y": 240, "width": 20, "height": 20})));
    let k2 = id(&ex("shape.rectangle", json!({"x": 105, "y": 245, "width": 5, "height": 5})));
    ex("select.set", json!({"ids": [k1, k2]}));
    let cp = id(&ex("object.compoundPath.make", json!({})));
    let b1 = id(&ex("shape.ellipse", json!({"x": 300, "y": 200, "width": 10, "height": 10})));
    let b2 = id(&ex("shape.ellipse", json!({"x": 360, "y": 200, "width": 10, "height": 10})));
    let b = id(&ex("object.blend.make", json!({"ids": [b1, b2], "steps": 2})));
    ex("select.set", json!({"ids": []}));
    let l2 = id(&ex("layer.new", json!({"name": "Layer 2"})));
    let r2 = id(&ex("shape.rectangle", json!({"x": 200, "y": 200, "width": 40, "height": 30})));
    ex("artboard.new", json!({"name": "Board 2"}));
    ex("guide.add", json!({"vertical": true, "pos": 100}));
    let sl = id(&ex("object.slice.add", json!({"x": 0, "y": 0, "width": 50, "height": 50})));
    let asset = ex("asset.add", json!({"ids": [e]}))["assets"][0].as_u64().unwrap();
    ex("variable.define", json!({"name": "V", "kind": "text"}));
    ex("variable.bind", json!({"variable": "V", "ids": [t]}));
    ex("dataset.new", json!({"name": "D1"}));
    ex("select.set", json!({"ids": []}));
    let bytes = d.save(None).unwrap();
    let mut d = VectorDocument::open("fx.kvector", &bytes).unwrap();
    let tree = d.inspect().unwrap();
    let layer1 = tree["layers"].as_array().unwrap().iter().find(|layer| layer["name"] == "Layer 1").unwrap()["id"].as_u64().unwrap();
    let mut children = |id: u64| -> Vec<u64> {
        let node = d.execute("file.node", json!({ "id": id })).unwrap();
        node["kind"]["children"].as_array().unwrap().iter().map(|child| child["id"].as_u64().unwrap()).collect()
    };
    let (k, b12) = (children(cp), children(b));
    let roles = vec![
        ("$L1", layer1),
        ("$R", r),
        ("$E", e),
        ("$C1", c1),
        ("$C2", c2),
        ("$G", g),
        ("$P", p),
        ("$T", t),
        ("$S", s),
        ("$PR", pr),
        ("$TA", ta),
        ("$CA", ca),
        ("$CK", ck),
        ("$CG", cg),
        ("$K1", k[0]),
        ("$K2", k[1]),
        ("$CP", cp),
        ("$B1", b12[0]),
        ("$B2", b12[1]),
        ("$B", b),
        ("$L2", l2),
        ("$R2", r2),
        ("$SL", sl),
        ("$AS", asset),
    ];
    (d.save(None).unwrap(), roles)
}

/// The layers among the roles.
const LAYERS: &[&str] = &["$L1", "$L2"];
/// Roles that are no nodes.
const NOT_NODES: &[&str] = &["$SL", "$AS"];

/// One code path of a command: its parameters (objects by role), the step's `on`, the editor
/// state it reads, and what a write rests on: (object, fields, whether where it is matters).
pub struct Row {
    command: &'static str,
    params: &'static str,
    on: &'static [&'static str],
    editor: &'static str,
    holds: &'static [(&'static str, &'static str, bool)],
    /// Not written blind from the reading: pinned from the declaration after review.
    pinned: bool,
}

/// A row written from the reading, before it was compared with the declarations.
const fn r(
    command: &'static str,
    params: &'static str,
    on: &'static [&'static str],
    editor: &'static str,
    holds: &'static [(&'static str, &'static str, bool)],
) -> Row {
    Row { command, params, on, editor, holds, pinned: false }
}

/// A row pinned from the declaration after review: the reading could not say exactly what the
/// write rests on (said in the comment before it).
#[allow(dead_code)]
const fn p(
    command: &'static str,
    params: &'static str,
    on: &'static [&'static str],
    editor: &'static str,
    holds: &'static [(&'static str, &'static str, bool)],
) -> Row {
    Row { command, params, on, editor, holds, pinned: true }
}

fn with_roles(value: &Value, roles: &[(&str, u64)]) -> Value {
    match value {
        Value::String(text) => roles.iter().find(|(role, _)| role == text).map_or_else(|| value.clone(), |(_, id)| json!(id)),
        Value::Array(items) => Value::Array(items.iter().map(|item| with_roles(item, roles)).collect()),
        Value::Object(fields) => Value::Object(fields.iter().map(|(key, item)| (key.clone(), with_roles(item, roles))).collect()),
        _ => value.clone(),
    }
}

/// The ids a declared object parameter (`a.b`, `items[].c`) names in `params`.
fn ids_at(params: &Value, path: &str) -> Vec<u64> {
    let mut current = vec![params.clone()];
    for part in path.split('.') {
        let key = part.trim_end_matches("[]");
        let mut next = Vec::new();
        for value in &current {
            match value.get(key) {
                Some(Value::Array(items)) if part.ends_with("[]") => next.extend(items.iter().cloned()),
                Some(item) => next.push(item.clone()),
                None => {}
            }
        }
        current = next;
    }
    current
        .iter()
        .flat_map(|value| if let Value::Array(items) = value { items.clone() } else { vec![value.clone()] })
        .filter_map(|value| value.as_u64())
        .collect()
}

/// A field list as a set: `*` every field (`None`), `-` none.
fn field_set(fields: &str) -> Option<BTreeSet<String>> {
    match fields.trim() {
        "*" => None,
        "-" | "" => Some(BTreeSet::new()),
        list => Some(list.split(',').map(|field| field.trim().to_owned()).collect()),
    }
}

fn spelled(fields: Option<&Vec<String>>) -> String {
    let Some(fields) = fields else { return "*".to_owned() };
    let set: BTreeSet<&str> = fields.iter().map(String::as_str).collect();
    if set.is_empty() { "-".to_owned() } else { set.into_iter().collect::<Vec<_>>().join(",") }
}

/// Names for what the guard reads: roles for nodes, slices and assets; the drawing's named
/// entries by their name (`swatch:Brand`), artboards by index (`artboard:0`).
struct Names {
    roles: BTreeMap<u64, &'static str>,
    tool_ids: BTreeMap<String, String>,
    children: BTreeMap<u64, usize>,
    layer1: u64,
    rectangle: u64,
}

impl Names {
    fn new(bytes: &[u8], roles: &[(&'static str, u64)], doc: &mut VectorDocument) -> Self {
        let file: Value = serde_json::from_slice(bytes).unwrap();
        let drawing = &file["document"];
        let mut tool_ids = BTreeMap::new();
        for (list, prefix) in [
            ("swatches", "swatch"),
            ("swatch_groups", "swatchGroup"),
            ("graphic_styles", "graphicStyle"),
            ("char_styles", "charStyle"),
            ("para_styles", "paraStyle"),
            ("symbols", "symbol"),
            ("patterns", "pattern"),
        ] {
            for entry in drawing[list].as_array().into_iter().flatten() {
                if let (Some(uid), Some(name)) = (entry["uid"].as_str(), entry["name"].as_str()) {
                    tool_ids.insert(format!("{prefix}:{uid}"), format!("{prefix}:{name}"));
                }
            }
        }
        for (index, board) in drawing["artboards"].as_array().into_iter().flatten().enumerate() {
            tool_ids.insert(format!("artboard:{}", board["id"]), format!("artboard:{index}"));
        }
        let mut children = BTreeMap::new();
        for (role, id) in roles {
            if NOT_NODES.contains(role) {
                continue;
            }
            let node = doc.execute("file.node", json!({ "id": id })).unwrap();
            let kind = &node["kind"];
            if let Some(list) = kind["children"].as_array().or_else(|| kind["content"].as_array()) {
                children.insert(*id, list.len());
            }
        }
        let layer1 = roles.iter().find(|(role, _)| *role == "$L1").unwrap().1;
        let rectangle = roles.iter().find(|(role, _)| *role == "$R").unwrap().1;
        Self { roles: roles.iter().map(|(role, id)| (*id, *role)).collect(), tool_ids, children, layer1, rectangle }
    }

    fn of(&self, id: &Value) -> String {
        if let Some(number) = id.as_u64().or_else(|| id.as_str().and_then(|text| text.parse().ok())) {
            return self.roles.get(&number).map_or_else(|| format!("#{number}"), |role| (*role).to_owned());
        }
        let text = id.as_str().unwrap_or_default();
        self.tool_ids.get(text).cloned().unwrap_or_else(|| text.to_owned())
    }
}

type Holds = BTreeMap<String, (Option<BTreeSet<String>>, bool)>;

/// What a write with `basis` of `command` with `params` and `on` holds, as the guard computes it.
fn held(bytes: &[u8], command: &KitCommand, params: &Value, on: &[u64], names: &Names) -> Result<(BTreeSet<String>, Holds), String> {
    let mut doc = VectorDocument::open("fx.kvector", bytes).unwrap();
    // Opening makes the top layer current: `Layer 1` is, as the fixture says. A copy of `$R` is
    // on the clipboard (what the paste rows read).
    doc.execute("layer.setCurrent", json!({ "id": names.layer1 })).unwrap();
    doc.execute("select.set", json!({ "ids": [names.rectangle] })).unwrap();
    doc.execute("edit.copy", json!({})).unwrap();
    doc.execute("select.set", json!({ "ids": [] })).unwrap();
    let editor: BTreeSet<String> = command
        .reads
        .iter()
        .map(|state| match state {
            EditorState::Selection => "selection".to_owned(),
            EditorState::Active => "active".to_owned(),
            EditorState::Mode => "mode".to_owned(),
            other => format!("{other:?}").to_lowercase(),
        })
        .collect();
    let mut asked = params.clone();
    let mut holds: Holds = BTreeMap::new();
    for id in on {
        holds.insert(names.of(&json!(id)), (None, true));
    }
    match command.targets {
        Some(param) if !on.is_empty() => {
            asked[param] = if param == "ids" || on.len() > 1 { json!(on) } else { json!(on[0]) };
        }
        _ if !on.is_empty() => {
            doc.set_context(&CallContext { on: on.iter().map(|id| json!(id)).collect(), ..Default::default() })?;
        }
        _ => {}
    }
    if let Some(param) = command.targets {
        for id in ids_at(&asked, param) {
            holds.insert(names.of(&json!(id)), (None, true));
        }
    }
    for path in command.objects {
        for id in ids_at(&asked, path) {
            holds.insert(names.of(&json!(id)), (None, true));
        }
    }
    if let Some(under) = doc.under(&command.id, &asked).unwrap_or_else(|e| panic!("{}: {e}", command.id)) {
        for premise in &under.premises {
            let fields = field_set(&spelled(premise.fields.as_ref()));
            holds.insert(names.of(&premise.id), (fields, premise.positions.as_ref().is_none_or(|positions| !positions.is_empty())));
        }
        for within in &under.within {
            if within.field == "artboards" {
                continue;
            }
            let parent = names.of(&within.parent);
            let count = within.parent.as_u64().and_then(|id| names.children.get(&id)).copied();
            let key = if within.from == 0 && Some(within.to) == count {
                format!("children:{parent}")
            } else {
                format!("slot:{parent}[{}..{}]", within.from, within.to)
            };
            holds.insert(key, (Some(BTreeSet::new()), false));
        }
        if let Some(covers) = &under.covers {
            holds.insert("picks".to_owned(), (Some(covers.fields.iter().cloned().collect()), false));
        }
    }
    Ok((editor, holds))
}

/// A row's holds, `$ALL` written out as every object that is not a layer (a later entry for one
/// of them says what it holds of that one; two entries for one object are joined).
fn expected(row: &Row, roles: &[(&str, u64)]) -> Holds {
    let mut holds = Holds::new();
    let mut explicit = BTreeSet::new();
    for (object, fields, at) in row.holds {
        // A whole table of the drawing (`~swatches`) is a derived value of the drawing.
        let (object, fields) = if object.starts_with('~') { (&"drawing", *object) } else { (object, *fields) };
        if *object == "$ALL" {
            for (role, _) in roles.iter().filter(|(role, _)| !LAYERS.contains(role) && !NOT_NODES.contains(role)) {
                holds.insert((*role).to_owned(), (field_set(fields), *at));
            }
            continue;
        }
        let fields = field_set(fields);
        match holds.get_mut(*object).filter(|_| explicit.contains(*object)) {
            // Two entries for one object: what either reads.
            Some((known, known_at)) => {
                *known = match (known.take(), fields) {
                    (Some(mut known), Some(more)) => {
                        known.extend(more);
                        Some(known)
                    }
                    _ => None,
                };
                *known_at |= *at;
            }
            None => {
                holds.insert((*object).to_owned(), (fields, *at));
            }
        }
        explicit.insert(*object);
    }
    // A type object's characters are its `text`, which what reads its `data` reads too.
    for object in TYPE_OBJECTS {
        if let Some((Some(fields), _)) = holds.get_mut(*object)
            && fields.contains("data")
        {
            fields.insert("text".to_owned());
        }
    }
    holds
}

/// The fixture's type objects.
const TYPE_OBJECTS: &[&str] = &["$T", "$TA"];

/// Whether `held` (a field, or every field) covers `wanted`.
fn covers(held: &Option<BTreeSet<String>>, wanted: &str) -> bool {
    held.as_ref().is_none_or(|held| held.iter().any(|field| field == wanted || wanted.starts_with(&format!("{field}."))))
}

/// What a declaration holds less than the reading (a write can land on what was not observed)
/// and more (a refusal no change needs).
#[derive(Default)]
struct Difference {
    less: Vec<String>,
    more: Vec<String>,
}

fn compare(editor: &BTreeSet<String>, holds: &Holds, row_editor: &BTreeSet<String>, wanted: &Holds) -> Difference {
    let mut difference = Difference::default();
    for state in row_editor.difference(editor) {
        difference.less.push(format!("reads {state}, not declared"));
    }
    for state in editor.difference(row_editor) {
        difference.more.push(format!("declares {state}, not read"));
    }
    for (object, (fields, at)) in wanted {
        let Some((held_fields, held_at)) = holds.get(object) else {
            difference.less.push(format!("{object} not held"));
            continue;
        };
        match fields {
            None if held_fields.is_some() => difference.less.push(format!("{object}: every field read, holds {}", show(held_fields))),
            None => {}
            Some(fields) => {
                for field in fields.iter().filter(|field| !covers(held_fields, field)) {
                    difference.less.push(format!("{object}.{field} not held"));
                }
            }
        }
        if *at && !held_at {
            difference.less.push(format!("{object}: where it is not held"));
        }
    }
    for (object, (held_fields, held_at)) in holds {
        let Some((fields, at)) = wanted.get(object) else {
            // That an object the step names is there is the least the guard holds of it.
            if held_fields.as_ref().is_some_and(BTreeSet::is_empty) && !held_at {
                continue;
            }
            difference.more.push(format!("{object} held ({}), not read", show(held_fields)));
            continue;
        };
        match held_fields {
            None if fields.is_some() => difference.more.push(format!("{object}: holds every field, reads {}", show(fields))),
            None => {}
            Some(held_fields) => {
                for field in held_fields.iter().filter(|field| !covers(fields, field)) {
                    difference.more.push(format!("{object}.{field} held, not read"));
                }
            }
        }
        if *held_at && !at {
            difference.more.push(format!("{object}: where it is held, not read"));
        }
    }
    difference
}

fn show(fields: &Option<BTreeSet<String>>) -> String {
    match fields {
        None => "*".to_owned(),
        Some(fields) if fields.is_empty() => "-".to_owned(),
        Some(fields) => fields.iter().cloned().collect::<Vec<_>>().join(","),
    }
}

/// A row's difference: none, or only what [`HELD_MORE`] lists for it (a refusal no change needs,
/// still the guard's); anything else, and every held-less, is wrong.
#[allow(clippy::too_many_arguments)]
fn judge(
    report: &mut Vec<String>,
    wrong: &mut Vec<String>,
    known: &mut Vec<(&str, &str, &str, &str)>,
    command: &str,
    params: &str,
    on: &str,
    pinned: bool,
    difference: &Difference,
) {
    if difference.less.is_empty() && difference.more.is_empty() {
        return;
    }
    let line = format!(
        "{command} {params} on [{on}]{}:\n  less: {}\n  more: {}",
        if pinned { " (pinned)" } else { "" },
        difference.less.join("; "),
        difference.more.join("; ")
    );
    report.push(line.clone());
    let more = difference.more.join("; ");
    let listed = known.iter().position(|(c, p, o, excess)| *c == command && *p == params && *o == on && *excess == more);
    match listed {
        Some(at) if difference.less.is_empty() => {
            known.remove(at);
        }
        _ => wrong.push(line),
    }
}

/// The editor state a row reads.
fn editor_of(row: &Row) -> BTreeSet<String> {
    row.editor.split(',').map(str::trim).filter(|state| !state.is_empty()).map(str::to_owned).collect()
}

/// Every write command's rows hold what its reading says, and every write command has a row.
#[test]
fn every_write_rests_on_what_its_handler_reads() {
    let (bytes, roles) = fixture();
    let mut doc = VectorDocument::open("fx.kvector", &bytes).unwrap();
    let names = Names::new(&bytes, &roles, &mut doc);
    let commands = doc.commands();
    let mut wrong = Vec::new();
    let mut report = Vec::new();
    let mut known: Vec<(&str, &str, &str, &str)> = HELD_MORE.to_vec();
    let mut editors: BTreeMap<&str, (BTreeSet<String>, BTreeSet<String>)> = BTreeMap::new();
    for row in ROWS {
        let command = commands.iter().find(|c| c.id == row.command).unwrap_or_else(|| panic!("no command {}", row.command));
        let params = with_roles(&serde_json::from_str(row.params).unwrap_or_else(|e| panic!("{} {}: {e}", row.command, row.params)), &roles);
        let on: Vec<u64> =
            row.on.iter().map(|role| roles.iter().find(|(name, _)| name == role).unwrap_or_else(|| panic!("no role {role}")).1).collect();
        let (editor, holds) = match held(&bytes, command, &params, &on, &names) {
            Ok(held) => held,
            Err(refused) => {
                wrong.push(format!("{} {} on {:?}: the host refuses the step: {refused}", row.command, row.params, row.on));
                continue;
            }
        };
        // The editor state a command reads is declared once for all its paths: compared below.
        let reads = editor_of(row);
        editors.entry(row.command).or_insert_with(|| (editor, BTreeSet::new())).1.extend(reads);
        let none = BTreeSet::new();
        let difference = compare(&none, &holds, &none, &expected(row, &roles));
        let on = row.on.join(",");
        judge(&mut report, &mut wrong, &mut known, row.command, row.params, &on, row.pinned, &difference);
    }
    for (command, (declared, read)) in &editors {
        let difference = compare(declared, &Holds::new(), read, &Holds::new());
        judge(&mut report, &mut wrong, &mut known, command, "(editor state)", "", false, &difference);
    }
    for (command, params, on, _) in known {
        wrong.push(format!("{command} {params} on [{on}]: listed in HELD_MORE, and holds no more than its reading now"));
    }
    let rows: BTreeSet<&str> = ROWS.iter().map(|row| row.command).collect();
    let writes: BTreeSet<&str> = commands.iter().filter(|c| c.mutates).map(|c| c.id.as_str()).collect();
    for missing in writes.difference(&rows) {
        wrong.push(format!("{missing}: a write command with no reading"));
    }
    for gone in rows.difference(&writes) {
        wrong.push(format!("{gone}: a reading of no write command"));
    }
    if let Ok(path) = std::env::var("DECLARATIONS_REPORT") {
        std::fs::write(path, report.join("\n")).unwrap();
    }
    assert_eq!(writes.len(), 508, "the write commands read");
    assert!(wrong.is_empty(), "{} rows differ from their readings:\n{}", wrong.len(), wrong.join("\n"));
}

/// What the guard still holds beyond a row's reading (a refusal no change needs), row by row:
/// (command, parameters, on, the excess). Each is a path the kit's row for the command states
/// more for than this path reads (see the guide: where the guard refuses a right write).
const HELD_MORE: &[(&str, &str, &str, &str)] = &[
    ("object.align", r#"{"horizontal":"left","to":"artboard"}"#, "$R", r#"artboard:1 held (rect), not read"#),
    ("object.blend.moveSpineAnchor", r#"{"id":"$B","anchor":0,"x":120,"y":110}"#, "", r#"$B2.appearance held, not read"#),
    ("object.blend.moveSpineAnchor", r#"{"anchor":1,"x":120,"y":110}"#, "$B", r#"$B1.appearance held, not read"#),
    ("object.clippingMask.make", r#"{}"#, "$R,$E", r#"$R.appearance held, not read; $R.data held, not read"#),
    ("object.clippingMask.release", r#"{}"#, "$CG", r#"$CA held (data), not read"#),
    ("object.envelope.makeWithTopObject", r#"{"ids":["$R","$E"]}"#, "", r#"$R: holds every field, reads -"#),
    ("object.hide.above", r#"{}"#, "$E", r#"$CA held (appearance,data), not read; $K1.appearance held, not read; $K2.appearance held, not read"#),
    ("object.liquify", r#"{"tool":"warp","points":[[50,50],[60,50]],"ids":["$P"]}"#, "", r#"picks held (data,locked,visible), not read"#),
    (
        "object.liquify",
        r#"{"tool":"bloat","points":[[50,50]]}"#,
        "",
        r#"$C1: where it is held, not read; $C2: where it is held, not read; $E: where it is held, not read; $R: where it is held, not read"#,
    ),
    (
        "object.path.cleanUp",
        r#"{}"#,
        "",
        r#"$B held (appearance,data,locked,visible), not read; $B1 held (appearance,data,locked,visible), not read; $B2 held (appearance,data,locked,visible), not read; $CG held (appearance,data,locked,visible), not read; $CK held (appearance,data,locked,visible), not read; $CP.data held, not read; $G held (appearance,data,locked,visible), not read; $K1.appearance held, not read; $K1.locked held, not read; $K1.visible held, not read; $K2.appearance held, not read; $K2.locked held, not read; $K2.visible held, not read; $S held (appearance,data,locked,visible), not read; $T.appearance held, not read; $TA.appearance held, not read"#,
    ),
    (
        "object.path.cleanUp",
        r#"{"unpaintedObjects":false,"emptyTextPaths":false}"#,
        "",
        r#"$B held (appearance,data,locked,visible), not read; $B1 held (appearance,data,locked,visible), not read; $B2 held (appearance,data,locked,visible), not read; $C1.appearance held, not read; $C2.appearance held, not read; $CA.appearance held, not read; $CG held (appearance,data,locked,visible), not read; $CK held (appearance,data,locked,visible), not read; $CP held (appearance,data,locked,visible), not read; $E.appearance held, not read; $G held (appearance,data,locked,visible), not read; $K1.appearance held, not read; $K1.locked held, not read; $K1.visible held, not read; $K2.appearance held, not read; $K2.locked held, not read; $K2.visible held, not read; $P.appearance held, not read; $PR.appearance held, not read; $R.appearance held, not read; $R2.appearance held, not read; $S held (appearance,data,locked,visible), not read; $T held (appearance,data,locked,text,visible), not read; $TA held (appearance,data,locked,text,visible), not read"#,
    ),
    ("object.path.divideObjectsBelow", r#"{}"#, "$R", r#"$L1 held (locked,visible), not read"#),
    (
        "object.setLiveShape",
        r#"{"items":[{"id":"$R","corners":[0]}],"radius":5}"#,
        "$E",
        r#"$R: holds every field, reads data; $R: where it is held, not read"#,
    ),
    ("object.textWrap.make", r#"{}"#, "$E", r#"$T held (data,text), not read; $TA held (data,text), not read"#),
    ("object.textWrap.make", r#"{"offset":4}"#, "$CG", r#"$CA held (data), not read; $T held (data,text), not read"#),
    ("object.textWrap.make", r#"{"ids":["$E"]}"#, "", r#"$T held (data,text), not read; $TA held (data,text), not read"#),
    (
        "arrange.ungroup",
        r#"{}"#,
        "$B",
        r#"$B: holds every field, reads data.type; $B: where it is held, not read; $B1 held (*), not read; $B2 held (*), not read"#,
    ),
    (
        "path.freehand",
        r#"{"points":[[0,0],[20,10],[40,0]],"extend":{"id":"$P","end":"end"}}"#,
        "",
        r#"$P: holds every field, reads appearance,data; $P: where it is held, not read"#,
    ),
    ("path.knife", r#"{"points":[[0,0],[1000,1000]]}"#, "$R", r#"picks held (data,locked,visible), not read"#),
    (
        "path.knife",
        r#"{"points":[[0,0],[1000,1000]]}"#,
        "",
        r#"$C1 held (*), not read; $C2 held (*), not read; $CA held (*), not read; $E held (*), not read; $PR held (*), not read; $R held (*), not read; $R2 held (*), not read"#,
    ),
    ("path.eraseRegion", r#"{"points":[[0,0],[1000,1000]],"size":10}"#, "$P", r#"picks held (data,locked,visible), not read"#),
    (
        "path.eraseRegion",
        r#"{"points":[[0,0],[1000,1000]],"size":10}"#,
        "",
        r#"$C1 held (*), not read; $C2 held (*), not read; $CA held (*), not read; $E held (*), not read; $P held (*), not read; $PR held (*), not read; $R held (*), not read; $R2 held (*), not read"#,
    ),
    ("path.blob", r#"{"points":[[5000,5000],[5010,5000]],"merge":false}"#, "", r#"picks held (data,locked,visible), not read"#),
    ("shaper.scribble", r#"{"ids":["$R","$E"],"points":[[20,20],[60,60]]}"#, "", r#"picks held (data,locked,visible), not read"#),
    ("shaper.scribble", r#"{"points":[[20,20],[60,60]]}"#, "", r#"$R: where it is held, not read"#),
    ("file.exportForScreens", r#"{"artboards":[1]}"#, "", r#"$AS held (*), not read"#),
    ("layer.new", r#"{"top":true,"name":"Top"}"#, "", r#"$L1: where it is held, not read"#),
    ("layer.move", r#"{"ids":["$R"],"target":"$L2","copy":true}"#, "", r#"$R: where it is held, not read"#),
    ("artboard.new", r#"{"x":0,"y":900,"width":100,"height":100,"name":"New"}"#, "", r#"artboard:1 held (rect), not read"#),
    (
        "artboard.setProps",
        r#"{"index":0,"x":50,"moveArt":true}"#,
        "",
        r#"$B1.locked held, not read; $B1.visible held, not read; $B2.locked held, not read; $B2.visible held, not read; $C1.locked held, not read; $C1.visible held, not read; $C2.locked held, not read; $C2.visible held, not read; $CA.locked held, not read; $CA.visible held, not read; $CK.locked held, not read; $CK.visible held, not read; $K1.locked held, not read; $K1.visible held, not read; $K2.locked held, not read; $K2.visible held, not read"#,
    ),
    ("layer.clippingMask.toggle", r#"{"id":"$G"}"#, "", r#"$C1 held (appearance,data), not read"#),
    ("layer.clippingMask.toggle", r#"{}"#, "$G", r#"$C1 held (appearance,data), not read"#),
    ("layer.clippingMask.toggle", r#"{"id":"$CG"}"#, "", r#"$CA held (data), not read; $CK: where it is held, not read"#),
    ("layer.merge", r#"{"ids":["$L2","$L1"]}"#, "", r#"$L1: holds every field, reads -"#),
    ("layer.flatten", r#"{}"#, "", r#"$L1: holds every field, reads -"#),
    ("layer.releaseToLayers", r#"{}"#, "", r#"$L1: holds every field, reads name; $L1: where it is held, not read"#),
    ("layer.releaseToLayersBuild", r#"{"id":"$G"}"#, "", r#"$C2: holds every field, reads -"#),
    ("layer.hideOthers", r#"{}"#, "", r#"$L1: where it is held, not read"#),
    (
        "artboard.move",
        r#"{"index":0,"dx":10,"dy":0,"moveArt":true}"#,
        "",
        r#"$B1.locked held, not read; $B1.visible held, not read; $B2.locked held, not read; $B2.visible held, not read; $C1.locked held, not read; $C1.visible held, not read; $C2.locked held, not read; $C2.visible held, not read; $CA.locked held, not read; $CA.visible held, not read; $CK.locked held, not read; $CK.visible held, not read; $K1.locked held, not read; $K1.visible held, not read; $K2.locked held, not read; $K2.visible held, not read"#,
    ),
    (
        "artboard.rearrange",
        r#"{}"#,
        "",
        r#"$B1.locked held, not read; $B1.visible held, not read; $B2.locked held, not read; $B2.visible held, not read; $C1.locked held, not read; $C1.visible held, not read; $C2.locked held, not read; $C2.visible held, not read; $CA.locked held, not read; $CA.visible held, not read; $CK.locked held, not read; $CK.visible held, not read; $K1.locked held, not read; $K1.visible held, not read; $K2.locked held, not read; $K2.visible held, not read"#,
    ),
    ("artboard.reorder", r#"{"index":1,"to":0}"#, "", r#"artboard:0 held (*), not read"#),
    ("file.setup", r#"{"bleed":3}"#, "", r#"$T held (data,text), not read; $TA held (data,text), not read"#),
    ("file.setup", r#"{"flattenerPreset":"Low Resolution"}"#, "", r#"$T held (data,text), not read; $TA held (data,text), not read"#),
    ("file.place", r#"{"path":"/tmp/a.png"}"#, "", r#"$L1: where it is held, not read"#),
    ("file.place", r#"{"path":"/tmp/a.svg","at":[100,100]}"#, "", r#"$L1: where it is held, not read"#),
    ("transparency.set", r#"{"ids":["$R"],"opacity":50}"#, "", r#"drawing held (settings.mask_edit), not read"#),
    ("transparency.makeOpacityMask", r#"{"ids":["$R","$E"]}"#, "", r#"$R: holds every field, reads props.mask"#),
    (
        "transparency.makeOpacityMask",
        r#"{"ids":["$R","$E","$P"],"clip":true,"invert":false}"#,
        "",
        r#"$E: holds every field, reads -; $R: holds every field, reads -"#,
    ),
    ("transparency.makeOpacityMask", r#"{}"#, "$R,$E", r#"$R: holds every field, reads props.mask"#),
    ("transparency.makeOpacityMask", r#"{"ids":["$E"]}"#, "", r#"$E: holds every field, reads props.mask; $E: where it is held, not read"#),
    ("transparency.releaseOpacityMask", r#"{"ids":["$E"]}"#, "", r#"drawing held (settings.mask_edit), not read"#),
    ("transparency.disableOpacityMask", r#"{"ids":["$E"]}"#, "", r#"drawing held (settings.mask_edit), not read"#),
    ("transparency.enableOpacityMask", r#"{"ids":["$E"]}"#, "", r#"drawing held (settings.mask_edit), not read"#),
    ("transparency.unlinkOpacityMask", r#"{"ids":["$E"]}"#, "", r#"drawing held (settings.mask_edit), not read"#),
    ("transparency.linkOpacityMask", r#"{"ids":["$E"]}"#, "", r#"drawing held (settings.mask_edit), not read"#),
    (
        "text.orientation.vertical",
        r#"{"ids":["$L1"]}"#,
        "",
        r#"$B held (data), not read; $B1 held (data), not read; $B2 held (data), not read; $C1 held (data), not read; $C2 held (data), not read; $CA held (data), not read; $CG held (data), not read; $CK held (data), not read; $CP held (data), not read; $E held (data), not read; $G held (data), not read; $K1 held (data), not read; $K2 held (data), not read; $P held (data), not read; $PR held (data), not read; $R held (data), not read; $S held (data), not read"#,
    ),
    (
        "text.orientation.horizontal",
        r#"{"ids":["$L1"]}"#,
        "",
        r#"$B held (data), not read; $B1 held (data), not read; $B2 held (data), not read; $C1 held (data), not read; $C2 held (data), not read; $CA held (data), not read; $CG held (data), not read; $CK held (data), not read; $CP held (data), not read; $E held (data), not read; $G held (data), not read; $K1 held (data), not read; $K2 held (data), not read; $P held (data), not read; $PR held (data), not read; $R held (data), not read; $S held (data), not read"#,
    ),
    ("text.areaOptions", r#"{"ids":["$TA","$T"],"columns":2}"#, "", r#"$T.data held, not read; $T.text held, not read"#),
    ("text.fitHeadline", r#"{}"#, "$TA,$T", r#"$T.data held, not read; $T.text held, not read"#),
    (
        "text.setStyle",
        r#"{"ids":["$L1"],"tracking":50}"#,
        "",
        r#"$B held (data), not read; $B1 held (data), not read; $B2 held (data), not read; $C1 held (data), not read; $C2 held (data), not read; $CA held (data), not read; $CG held (data), not read; $CK held (data), not read; $CP held (data), not read; $E held (data), not read; $G held (data), not read; $K1 held (data), not read; $K2 held (data), not read; $P held (data), not read; $PR held (data), not read; $R held (data), not read; $S held (data), not read"#,
    ),
    ("graphicStyle.new", r#"{"id":"$G"}"#, "", r#"$C1 held (appearance), not read"#),
    ("graphicStyle.move", r#"{"name":"Style 1","to":0}"#, "", r#"graphicStyle:Style 1: holds every field, reads -"#),
    ("brush.apply", r#"{"name":"Charcoal"}"#, "$CG", r#"$CA.data held, not read; $CK.appearance held, not read"#),
    (
        "brush.freehand",
        r#"{"points":[[0,0],[50,50],[100,0]],"brush":"Charcoal","extend":{"id":"$P","end":"end"}}"#,
        "",
        r#"$P: holds every field, reads appearance,data; $P: where it is held, not read"#,
    ),
    ("symbol.place", r#"{}"#, "", r#"artboard:1 held (rect), not read"#),
    ("symbol.delete", r#"{"name":"Sym","expandInstances":false}"#, "", r#"$S: where it is held, not read"#),
    (
        "symbol.adjust",
        r#"{"tool":"shift","points":[[100,100],[110,100]]}"#,
        "",
        r#"$L2 held (locked,visible), not read; picks held (data,locked,visible), not read"#,
    ),
    (
        "symbol.adjust",
        r#"{"tool":"stain","points":[[100,100]]}"#,
        "",
        r#"$L2 held (locked,visible), not read; picks held (data,locked,visible), not read"#,
    ),
    (
        "symbol.adjust",
        r#"{"tool":"style","style":"Style 1","points":[[100,100]]}"#,
        "",
        r#"$L2 held (locked,visible), not read; picks held (data,locked,visible), not read"#,
    ),
    ("library.use", r#"{"kind":"fillColor","item":"Red"}"#, "$R", r#"swatch:Red held (*), not read"#),
    ("imageTrace.setView", r#"{"id":"$G","view":"outlines"}"#, "", r#"$C2 held (data.type), not read"#),
    ("asset.add", r#"{"ids":["$E"]}"#, "", r#"$E.name held, not read"#),
    ("appearance.addEffect", r#"(editor state)"#, "", r#"declares mode, not read"#),
    ("appearance.setItem", r#"(editor state)"#, "", r#"declares mode, not read"#),
    ("effect.apply", r#"(editor state)"#, "", r#"declares mode, not read"#),
    ("layer.clippingMask.toggle", r#"(editor state)"#, "", r#"declares active, not read"#),
    ("paint.setFill", r#"(editor state)"#, "", r#"declares mode, not read"#),
    ("transparency.enableOpacityMask", r#"(editor state)"#, "", r#"declares mode, not read"#),
    ("transparency.releaseOpacityMask", r#"(editor state)"#, "", r#"declares mode, not read"#),
    ("transparency.stopEditingOpacityMask", r#"(editor state)"#, "", r#"declares mode, not read"#),
];

const ROWS: &[Row] = &[
    // object.align: selected roots; reference = union of their bounds (default, no key), the artboard containing their centre (to:artboard), or the key; translates each; prefs Use Preview Bounds unless `bounds`.
    r(
        "object.align",
        r#"{"horizontal":"left"}"#,
        &["$R", "$E"],
        "mode,selection",
        &[("$R", "data,appearance", false), ("$E", "data,appearance", false)],
    ),
    // UNSURE: assumes $R lies on artboard 0; artboard_at takes the last artboard containing the centre (else index 0).
    r(
        "object.align",
        r#"{"horizontal":"left","to":"artboard"}"#,
        &["$R"],
        "mode,selection",
        &[("$R", "data,appearance", false), ("artboard:0", "rect", false), ("~artboards", "-", false)],
    ),
    r(
        "object.align",
        r#"{"vertical":"top","bounds":"geometric"}"#,
        &["$G", "$R"],
        "selection",
        &[
            ("$G", "data", false),
            ("children:$G", "-", false),
            ("$C1", "data,appearance", false),
            ("$C2", "data,appearance", false),
            ("$R", "data,appearance", false),
        ],
    ),
    // arrange.bringForward: selected roots, per parent; new index from own index and sibling count.
    r("arrange.bringForward", r#"{}"#, &["$R"], "selection", &[("$R", "-", true), ("children:$L1", "-", false)]),
    r("arrange.bringForward", r#"{}"#, &["$C1"], "selection", &[("$C1", "-", true), ("children:$G", "-", false)]),
    // arrange.bringToFront: selected roots moved to the top of their own parent.
    r("arrange.bringToFront", r#"{}"#, &["$R"], "selection", &[("$R", "-", true), ("children:$L1", "-", false)]),
    // arrange.sendBackward: selected roots one step down in their parent.
    r("arrange.sendBackward", r#"{}"#, &["$B"], "selection", &[("$B", "-", true), ("children:$L1", "-", false)]),
    // arrange.sendToBack: selected roots to the bottom of their parent.
    r("arrange.sendToBack", r#"{}"#, &["$B"], "selection", &[("$B", "-", true), ("children:$L1", "-", false)]),
    // arrange.sendToCurrentLayer: selected roots appended on top of the current layer (active_layer if still a layer, else default layer); no lock check.
    r("arrange.sendToCurrentLayer", r#"{}"#, &["$R2"], "active,selection", &[("$R2", "-", false), ("$L1", "data.type", false)]),
    // object.blend.expand: selected blends (or their contents' blend) replaced in place by a group of steps keeping every own field.
    r(
        "object.blend.expand",
        r#"{}"#,
        &["$B"],
        "selection",
        &[("$B", "*", false), ("children:$B", "-", false), ("$B1", "*", false), ("$B2", "*", false)],
    ),
    r(
        "object.blend.expand",
        r#"{}"#,
        &["$B1"],
        "selection",
        &[("$B", "*", false), ("children:$B", "-", false), ("$B1", "*", false), ("$B2", "*", false)],
    ),
    // object.blend.make: ids (paint order) or selected roots wrapped into a new blend above the topmost; spacing/orientation from params, else a blend among them, else prefs.blend_options.
    r("object.blend.make", r#"{"ids":["$R","$E"]}"#, &[], "mode", &[("$R", "-", true), ("$E", "-", true)]),
    r("object.blend.make", r#"{"ids":["$R","$E"],"steps":5,"orientation":"page"}"#, &[], "", &[("$R", "-", true), ("$E", "-", true)]),
    r("object.blend.make", r#"{}"#, &["$B", "$E"], "selection", &[("$B", "*", true), ("children:$B", "-", false), ("$E", "-", true)]),
    // object.blend.release: selected blends replaced by a spine path (from the key centres) and the keys, or a group keeping the blend's own fields.
    r(
        "object.blend.release",
        r#"{}"#,
        &["$B"],
        "selection",
        &[("$B", "*", false), ("children:$B", "-", false), ("$B1", "data", false), ("$B2", "data", false)],
    ),
    // object.blend.replaceSpine: first selected blend; first selected (selection order) plain path not inside it becomes the spine (path removed), keys moved onto it.
    r(
        "object.blend.replaceSpine",
        r#"{}"#,
        &["$B", "$P"],
        "selection",
        &[
            ("$B", "data", false),
            ("children:$B", "-", false),
            ("$B1", "data,appearance", false),
            ("$B2", "data,appearance", false),
            ("$P", "*", false),
        ],
    ),
    // object.blend.reverseFrontToBack: selected blends: children order, starts, key anchors and spine reversed.
    r("object.blend.reverseFrontToBack", r#"{}"#, &["$B"], "selection", &[("$B", "data", false), ("children:$B", "-", false)]),
    // object.blend.reverseSpine: selected blends; without an explicit spine each key moves to the mirrored key's centre.
    r(
        "object.blend.reverseSpine",
        r#"{}"#,
        &["$B"],
        "selection",
        &[("$B", "data", false), ("children:$B", "-", false), ("$B1", "data,appearance", false), ("$B2", "data,appearance", false)],
    ),
    // object.blend.addSpineAnchor: blend by id (else first selected blend); spine pinned from the keys' centres, anchor inserted.
    r(
        "object.blend.addSpineAnchor",
        r#"{"id":"$B","x":100,"y":100}"#,
        &[],
        "",
        &[("$B", "data", false), ("children:$B", "-", false), ("$B1", "data", false), ("$B2", "data", false)],
    ),
    r(
        "object.blend.addSpineAnchor",
        r#"{"x":100,"y":100}"#,
        &["$B"],
        "selection",
        &[("$B", "data", false), ("children:$B", "-", false), ("$B1", "data", false), ("$B2", "data", false)],
    ),
    // object.blend.moveSpineAnchor: blend by id (else selected); moves the spine point and the key sitting on it (anchor 0 = $B1); with handle only the handle.
    r(
        "object.blend.moveSpineAnchor",
        r#"{"id":"$B","anchor":0,"x":120,"y":110}"#,
        &[],
        "",
        &[("$B", "data", false), ("children:$B", "-", false), ("$B1", "data,appearance", false), ("$B2", "data", false)],
    ),
    r(
        "object.blend.moveSpineAnchor",
        r#"{"id":"$B","anchor":0,"x":120,"y":110,"handle":"out"}"#,
        &[],
        "",
        &[("$B", "data", false), ("children:$B", "-", false), ("$B1", "data", false), ("$B2", "data", false)],
    ),
    r(
        "object.blend.moveSpineAnchor",
        r#"{"anchor":1,"x":120,"y":110}"#,
        &["$B"],
        "selection",
        &[("$B", "data", false), ("children:$B", "-", false), ("$B1", "data", false), ("$B2", "data,appearance", false)],
    ),
    // object.blend.removeSpineAnchor: blend by id (else selected); removes a spine point no key sits on.
    // UNSURE: on the fixture's straight two-point spine both points carry keys, so this errors; holds are what it reads.
    // Pinned after review: the reading's rows give the target; without it the handler takes the selection (the reading's own defect note: a missing or non-integer id falls back to it; the earlier table check, tests/table_check.rs), so `selection` is read.
    p(
        "object.blend.removeSpineAnchor",
        r#"{"id":"$B","anchor":1}"#,
        &[],
        "selection",
        &[("$B", "data", false), ("children:$B", "-", false), ("$B1", "data", false), ("$B2", "data", false)],
    ),
    // object.clippingMask.make: selected roots; topmost becomes the clipping path (paint removed) of a new clip group placed where it was; others moved in, paint order.
    r("object.clippingMask.make", r#"{}"#, &["$R", "$E"], "selection", &[("$E", "data,appearance", true), ("$R", "-", true)]),
    r("object.clippingMask.make", r#"{}"#, &["$G", "$T"], "selection", &[("$T", "data,appearance", true), ("$G", "-", true)]),
    // object.clippingMask.release: selected roots that clip: clip flag off, first child's clipping flag off; others silently skipped.
    r(
        "object.clippingMask.release",
        r#"{}"#,
        &["$CG"],
        "selection",
        &[("$CG", "data.clip", false), ("children:$CG", "-", false), ("$CK", "data", false)],
    ),
    r("object.clippingMask.release", r#"{}"#, &["$R"], "selection", &[("$R", "data.type", false)]),
    // object.compoundPath.make: selected roots; new compound where the topmost was, with the topmost's appearance; members cloned with appearance reset and live shape dropped; group roots give their paths.
    r("object.compoundPath.make", r#"{}"#, &["$R", "$E"], "selection", &[("$R", "data,appearance", true), ("$E", "data,appearance", true)]),
    r(
        "object.compoundPath.make",
        r#"{}"#,
        &["$G", "$R"],
        "selection",
        &[
            ("$G", "*", true),
            ("children:$G", "-", false),
            ("$C1", "data,appearance", false),
            ("$C2", "data,appearance", false),
            ("$R", "data,appearance", true),
        ],
    ),
    // object.compoundPath.release: selected compounds replaced in place by their members, each given the compound's appearance; non-compounds skipped.
    r(
        "object.compoundPath.release",
        r#"{}"#,
        &["$CP"],
        "selection",
        &[("$CP", "*", false), ("children:$CP", "-", false), ("$K1", "appearance", false), ("$K2", "appearance", false)],
    ),
    r(
        "object.compoundPath.release",
        r#"{}"#,
        &["$K1"],
        "selection",
        &[("$CP", "*", false), ("children:$CP", "-", false), ("$K1", "appearance", false), ("$K2", "appearance", false)],
    ),
    // object.addObjectMosaic: first selected image (selection order); reads its pixels and bounds; tiles group above it.
    // UNSURE: the fixture has no image; the command is disabled (select an image). Holds = the kind check.
    // Pinned after review: `mode` from the preferences this command reads on a path the reading's rows do not take (tests/preferences.rs, READERS).
    p("object.addObjectMosaic", r#"{"columns":4,"rows":4}"#, &["$R"], "mode,selection", &[("$R", "data.type", false)]),
    // object.addTrimMarks: nothing selected -> every artboard's rect, else the selection's visual bounds; style from prefs unless given; groups appended to the current/insertion layer.
    r(
        "object.addTrimMarks",
        r#"{}"#,
        &[],
        "active,mode,selection",
        &[("artboard:0", "rect", false), ("artboard:1", "rect", false), ("~artboards", "-", false), ("$L1", "visible,locked", false)],
    ),
    r(
        "object.addTrimMarks",
        r#"{"style":"roman"}"#,
        &["$R"],
        "active,selection",
        &[("$R", "data,appearance", false), ("$L1", "visible,locked", false)],
    ),
    // object.cropImage: first selected image; rect given or the artboard under its centre; replaces its image data.
    // UNSURE: the fixture has no image; the command is disabled. Holds = the kind check.
    r("object.cropImage", r#"{"rect":[0,0,50,50]}"#, &["$R"], "selection", &[("$R", "data.type", false)]),
    // object.distort: ids (checked, not root-filtered) or selected roots; source box = their geometric bounds unless `from`; anchors projected, live shapes dropped, gradients warped.
    r("object.distort", r#"{"ids":["$R"],"corners":[[0,0],[100,0],[110,100],[-10,100]]}"#, &[], "", &[("$R", "data,appearance", false)]),
    r(
        "object.distort",
        r#"{"corners":[[0,0],[100,0],[110,100],[-10,100]]}"#,
        &["$G"],
        "selection",
        &[("$G", "appearance", false), ("children:$G", "-", false), ("$C1", "data,appearance", false), ("$C2", "data,appearance", false)],
    ),
    r(
        "object.distort",
        r#"{"ids":["$T"],"corners":[[0,0],[100,0],[110,100],[-10,100]],"from":[0,0,100,100]}"#,
        &[],
        "",
        &[("$T", "data,appearance,props.bbox_angle", false)],
    ),
    // object.distribute: selected roots (>=3), sorted by the chosen edge/centre of their bounds; evenly spaced translations; prefs Use Preview Bounds unless `bounds`.
    r(
        "object.distribute",
        r#"{"horizontal":"center"}"#,
        &["$R", "$E", "$T"],
        "mode,selection",
        &[("$R", "data,appearance", false), ("$E", "data,appearance", false), ("$T", "data,appearance", false)],
    ),
    // object.distributeSpacing: selected roots sorted by start; equal gaps (first stays), or a given spacing from the key object (none: the first).
    r(
        "object.distributeSpacing",
        r#"{"axis":"horizontal"}"#,
        &["$R", "$E", "$T"],
        "mode,selection",
        &[("$R", "data,appearance", false), ("$E", "data,appearance", false), ("$T", "data,appearance", false)],
    ),
    r(
        "object.distributeSpacing",
        r#"{"axis":"vertical","spacing":10,"bounds":"geometric"}"#,
        &["$R", "$E"],
        "selection",
        &[("$R", "data,appearance", false), ("$E", "data,appearance", false)],
    ),
    // object.envelope.editContents: selected envelopes' editing flag toggled.
    // UNSURE: no envelope in the fixture; disabled (select an envelope). Holds = the kind check.
    r("object.envelope.editContents", r#"{}"#, &["$E"], "selection", &[("$E", "data.type", false)]),
    // object.envelope.expand: selected envelopes replaced by groups of distorted content.
    // UNSURE: no envelope in the fixture; disabled. Holds = the kind check.
    r("object.envelope.expand", r#"{}"#, &["$E"], "selection", &[("$E", "data.type", false)]),
    // object.envelope.makeWithMesh: ids or selected roots; mesh grid over their geometric bounds; envelope options from the session defaults; placed above the topmost.
    r("object.envelope.makeWithMesh", r#"{"ids":["$R","$E"],"rows":2,"cols":2}"#, &[], "mode", &[("$R", "data", true), ("$E", "data", true)]),
    r(
        "object.envelope.makeWithMesh",
        r#"{}"#,
        &["$G"],
        "mode,selection",
        &[("$G", "data", true), ("children:$G", "-", false), ("$C1", "data", false), ("$C2", "data", false)],
    ),
    // object.envelope.makeWithTopObject: ids or selected roots; topmost's outline becomes the envelope (it is removed); the rest wrapped where the topmost content was.
    r("object.envelope.makeWithTopObject", r#"{"ids":["$R","$E"]}"#, &[], "mode", &[("$R", "-", true), ("$E", "*", true)]),
    r("object.envelope.makeWithTopObject", r#"{}"#, &["$T", "$E"], "mode,selection", &[("$T", "-", true), ("$E", "*", true)]),
    // object.envelope.makeWithWarp: ids or selected roots wrapped (above the topmost) in a warp envelope; options from session defaults.
    r("object.envelope.makeWithWarp", r#"{"ids":["$R","$E"],"style":"arc","bend":50}"#, &[], "mode", &[("$R", "-", true), ("$E", "-", true)]),
    r("object.envelope.makeWithWarp", r#"{"style":"flag"}"#, &["$G"], "mode,selection", &[("$G", "-", true)]),
    // object.envelope.options: selected envelopes' options; with none selected only the session defaults change (no document write).
    r("object.envelope.options", r#"{"fidelity":50}"#, &[], "mode,selection", &[]),
    r("object.envelope.options", r#"{"antiAlias":false}"#, &["$R"], "mode,selection", &[]),
    // object.envelope.release: selected envelopes replaced by content plus a shape.
    // UNSURE: no envelope in the fixture; disabled. Holds = the kind check.
    r("object.envelope.release", r#"{}"#, &["$E"], "selection", &[("$E", "data.type", false)]),
    // object.envelope.resetWithMesh: selected envelopes' kind rebuilt as a mesh from their current map.
    // UNSURE: no envelope in the fixture; disabled. Holds = the kind check.
    r("object.envelope.resetWithMesh", r#"{"rows":2,"cols":2}"#, &["$E"], "selection", &[("$E", "data.type", false)]),
    // object.envelope.resetWithWarp: selected envelopes' kind set to a warp (current warp params as defaults).
    // UNSURE: no envelope in the fixture; disabled. Holds = the kind check.
    r("object.envelope.resetWithWarp", r#"{"style":"arc"}"#, &["$E"], "selection", &[("$E", "data.type", false)]),
    // object.envelope.setMeshPoint: envelope by id; a mesh point or handle set.
    // UNSURE: $E is not a mesh envelope (none in the fixture): error. Holds = the kind check.
    r("object.envelope.setMeshPoint", r#"{"id":"$E","index":0,"x":10,"y":10}"#, &[], "", &[("$E", "data.type", false)]),
    // object.expand: selected roots; composite of blend/envelope/effect/live-shape/type/stroke/gradient expansions decided by each descendant's kind and appearance.
    // UNSURE: assumes $R has no stroke (only the live shape drops); with a stroke it would be replaced whole by outline art.
    r("object.expand", r#"{}"#, &["$R"], "selection", &[("$R", "data,appearance", false)]),
    r("object.expand", r#"{}"#, &["$T"], "selection", &[("$T", "*", false)]),
    r("object.expand", r#"{}"#, &["$B"], "selection", &[("$B", "*", false), ("children:$B", "-", false), ("$B1", "*", false), ("$B2", "*", false)]),
    // object.expandBrush: ids/id (layers dropped) or selected roots -> brushed paths under them replaced by brush art.
    // UNSURE: the fixture has no brush stroke: error "no brush strokes"; holds = what it checks.
    r("object.expandBrush", r#"{"ids":["$P"]}"#, &[], "", &[("$P", "appearance", false)]),
    // object.expandShape: selected roots that are paths lose their live shape; anything else (groups too) silently untouched.
    r("object.expandShape", r#"{}"#, &["$R"], "selection", &[("$R", "data", false)]),
    r("object.expandShape", r#"{}"#, &["$G"], "selection", &[("$G", "data.type", false)]),
    // object.flattenTransparency: ids/id or the selection (layers -> their children; hidden/locked and clipping paths skipped); reads each root's whole look; opaque art with default options -> no change.
    r(
        "object.flattenTransparency",
        r#"{"ids":["$R"]}"#,
        &[],
        "",
        &[("$R", "data,appearance,visible,locked,props.opacity,props.blend,props.mask", false), ("$L1", "visible,locked", false)],
    ),
    r(
        "object.flattenTransparency",
        r#"{}"#,
        &["$R", "$E"],
        "selection",
        &[
            ("$R", "data,appearance,visible,locked,props.opacity,props.blend,props.mask", false),
            ("$E", "data,appearance,visible,locked,props.opacity,props.blend,props.mask", false),
            ("$L1", "visible,locked", false),
        ],
    ),
    // arrange.group: selected roots moved, paint order, into a new group where the topmost was.
    r("arrange.group", r#"{}"#, &["$R", "$E"], "selection", &[("$R", "-", true), ("$E", "-", true)]),
    r("arrange.group", r#"{}"#, &["$R2", "$R"], "selection", &[("$R2", "-", true), ("$R", "-", true)]),
    // object.hide: selected roots' visible flag set false (the root only).
    r("object.hide", r#"{}"#, &["$R"], "selection", &[("$R", "visible", false)]),
    r("object.hide", r#"{}"#, &["$G"], "selection", &[("$G", "visible", false)]),
    // object.hide.above: per selected root, the siblings above its top-level ancestor in its layer whose visual bounds overlap its visual bounds get hidden.
    r("object.hide.above", r#"{}"#, &["$B"], "selection", &[("$B", "data,appearance", true), ("children:$L1", "-", false)]),
    // UNSURE: which siblings above $E overlap it depends on geometry the fixture does not give; every sibling above is read, hits get visible=false.
    r(
        "object.hide.above",
        r#"{}"#,
        &["$E"],
        "selection",
        &[
            ("$E", "data,appearance", true),
            ("children:$L1", "-", false),
            ("$G", "data,appearance", false),
            ("children:$G", "-", false),
            ("$C1", "data,appearance", false),
            ("$C2", "data,appearance", false),
            ("$P", "data,appearance", false),
            ("$T", "data,appearance", false),
            ("$S", "data,appearance", false),
            ("$PR", "data,appearance", false),
            ("$TA", "data,appearance", false),
            ("$CG", "data,appearance", false),
            ("children:$CG", "-", false),
            ("$CK", "data,appearance", false),
            ("$CP", "data,appearance", false),
            ("children:$CP", "-", false),
            ("$K1", "data", false),
            ("$K2", "data", false),
            ("$B", "data,appearance", false),
            ("children:$B", "-", false),
            ("$B1", "data,appearance", false),
            ("$B2", "data,appearance", false),
        ],
    ),
    // object.hide.otherLayers: every top-level layer not an ancestor of a selected object is hidden.
    r("object.hide.otherLayers", r#"{}"#, &["$R"], "selection", &[("$R", "-", true), ("$L2", "visible", false)]),
    // object.liquify: ids/id (not root-filtered; non-editable skipped) or selection.objects, else every visible unlocked path under the brush; reshapes reached paths only.
    // UNSURE: whether the stroke reaches $P depends on geometry; assumes it does.
    r(
        "object.liquify",
        r#"{"tool":"warp","points":[[50,50],[60,50]],"ids":["$P"]}"#,
        &[],
        "",
        &[("$P", "data,visible,locked", false), ("$L1", "visible,locked", false)],
    ),
    // UNSURE: brush hit-test with nothing selected; the objects it reaches depend on geometry; every visible unlocked path is a candidate.
    // Pinned after review: the reading could not place the stroke on the fixture (no coordinates); the objects under it are those the kit's own query finds there, each held whole, and the place is told by asking (`covers`: geometry, shown, unlocked; a candidate's paint is in its fields).
    p(
        "object.liquify",
        r#"{"tool":"bloat","points":[[50,50]]}"#,
        &[],
        "selection",
        &[
            ("picks", "data,locked,visible", false),
            ("$L1", "visible,locked", false),
            ("$L2", "visible,locked", false),
            ("$C1", "*", false),
            ("$C2", "*", false),
            ("$E", "*", false),
            ("$R", "*", false),
        ],
    ),
    // object.lock: selected roots' locked flag set true; selection cleared.
    r("object.lock", r#"{}"#, &["$R"], "selection", &[("$R", "locked", false)]),
    r("object.lock", r#"{}"#, &["$G"], "selection", &[("$G", "locked", false)]),
    // object.lock.above: as hide.above, locking the overlapping siblings above.
    r("object.lock.above", r#"{}"#, &["$B"], "selection", &[("$B", "data,appearance", true), ("children:$L1", "-", false)]),
    // object.lock.otherLayers: every top-level layer holding no selected object gets locked.
    r("object.lock.otherLayers", r#"{}"#, &["$R"], "selection", &[("$R", "-", true), ("$L2", "locked", false)]),
    r("object.lock.otherLayers", r#"{}"#, &["$R2"], "selection", &[("$R2", "-", true), ("$L1", "locked", false)]),
    // object.makePixelPerfect: ids (as given) or selected roots; own stroke widths rounded; snapped by its geometric bounds.
    r("object.makePixelPerfect", r#"{}"#, &["$R"], "selection", &[("$R", "data,appearance", false)]),
    r(
        "object.makePixelPerfect",
        r#"{"ids":["$G"]}"#,
        &[],
        "",
        &[("$G", "data,appearance", false), ("children:$G", "-", false), ("$C1", "data,appearance", false), ("$C2", "data,appearance", false)],
    ),
    // object.maskImage: first selected image clipped by a rectangle on its outline.
    // UNSURE: no image in the fixture; disabled. Holds = the kind check.
    r("object.maskImage", r#"{}"#, &["$R"], "selection", &[("$R", "data.type", false)]),
    // object.mesh.addLine: gradient mesh (or mesh envelope) by id; a row and column added.
    // UNSURE: no mesh in the fixture: error "not a gradient mesh". Holds = the kind check.
    r("object.mesh.addLine", r#"{"id":"$E","x":10,"y":10}"#, &[], "", &[("$E", "data.type", false)]),
    // object.mesh.add: ids or selected roots that are paths/compounds replaced in place by a mesh in their fill colour (keeps id, name, opacity, blend, locked, visible).
    r("object.mesh.add", r#"{"ids":["$E"]}"#, &[], "", &[("$E", "*", false)]),
    r(
        "object.mesh.add",
        r#"{"rows":2,"cols":2}"#,
        &["$CP"],
        "selection",
        &[("$CP", "*", false), ("children:$CP", "-", false), ("$K1", "*", false), ("$K2", "*", false)],
    ),
    // object.mesh.deletePoint: mesh by id; lines through a point deleted.
    // UNSURE: no mesh in the fixture: error. Holds = the kind check.
    r("object.mesh.deletePoint", r#"{"id":"$E","index":5}"#, &[], "", &[("$E", "data.type", false)]),
    // object.mesh.expand: selected meshes replaced by groups of flat pieces.
    // UNSURE: no mesh in the fixture; disabled. Holds = the kind check.
    r("object.mesh.expand", r#"{}"#, &["$E"], "selection", &[("$E", "data.type", false)]),
    // object.mesh.movePoint: mesh by id; a point or handle moved.
    // UNSURE: no mesh in the fixture: error. Holds = the kind check.
    r("object.mesh.movePoint", r#"{"id":"$E","index":0,"x":10,"y":10}"#, &[], "", &[("$E", "data.type", false)]),
    // object.mesh.setPointColor: mesh by id; a point's colour/opacity.
    // UNSURE: no mesh in the fixture: error. Holds = the kind check.
    r("object.mesh.setPointColor", r##"{"id":"$E","index":0,"color":"#ff0000"}"##, &[], "", &[("$E", "data.type", false)]),
    // object.move: selected roots translated by dx,dy (containers with their children); prefs Scale Strokes/Corners and Transform Patterns; copy duplicates each above itself first.
    r("object.move", r#"{"dx":10,"dy":0}"#, &["$R"], "mode,selection", &[("$R", "data,appearance", false)]),
    r(
        "object.move",
        r#"{"dx":10,"dy":0}"#,
        &["$G"],
        "mode,selection",
        &[("$G", "data", false), ("children:$G", "-", false), ("$C1", "data,appearance", false), ("$C2", "data,appearance", false)],
    ),
    r("object.move", r#"{"dx":10,"dy":0,"copy":true}"#, &["$R"], "mode,selection", &[("$R", "*", true)]),
    // object.nudge: dx,dy times prefs keyboard increment (x10 big); selected anchors, else selected roots (as object.move), else selected guides.
    r("object.nudge", r#"{"dx":1,"dy":0}"#, &["$R"], "mode,selection", &[("$R", "data,appearance", false)]),
    r(
        "object.nudge",
        r#"{"dx":0,"dy":-1,"big":true}"#,
        &["$G"],
        "mode,selection",
        &[("$G", "data", false), ("children:$G", "-", false), ("$C1", "data,appearance", false), ("$C2", "data,appearance", false)],
    ),
    r("object.nudge", r#"{"dx":1,"dy":0,"copy":true}"#, &["$E"], "mode,selection", &[("$E", "*", true)]),
    // object.path.addAnchorPoints: every non-guide path under the selected roots (group children, compound members, clipping paths, blend keys) gets anchors added; live shape dropped.
    r("object.path.addAnchorPoints", r#"{}"#, &["$R"], "selection", &[("$R", "data", false)]),
    r("object.path.addAnchorPoints", r#"{}"#, &["$CG"], "selection", &[("children:$CG", "-", false), ("$CA", "data", false), ("$CK", "data", false)]),
    r("object.path.addAnchorPoints", r#"{}"#, &["$B"], "selection", &[("children:$B", "-", false), ("$B1", "data", false), ("$B2", "data", false)]),
    // object.path.cleanUp: document-wide scan for stray points, unpainted paths/compounds and empty text (editable ones removed); fixture: nothing found, no write.
    r(
        "object.path.cleanUp",
        r#"{}"#,
        &[],
        "",
        &[
            ("children:$L1", "-", false),
            ("children:$L2", "-", false),
            ("$R2", "data,appearance,visible,locked", false),
            ("$R", "data,appearance,visible,locked", false),
            ("$E", "data,appearance,visible,locked", false),
            ("$P", "data,appearance,visible,locked", false),
            ("$PR", "data,appearance,visible,locked", false),
            ("children:$G", "-", false),
            ("$C1", "data,appearance,visible,locked", false),
            ("$C2", "data,appearance,visible,locked", false),
            ("children:$CG", "-", false),
            ("$CA", "data,appearance,visible,locked", false),
            ("$CP", "appearance,visible,locked", false),
            ("children:$CP", "-", false),
            ("$K1", "data", false),
            ("$K2", "data", false),
            ("$T", "data,visible,locked", false),
            ("$TA", "data,visible,locked", false),
            ("$L1", "visible,locked", false),
            ("$L2", "visible,locked", false),
        ],
    ),
    r(
        "object.path.cleanUp",
        r#"{"unpaintedObjects":false,"emptyTextPaths":false}"#,
        &[],
        "",
        &[
            ("children:$L1", "-", false),
            ("children:$L2", "-", false),
            ("$R2", "data,visible,locked", false),
            ("$R", "data,visible,locked", false),
            ("$E", "data,visible,locked", false),
            ("$P", "data,visible,locked", false),
            ("$PR", "data,visible,locked", false),
            ("children:$G", "-", false),
            ("$C1", "data,visible,locked", false),
            ("$C2", "data,visible,locked", false),
            ("children:$CG", "-", false),
            ("$CA", "data,visible,locked", false),
            ("children:$CP", "-", false),
            ("$K1", "data", false),
            ("$K2", "data", false),
            ("$L1", "visible,locked", false),
            ("$L2", "visible,locked", false),
        ],
    ),
    // object.path.divideObjectsBelow: exactly one selected root path cuts every editable visible closed path/compound below it (index path) whose bounds meet it; cutter removed.
    r("object.path.divideObjectsBelow", r#"{}"#, &["$R"], "selection", &[("$R", "*", true)]),
    // UNSURE: whether $E's bounds meet $R's is geometry the fixture does not give; assumes they do (then $R is replaced by two pieces).
    r(
        "object.path.divideObjectsBelow",
        r#"{}"#,
        &["$E"],
        "selection",
        &[("$E", "*", true), ("children:$L1", "-", false), ("$R", "*", false), ("$L1", "visible,locked", false)],
    ),
    // object.path.offsetPath: leaves (paths, compounds; not text) of each selected root offset; new path/group (leaf appearance, opacity, blend) inserted above the root.
    r("object.path.offsetPath", r#"{"offset":5}"#, &["$R"], "selection", &[("$R", "data,appearance,props.opacity,props.blend", true)]),
    r(
        "object.path.offsetPath",
        r#"{"offset":-3,"joins":"round"}"#,
        &["$G"],
        "selection",
        &[
            ("$G", "-", true),
            ("children:$G", "-", false),
            ("$C1", "data,appearance,props.opacity,props.blend", false),
            ("$C2", "data,appearance,props.opacity,props.blend", false),
        ],
    ),
    // object.path.outlineStroke: each non-text leaf under the selected roots with a visible stroke replaced in place by outline art (+ fill copy); errors if none.
    // UNSURE: assumes $R has a stroke (the fixture only names fills); without one this errors.
    r("object.path.outlineStroke", r#"{}"#, &["$R"], "selection", &[("$R", "*", false)]),
    r("object.path.outlineStroke", r#"{}"#, &["$G"], "selection", &[("children:$G", "-", false), ("$C1", "*", false), ("$C2", "*", false)]),
    // object.path.simplify: selected_roots, then every non-guide path under each root (compound members, group children); reads and replaces their anchors, drops live shapes.
    r("object.path.simplify", r#"{"tolerance":2}"#, &["$P"], "selection", &[("$P", "data", false)]),
    r(
        "object.path.simplify",
        r#"{"tolerance":2}"#,
        &["$G"],
        "selection",
        &[("$G", "-", false), ("children:$G", "-", false), ("$C1", "data", false), ("$C2", "data", false)],
    ),
    r(
        "object.path.simplify",
        r#"{"tolerance":2}"#,
        &["$CP"],
        "selection",
        &[("$CP", "-", false), ("children:$CP", "-", false), ("$K1", "data", false), ("$K2", "data", false)],
    ),
    // object.path.splitIntoGrid: selected_roots, path/compound roots only (others skipped); reads bounds + style, removes the root, inserts cells at its place.
    r("object.path.splitIntoGrid", r#"{"rows":2,"columns":2}"#, &["$R"], "selection", &[("$R", "*", true)]),
    r("object.path.splitIntoGrid", r#"{"rows":2,"columns":2}"#, &["$R", "$G"], "selection", &[("$R", "*", true), ("$G", "data.type", false)]),
    // object.pathfinder.crop: selected_roots (≥2); each leaf with its own style; result group where the topmost root was; all roots removed.
    r("object.pathfinder.crop", r#"{}"#, &["$R", "$E"], "selection", &[("$E", "*", true), ("$R", "*", false)]),
    r(
        "object.pathfinder.crop",
        r#"{}"#,
        &["$G", "$E"],
        "selection",
        &[("$G", "*", true), ("children:$G", "-", false), ("$C1", "*", false), ("$C2", "*", false), ("$E", "*", false)],
    ),
    // object.pathfinder.divide: selected_roots; leaves (paths/compounds/text) with own styles; group at the topmost root's place; roots removed.
    r("object.pathfinder.divide", r#"{}"#, &["$R", "$E"], "selection", &[("$E", "*", true), ("$R", "*", false)]),
    r(
        "object.pathfinder.divide",
        r#"{}"#,
        &["$G", "$E"],
        "selection",
        &[("$G", "*", true), ("children:$G", "-", false), ("$C1", "*", false), ("$C2", "*", false), ("$E", "*", false)],
    ),
    // object.pathfinder.exclude: selected_roots; shape mode (a group = union of its leaves, top leaf's style); one shape at the topmost root's place; roots removed.
    r("object.pathfinder.exclude", r#"{}"#, &["$R", "$E"], "selection", &[("$E", "*", true), ("$R", "*", false)]),
    r(
        "object.pathfinder.exclude",
        r#"{}"#,
        &["$G", "$E"],
        "selection",
        &[("$G", "*", true), ("children:$G", "-", false), ("$C1", "*", false), ("$C2", "*", false), ("$E", "*", false)],
    ),
    // object.pathfinder.intersect: as exclude (shape mode).
    r("object.pathfinder.intersect", r#"{}"#, &["$R", "$E"], "selection", &[("$E", "*", true), ("$R", "*", false)]),
    r(
        "object.pathfinder.intersect",
        r#"{}"#,
        &["$CG", "$E"],
        "selection",
        &[("$CG", "*", true), ("children:$CG", "-", false), ("$CA", "*", false), ("$CK", "*", false), ("$E", "*", false)],
    ),
    // object.pathfinder.merge: as divide (per-leaf styles, result group).
    r("object.pathfinder.merge", r#"{}"#, &["$R", "$E"], "selection", &[("$E", "*", true), ("$R", "*", false)]),
    // object.pathfinder.minusBack: as exclude (shape mode).
    // Pinned after review: which object is cut from which is their stacking order: where each is matters, not the frontmost's alone (tests/outside.rs s12).
    p("object.pathfinder.minusBack", r#"{}"#, &["$R", "$E"], "selection", &[("$E", "*", true), ("$R", "*", true)]),
    // object.pathfinder.minusFront: as exclude (shape mode).
    // Pinned after review: which object is cut from which is their stacking order: where each is matters, not the frontmost's alone (tests/outside.rs s12).
    p("object.pathfinder.minusFront", r#"{}"#, &["$R", "$E"], "selection", &[("$E", "*", true), ("$R", "*", true)]),
    // Pinned after review: which object is cut from which is their stacking order: where each is matters, not the frontmost's alone (tests/outside.rs s12).
    p(
        "object.pathfinder.minusFront",
        r#"{}"#,
        &["$CP", "$R"],
        "selection",
        &[("$CP", "*", true), ("children:$CP", "-", false), ("$K1", "*", false), ("$K2", "*", false), ("$R", "*", true)],
    ),
    // object.pathfinder.outline: as divide; faces re-stroked with their fill paint.
    r("object.pathfinder.outline", r#"{}"#, &["$R", "$E"], "selection", &[("$E", "*", true), ("$R", "*", false)]),
    // object.pathfinder.trim: as divide.
    r("object.pathfinder.trim", r#"{}"#, &["$R", "$E"], "selection", &[("$E", "*", true), ("$R", "*", false)]),
    // object.pathfinder.unite: as exclude (shape mode); text roots contribute glyph outlines.
    r("object.pathfinder.unite", r#"{}"#, &["$R", "$E"], "selection", &[("$E", "*", true), ("$R", "*", false)]),
    r(
        "object.pathfinder.unite",
        r#"{}"#,
        &["$G", "$R"],
        "selection",
        &[("$G", "*", true), ("children:$G", "-", false), ("$C1", "*", false), ("$C2", "*", false), ("$R", "*", false)],
    ),
    // object.pattern.cancel: only in pattern edit mode (fixture is not); restores the pattern whole from the saved original, removes the edit layer.
    // UNSURE: disabled on the fixture (not editing a pattern); row as if editing "Pat" via object.pattern.edit.
    // Pinned after review: the fixture is not in pattern editing mode, so there is no pattern being edited to hold; with one, `refs: pattern` holds it.
    p("object.pattern.cancel", r#"{}"#, &[], "", &[("drawing", "settings.pattern_edit", false)]),
    // object.pattern.done: only in pattern edit mode; reads the edit record and the edit layer's art, writes the pattern's art (and tile if size-to-art).
    // UNSURE: disabled on the fixture (not editing a pattern); the edit layer's art is not nameable in the fixture.
    // Pinned after review: the fixture is not in pattern editing mode, so there is no pattern being edited to hold; with one, `refs: pattern` holds it.
    p("object.pattern.done", r#"{}"#, &[], "", &[("drawing", "settings.pattern_edit", false)]),
    // object.pattern.edit: pattern by name (else the first selected object's pattern paint); copies its art into a new edit layer, sets pattern_edit, isolation, current layer.
    r("object.pattern.edit", r#"{"name":"Pat"}"#, &[], "", &[("pattern:Pat", "*", false), ("drawing", "settings.pattern_edit", false)]),
    // UNSURE: no name: $E is filled with Brand (not a pattern) in the fixture, so this fails; it reads $E's appearance to find a pattern.
    r("object.pattern.edit", r#"{}"#, &["$E"], "selection", &[("$E", "appearance", false), ("drawing", "settings.pattern_edit", false)]),
    // object.pattern.make: ids (existing non-layers, no root normalisation) else selected_roots; deep copies of the art (originals stay); edit=true enters pattern edit mode.
    r("object.pattern.make", r#"{"name":"P2"}"#, &["$R"], "selection", &[("$R", "*", false), ("drawing", "settings.pattern_edit", false)]),
    r(
        "object.pattern.make",
        r#"{"edit":false}"#,
        &["$G"],
        "selection",
        &[("$G", "*", false), ("children:$G", "-", false), ("$C1", "*", false), ("$C2", "*", false)],
    ),
    // Pinned after review: the reading gave `ids` and `on` together (the engine refuses `ids` with nothing selected, a defect); a host refuses a step with both, so the row gives `ids` alone.
    p("object.pattern.make", r#"{"ids":["$R"],"edit":false}"#, &[], "", &[("$R", "*", false)]),
    // object.pattern.saveCopy: only in pattern edit mode; reads the edited pattern whole and the edit layer's art, adds a copy.
    // UNSURE: disabled on the fixture (not editing a pattern).
    // Pinned after review: the fixture is not in pattern editing mode, so there is no pattern being edited to hold; with one, `refs: pattern` holds it.
    p("object.pattern.saveCopy", r#"{"name":"Pat copy"}"#, &[], "", &[("drawing", "settings.pattern_edit", false)]),
    // object.pinWarp: rest=false: ids as given else selected_roots, mesh from their anchors, warps data (+gradients); rest=true: ids else raw selection, reads stored pins, replaces nodes whole, sets doc.pin_warp.
    r("object.pinWarp", r#"{"pins":[[0,0]],"moved":[[5,5]]}"#, &["$P"], "selection", &[("$P", "data,appearance", false)]),
    r(
        "object.pinWarp",
        r#"{"ids":["$G"],"pins":[[0,0]],"moved":[[5,5]]}"#,
        &[],
        "",
        &[("$G", "appearance", false), ("children:$G", "-", false), ("$C1", "data,appearance", false), ("$C2", "data,appearance", false)],
    ),
    r(
        "object.pinWarp",
        r#"{"ids":["$P"],"pins":[],"moved":[],"rest":true}"#,
        &[],
        "",
        &[("$P", "*", false), ("drawing", "settings.pin_warp", false)],
    ),
    // UNSURE: the pin must lie on $P's mesh (fixture coordinates unknown); rest=true replaces $P whole by a warped copy of its rest shape.
    r(
        "object.pinWarp",
        r#"{"ids":["$P"],"pins":[[0,0]],"moved":[[5,5]],"rest":true}"#,
        &[],
        "",
        &[("$P", "*", false), ("drawing", "settings.pin_warp", false)],
    ),
    // object.pinWarp.pins: ids else raw selection; read-only (stored pins or automatic ones over the mesh); writes nothing.
    r("object.pinWarp.pins", r#"{"ids":["$P"]}"#, &[], "", &[("$P", "data", false), ("drawing", "settings.pin_warp", false)]),
    // object.rasterize: selected_roots; renders them whole with the document raster settings; image inserted where the topmost was; roots removed.
    r(
        "object.rasterize",
        r#"{}"#,
        &["$R"],
        "selection",
        &[("$R", "*", true), ("drawing", "settings.raster_effects_ppi,settings.raster_effects", false)],
    ),
    r(
        "object.rasterize",
        r#"{"ppi":72}"#,
        &["$S", "$R"],
        "selection",
        &[("$S", "*", true), ("symbol:Sym", "*", false), ("$R", "*", false), ("drawing", "settings.raster_effects", false)],
    ),
    // object.reflect: selected_roots; origin = centre of their oriented bounds (prefs preview bounds) unless given; transforms geometry, appearance, bbox angle; copy duplicates.
    r("object.reflect", r#"{"axis":"vertical"}"#, &["$R"], "mode,selection", &[("$R", "data,appearance,props.bbox_angle", false)]),
    r("object.reflect", r#"{"axis":"horizontal","copy":true}"#, &["$R"], "mode,selection", &[("$R", "*", true)]),
    r(
        "object.reflect",
        r#"{"axis":"vertical","origin":[0,0]}"#,
        &["$G"],
        "mode,selection",
        &[
            ("$G", "data,appearance,props.bbox_angle", false),
            ("children:$G", "-", false),
            ("$C1", "data,appearance,props.bbox_angle", false),
            ("$C2", "data,appearance,props.bbox_angle", false),
        ],
    ),
    // object.repeat.expand: only with a repeat selected (fixture has none); replaces each by a group of its instances.
    // UNSURE: disabled on the fixture (no repeat object).
    r("object.repeat.expand", r#"{}"#, &["$R"], "selection", &[]),
    // object.repeat.grid: one repeat selected and no ids → switch its kind; else ids/selected_roots moved whole into a new repeat at the topmost's place.
    r("object.repeat.grid", r#"{}"#, &["$R"], "selection", &[("$R", "*", true)]),
    r("object.repeat.grid", r#"{"rows":2,"cols":2}"#, &["$R", "$E"], "selection", &[("$E", "*", true), ("$R", "*", false)]),
    // object.repeat.mirror: as grid.
    r("object.repeat.mirror", r#"{}"#, &["$E"], "selection", &[("$E", "*", true)]),
    // Pinned after review: the reading gave `ids` and `on` together (the engine refuses `ids` with nothing selected, a defect); a host refuses a step with both, so the row gives `ids` alone.
    p("object.repeat.mirror", r#"{"ids":["$R"]}"#, &[], "", &[("$R", "*", true)]),
    // object.repeat.options: only with a repeat selected; edits that repeat's spec fields.
    // UNSURE: disabled on the fixture (no repeat object).
    r("object.repeat.options", r#"{"instances":6}"#, &["$R"], "selection", &[]),
    // object.repeat.radial: as grid.
    r("object.repeat.radial", r#"{"instances":8}"#, &["$R"], "selection", &[("$R", "*", true)]),
    r(
        "object.repeat.radial",
        r#"{"instances":8}"#,
        &["$G"],
        "selection",
        &[("$G", "*", true), ("children:$G", "-", false), ("$C1", "*", false), ("$C2", "*", false)],
    ),
    // object.repeat.release: only with a repeat selected; repeat removed, its source art put back in its place.
    // UNSURE: disabled on the fixture (no repeat object).
    r("object.repeat.release", r#"{}"#, &["$R"], "selection", &[]),
    // object.resetBoundingBox: selected_roots with a rotated bounding box (others skipped); sets the angle to 0.
    r("object.resetBoundingBox", r#"{}"#, &["$R"], "selection", &[("$R", "props.bbox_angle", false)]),
    // object.rotate: selected_roots; origin from oriented bounds unless given; absolute reads the current angle; transforms like reflect.
    r("object.rotate", r#"{"angle":30}"#, &["$R"], "mode,selection", &[("$R", "data,appearance,props.bbox_angle", false)]),
    r("object.rotate", r#"{"angle":30,"absolute":true}"#, &["$R"], "mode,selection", &[("$R", "data,appearance,props.bbox_angle", false)]),
    r("object.rotate", r#"{"angle":30,"copy":true}"#, &["$R"], "mode,selection", &[("$R", "*", true)]),
    r(
        "object.rotate",
        r#"{"angle":30}"#,
        &["$B"],
        "mode,selection",
        &[
            ("$B", "data,appearance,props.bbox_angle", false),
            ("children:$B", "-", false),
            ("$B1", "data,appearance,props.bbox_angle", false),
            ("$B2", "data,appearance,props.bbox_angle", false),
        ],
    ),
    // object.scale: selected_roots; origin from oriented bounds unless given; strokes/corners from params else prefs.
    r("object.scale", r#"{"sx":50}"#, &["$R"], "mode,selection", &[("$R", "data,appearance,props.bbox_angle", false)]),
    r(
        "object.scale",
        r#"{"sx":50,"sy":80,"strokes":true,"corners":true,"patterns":false,"origin":[0,0]}"#,
        &["$T"],
        "selection",
        &[("$T", "data,appearance,props.bbox_angle", false)],
    ),
    // object.setBounds: selected_roots; oriented geometric (and with prefs preview) bounds give the scale and move; transform as object.transform.
    r("object.setBounds", r#"{"width":200}"#, &["$R"], "mode,selection", &[("$R", "data,appearance,props.bbox_angle", false)]),
    r(
        "object.setBounds",
        r#"{"x":10,"y":10}"#,
        &["$G"],
        "mode,selection",
        &[
            ("$G", "data,appearance,props.bbox_angle", false),
            ("children:$G", "-", false),
            ("$C1", "data,appearance,props.bbox_angle", false),
            ("$C2", "data,appearance,props.bbox_angle", false),
        ],
    ),
    // object.setLiveShape: items / ids / raw selection; path targets only (groups skipped); corners param, else Direct-Selected anchors, else every corner; writes the path + live shape.
    r("object.setLiveShape", r#"{"radius":5}"#, &["$R"], "selection", &[("$R", "data", false)]),
    r("object.setLiveShape", r#"{"radius":5,"corners":[0]}"#, &["$R"], "selection", &[("$R", "data", false)]),
    r("object.setLiveShape", r#"{"pieStart":45}"#, &["$E"], "selection", &[("$E", "data", false)]),
    // Pinned after review: the reading gave `ids` and `on` together (the engine refuses `ids` with nothing selected, a defect); a host refuses a step with both, so the row gives `ids` alone.
    p("object.setLiveShape", r#"{"ids":["$R"],"radius":5,"corners":[0]}"#, &[], "", &[("$R", "data", false)]),
    r("object.setLiveShape", r#"{"items":[{"id":"$R","corners":[0]}],"radius":5}"#, &["$E"], "", &[("$R", "data", false)]),
    r("object.setLiveShape", r#"{"radius":5}"#, &["$G"], "selection", &[("$G", "data.type", false)]),
    // object.setOverprint: item/ids absent → Appearance panel active item if it fits, else every fill/stroke of the painted leaves (groups expanded; type runs too).
    r("object.setOverprint", r#"{"fill":true}"#, &["$R"], "selection", &[("$R", "appearance", false)]),
    r("object.setOverprint", r#"{"fill":true,"ids":["$R"]}"#, &[], "", &[("$R", "appearance", false)]),
    r(
        "object.setOverprint",
        r#"{"stroke":true}"#,
        &["$G"],
        "selection",
        &[("$G", "data.type", false), ("children:$G", "-", false), ("$C1", "appearance", false), ("$C2", "appearance", false)],
    ),
    r("object.setOverprint", r#"{"fill":true,"ids":["$T"]}"#, &[], "", &[("$T", "appearance,data", false)]),
    r("object.setOverprint", r#"{"fill":true,"item":0,"ids":["$R"]}"#, &[], "", &[("$R", "appearance", false)]),
    // object.setProps: ids/id or the raw selection (layers too); sets only the fields given, each whole; data merges keys into the attrs.
    r("object.setProps", r#"{"ids":["$R"],"name":"N","opacity":50}"#, &[], "", &[("$R", "name,props.opacity", false)]),
    r("object.setProps", r#"{"visible":false}"#, &["$G"], "selection", &[("$G", "visible", false)]),
    r(
        "object.setProps",
        r#"{"ids":["$R"],"locked":true,"blend":"Multiply","isolate":true,"knockout":"on","knockoutShape":true}"#,
        &[],
        "",
        &[("$R", "locked,props.blend,props.isolate,props.knockout,props.knockout_shape", false)],
    ),
    r("object.setProps", r#"{"id":"$R","data":{"pivot":"1,2"}}"#, &[], "", &[("$R", "props.attrs", false)]),
    // object.shape.convertToShape: selected_roots that are plain paths without a live shape; recognises rectangle/ellipse from the anchors.
    // UNSURE: fixture rectangles/ellipses are probably already live shapes (skipped → error); with a plain 4-anchor path it writes its data.
    r("object.shape.convertToShape", r#"{}"#, &["$R"], "selection", &[("$R", "data", false)]),
    // object.shear: selected_roots; origin from oriented bounds unless given; axis vertical or (anything else) horizontal.
    r("object.shear", r#"{"angle":20}"#, &["$R"], "mode,selection", &[("$R", "data,appearance,props.bbox_angle", false)]),
    // object.showAll: every non-layer node; hidden ones shown and selected.
    r("object.showAll", r#"{}"#, &[], "", &[("$ALL", "visible", false)]),
    // object.slice.clipToArtboard: toggles (or sets) the document's Clip to Artboard flag.
    r("object.slice.clipToArtboard", r#"{}"#, &[], "", &[("drawing", "settings.slices_clip_to_artboard", false)]),
    r("object.slice.clipToArtboard", r#"{"on":true}"#, &[], "", &[("drawing", "settings.slices_clip_to_artboard", false)]),
    // object.slice.combine: the selected slices (≥2): union of their bounds, the first one's options; all removed.
    // UNSURE: the fixture has one slice only (fails); `on` assumed to select slices.
    // Pinned after review: `on` takes objects, not slices (the host refuses `$SL` there); the row acts on the slice selection, empty in the fixture.
    p("object.slice.combine", r#"{}"#, &[], "selection", &[]),
    // object.slice.add: a new user slice over the rectangle given; selects it.
    r("object.slice.add", r#"{"x":0,"y":0,"width":50,"height":50}"#, &[], "", &[]),
    // object.slice.delete: `slices` param (all must be slices) else the selected slices; user slices removed, object slices released.
    r("object.slice.delete", r#"{"slices":["$SL"]}"#, &[], "", &[("$SL", "*", false)]),
    // UNSURE: `on` assumed to put the slice in the slice selection.
    r("object.slice.delete", r#"{}"#, &["$SL"], "selection", &[("$SL", "*", false)]),
    // object.slice.deleteAll: every user slice and every node's object slice options.
    r("object.slice.deleteAll", r#"{}"#, &[], "", &[("$SL", "*", false), ("$ALL", "props.slice", false)]),
    // object.slice.divide: the selected slices; each cut into a grid of user slices with its options; the slice removed.
    // UNSURE: `on` assumed to select the slice.
    // Pinned after review: `on` takes objects, not slices (the host refuses `$SL` there); the row acts on the slice selection, empty in the fixture.
    p("object.slice.divide", r#"{"rows":2}"#, &[], "selection", &[]),
    // object.slice.duplicate: the selected slices; copies their bounds + options offset by dx,dy.
    // UNSURE: `on` assumed to select the slice.
    // Pinned after review: `on` takes objects, not slices (the host refuses `$SL` there); the row acts on the slice selection, empty in the fixture.
    p("object.slice.duplicate", r#"{}"#, &[], "selection", &[]),
    // object.slice.fromGuides: the ruler guides cut the union of all artboards; replaces every user slice.
    r("object.slice.fromGuides", r#"{}"#, &[], "", &[("drawing", "guides,~artboards", false), ("$SL", "*", false)]),
    // object.slice.fromSelection: ids/id or raw selection; a user slice over their visual bounds.
    r("object.slice.fromSelection", r#"{}"#, &["$R"], "selection", &[("$R", "data,appearance", false)]),
    r(
        "object.slice.fromSelection",
        r#"{}"#,
        &["$G"],
        "selection",
        &[("$G", "data,appearance", false), ("children:$G", "-", false), ("$C1", "data,appearance", false), ("$C2", "data,appearance", false)],
    ),
    // Pinned after review: the reading gave `ids` and `on` together (the engine refuses `ids` with nothing selected, a defect); a host refuses a step with both, so the row gives `ids` alone.
    p("object.slice.fromSelection", r#"{"ids":["$R"]}"#, &[], "", &[("$R", "data,appearance", false)]),
    // object.slice.make: ids else selected_roots; layers and objects already slices skipped; sets default slice options.
    r("object.slice.make", r#"{}"#, &["$R"], "selection", &[("$R", "props.slice", false)]),
    // Pinned after review: the reading gave `ids` and `on` together (the engine refuses `ids` with nothing selected, a defect); a host refuses a step with both, so the row gives `ids` alone.
    p("object.slice.make", r#"{"ids":["$R"]}"#, &[], "", &[("$R", "props.slice", false)]),
    // object.slice.move: `slices` else selected slices; user slices' rect moved (object slices: their objects transformed).
    r("object.slice.move", r#"{"slices":["$SL"],"dx":5,"dy":0}"#, &[], "", &[("$SL", "*", false)]),
    // object.slice.options: the selected slices; sets the options given (none: read only).
    // UNSURE: `on` assumed to select the slice.
    // Pinned after review: `on` takes objects, not slices (the host refuses `$SL` there); the row acts on the slice selection, empty in the fixture.
    p("object.slice.options", r#"{"name":"S"}"#, &[], "selection", &[]),
    // object.slice.release: the selected slices; a user slice becomes an unpainted rectangle appended to the current layer (default layer if it is locked/hidden).
    // UNSURE: `on` assumed to select the slice.
    // Pinned after review: `on` takes objects, not slices (the host refuses `$SL` there); the row acts on the slice selection, empty in the fixture.
    p("object.slice.release", r#"{}"#, &[], "active,selection", &[("$L1", "visible,locked", false)]),
    // object.slice.select: selection change only (slices must exist); toggle reads the current slice selection.
    r("object.slice.select", r#"{"slices":["$SL"]}"#, &[], "", &[]),
    r("object.slice.select", r#"{"slices":["$SL"],"toggle":true}"#, &[], "selection", &[]),
    // object.slice.setRect: a user slice by id; sets its rectangle.
    r("object.slice.setRect", r#"{"id":"$SL","x":0,"y":0,"width":40,"height":40}"#, &[], "", &[("$SL", "*", false)]),
    // object.textWrap.make: ids else selected_roots; sets the wrap options whole; area type below it in its layer re-wraps around its outline (refresh).
    r("object.textWrap.make", r#"{}"#, &["$E"], "selection", &[("$E", "props.wrap", true)]),
    r(
        "object.textWrap.make",
        r#"{"offset":4}"#,
        &["$CG"],
        "selection",
        &[("$CG", "props.wrap", true), ("children:$CG", "-", false), ("$CK", "data", false), ("$TA", "data", false)],
    ),
    // Pinned after review: the reading gave `ids` and `on` together (the engine refuses `ids` with nothing selected, a defect); a host refuses a step with both, so the row gives `ids` alone.
    p("object.textWrap.make", r#"{"ids":["$E"]}"#, &[], "", &[("$E", "props.wrap", true)]),
    // object.textWrap.options: ids else selected_roots; missing options from the first target's wrap; sets the wrap whole on every non-layer target.
    r("object.textWrap.options", r#"{"offset":10}"#, &["$E"], "selection", &[("$E", "props.wrap", true)]),
    // object.textWrap.release: ids else selected_roots, filtered to wrap objects; clears their wrap.
    // UNSURE: the fixture has no wrap object (fails).
    r("object.textWrap.release", r#"{}"#, &["$E"], "selection", &[("$E", "props.wrap", true)]),
    // object.transform: ids AS GIVEN (no validation/normalisation) else selected_roots; matrix applied; copy duplicates; prefs give strokes/corners/patterns.
    r("object.transform", r#"{"matrix":[1,0,0,1,10,0],"ids":["$R"]}"#, &[], "mode", &[("$R", "data,appearance,props.bbox_angle", false)]),
    r("object.transform", r#"{"matrix":[2,0,0,2,0,0]}"#, &["$R"], "mode,selection", &[("$R", "data,appearance,props.bbox_angle", false)]),
    r("object.transform", r#"{"matrix":[1,0,0,1,10,0],"copy":true}"#, &["$R"], "mode,selection", &[("$R", "*", true)]),
    r(
        "object.transform",
        r#"{"matrix":[1,0,0,1,10,0],"ids":["$G"]}"#,
        &[],
        "mode",
        &[
            ("$G", "data,appearance,props.bbox_angle", false),
            ("children:$G", "-", false),
            ("$C1", "data,appearance,props.bbox_angle", false),
            ("$C2", "data,appearance,props.bbox_angle", false),
        ],
    ),
    r(
        "object.transform",
        r#"{"matrix":[1,0,0,1,0,0],"typeAreas":true,"ids":["$TA"]}"#,
        &[],
        "mode",
        &[("$TA", "data,appearance,props.bbox_angle", false)],
    ),
    // object.transformAgain: the session's last transform (matrix, copy) or last perspective move, on selected_roots; strokes/corners/patterns from current prefs.
    // UNSURE: the fixture session has no previous transform (fails).
    r("object.transformAgain", r#"{}"#, &["$R"], "mode,selection", &[("$R", "data,appearance,props.bbox_angle", false)]),
    // object.transformEach: selected_roots; each about its own bounds' reference point; random consumes one RNG stream in paint order.
    r(
        "object.transformEach",
        r#"{"rotate":30}"#,
        &["$R", "$E"],
        "mode,selection",
        &[("$R", "data,appearance,props.bbox_angle", false), ("$E", "data,appearance,props.bbox_angle", false)],
    ),
    r(
        "object.transformEach",
        r#"{"moveH":10,"random":true,"seed":1}"#,
        &["$R", "$E"],
        "mode,selection",
        &[("$R", "data,appearance,props.bbox_angle", true), ("$E", "data,appearance,props.bbox_angle", true)],
    ),
    r("object.transformEach", r#"{"scaleH":50,"copy":true}"#, &["$R"], "mode,selection", &[("$R", "*", true)]),
    // arrange.ungroup: selected_roots; plain/clip groups removed and their children put in their place (opacity multiplied); other objects kept.
    r("arrange.ungroup", r#"{}"#, &["$G"], "selection", &[("$G", "*", true), ("children:$G", "-", false), ("$C1", "*", false), ("$C2", "*", false)]),
    r(
        "arrange.ungroup",
        r#"{}"#,
        &["$CG"],
        "selection",
        &[("$CG", "*", true), ("children:$CG", "-", false), ("$CK", "*", false), ("$CA", "*", false)],
    ),
    r("arrange.ungroup", r#"{}"#, &["$B"], "selection", &[("$B", "data.type", false)]),
    // object.unlockAll: every non-layer node; locked ones unlocked and selected.
    r("object.unlockAll", r#"{}"#, &[], "", &[("$ALL", "locked", false)]),
    // object.vectorHalftone: selected_roots rendered whole and sampled; dots group where the topmost was; roots removed unless keepOriginal.
    r("object.vectorHalftone", r#"{}"#, &["$E"], "selection", &[("$E", "*", true)]),
    r("object.vectorHalftone", r#"{"keepOriginal":true,"mode":"cmyk"}"#, &["$S"], "selection", &[("$S", "*", true), ("symbol:Sym", "*", false)]),
    // shape.rectangle: new art, no object resolved; goes on top of the active layer if it is visible+unlocked (else silently the top editable layer); look from session paint/template; draw mode.
    r("shape.rectangle", r#"{"x":10,"y":10,"width":50,"height":40}"#, &[], "active,mode", &[("$L1", "locked,visible", false)]),
    // shape.ellipse: as shape.rectangle.
    r("shape.ellipse", r#"{"x":10,"y":10,"width":50,"height":40}"#, &[], "active,mode", &[("$L1", "locked,visible", false)]),
    // shape.polygon: as shape.rectangle.
    r("shape.polygon", r#"{"cx":50,"cy":50,"radius":30,"sides":6}"#, &[], "active,mode", &[("$L1", "locked,visible", false)]),
    // shape.star: as shape.rectangle.
    r("shape.star", r#"{"cx":50,"cy":50,"radius1":30,"radius2":15,"points":5}"#, &[], "active,mode", &[("$L1", "locked,visible", false)]),
    // shape.flare: new group at top of the insertion parent (active layer / fallback); no session paint, no draw mode.
    r("shape.flare", r#"{"cx":100,"cy":100}"#, &[], "active", &[("$L1", "locked,visible", false)]),
    // shape.line: as shape.rectangle (stroke from session paint).
    r("shape.line", r#"{"x1":0,"y1":0,"x2":100,"y2":50}"#, &[], "active,mode", &[("$L1", "locked,visible", false)]),
    // shape.spiral: as shape.rectangle.
    r("shape.spiral", r#"{"cx":50,"cy":50,"radius":40}"#, &[], "active,mode", &[("$L1", "locked,visible", false)]),
    // shape.arc: as shape.rectangle.
    r("shape.arc", r#"{"x1":0,"y1":0,"x2":100,"y2":50}"#, &[], "active,mode", &[("$L1", "locked,visible", false)]),
    // shape.rectangularGrid: new group at top of the insertion parent; session stroke/template (draw mode ignored).
    r("shape.rectangularGrid", r#"{"x":0,"y":0,"width":100,"height":100}"#, &[], "active,mode", &[("$L1", "locked,visible", false)]),
    // shape.polarGrid: as shape.rectangularGrid.
    r("shape.polarGrid", r#"{"x":0,"y":0,"width":100,"height":100}"#, &[], "active,mode", &[("$L1", "locked,visible", false)]),
    // path.add: new art from anchors/d, as shape.rectangle.
    r("path.add", r#"{"anchors":[{"x":0,"y":0},{"x":50,"y":50}]}"#, &[], "active,mode", &[("$L1", "locked,visible", false)]),
    // path.appendAnchor: path by id; reads whether its last subpath is open, appends (geometry only).
    r("path.appendAnchor", r#"{"id":"$P","x":300,"y":300}"#, &[], "", &[("$P", "data", false)]),
    // path.close: path by id; closes its last open subpath.
    r("path.close", r#"{"id":"$P"}"#, &[], "", &[("$P", "data", false)]),
    // path.moveAnchors: the selection; objects without direct-selected anchors are translated whole (descendants too, gradients follow); prefs.scale_strokes read and discarded.
    r("path.moveAnchors", r#"{"dx":10,"dy":0}"#, &["$R"], "selection", &[("$R", "data,appearance", false)]),
    r(
        "path.moveAnchors",
        r#"{"dx":10,"dy":0}"#,
        &["$G"],
        "selection",
        &[("$G", "data,appearance", false), ("children:$G", "-", false), ("$C1", "data,appearance", false), ("$C2", "data,appearance", false)],
    ),
    // path.setHandle: path by id, anchor by index; one handle.
    r("path.setHandle", r#"{"id":"$P","subpath":0,"anchor":1,"which":"out","x":120,"y":80}"#, &[], "", &[("$P", "data", false)]),
    // path.setAnchors: path by id; geometry replaced whole.
    r("path.setAnchors", r#"{"id":"$P","subpaths":[{"anchors":[{"x":0,"y":0},{"x":10,"y":10}],"closed":false}]}"#, &[], "", &[("$P", "data", false)]),
    // path.reverse: ids/id/selection; every non-guide path in the subtrees (compound members, children) flipped by its own direction.
    r("path.reverse", r#"{"ids":["$R"]}"#, &[], "", &[("$R", "data", false)]),
    r(
        "path.reverse",
        r#"{}"#,
        &["$G"],
        "selection",
        &[("$G", "-", false), ("children:$G", "-", false), ("$C1", "data", false), ("$C2", "data", false)],
    ),
    r(
        "path.reverse",
        r#"{"ids":["$CP"],"reversed":true}"#,
        &[],
        "",
        &[("$CP", "-", false), ("children:$CP", "-", false), ("$K1", "data", false), ("$K2", "data", false)],
    ),
    // path.join: ids as given (paths) else the selected paths; one open path → closes it.
    r("path.join", r#"{"ids":["$P"]}"#, &[], "", &[("$P", "data", false)]),
    r("path.join", r#"{}"#, &["$P"], "selection", &[("$P", "data", false)]),
    // path.average: selected paths, their direct-selected anchors or all anchors; centroid over all of them.
    r("path.average", r#"{"axis":"both"}"#, &["$R", "$E"], "selection", &[("$R", "data", false), ("$E", "data", false)]),
    // path.convertAnchors: selected paths' (direct-selected or all) anchors.
    r("path.convertAnchors", r#"{"to":"corner"}"#, &["$E"], "selection", &[("$E", "data", false)]),
    // path.deleteAnchors: only direct-selected anchors (selection.anchors); a path emptied is deleted.
    // UNSURE: `on` selects whole objects; whether that populates selection.anchors is unknown here (none → no-op step).
    r("path.deleteAnchors", r#"{}"#, &["$P"], "selection", &[("$P", "data", false)]),
    // path.reshape: path by id; nearest point, falloff radius from the subpath's bounds.
    r("path.reshape", r#"{"id":"$P","x":50,"y":50,"dx":10,"dy":10}"#, &[], "", &[("$P", "data", false)]),
    // path.insertAnchor: path by id, segment by index.
    r("path.insertAnchor", r#"{"id":"$P","subpath":0,"segment":0,"t":0.5}"#, &[], "", &[("$P", "data", false)]),
    // path.cutAtAnchors: direct-selected anchors; an open one-subpath path becomes several paths, the extra ones full copies inserted just above it.
    // UNSURE: needs direct-selected anchors, which `on` may not give.
    r("path.cutAtAnchors", r#"{}"#, &["$P"], "selection", &[("$P", "*", true)]),
    // path.freehand: new art; with extend: the path by id, its last open subpath and its stroke's pressure profile.
    r("path.freehand", r#"{"points":[[0,0],[20,10],[40,0]]}"#, &[], "active,mode", &[("$L1", "locked,visible", false)]),
    r("path.freehand", r#"{"points":[[0,0],[20,10],[40,0]],"extend":{"id":"$P","end":"end"}}"#, &[], "", &[("$P", "data,appearance", false)]),
    // path.curvature: new art; with id the geometry is replaced whole by the fitted curve.
    r("path.curvature", r#"{"points":[{"x":0,"y":0},{"x":50,"y":30},{"x":100,"y":0}]}"#, &[], "active,mode", &[("$L1", "locked,visible", false)]),
    r("path.curvature", r#"{"points":[{"x":0,"y":0},{"x":50,"y":30},{"x":100,"y":0}],"id":"$P"}"#, &[], "", &[("$P", "data", false)]),
    // path.curvatureEdit: path by id, subpath/anchor by index.
    r("path.curvatureEdit", r#"{"id":"$P","op":"move","anchor":1,"x":60,"y":60}"#, &[], "", &[("$P", "data", false)]),
    // path.removeAnchor: path by id, anchor by index; refits (path deleted if emptied).
    r("path.removeAnchor", r#"{"id":"$P","subpath":0,"anchor":1}"#, &[], "", &[("$P", "data", false)]),
    // path.removeAnchors: direct-selected anchors of selected paths.
    // UNSURE: needs direct-selected anchors, which `on` may not give.
    r("path.removeAnchors", r#"{}"#, &["$P"], "selection", &[("$P", "data", false)]),
    // path.convertAnchor: path by id, anchor by index.
    r("path.convertAnchor", r#"{"id":"$P","subpath":0,"anchor":1,"to":"smooth"}"#, &[], "", &[("$P", "data", false)]),
    // path.reshapeSegment: path by id, segment by index.
    r("path.reshapeSegment", r#"{"id":"$P","subpath":0,"segment":0,"t":0.5,"dx":0,"dy":20}"#, &[], "", &[("$P", "data", false)]),
    // path.split: path by id; open path → second half is a full copy of it inserted above; closed path → opened (geometry only).
    r("path.split", r#"{"id":"$P","subpath":0,"anchor":1}"#, &[], "", &[("$P", "*", true)]),
    r("path.split", r#"{"id":"$R","subpath":0,"segment":0,"t":0.5}"#, &[], "", &[("$R", "data", false)]),
    // path.knife: the selection's paths, else every editable non-guide path in the document whose bounds meet the cut (closed only); pieces are full copies inserted above.
    // UNSURE: fixture coordinates unknown; the points are meant to cross $R only.
    r("path.knife", r#"{"points":[[0,0],[1000,1000]]}"#, &["$R"], "selection", &[("$R", "*", true)]),
    // UNSURE: fixture coordinates unknown; the hits of the pick cannot be listed.
    r("path.knife", r#"{"points":[[0,0],[1000,1000]]}"#, &[], "selection", &[("picks", "data,locked,visible", false)]),
    // path.eraseRegion: as path.knife (closed and open paths); a piece-less path is deleted.
    // UNSURE: fixture coordinates unknown; points meant to cross $P only.
    r("path.eraseRegion", r#"{"points":[[0,0],[1000,1000]],"size":10}"#, &["$P"], "selection", &[("$P", "*", true)]),
    // UNSURE: fixture coordinates unknown.
    r("path.eraseRegion", r#"{"points":[[0,0],[1000,1000]],"size":10}"#, &[], "selection", &[("picks", "data,locked,visible", false)]),
    // path.eraseSegments: selected subtrees' editable paths under the drag; pieces as path.knife.
    // UNSURE: fixture coordinates unknown.
    r("path.eraseSegments", r#"{"points":[[0,0],[1000,1000]]}"#, &["$P"], "selection", &[("$P", "*", true)]),
    // path.smoothRegion: selected subtrees' editable paths near the drag; geometry replaced.
    // UNSURE: fixture coordinates unknown.
    r("path.smoothRegion", r#"{"points":[[0,0],[1000,1000]]}"#, &["$P"], "selection", &[("$P", "data", false)]),
    // path.joinScrub: open-path ends near the scrub (selection's, else every editable path's); the two first along it are joined.
    // UNSURE: fixture coordinates unknown; meant to scrub both ends of $P (closes it).
    // Pinned after review: the reading could not place the stroke on the fixture (no coordinates); the objects under it are those the kit's own query finds there, each held whole, and the place is told by asking (`covers`: geometry, shown, unlocked; a candidate's paint is in its fields). The line meets no open path's end in the fixture.
    p("path.joinScrub", r#"{"points":[[0,0],[1000,1000]]}"#, &[], "selection", &[("picks", "data,locked,visible", false)]),
    // path.blob: new art, merged with every editable closed path (whatever is selected) filled with the session paint and unstroked that it overlaps.
    // UNSURE: fixture coordinates unknown; points meant to miss all art (no merge).
    // Pinned after review: the reading could not place the stroke on the fixture (no coordinates); the objects under it are those the kit's own query finds there, each held whole, and the place is told by asking (`covers`: geometry, shown, unlocked; a candidate's paint is in its fields).
    p(
        "path.blob",
        r#"{"points":[[5000,5000],[5010,5000]],"size":10}"#,
        &[],
        "active,mode",
        &[("$L1", "locked,visible", false), ("picks", "data,locked,visible", false)],
    ),
    r("path.blob", r#"{"points":[[5000,5000],[5010,5000]],"merge":false}"#, &[], "active,mode", &[("$L1", "locked,visible", false)]),
    // shapeBuilder.merge: ids else selected roots, sorted by stacking; leaves (visible paths/compounds) rebuilt; roots removed, results inserted where the front-most root was (copies carry every field).
    // UNSURE: fixture coordinates unknown; the point is meant to hit the overlap of $R and $E.
    r("shapeBuilder.merge", r#"{"ids":["$R","$E"],"points":[[50,50]]}"#, &[], "mode", &[("$R", "*", true), ("$E", "*", true)]),
    r(
        "shapeBuilder.merge",
        r#"{"points":[[50,50]],"fill":{"swatch":"Other"}}"#,
        &["$R", "$E"],
        "selection",
        &[("$R", "*", true), ("$E", "*", true), ("swatch:Other", "*", false)],
    ),
    r(
        "shapeBuilder.merge",
        r#"{"ids":["$G"],"points":[[50,50]]}"#,
        &[],
        "mode",
        &[("$G", "*", true), ("children:$G", "-", false), ("$C1", "*", false), ("$C2", "*", false)],
    ),
    // livePaint.make: ids else selected roots; each root cloned whole into the hidden sources; faces/edges from leaves' outlines and paints; replaces the roots at the front-most one's place.
    r("livePaint.make", r#"{"ids":["$R","$E"]}"#, &[], "", &[("$R", "*", true), ("$E", "*", true)]),
    r("livePaint.make", r#"{}"#, &["$G"], "selection", &[("$G", "*", true), ("children:$G", "-", false), ("$C1", "*", false), ("$C2", "*", false)]),
    // livePaint.merge: needs a Region Paint group (by name) among the roots; others' sources added, paint carried over.
    // UNSURE: no Region Paint group in the fixture; this fails. Reading for a Region Paint group $X: ($X,"*",true), children:$X, other roots "*".
    // Pinned after review: the reading's rows give the target; without it the handler takes the selection (the reading's own defect note: a missing or non-integer id falls back to it; the earlier table check, tests/table_check.rs), so `selection` is read.
    p("livePaint.merge", r#"{"ids":["$G","$R"]}"#, &[], "selection", &[("$G", "*", true), ("children:$G", "-", false), ("$R", "*", true)]),
    // livePaint.release: Region Paint groups (by name) among roots replaced by their sources.
    // UNSURE: no Region Paint group in the fixture; fails.
    // Pinned after review: the reading's rows give the target; without it the handler takes the selection (the reading's own defect note: a missing or non-integer id falls back to it; the earlier table check, tests/table_check.rs), so `selection` is read.
    p("livePaint.release", r#"{"id":"$G"}"#, &[], "selection", &[("$G", "*", true), ("children:$G", "-", false)]),
    // livePaint.expand: Region Paint groups (by name): name/trace cleared, sources and unpainted faces/edges dropped.
    // UNSURE: no Region Paint group in the fixture; fails.
    // Pinned after review: the reading's rows give the target; without it the handler takes the selection (the reading's own defect note: a missing or non-integer id falls back to it; the earlier table check, tests/table_check.rs), so `selection` is read. Expand also clears the group's Region Paint mark (`props.region_paint`), a field it writes.
    p("livePaint.expand", r#"{"id":"$G"}"#, &[], "selection", &[("$G", "name,props.region_paint,props.trace", false), ("children:$G", "-", false)]),
    // livePaint.fill: group by id, else ids/selection made live first; fills the face at the point (session fill by default).
    // UNSURE: fixture coordinates unknown; the point is meant to lie inside $R.
    r("livePaint.fill", r#"{"ids":["$R","$E"],"point":[20,20]}"#, &[], "mode", &[("$R", "*", true), ("$E", "*", true)]),
    r("livePaint.fill", r#"{"point":[20,20],"swatch":"Other"}"#, &["$R"], "selection", &[("$R", "*", true), ("swatch:Other", "*", false)]),
    // livePaint.strokeEdge: Region Paint group by id; nearest edge stroked (session stroke/weight by default).
    // UNSURE: no Region Paint group in the fixture; fails.
    r("livePaint.strokeEdge", r#"{"group":"$G","point":[20,20]}"#, &[], "mode", &[("$G", "name", false), ("children:$G", "-", false)]),
    // shaper.scribble: ids (filtered to editable paths) else ≥2 selected else every selectable object near the scribble plus its same-parent neighbourhood; roots cloned whole into the sources, replaced by a Shaper group at the front-most root's place.
    // UNSURE: fixture coordinates unknown; scribble meant to run inside $R and $E.
    r("shaper.scribble", r#"{"ids":["$R","$E"],"points":[[20,20],[60,60]]}"#, &[], "", &[("$R", "*", true), ("$E", "*", true)]),
    // UNSURE: fixture coordinates unknown; hits cannot be listed.
    // Pinned after review: the reading could not place the stroke on the fixture (no coordinates); the objects under it are those the kit's own query finds there, each held whole, and the place is told by asking (`covers`: geometry, shown, unlocked; a candidate's paint is in its fields).
    p("shaper.scribble", r#"{"points":[[20,20],[60,60]]}"#, &[], "selection", &[("picks", "data,locked,visible", false), ("$R", "*", false)]),
    // shaper.select: Shaper group (props.shaper, editable) whose bounds contain the point; selection/isolation only, no document write.
    r("shaper.select", r#"{"point":[20,20]}"#, &[], "selection", &[("picks", "data,locked,visible", false)]),
    // shaper.release: id else first selected; must be a Shaper group; replaced by its sources at its place.
    // UNSURE: no Shaper group in the fixture; fails.
    r("shaper.release", r#"{"id":"$G"}"#, &[], "", &[("$G", "*", true), ("children:$G", "-", false)]),
    // shaper.expand: same target; props.shaper and name cleared, sources removed.
    // UNSURE: no Shaper group in the fixture; fails.
    r("shaper.expand", r#"{"id":"$G"}"#, &[], "", &[("$G", "name,props.shaper", false), ("children:$G", "-", false)]),
    // perspective.grid.set: the stored grid (else the default fitted to the first artboard) merged with the patch, stored whole; reproject warps every attached object.
    r("perspective.grid.set", r#"{"horizon":300}"#, &[], "", &[("drawing", "settings.unknown", false), ("artboard:0", "rect", true)]),
    r(
        "perspective.grid.set",
        r#"{"horizon":300,"reproject":true}"#,
        &[],
        "",
        &[("drawing", "settings.unknown", false), ("artboard:0", "rect", true), ("$ALL", "props.perspective", false)],
    ),
    // perspective.grid.preset: preset (kind, or name: built-in or saved in prefs) fitted to the first artboard; old view options kept.
    r("perspective.grid.preset", r#"{"kind":2}"#, &[], "", &[("drawing", "settings.unknown", false), ("artboard:0", "rect", true)]),
    r(
        "perspective.grid.preset",
        r#"{"name":"1P-Normal View"}"#,
        &[],
        "mode",
        &[("drawing", "settings.unknown", false), ("artboard:0", "rect", true)],
    ),
    // perspective.grid.show: toggles (or sets) visible; whole grid stored, no undo step.
    r("perspective.grid.show", r#"{}"#, &[], "", &[("drawing", "settings.unknown", false), ("artboard:0", "rect", true)]),
    r("perspective.grid.show", r#"{"visible":true}"#, &[], "", &[("drawing", "settings.unknown", false), ("artboard:0", "rect", true)]),
    // perspective.plane.set: active plane; whole grid stored, no undo step.
    r("perspective.plane.set", r#"{"plane":"right"}"#, &[], "", &[("drawing", "settings.unknown", false), ("artboard:0", "rect", true)]),
    // perspective.attach: ids/id else selected roots; plane given or the grid's active plane; geometry projected from its bounds, attachment recorded.
    r(
        "perspective.attach",
        r#"{"ids":["$R"]}"#,
        &[],
        "",
        &[("$R", "data,props.perspective", false), ("drawing", "settings.unknown", false), ("artboard:0", "rect", true)],
    ),
    r(
        "perspective.attach",
        r#"{"plane":"ground"}"#,
        &["$G"],
        "selection",
        &[
            ("$G", "props.perspective", false),
            ("children:$G", "-", false),
            ("$C1", "data", false),
            ("$C2", "data", false),
            ("drawing", "settings.unknown", false),
            ("artboard:0", "rect", true),
        ],
    ),
    r(
        "perspective.attach",
        r#"{"ids":["$T"]}"#,
        &[],
        "",
        &[("$T", "data,props.perspective", false), ("drawing", "settings.unknown", false), ("artboard:0", "rect", true)],
    ),
    // perspective.release: ids/id else selected roots; attachment removed (projection kept for type/symbols); grid stored.
    r(
        "perspective.release",
        r#"{"ids":["$R"]}"#,
        &[],
        "",
        &[("$R", "props.perspective", false), ("drawing", "settings.unknown", false), ("artboard:0", "rect", true)],
    ),
    r(
        "perspective.release",
        r#"{}"#,
        &["$R"],
        "selection",
        &[("$R", "props.perspective", false), ("drawing", "settings.unknown", false), ("artboard:0", "rect", true)],
    ),
    // perspective.move: roots; unattached objects attach to the plane/active plane first, then slide (snap per grid); copy moves full copies inserted above; Transform Again recorded.
    r(
        "perspective.move",
        r#"{"ids":["$R"],"from":[10,10],"to":[30,10]}"#,
        &[],
        "",
        &[("$R", "data,props.perspective", false), ("drawing", "settings.unknown", false), ("artboard:0", "rect", true)],
    ),
    r(
        "perspective.move",
        r#"{"ids":["$R"],"from":[10,10],"to":[30,10],"copy":true}"#,
        &[],
        "",
        &[("$R", "*", true), ("drawing", "settings.unknown", false), ("artboard:0", "rect", true)],
    ),
    // perspective.draw: runs the shape command (new art) and attaches the result to the plane.
    r(
        "perspective.draw",
        r#"{"command":"shape.rectangle","params":{"x":10,"y":10,"width":50,"height":40}}"#,
        &[],
        "active,mode",
        &[("$L1", "locked,visible", false), ("drawing", "settings.unknown", false), ("artboard:0", "rect", true)],
    ),
    // perspective.transform: as perspective.move with a plane-space matrix.
    r(
        "perspective.transform",
        r#"{"ids":["$R"],"matrix":[2,0,0,2,0,0]}"#,
        &[],
        "",
        &[("$R", "data,props.perspective", false), ("drawing", "settings.unknown", false), ("artboard:0", "rect", true)],
    ),
    r(
        "perspective.transform",
        r#"{"matrix":[2,0,0,2,0,0]}"#,
        &["$R"],
        "selection",
        &[("$R", "data,props.perspective", false), ("drawing", "settings.unknown", false), ("artboard:0", "rect", true)],
    ),
    // perspective.nudge: selected roots, which must be in perspective; step from prefs keyboard increment.
    // UNSURE: nothing is in perspective in the fixture; this fails.
    r(
        "perspective.nudge",
        r#"{"dx":1,"dy":0}"#,
        &["$R"],
        "mode,selection",
        &[("$R", "data,props.perspective", false), ("drawing", "settings.unknown", false), ("artboard:0", "rect", true)],
    ),
    // perspective.plane.move: plane given or active; offset/by; objects none: grid only; move/copy: the editable objects on that plane at its depth.
    r("perspective.plane.move", r#"{"plane":"left","by":20}"#, &[], "", &[("drawing", "settings.unknown", false), ("artboard:0", "rect", true)]),
    r(
        "perspective.plane.move",
        r#"{"plane":"left","by":20,"objects":"move"}"#,
        &[],
        "",
        &[("drawing", "settings.unknown", false), ("artboard:0", "rect", true), ("$ALL", "props.perspective", false)],
    ),
    // perspective.plane.matchObject: id else first selected; its attachment moves its plane there and makes it active.
    // UNSURE: nothing is in perspective in the fixture; this fails.
    r(
        "perspective.plane.matchObject",
        r#"{"id":"$R"}"#,
        &[],
        "",
        &[("$R", "props.perspective", false), ("drawing", "settings.unknown", false), ("artboard:0", "rect", true)],
    ),
    // perspective.editText: id else first selected; type in perspective shown flat (text moved, projection compensated) in isolation.
    // UNSURE: no type in perspective in the fixture; this fails.
    r("perspective.editText", r#"{"id":"$T"}"#, &[], "", &[("$T", "data,props.perspective", false)]),
    // path.setFillRule: ids/id/selection; paths and compound paths (groups descend, compound members not) get the rule.
    r("path.setFillRule", r#"{"rule":"evenOdd","ids":["$CP"]}"#, &[], "", &[("$CP", "data", false)]),
    r(
        "path.setFillRule",
        r#"{"rule":"evenOdd"}"#,
        &["$G"],
        "selection",
        &[("$G", "-", false), ("children:$G", "-", false), ("$C1", "data", false), ("$C2", "data", false)],
    ),
    // path.mirrorCut: selected roots' editable paths/compounds (groups descend); axis through from, else the selection's bounds centre; each target replaced by its mirrored half (deleted when nothing is kept).
    // Pinned after review: `mode` from the preferences this command reads on a path the reading's rows do not take (tests/preferences.rs, READERS).
    p("path.mirrorCut", r#"{"axis":"vertical"}"#, &["$R"], "mode,selection", &[("$R", "data", false)]),
    r(
        "path.mirrorCut",
        r#"{"axis":"vertical","from":[100,0]}"#,
        &["$G"],
        "selection",
        &[("$G", "-", false), ("children:$G", "-", false), ("$C1", "data", false), ("$C2", "data", false)],
    ),
    r(
        "path.mirrorCut",
        r#"{"axis":"vertical"}"#,
        &["$CP"],
        "selection",
        &[("$CP", "data", false), ("children:$CP", "-", false), ("$K1", "*", false), ("$K2", "*", false)],
    ),
    // path.lineCut: selection's editable paths/compounds crossed by the line; extra pieces are full copies inserted above.
    // UNSURE: fixture coordinates unknown; the line is meant to cross $R.
    // Pinned after review: `mode` from the preferences this command reads on a path the reading's rows do not take (tests/preferences.rs, READERS).
    p("path.lineCut", r#"{"from":[0,0],"to":[1000,1000]}"#, &["$R"], "mode,selection", &[("$R", "*", true)]),
    // path.rectCut: selection's editable paths/compounds cropped to the rectangle (deleted when wholly outside).
    // Pinned after review: `mode` from the preferences this command reads on a path the reading's rows do not take (tests/preferences.rs, READERS).
    p("path.rectCut", r#"{"rect":[0,0,50,50]}"#, &["$R"], "mode,selection", &[("$R", "data", false)]),
    // perspective.grid.define: Define Grid fields over the grid (or the default for the first artboard); preset name matched against built-ins and saved presets (prefs).
    r("perspective.grid.define", r#"{"angle":30}"#, &[], "mode", &[("drawing", "settings.unknown", false), ("artboard:0", "rect", true)]),
    // perspective.grid.lock: toggles (or sets) Lock Grid; whole grid stored, no undo step.
    r("perspective.grid.lock", r#"{}"#, &[], "", &[("drawing", "settings.unknown", false), ("artboard:0", "rect", true)]),
    // perspective.grid.lockStation: as perspective.grid.lock.
    r("perspective.grid.lockStation", r#"{"on":true}"#, &[], "", &[("drawing", "settings.unknown", false), ("artboard:0", "rect", true)]),
    // perspective.grid.snap: as perspective.grid.lock.
    r("perspective.grid.snap", r#"{}"#, &[], "", &[("drawing", "settings.unknown", false), ("artboard:0", "rect", true)]),
    // perspective.grid.rulers: as perspective.grid.lock.
    r("perspective.grid.rulers", r#"{}"#, &[], "", &[("drawing", "settings.unknown", false), ("artboard:0", "rect", true)]),
    // edit.undo: the top of the undo history (session) in the caller's or the named scope; with a replica it undoes the step's transactions, without one it puts back the whole snapshot, restoring the saved selection.
    // UNSURE: what it rests on is whatever the undone step wrote; the fixture names no history, so no object entry can be given.
    r("edit.undo", r#"{}"#, &[], "mode", &[]),
    // UNSURE: as edit.undo, scope given.
    r("edit.undo", r#"{"scope":"agent"}"#, &[], "mode", &[]),
    // edit.redo: the top of the redo stack, as edit.undo.
    // UNSURE: as edit.undo.
    r("edit.redo", r#"{}"#, &[], "mode", &[]),
    // edit.cut: selection roots (a compound member stands for its compound; tool id checked first); copies each root whole with its layer's name, the artboard its centre falls in and the resources it uses, then deletes it.
    r(
        "edit.cut",
        r#"{}"#,
        &["$R"],
        "mode,selection",
        &[("$R", "*", false), ("$L1", "name", false), ("drawing", "~artboards", false), ("swatch:Brand", "*", false)],
    ),
    r(
        "edit.cut",
        r#"{}"#,
        &["$G"],
        "mode,selection",
        &[
            ("$G", "*", false),
            ("children:$G", "-", false),
            ("$C1", "*", false),
            ("$C2", "*", false),
            ("$L1", "name", false),
            ("drawing", "~artboards", false),
            ("swatch:Brand", "*", false),
        ],
    ),
    r(
        "edit.cut",
        r#"{}"#,
        &["$K1"],
        "mode,selection",
        &[
            ("$K1", "*", true),
            ("$CP", "*", false),
            ("children:$CP", "-", false),
            ("$K2", "*", false),
            ("$L1", "name", false),
            ("drawing", "~artboards", false),
            ("swatch:Brand", "*", false),
        ],
    ),
    // edit.paste: clipboard (assumed: a copy of $R from this document) pasted at the top of the insertion parent (active layer if editable, else the default layer), offset by dx/dy or the Paste Offset pref, or centred on `center`; Paste Remembers Layers read; colours relinked to the document's current global swatch.
    r(
        "edit.paste",
        r#"{}"#,
        &[],
        "active,mode",
        &[("$L1", "visible,locked", false), ("drawing", "settings.paste_remembers_layers", false), ("swatch:Brand", "*", false)],
    ),
    r(
        "edit.paste",
        r#"{"center":[300,200]}"#,
        &[],
        "active",
        &[("$L1", "visible,locked", false), ("drawing", "settings.paste_remembers_layers", false), ("swatch:Brand", "*", false)],
    ),
    // edit.pasteInFront: just above the topmost selected object (its parent and index), else the top of the current layer; `artboard` index moves it by that artboard's offset.
    r(
        "edit.pasteInFront",
        r#"{}"#,
        &["$R"],
        "active,selection",
        &[("$R", "-", true), ("$L1", "visible,locked", false), ("drawing", "settings.paste_remembers_layers", false), ("swatch:Brand", "*", false)],
    ),
    r(
        "edit.pasteInFront",
        r#"{}"#,
        &[],
        "active,selection",
        &[("$L1", "visible,locked", false), ("drawing", "settings.paste_remembers_layers", false), ("swatch:Brand", "*", false)],
    ),
    r(
        "edit.pasteInFront",
        r#"{"artboard":1}"#,
        &[],
        "active,selection",
        &[
            ("artboard:1", "rect", true),
            ("$L1", "visible,locked", false),
            ("drawing", "settings.paste_remembers_layers", false),
            ("swatch:Brand", "*", false),
        ],
    ),
    // edit.pasteInBack: at the index of the bottommost selected object, else index 0 of the current layer.
    r(
        "edit.pasteInBack",
        r#"{}"#,
        &["$R"],
        "active,selection",
        &[("$R", "-", true), ("$L1", "visible,locked", false), ("drawing", "settings.paste_remembers_layers", false), ("swatch:Brand", "*", false)],
    ),
    r(
        "edit.pasteInBack",
        r#"{}"#,
        &[],
        "active,selection",
        &[("$L1", "visible,locked", false), ("drawing", "settings.paste_remembers_layers", false), ("swatch:Brand", "*", false)],
    ),
    // edit.pasteInPlace: where copied, on top of the current layer; `artboard` index shifts by that artboard's origin.
    r(
        "edit.pasteInPlace",
        r#"{}"#,
        &[],
        "active",
        &[("$L1", "visible,locked", false), ("drawing", "settings.paste_remembers_layers", false), ("swatch:Brand", "*", false)],
    ),
    r(
        "edit.pasteInPlace",
        r#"{"artboard":1}"#,
        &[],
        "active",
        &[
            ("artboard:1", "rect", true),
            ("$L1", "visible,locked", false),
            ("drawing", "settings.paste_remembers_layers", false),
            ("swatch:Brand", "*", false),
        ],
    ),
    // edit.pasteOnAllArtboards: one copy per artboard at its offset from the source artboard: every artboard's rect and the list.
    r(
        "edit.pasteOnAllArtboards",
        r#"{}"#,
        &[],
        "active",
        &[
            ("drawing", "~artboards", false),
            ("$L1", "visible,locked", false),
            ("drawing", "settings.paste_remembers_layers", false),
            ("swatch:Brand", "*", false),
        ],
    ),
    // edit.delete: `ids` (checked; layers dropped, descendants of listed ancestors dropped) else the selection (anchors → delete anchors; nothing → selected guides); deletes each whole.
    r("edit.delete", r#"{"ids":["$R"]}"#, &[], "", &[("$R", "*", false)]),
    r("edit.delete", r#"{}"#, &["$R"], "selection", &[("$R", "*", false)]),
    r("edit.delete", r#"{}"#, &["$G"], "selection", &[("$G", "*", false), ("children:$G", "-", false), ("$C1", "*", false), ("$C2", "*", false)]),
    // edit.duplicate: selection roots; each copied whole just above itself (its parent and index), offset dx/dy.
    r("edit.duplicate", r#"{"dx":10,"dy":0}"#, &["$R"], "selection", &[("$R", "*", true)]),
    r("edit.duplicate", r#"{}"#, &["$G"], "selection", &[("$G", "*", true), ("children:$G", "-", false), ("$C1", "*", false), ("$C2", "*", false)]),
    // recolor.apply: selection.objects as they are; every colour of their subtrees mapped; limitTo "document" reads the document's swatches.
    r("recolor.apply", r##"{"map":{"#ff0000":"#00ff00"}}"##, &["$R"], "selection", &[("$R", "appearance", false)]),
    r(
        "recolor.apply",
        r##"{"map":{"#ff0000":"#00ff00"}}"##,
        &["$G"],
        "selection",
        &[("$G", "appearance", false), ("children:$G", "-", false), ("$C1", "appearance", false), ("$C2", "appearance", false)],
    ),
    r(
        "recolor.apply",
        r##"{"map":{"#ff0000":"#00ff00"},"limitTo":"document"}"##,
        &["$R"],
        "selection",
        &[("$R", "appearance", false), ("drawing", "~swatches", false)],
    ),
    // paint.setFill: ids → paint leaves of ids (top fill); else the Appearance panel's active item or the paint leaves of the selection; a colour converts to the document colour mode; a swatch's definition is read; text sets every run.
    r("paint.setFill", r##"{"color":"#ff0000"}"##, &["$R"], "selection", &[("$R", "appearance", false), ("drawing", "settings.color_mode", false)]),
    r("paint.setFill", r##"{"ids":["$R"],"color":"#ff0000"}"##, &[], "", &[("$R", "appearance", false), ("drawing", "settings.color_mode", false)]),
    r("paint.setFill", r#"{"swatch":"Brand"}"#, &["$R"], "selection", &[("$R", "appearance", false), ("swatch:Brand", "*", false)]),
    r(
        "paint.setFill",
        r##"{"color":"#ff0000"}"##,
        &["$G"],
        "selection",
        &[("children:$G", "-", false), ("$C1", "appearance", false), ("$C2", "appearance", false), ("drawing", "settings.color_mode", false)],
    ),
    r("paint.setFill", r##"{"color":"#ff0000"}"##, &["$T"], "selection", &[("$T", "data", false), ("drawing", "settings.color_mode", false)]),
    r("paint.setFill", r##"{"color":"#ff0000"}"##, &[], "selection", &[("drawing", "settings.color_mode", false)]),
    // paint.setStroke: as paint.setFill on the top stroke.
    r("paint.setStroke", r##"{"color":"#ff0000"}"##, &["$R"], "selection", &[("$R", "appearance", false), ("drawing", "settings.color_mode", false)]),
    r("paint.setStroke", r#"{"ids":["$R"],"none":true}"#, &[], "", &[("$R", "appearance", false)]),
    r(
        "paint.setStroke",
        r#"{"swatch":"Brand"}"#,
        &["$G"],
        "selection",
        &[("children:$G", "-", false), ("$C1", "appearance", false), ("$C2", "appearance", false), ("swatch:Brand", "*", false)],
    ),
    // paint.swap: paint leaves of ids/id/selection; swaps each top fill and stroke (text: every run); swaps the session defaults.
    r("paint.swap", r#"{}"#, &["$R"], "mode,selection", &[("$R", "appearance", false)]),
    r("paint.swap", r#"{"ids":["$T"]}"#, &[], "mode", &[("$T", "data", false)]),
    r("paint.swap", r#"{}"#, &[], "mode,selection", &[]),
    // paint.default: paint leaves of ids/id/selection; appearance set whole to default art (text: runs); session defaults reset.
    r("paint.default", r#"{}"#, &["$R"], "selection", &[("$R", "appearance", false)]),
    r("paint.default", r#"{"ids":["$G"]}"#, &[], "", &[("children:$G", "-", false), ("$C1", "appearance", false), ("$C2", "appearance", false)]),
    // paint.none: the active proxy (fill_active) set to None on the selection as paint.setFill/Stroke.
    r("paint.none", r#"{}"#, &["$R"], "mode,selection", &[("$R", "appearance", false)]),
    // stroke.set: item (active item) or paint leaves of ids/selection minus images and symbol instances (by kind); creates a missing top stroke, sets the given fields; text: run strokes; nothing selected: new-art template; profile names from prefs.
    r("stroke.set", r#"{"weight":2}"#, &["$R"], "selection", &[("$R", "appearance,data.type", false)]),
    r("stroke.set", r#"{"ids":["$R"],"weight":2,"join":"round"}"#, &[], "", &[("$R", "appearance,data.type", false)]),
    r(
        "stroke.set",
        r#"{"weight":2}"#,
        &["$G"],
        "selection",
        &[("children:$G", "-", false), ("$C1", "appearance,data.type", false), ("$C2", "appearance,data.type", false)],
    ),
    r("stroke.set", r#"{"weight":2}"#, &["$T"], "selection", &[("$T", "data", false)]),
    r("stroke.set", r#"{"weight":2}"#, &["$S"], "selection", &[("$S", "data.type", false)]),
    r("stroke.set", r#"{"profile":"lens"}"#, &["$R"], "mode,selection", &[("$R", "appearance,data.type", false)]),
    r("stroke.set", r#"{"weight":2}"#, &[], "mode,selection", &[]),
    // stroke.setAdvanced: as stroke.set's targets (no kind filter); arrows, arrow scale, profile flip, brush name on the existing top stroke; nothing selected: new-art template.
    r("stroke.setAdvanced", r#"{"swapArrows":true}"#, &["$R"], "selection", &[("$R", "appearance", false)]),
    r("stroke.setAdvanced", r#"{"ids":["$P"],"arrowScale":[50,200]}"#, &[], "", &[("$P", "appearance", false)]),
    r("stroke.setAdvanced", r#"{"swapArrows":true}"#, &[], "mode,selection", &[]),
    // stroke.widthProfile.add: the shown stroke's profile (active item / first selected object's painted leaf's top stroke / new art) saved to the preferences.
    r("stroke.widthProfile.add", r#"{"name":"Mine"}"#, &["$P"], "mode,selection", &[("$P", "appearance", false)]),
    r("stroke.widthProfile.add", r#"{}"#, &[], "mode,selection", &[]),
    // stroke.widthProfile.delete: a saved profile by name (prefs), else the shown stroke's.
    r("stroke.widthProfile.delete", r#"{"name":"Mine"}"#, &[], "mode", &[]),
    r("stroke.widthProfile.delete", r#"{}"#, &["$P"], "mode,selection", &[("$P", "appearance", false)]),
    // stroke.widthProfile.reset: clears the saved profiles (prefs).
    r("stroke.widthProfile.reset", r#"{}"#, &[], "mode", &[]),
    // appearance.addFill: appearance targets (target circle / selection roots / ids; contents → leaves); pushes a copy of the top fill, else text's run fill, else the session default fill.
    r("appearance.addFill", r#"{}"#, &["$R"], "mode,selection", &[("$R", "appearance", false)]),
    r("appearance.addFill", r#"{"ids":["$G"]}"#, &[], "mode", &[("$G", "appearance", false)]),
    r(
        "appearance.addFill",
        r#"{"target":"contents"}"#,
        &["$G"],
        "mode,selection",
        &[("children:$G", "-", false), ("$C1", "appearance", false), ("$C2", "appearance", false)],
    ),
    r("appearance.addFill", r#"{}"#, &["$T"], "mode,selection", &[("$T", "appearance,data", false)]),
    // appearance.addStroke: as addFill with the top stroke (weight at least the current stroke width), else the default stroke.
    r("appearance.addStroke", r#"{}"#, &["$R"], "mode,selection", &[("$R", "appearance", false)]),
    r("appearance.addStroke", r#"{"ids":["$L1"]}"#, &[], "mode", &[("$L1", "appearance", false)]),
    // appearance.clear: appearance targets; appearance set whole (group: empty), opacity and blend reset.
    r("appearance.clear", r#"{}"#, &["$R"], "selection", &[("$R", "appearance,props.opacity,props.blend", false)]),
    r("appearance.clear", r#"{"ids":["$G"]}"#, &[], "", &[("$G", "appearance,props.opacity,props.blend", false)]),
    // appearance.reduceToBasic: appearance targets; keeps the top visible fill and stroke (appearance set whole).
    r("appearance.reduceToBasic", r#"{}"#, &["$E"], "selection", &[("$E", "appearance", false)]),
    r(
        "appearance.reduceToBasic",
        r#"{"target":"contents"}"#,
        &["$G"],
        "selection",
        &[("children:$G", "-", false), ("$C1", "appearance", false), ("$C2", "appearance", false)],
    ),
    // appearance.setItem: appearance targets (no leaf expansion); item `index` of each (skipped where missing); paint via paint.setFill's parsing (colour mode / swatch).
    r("appearance.setItem", r#"{"index":0,"opacity":50}"#, &["$R"], "selection", &[("$R", "appearance", false)]),
    r(
        "appearance.setItem",
        r##"{"ids":["$R"],"index":0,"color":"#ff0000"}"##,
        &[],
        "",
        &[("$R", "appearance", false), ("drawing", "settings.color_mode", false)],
    ),
    // appearance.removeItem: appearance targets; removes items by index.
    r("appearance.removeItem", r#"{"index":0}"#, &["$R"], "selection", &[("$R", "appearance", false)]),
    r("appearance.removeItem", r#"{"ids":["$E"],"indices":[0,1]}"#, &[], "", &[("$E", "appearance", false)]),
    // appearance.addEffect: effect.apply: appearance targets, the active item's effect list or the object's; a Pathfinder effect on several loose selected objects groups them (above the topmost) first.
    r("appearance.addEffect", r#"{"effect":"stylize.roundCorners"}"#, &["$R"], "selection", &[("$R", "appearance", false)]),
    r("appearance.addEffect", r#"{"ids":["$G"],"effect":"stylize.roundCorners"}"#, &[], "", &[("$G", "appearance", false)]),
    r("appearance.addEffect", r#"{"effect":"pathfinder.add"}"#, &["$R", "$E"], "selection", &[("$R", "data.type", true), ("$E", "data.type", true)]),
    // appearance.duplicateItem: appearance targets; copies item(s) within each stack.
    r("appearance.duplicateItem", r#"{"index":0}"#, &["$R"], "selection", &[("$R", "appearance", false)]),
    // appearance.moveItem: appearance targets; reorders items; from "contents" moves a group/layer/type contents slot.
    r("appearance.moveItem", r#"{"from":0,"to":1}"#, &["$R"], "selection", &[("$R", "appearance", false)]),
    r("appearance.moveItem", r#"{"ids":["$G"],"from":"contents","to":0}"#, &[], "", &[("$G", "appearance,data.type", false)]),
    // appearance.copyFrom: source by id; targets = paint leaves of ids or the selection roots; source appearance, opacity and blend (Eyedropper prefs) onto each target, and onto the session defaults.
    r(
        "appearance.copyFrom",
        r#"{"source":"$E"}"#,
        &["$R"],
        "mode,selection",
        &[("$E", "appearance,props.opacity,props.blend", false), ("$R", "appearance,props.opacity,props.blend", false)],
    ),
    r(
        "appearance.copyFrom",
        r#"{"source":"$E","ids":["$G"]}"#,
        &[],
        "mode",
        &[
            ("$E", "appearance,props.opacity,props.blend", false),
            ("children:$G", "-", false),
            ("$C1", "appearance,props.opacity,props.blend", false),
            ("$C2", "appearance,props.opacity,props.blend", false),
        ],
    ),
    r(
        "appearance.copyFrom",
        r#"{"source":"$T"}"#,
        &["$TA"],
        "mode,selection",
        &[("$T", "data,props.opacity,props.blend", false), ("$TA", "data,props.opacity,props.blend", false)],
    ),
    r(
        "appearance.copyFrom",
        r#"{"source":"$E","reverse":true}"#,
        &["$R"],
        "mode,selection",
        &[("$R", "appearance,props.opacity,props.blend", false), ("$E", "appearance,props.opacity,props.blend", false)],
    ),
    // appearance.showAllHidden: appearance targets; every hidden item/effect made visible.
    r("appearance.showAllHidden", r#"{}"#, &["$R"], "selection", &[("$R", "appearance", false)]),
    // appearance.transfer: source and target by id; target takes source's appearance (gradients rebased on both bounds) and transparency; source cleared unless copy.
    r(
        "appearance.transfer",
        r#"{"source":"$E","target":"$R"}"#,
        &[],
        "",
        &[
            ("$E", "appearance,props.opacity,props.blend,props.isolate,props.knockout,props.knockout_shape", false),
            ("$R", "appearance,props.opacity,props.blend,props.isolate,props.knockout,props.knockout_shape,data.type", false),
        ],
    ),
    r(
        "appearance.transfer",
        r#"{"source":"$E","target":"$R","copy":true}"#,
        &[],
        "",
        &[
            ("$E", "appearance,props.opacity,props.blend,props.isolate,props.knockout,props.knockout_shape", false),
            ("$R", "appearance,props.opacity,props.blend,props.isolate,props.knockout,props.knockout_shape,data.type", false),
        ],
    ),
    // paint.editGradient: fill or stroke (stroke param / item kind / fill_active); item targets or paint leaves; edits the current paint in place (a solid becomes the default gradient); session default gradient replaced; stop swatches read.
    r("paint.editGradient", r#"{"angle":45}"#, &["$R"], "mode,selection", &[("$R", "appearance", false)]),
    r(
        "paint.editGradient",
        r##"{"ids":["$R"],"stroke":false,"stops":[{"offset":0,"swatch":"Brand"},{"offset":1,"color":"#000000"}]}"##,
        &[],
        "mode",
        &[("$R", "appearance", false), ("swatch:Brand", "*", false)],
    ),
    r("paint.editGradient", r#"{"kind":"freeform"}"#, &["$R"], "mode,selection", &[("$R", "appearance,data", false)]),
    // paint.setGradientGeom: paint leaves of ids or selection roots (or the active item's targets); sets the gradient vector/aspect/focal of the fill or stroke.
    r("paint.setGradientGeom", r#"{"start":[0,0],"end":[100,0]}"#, &["$R"], "mode,selection", &[("$R", "appearance", false)]),
    r("paint.setGradientGeom", r#"{"ids":["$R"],"stroke":false,"aspect":50}"#, &[], "", &[("$R", "appearance,data", false)]),
    // paint.sampleColor: becomes paint.setFill/Stroke {color, ids} on the active proxy; with `stop`, the selection's first object's gradient with that stop recoloured goes to paint.editGradient.
    r(
        "paint.sampleColor",
        r##"{"color":"#ff0000"}"##,
        &["$R"],
        "mode,selection",
        &[("$R", "appearance", false), ("drawing", "settings.color_mode", false)],
    ),
    r(
        "paint.sampleColor",
        r##"{"color":"#ff0000","ids":["$R"],"stroke":false}"##,
        &[],
        "",
        &[("$R", "appearance", false), ("drawing", "settings.color_mode", false)],
    ),
    r("paint.sampleColor", r##"{"color":"#ff0000","stop":0}"##, &["$R"], "mode,selection", &[("$R", "appearance", false)]),
    // edit.colors.invert: ids (non-integers dropped) else selection roots; every colour of each subtree inverted (fills, strokes, text runs, meshes, images, pattern tiles).
    r("edit.colors.invert", r#"{}"#, &["$R"], "selection", &[("$R", "appearance", false)]),
    r("edit.colors.invert", r#"{"ids":["$T"]}"#, &[], "", &[("$T", "appearance,data", false)]),
    r(
        "edit.colors.invert",
        r#"{}"#,
        &["$G"],
        "selection",
        &[("$G", "appearance", false), ("children:$G", "-", false), ("$C1", "appearance", false), ("$C2", "appearance", false)],
    ),
    // edit.colors.toCMYK: as edit.colors.invert, converting to CMYK.
    r("edit.colors.toCMYK", r#"{}"#, &["$R"], "selection", &[("$R", "appearance", false)]),
    r(
        "edit.colors.toCMYK",
        r#"{"ids":["$B"]}"#,
        &[],
        "",
        &[("$B", "appearance", false), ("children:$B", "-", false), ("$B1", "appearance", false), ("$B2", "appearance", false)],
    ),
    // edit.colors.toGrayscale: as edit.colors.invert.
    r("edit.colors.toGrayscale", r#"{}"#, &["$R"], "selection", &[("$R", "appearance", false)]),
    // edit.colors.toRGB: as edit.colors.invert.
    r("edit.colors.toRGB", r#"{}"#, &["$R"], "selection", &[("$R", "appearance", false)]),
    // edit.colors.saturate: as edit.colors.invert, scaling saturation.
    r("edit.colors.saturate", r#"{"intensity":20}"#, &["$R"], "selection", &[("$R", "appearance", false)]),
    r(
        "edit.colors.saturate",
        r#"{"intensity":20,"ids":["$CP"]}"#,
        &[],
        "",
        &[("$CP", "appearance", false), ("children:$CP", "-", false), ("$K1", "appearance", false), ("$K2", "appearance", false)],
    ),
    // edit.colors.adjustBalance: as edit.colors.invert; global/tint mode shifts swatch-linked colours to a tint of the swatch's current colour.
    r("edit.colors.adjustBalance", r#"{"mode":"rgb","r":10}"#, &["$R"], "selection", &[("$R", "appearance", false)]),
    r("edit.colors.adjustBalance", r#"{"tint":-30}"#, &["$R"], "selection", &[("$R", "appearance", false), ("swatch:Brand", "*", false)]),
    // edit.colors.blendFrontToBack: filled leaves of the selection roots in paint order; the middle ones take fills graded between the frontmost and backmost.
    r(
        "edit.colors.blendFrontToBack",
        r#"{}"#,
        &["$R", "$E", "$P"],
        "selection",
        &[("$R", "appearance", true), ("$E", "appearance", true), ("$P", "appearance", true)],
    ),
    // edit.colors.blendHorizontally: filled leaves ordered by bounds centre x.
    r(
        "edit.colors.blendHorizontally",
        r#"{}"#,
        &["$R", "$E", "$P"],
        "selection",
        &[("$R", "appearance,data", false), ("$E", "appearance,data", false), ("$P", "appearance,data", false)],
    ),
    // edit.colors.blendVertically: filled leaves ordered by bounds centre y.
    r(
        "edit.colors.blendVertically",
        r#"{}"#,
        &["$R", "$E", "$P"],
        "selection",
        &[("$R", "appearance,data", false), ("$E", "appearance,data", false), ("$P", "appearance,data", false)],
    ),
    // color.loadProfile: registers an ICC file (file system, process-wide CMS); nothing of the document.
    r("color.loadProfile", r#"{"path":"/tmp/x.icc"}"#, &[], "", &[]),
    // edit.assignProfile: the document's assigned profiles read and set whole.
    r("edit.assignProfile", r#"{"cmyk":"Generic CMYK"}"#, &[], "", &[("drawing", "settings.color_profiles", false)]),
    // edit.colors.overprintBlack: ids else selection roots, whole subtrees; black fills/strokes (colour and spot-swatch link) get the overprint flag.
    r("edit.colors.overprintBlack", r#"{}"#, &["$R"], "selection", &[("$R", "appearance", false), ("swatch:Brand", "*", false)]),
    r("edit.colors.overprintBlack", r#"{"ids":["$T"]}"#, &[], "", &[("$T", "appearance,data", false), ("swatch:Brand", "*", false)]),
    // edit.findReplace: ids → text objects among them and their descendants; else every visible unlocked text in the visible unlocked top-level layers; run texts rewritten.
    r(
        "edit.findReplace",
        r#"{"find":"Hello","replace":"Bye"}"#,
        &[],
        "",
        &[
            ("$L1", "visible,locked", false),
            ("$L2", "visible,locked", false),
            ("children:$L1", "-", false),
            ("children:$L2", "-", false),
            ("children:$B", "-", false),
            ("children:$CP", "-", false),
            ("children:$CG", "-", false),
            ("children:$G", "-", false),
            ("$ALL", "data.type", false),
            ("$T", "data,visible,locked", false),
            ("$TA", "data,visible,locked", false),
        ],
    ),
    r("edit.findReplace", r#"{"find":"Hello","ids":["$T"]}"#, &[], "", &[("$T", "data", false)]),
    // edit.pasteWithoutFormatting: edit.paste of the clipboard with its type reduced to one default run.
    r(
        "edit.pasteWithoutFormatting",
        r#"{}"#,
        &[],
        "active,mode",
        &[("$L1", "visible,locked", false), ("drawing", "settings.paste_remembers_layers", false), ("swatch:Brand", "*", false)],
    ),
    // stroke.widthPoint.set: `id` (its compound when a member); the top stroke's weight and profile; a width point placed/edited.
    // Pinned after review: `mode` from the preferences this command reads on a path the reading's rows do not take (tests/preferences.rs, READERS).
    p("stroke.widthPoint.set", r#"{"id":"$P","t":0.5,"left":4,"right":4}"#, &[], "mode", &[("$P", "appearance", true)]),
    r("stroke.widthPoint.set", r#"{"id":"$K1","t":0.5,"left":4,"right":4}"#, &[], "", &[("$K1", "-", true), ("$CP", "appearance", false)]),
    // stroke.widthPoint.remove: `id` (compound owner); removes points from the top stroke's profile.
    r("stroke.widthPoint.remove", r#"{"id":"$P","index":0}"#, &[], "", &[("$P", "appearance", true)]),
    // stroke.widthPoint.copy: `id` (compound owner); copies point `index` to t.
    r("stroke.widthPoint.copy", r#"{"id":"$P","index":0,"t":0.25}"#, &[], "", &[("$P", "appearance", true)]),
    // stroke.widthProfile.set: ids/id/selection.objects, each mapped to its compound owner; the top stroke's profile set (objects without a stroke skipped).
    r("stroke.widthProfile.set", r#"{"ids":["$P"],"points":[[0,1,1],[1,2,2]]}"#, &[], "", &[("$P", "appearance", true)]),
    r("stroke.widthProfile.set", r#"{"points":null}"#, &["$P"], "selection", &[("$P", "appearance", true)]),
    // paint.invert: active proxy (stroke param / fill_active); ids else selection roots (none: the session default); active item's paint, else the fill-or-stroke colours of each whole subtree.
    r("paint.invert", r#"{}"#, &["$R"], "mode,selection", &[("$R", "appearance", false)]),
    r(
        "paint.invert",
        r#"{"ids":["$G"],"stroke":false}"#,
        &[],
        "",
        &[("$G", "appearance", false), ("children:$G", "-", false), ("$C1", "appearance", false), ("$C2", "appearance", false)],
    ),
    r("paint.invert", r#"{}"#, &[], "mode,selection", &[]),
    // paint.complement: as paint.invert.
    r("paint.complement", r#"{}"#, &["$R"], "mode,selection", &[("$R", "appearance", false)]),
    r("paint.complement", r#"{"ids":["$T"],"stroke":false}"#, &[], "", &[("$T", "appearance,data", false)]),
    // paint.lastColor: the session's last solid colour applied as paint.setFill/Stroke (proxy from stroke / fill_active).
    r("paint.lastColor", r#"{}"#, &["$R"], "mode,selection", &[("$R", "appearance", false)]),
    r("paint.lastColor", r#"{"ids":["$R"],"stroke":true}"#, &[], "mode", &[("$R", "appearance", false)]),
    // paint.lastGradient: the session's last gradient, unplaced, applied as paint.setFill/Stroke.
    r("paint.lastGradient", r#"{}"#, &["$R"], "mode,selection", &[("$R", "appearance", false)]),
    // paint.freeform.addPoint: the freeform gradient of the targeted fill/stroke (item / paint leaves; stroke from fill_active); its bounds; `line` joins the session's selected point.
    r("paint.freeform.addPoint", r#"{"at":[10,10]}"#, &["$R"], "mode,selection", &[("$R", "appearance,data", false)]),
    r("paint.freeform.addPoint", r#"{"ids":["$R"],"stroke":false,"at":[10,10],"line":true}"#, &[], "mode", &[("$R", "appearance,data", false)]),
    // paint.freeform.setPoint: point `index` (else the session's selected point) of the freeform gradient moved/recoloured.
    r("paint.freeform.setPoint", r##"{"index":0,"color":"#ff0000"}"##, &["$R"], "mode,selection", &[("$R", "appearance,data", false)]),
    r("paint.freeform.setPoint", r##"{"color":"#ff0000"}"##, &["$R"], "mode,selection", &[("$R", "appearance,data", false)]),
    // paint.freeform.deletePoint: as setPoint, removing the point.
    r("paint.freeform.deletePoint", r#"{"index":0}"#, &["$R"], "mode,selection", &[("$R", "appearance,data", false)]),
    // paint.freeform.addLine: joins point indices of the freeform gradient.
    r("paint.freeform.addLine", r#"{"points":[0,1]}"#, &["$R"], "mode,selection", &[("$R", "appearance,data", false)]),
    // paint.freeform.splitLine: adds a point on line `line` segment `segment`.
    r("paint.freeform.splitLine", r#"{"line":0,"segment":0}"#, &["$R"], "mode,selection", &[("$R", "appearance,data", false)]),
    // paint.freeform.selectPoint: session state only; checks the index against the freeform gradient behind the active proxy of the first selected object.
    r("paint.freeform.selectPoint", r#"{"index":0}"#, &["$R"], "mode,selection", &[("$R", "appearance", false)]),
    // attribute.set: url/note/imageMap/showCenter on targets() (ids/id/selection.objects), attrs read and set whole (show-centre default by kind); overprint flags on the item / paint leaves.
    r("attribute.set", r#"{"note":"n"}"#, &["$R"], "selection", &[("$R", "props.attrs", false)]),
    r("attribute.set", r#"{"ids":["$R"],"url":"u"}"#, &[], "", &[("$R", "props.attrs", false)]),
    r("attribute.set", r#"{"showCenter":true}"#, &["$R"], "selection", &[("$R", "props.attrs,data.type", false)]),
    r(
        "attribute.set",
        r#"{"overprintFill":true}"#,
        &["$G"],
        "selection",
        &[("children:$G", "-", false), ("$C1", "appearance", false), ("$C2", "appearance", false)],
    ),
    // appearance.setNewArtBasic: the preference; toggled (read) without `on`.
    r("appearance.setNewArtBasic", r#"{"on":true}"#, &[], "", &[]),
    r("appearance.setNewArtBasic", r#"{}"#, &[], "mode", &[]),
    // file.new: makes a new document from prefs (default units, presets, Untitled-N counter, Recent); touches no open document.
    r("file.new", r#"{"width":400,"height":300}"#, &[], "mode", &[]),
    // file.close: closes document `index` (default the active one) without asking: every unsaved change of it is discarded.
    r(
        "file.close",
        r#"{}"#,
        &[],
        "active",
        &[
            ("drawing", "*", false),
            ("~swatches", "*", false),
            ("~swatchGroups", "*", false),
            ("~graphicStyles", "*", false),
            ("~charStyles", "*", false),
            ("~paraStyles", "*", false),
            ("~symbols", "*", false),
            ("~patterns", "*", false),
            ("~artboards", "*", false),
            ("$ALL", "*", false),
            ("$L1", "*", false),
            ("$L2", "*", false),
            ("children:$L1", "-", false),
            ("children:$L2", "-", false),
            ("children:$B", "-", false),
            ("children:$CP", "-", false),
            ("children:$CG", "-", false),
            ("children:$G", "-", false),
            ("$SL", "*", false),
            ("$AS", "*", false),
        ],
    ),
    r(
        "file.close",
        r#"{"index":0}"#,
        &[],
        "",
        &[
            ("drawing", "*", false),
            ("~swatches", "*", false),
            ("~swatchGroups", "*", false),
            ("~graphicStyles", "*", false),
            ("~charStyles", "*", false),
            ("~paraStyles", "*", false),
            ("~symbols", "*", false),
            ("~patterns", "*", false),
            ("~artboards", "*", false),
            ("$ALL", "*", false),
            ("$L1", "*", false),
            ("$L2", "*", false),
            ("children:$L1", "-", false),
            ("children:$L2", "-", false),
            ("children:$B", "-", false),
            ("children:$CP", "-", false),
            ("children:$CG", "-", false),
            ("children:$G", "-", false),
            ("$SL", "*", false),
            ("$AS", "*", false),
        ],
    ),
    // file.setUnits: compares and sets the document's units.
    r("file.setUnits", r#"{"units":"Millimeters"}"#, &[], "", &[("drawing", "settings.units", false)]),
    // file.open: reads a file into a new document (prefs updateLinks; Untitled-N for templates); no open document read.
    r("file.open", r#"{"path":"/tmp/in.kvector"}"#, &[], "mode", &[]),
    // file.exportForScreens: renders the chosen artboards (default all) / assets / whole document and stores the params whole as the document's export settings (not an undo step).
    r(
        "file.exportForScreens",
        r#"{}"#,
        &[],
        "",
        &[
            ("drawing", "*", false),
            ("~swatches", "*", false),
            ("~swatchGroups", "*", false),
            ("~graphicStyles", "*", false),
            ("~charStyles", "*", false),
            ("~paraStyles", "*", false),
            ("~symbols", "*", false),
            ("~patterns", "*", false),
            ("~artboards", "*", false),
            ("$ALL", "*", false),
            ("$L1", "*", false),
            ("$L2", "*", false),
            ("children:$L1", "-", false),
            ("children:$L2", "-", false),
            ("children:$B", "-", false),
            ("children:$CP", "-", false),
            ("children:$CG", "-", false),
            ("children:$G", "-", false),
            ("$SL", "*", false),
            ("$AS", "*", false),
        ],
    ),
    r(
        "file.exportForScreens",
        r#"{"assets":["$AS"]}"#,
        &[],
        "",
        &[("$AS", "*", false), ("$E", "*", false), ("drawing", "settings.export_settings,settings.setup", false)],
    ),
    r(
        "file.exportForScreens",
        r#"{"artboards":[1]}"#,
        &[],
        "",
        &[
            ("artboard:1", "*", true),
            ("$ALL", "*", false),
            ("$L1", "*", false),
            ("$L2", "*", false),
            ("children:$L1", "-", false),
            ("children:$L2", "-", false),
            ("children:$B", "-", false),
            ("children:$CP", "-", false),
            ("children:$CG", "-", false),
            ("children:$G", "-", false),
            ("drawing", "settings.export_settings,settings.title", false),
            ("~swatches", "*", false),
            ("~symbols", "*", false),
            ("~patterns", "*", false),
        ],
    ),
    // command.batch: runs its steps as one undo step; it rests on the union of what its steps rest on (here: one units step; one rename step).
    r(
        "command.batch",
        r#"{"commands":[{"command":"file.setUnits","params":{"units":"Millimeters"}}]}"#,
        &[],
        "",
        &[("drawing", "settings.units", false)],
    ),
    r("command.batch", r#"{"commands":[{"command":"layer.setProps","params":{"ids":["$R"],"name":"Box"}}]}"#, &[], "", &[("$R", "name", false)]),
    // file.save: encodes the whole active document (with session view, open Layers rows, compression pref); a save to a path stamps File Info modified (and created if none).
    r(
        "file.save",
        r#"{"path":"/tmp/out.kvector"}"#,
        &[],
        "mode",
        &[
            ("drawing", "*", false),
            ("~swatches", "*", false),
            ("~swatchGroups", "*", false),
            ("~graphicStyles", "*", false),
            ("~charStyles", "*", false),
            ("~paraStyles", "*", false),
            ("~symbols", "*", false),
            ("~patterns", "*", false),
            ("~artboards", "*", false),
            ("$ALL", "*", false),
            ("$L1", "*", false),
            ("$L2", "*", false),
            ("children:$L1", "-", false),
            ("children:$L2", "-", false),
            ("children:$B", "-", false),
            ("children:$CP", "-", false),
            ("children:$CG", "-", false),
            ("children:$G", "-", false),
            ("$SL", "*", false),
            ("$AS", "*", false),
        ],
    ),
    r(
        "file.save",
        r#"{}"#,
        &[],
        "mode",
        &[
            ("drawing", "*", false),
            ("~swatches", "*", false),
            ("~swatchGroups", "*", false),
            ("~graphicStyles", "*", false),
            ("~charStyles", "*", false),
            ("~paraStyles", "*", false),
            ("~symbols", "*", false),
            ("~patterns", "*", false),
            ("~artboards", "*", false),
            ("$ALL", "*", false),
            ("$L1", "*", false),
            ("$L2", "*", false),
            ("children:$L1", "-", false),
            ("children:$L2", "-", false),
            ("children:$B", "-", false),
            ("children:$CP", "-", false),
            ("children:$CG", "-", false),
            ("children:$G", "-", false),
            ("$SL", "*", false),
            ("$AS", "*", false),
        ],
    ),
    // file.saveAs: as file.save to a path; the document takes the new path/format.
    r(
        "file.saveAs",
        r#"{"path":"/tmp/out2.kvector"}"#,
        &[],
        "mode",
        &[
            ("drawing", "*", false),
            ("~swatches", "*", false),
            ("~swatchGroups", "*", false),
            ("~graphicStyles", "*", false),
            ("~charStyles", "*", false),
            ("~paraStyles", "*", false),
            ("~symbols", "*", false),
            ("~patterns", "*", false),
            ("~artboards", "*", false),
            ("$ALL", "*", false),
            ("$L1", "*", false),
            ("$L2", "*", false),
            ("children:$L1", "-", false),
            ("children:$L2", "-", false),
            ("children:$B", "-", false),
            ("children:$CP", "-", false),
            ("children:$CG", "-", false),
            ("children:$G", "-", false),
            ("$SL", "*", false),
            ("$AS", "*", false),
        ],
    ),
    // file.saveAsTemplate: whole document as a template copy; no date stamp, document unchanged.
    r(
        "file.saveAsTemplate",
        r#"{"path":"/tmp/t.vctemplate"}"#,
        &[],
        "mode",
        &[
            ("drawing", "*", false),
            ("~swatches", "*", false),
            ("~swatchGroups", "*", false),
            ("~graphicStyles", "*", false),
            ("~charStyles", "*", false),
            ("~paraStyles", "*", false),
            ("~symbols", "*", false),
            ("~patterns", "*", false),
            ("~artboards", "*", false),
            ("$ALL", "*", false),
            ("$L1", "*", false),
            ("$L2", "*", false),
            ("children:$L1", "-", false),
            ("children:$L2", "-", false),
            ("children:$B", "-", false),
            ("children:$CP", "-", false),
            ("children:$CG", "-", false),
            ("children:$G", "-", false),
            ("$SL", "*", false),
            ("$AS", "*", false),
        ],
    ),
    // file.newFromTemplate: reads a file into a new untitled document; no open document read.
    r("file.newFromTemplate", r#"{"path":"/tmp/t.vctemplate"}"#, &[], "mode", &[]),
    // file.revert: replaces the active document by its saved file (the document's path): every change since the save is lost.
    r(
        "file.revert",
        r#"{}"#,
        &[],
        "active,mode",
        &[
            ("drawing", "*", false),
            ("~swatches", "*", false),
            ("~swatchGroups", "*", false),
            ("~graphicStyles", "*", false),
            ("~charStyles", "*", false),
            ("~paraStyles", "*", false),
            ("~symbols", "*", false),
            ("~patterns", "*", false),
            ("~artboards", "*", false),
            ("$ALL", "*", false),
            ("$L1", "*", false),
            ("$L2", "*", false),
            ("children:$L1", "-", false),
            ("children:$L2", "-", false),
            ("children:$B", "-", false),
            ("children:$CP", "-", false),
            ("children:$CG", "-", false),
            ("children:$G", "-", false),
            ("$SL", "*", false),
            ("$AS", "*", false),
        ],
    ),
    // layer.new: a layer above the current layer at its level (top: above all); next 'Layer N' name and colour from every layer's name and the layer count.
    r("layer.new", r#"{}"#, &[], "active", &[("$L1", "name", true), ("$L2", "name", false)]),
    r("layer.new", r#"{"top":true,"name":"Top"}"#, &[], "", &[("$L1", "-", false), ("$L2", "-", false)]),
    // layer.newSublayer: parent = `parent`, else the last highlighted row's layer, else the current layer; inserted on top of it; default name/colour from all layers.
    r("layer.newSublayer", r#"{}"#, &[], "active,selection", &[("$L1", "name", false), ("$L2", "name", false)]),
    r("layer.newSublayer", r#"{"parent":"$L2","name":"Sub"}"#, &[], "", &[("$L2", "-", false), ("$L1", "-", false)]),
    // layer.delete: rows = ids|id, else highlighted rows, else the current layer; removes each with everything it holds; guides of deleted layers go.
    r("layer.delete", r#"{"ids":["$R"]}"#, &[], "", &[("$R", "*", false)]),
    r("layer.delete", r#"{"ids":["$G"]}"#, &[], "", &[("$G", "*", false), ("children:$G", "-", false), ("$C1", "*", false), ("$C2", "*", false)]),
    r(
        "layer.delete",
        r#"{"ids":["$L2"]}"#,
        &[],
        "",
        &[("$L2", "*", false), ("children:$L2", "-", false), ("$R2", "*", false), ("drawing", "guides", false)],
    ),
    r(
        "layer.delete",
        r#"{}"#,
        &[],
        "active,selection",
        &[
            ("$L1", "*", false),
            ("children:$L1", "-", false),
            ("children:$B", "-", false),
            ("children:$CP", "-", false),
            ("children:$CG", "-", false),
            ("children:$G", "-", false),
            ("$B", "*", false),
            ("$B2", "*", false),
            ("$B1", "*", false),
            ("$CP", "*", false),
            ("$K2", "*", false),
            ("$K1", "*", false),
            ("$CG", "*", false),
            ("$CA", "*", false),
            ("$CK", "*", false),
            ("$TA", "*", false),
            ("$PR", "*", false),
            ("$S", "*", false),
            ("$T", "*", false),
            ("$P", "*", false),
            ("$G", "*", false),
            ("$C2", "*", false),
            ("$C1", "*", false),
            ("$E", "*", false),
            ("$R", "*", false),
            ("drawing", "guides", false),
        ],
    ),
    // layer.duplicate: rows as layer.delete; each copied (all fields, children too) just above its original; a layer's copy is named '<name> copy' and gets copies of its guides.
    r("layer.duplicate", r#"{"ids":["$R"]}"#, &[], "", &[("$R", "*", true)]),
    r("layer.duplicate", r#"{"ids":["$G"]}"#, &[], "", &[("$G", "*", true), ("children:$G", "-", false), ("$C1", "*", false), ("$C2", "*", false)]),
    r(
        "layer.duplicate",
        r#"{"ids":["$L2"]}"#,
        &[],
        "",
        &[("$L2", "*", true), ("children:$L2", "-", false), ("$R2", "*", false), ("drawing", "guides", false)],
    ),
    // layer.setProps: rows as layer.delete; sets the given fields; template compares data.template and then also sets locked and dimImages; deselects what became hidden/locked.
    r("layer.setProps", r#"{"ids":["$R"],"name":"Box"}"#, &[], "selection", &[("$R", "name", false)]),
    r("layer.setProps", r#"{"ids":["$L2"],"template":true}"#, &[], "selection", &[("$L2", "data.template,locked,data.dim_images", false)]),
    r("layer.setProps", r#"{"visible":false}"#, &[], "active,selection", &[("$L1", "visible", false)]),
    // layer.collectInNew: ids, else highlighted rows, else the selected roots; a new layer where the topmost was (index+1 in its layer); the rows move into it in paint order; name/colour from all layers.
    r(
        "layer.collectInNew",
        r#"{"ids":["$R","$E"]}"#,
        &[],
        "",
        &[("$R", "-", true), ("$E", "-", true), ("$L1", "name", false), ("$L2", "name", false)],
    ),
    r(
        "layer.collectInNew",
        r#"{}"#,
        &["$R", "$E"],
        "selection",
        &[("$R", "-", true), ("$E", "-", true), ("$L1", "name", false), ("$L2", "name", false)],
    ),
    // layer.move: ids beside (above/below) target in target's parent or inside target at the top; a clipping parent keeps index >= 1; copy copies whole.
    // Pinned after review: the move replaces where the row is (the reading's own rule: a field it sets whole is one it rests on): a colleague's move of it would be undone, so where it is is held.
    p("layer.move", r#"{"ids":["$R"],"target":"$G","place":"above"}"#, &[], "", &[("$G", "-", true), ("$L1", "data.clip", false), ("$R", "-", true)]),
    // Pinned after review: the move replaces where the row is (the reading's own rule: a field it sets whole is one it rests on): a colleague's move of it would be undone, so where it is is held.
    p("layer.move", r#"{"ids":["$R"],"target":"$L2"}"#, &[], "", &[("$L2", "data.clip", false), ("$R", "-", true)]),
    r("layer.move", r#"{"ids":["$R"],"target":"$L2","copy":true}"#, &[], "", &[("$R", "*", false), ("$L2", "data.clip", false)]),
    // node.move: puts id at index (0 = bottom, measured after removal; absent = top) of parent (absent = top level).
    // Pinned after review: the move replaces where the row is (the reading's own rule: a field it sets whole is one it rests on): a colleague's move of it would be undone, so where it is is held.
    p("node.move", r#"{"id":"$R","parent":"$L2","index":0}"#, &[], "", &[("slot:$L2[0..0]", "-", false), ("$R", "-", true)]),
    // Pinned after review: the move replaces where the row is (the reading's own rule: a field it sets whole is one it rests on): a colleague's move of it would be undone, so where it is is held.
    p(
        "node.move",
        r#"{"id":"$R","parent":"$L1","index":3}"#,
        &[],
        "",
        &[("slot:$L1[3..3]", "-", false), ("children:$L1", "-", false), ("$R", "-", true)],
    ),
    // artboard.new: rect from params, defaults right of the last artboard with its size; name 'Artboard <count+1>'; id from all artboard ids.
    r("artboard.new", r#"{}"#, &[], "", &[("artboard:1", "rect", true), ("~artboards", "*", false)]),
    r("artboard.new", r#"{"x":0,"y":900,"width":100,"height":100,"name":"New"}"#, &[], "", &[("~artboards", "*", false)]),
    // artboard.delete: removes artboard `index` (positional) and the guides of that artboard.
    r("artboard.delete", r#"{"index":1}"#, &[], "", &[("artboard:1", "*", true), ("drawing", "guides", false)]),
    // artboard.setProps: index (default 0); rect from params over the old rect, name; a move takes its artboard guides; moveArt/scaleArt move/scale the art fully inside it (prefs moveLockedWithArtboard, scale strokes...).
    r("artboard.setProps", r#"{"index":0,"name":"Front"}"#, &[], "", &[("artboard:0", "rect,props", true), ("drawing", "guides", false)]),
    // UNSURE: which fixture objects lie fully inside Artboard 1 is geometry the fixture does not give; assumed: every top-level object of both layers.
    r(
        "artboard.setProps",
        r#"{"index":0,"x":50,"moveArt":true}"#,
        &[],
        "mode",
        &[
            ("artboard:0", "rect", true),
            ("drawing", "guides", false),
            ("$L1", "locked,visible", false),
            ("$L2", "locked,visible", false),
            ("children:$L1", "-", false),
            ("children:$L2", "-", false),
            ("$B", "data,locked,visible", false),
            ("$CP", "data,locked,visible", false),
            ("$CG", "data,locked,visible", false),
            ("$TA", "data,locked,visible", false),
            ("$PR", "data,locked,visible", false),
            ("$S", "data,locked,visible", false),
            ("$T", "data,locked,visible", false),
            ("$P", "data,locked,visible", false),
            ("$G", "data,locked,visible", false),
            ("$E", "data,locked,visible", false),
            ("$R", "data,locked,visible", false),
            ("$R2", "data,locked,visible", false),
            ("$B1", "data", false),
            ("$B2", "data", false),
            ("$K1", "data", false),
            ("$K2", "data", false),
            ("$CA", "data", false),
            ("$CK", "data", false),
            ("$C1", "data", false),
            ("$C2", "data", false),
            ("children:$B", "-", false),
            ("children:$CP", "-", false),
            ("children:$CG", "-", false),
            ("children:$G", "-", false),
        ],
    ),
    // artboard.fitToArt: bounds of the visible, non-template art (strokes, effects) → artboard.setProps of index (default 0) with that rect.
    r(
        "artboard.fitToArt",
        r#"{"index":0}"#,
        &[],
        "",
        &[
            ("artboard:0", "rect", true),
            ("$ALL", "data,appearance,visible", false),
            ("$L1", "visible,data.template", false),
            ("$L2", "visible,data.template", false),
            ("children:$L1", "-", false),
            ("children:$L2", "-", false),
            ("children:$B", "-", false),
            ("children:$CP", "-", false),
            ("children:$CG", "-", false),
            ("children:$G", "-", false),
            ("drawing", "guides", false),
        ],
    ),
    // artboard.fitToSelection: visual bounds of the selected objects → artboard.setProps of index (default 0).
    r(
        "artboard.fitToSelection",
        r#"{"index":0}"#,
        &["$R"],
        "selection",
        &[("artboard:0", "rect", true), ("$R", "data,appearance", false), ("drawing", "guides", false)],
    ),
    r(
        "artboard.fitToSelection",
        r#"{"index":1}"#,
        &["$G"],
        "selection",
        &[
            ("artboard:1", "rect", true),
            ("$G", "data,appearance", false),
            ("children:$G", "-", false),
            ("$C1", "data,appearance", false),
            ("$C2", "data,appearance", false),
            ("drawing", "guides", false),
        ],
    ),
    // layer.clippingMask.toggle: id, else the one highlighted layer/group row, else the one selected group, else the current layer; clipping → release (first child unclipped); else top child becomes the clipping path (paint dropped) and moves to the bottom.
    r(
        "layer.clippingMask.toggle",
        r#"{"id":"$G"}"#,
        &[],
        "",
        &[("$G", "data.clip", false), ("children:$G", "-", false), ("$C2", "appearance,data", true)],
    ),
    r(
        "layer.clippingMask.toggle",
        r#"{}"#,
        &["$G"],
        "selection",
        &[("$G", "data.clip", false), ("children:$G", "-", false), ("$C2", "appearance,data", true)],
    ),
    r("layer.clippingMask.toggle", r#"{"id":"$CG"}"#, &[], "", &[("$CG", "data.clip", false), ("children:$CG", "-", false), ("$CK", "data", false)]),
    // layer.pasteRemembersLayers: toggles (or sets) the document option, comparing with its current value.
    r("layer.pasteRemembersLayers", r#"{}"#, &[], "", &[("drawing", "settings.paste_remembers_layers", false)]),
    r("layer.pasteRemembersLayers", r#"{"on":true}"#, &[], "", &[("drawing", "settings.paste_remembers_layers", false)]),
    // layer.merge: rows (ids, else highlighted); into = last layer listed; the other layers' children and other rows move into it sorted by document stacking order; emptied layers deleted; lost guides rehomed to it.
    r(
        "layer.merge",
        r#"{"ids":["$L2","$L1"]}"#,
        &[],
        "",
        &[
            ("$L2", "*", false),
            ("children:$L2", "-", false),
            ("$R2", "-", false),
            ("$L1", "-", false),
            ("children:$L1", "-", false),
            ("drawing", "guides", false),
        ],
    ),
    r(
        "layer.merge",
        r#"{"ids":["$R","$L2"]}"#,
        &[],
        "",
        &[("$R", "-", true), ("$L2", "-", false), ("children:$L2", "-", false), ("drawing", "guides", false)],
    ),
    // layer.flatten: into = top-level layer of id, else of the last highlighted row, else the current layer; other visible non-template top-level layers merge into it, hidden ones are deleted (with their guides).
    r(
        "layer.flatten",
        r#"{}"#,
        &[],
        "active,selection",
        &[
            ("$L1", "-", false),
            ("children:$L1", "-", false),
            ("$L2", "*", false),
            ("children:$L2", "-", false),
            ("$R2", "-", false),
            ("drawing", "guides", false),
        ],
    ),
    r(
        "layer.flatten",
        r#"{"id":"$L2"}"#,
        &[],
        "selection",
        &[
            ("$L2", "-", false),
            ("children:$L2", "-", false),
            ("$L1", "*", false),
            ("children:$L1", "-", false),
            ("$B", "-", false),
            ("$CP", "-", false),
            ("$CG", "-", false),
            ("$TA", "-", false),
            ("$PR", "-", false),
            ("$S", "-", false),
            ("$T", "-", false),
            ("$P", "-", false),
            ("$G", "-", false),
            ("$E", "-", false),
            ("$R", "-", false),
            ("drawing", "guides", false),
        ],
    ),
    // layer.releaseToLayers: id (layer or group), else the one highlighted row, else the current layer; each non-layer child into its own new sublayer (a group's into its layer where the group was, the group deleted); names/colours from all layers.
    r(
        "layer.releaseToLayers",
        r#"{"id":"$G"}"#,
        &[],
        "",
        &[("$G", "*", true), ("children:$G", "-", false), ("$C1", "-", false), ("$C2", "-", false), ("$L1", "name", false), ("$L2", "name", false)],
    ),
    r(
        "layer.releaseToLayers",
        r#"{}"#,
        &[],
        "active,selection",
        &[
            ("children:$L1", "-", false),
            ("$B", "-", false),
            ("$CP", "-", false),
            ("$CG", "-", false),
            ("$TA", "-", false),
            ("$PR", "-", false),
            ("$S", "-", false),
            ("$T", "-", false),
            ("$P", "-", false),
            ("$G", "-", false),
            ("$E", "-", false),
            ("$R", "-", false),
            ("$L1", "name", false),
            ("$L2", "name", false),
        ],
    ),
    // layer.releaseToLayersBuild: layer.releaseToLayers with build: each sublayer holds copies of the objects up to its own; the top keeps the originals.
    r(
        "layer.releaseToLayersBuild",
        r#"{"id":"$G"}"#,
        &[],
        "",
        &[("$G", "*", true), ("children:$G", "-", false), ("$C1", "*", false), ("$C2", "-", false), ("$L1", "name", false), ("$L2", "name", false)],
    ),
    // layer.reverse: rows (ids, else highlighted, else current layer) grouped by parent; their slots are reversed.
    r("layer.reverse", r#"{"ids":["$R","$E"]}"#, &[], "", &[("$R", "-", true), ("$E", "-", true)]),
    r("layer.reverse", r#"{"ids":["$L1","$L2"]}"#, &[], "", &[("$L1", "-", true), ("$L2", "-", true)]),
    // layer.template: rows (ids, else highlighted, else current layer) that are layers; on = given, else toggle of the first's template; then layer.setProps template (locks, dims).
    r("layer.template", r#"{"ids":["$L2"]}"#, &[], "selection", &[("$L2", "data.template,locked,data.dim_images", false)]),
    r("layer.template", r#"{"on":true}"#, &[], "active,selection", &[("$L1", "data.template,locked,data.dim_images", false)]),
    // layer.hideOthers: hides every top-level layer but those holding the rows (ids, else highlighted, else current layer); compares visible first; deselects hidden art.
    r("layer.hideOthers", r#"{}"#, &[], "active,selection", &[("$L1", "-", false), ("$L2", "visible", false)]),
    r("layer.hideOthers", r#"{"ids":["$R2"]}"#, &[], "selection", &[("$R2", "-", true), ("$L1", "visible", false), ("$L2", "-", false)]),
    // layer.showAll: every layer and sublayer whose visible differs is shown.
    r("layer.showAll", r#"{}"#, &[], "selection", &[("$L1", "visible", false), ("$L2", "visible", false)]),
    // layer.outlineOthers: Preview off for every top-level layer but those holding the rows.
    r("layer.outlineOthers", r#"{}"#, &[], "active,selection", &[("$L1", "-", false), ("$L2", "data.preview", false)]),
    // layer.previewAll: Preview on for every layer and sublayer.
    r("layer.previewAll", r#"{}"#, &[], "selection", &[("$L1", "data.preview", false), ("$L2", "data.preview", false)]),
    // layer.lockOthers: locks every top-level layer but those holding the rows; deselects locked art.
    r("layer.lockOthers", r#"{}"#, &[], "active,selection", &[("$L1", "-", false), ("$L2", "locked", false)]),
    // layer.unlockAll: unlocks every layer and sublayer.
    r("layer.unlockAll", r#"{}"#, &[], "selection", &[("$L1", "locked", false), ("$L2", "locked", false)]),
    // artboard.move: artboard index moves by dx,dy with its guides; moveArt moves the art fully inside it (prefs moveLockedWithArtboard, scale strokes); copy adds a copy instead.
    r("artboard.move", r#"{"index":0,"dx":10,"dy":0}"#, &[], "", &[("artboard:0", "rect", true), ("drawing", "guides", false)]),
    r(
        "artboard.move",
        r#"{"index":0,"dx":10,"dy":0,"copy":true}"#,
        &[],
        "",
        &[("artboard:0", "*", true), ("~artboards", "*", false), ("drawing", "guides", false)],
    ),
    // UNSURE: art fully inside Artboard 1 assumed to be every top-level object (geometry not in the fixture).
    r(
        "artboard.move",
        r#"{"index":0,"dx":10,"dy":0,"moveArt":true}"#,
        &[],
        "mode",
        &[
            ("artboard:0", "rect", true),
            ("drawing", "guides", false),
            ("$L1", "locked,visible", false),
            ("$L2", "locked,visible", false),
            ("children:$L1", "-", false),
            ("children:$L2", "-", false),
            ("$B", "data,locked,visible", false),
            ("$CP", "data,locked,visible", false),
            ("$CG", "data,locked,visible", false),
            ("$TA", "data,locked,visible", false),
            ("$PR", "data,locked,visible", false),
            ("$S", "data,locked,visible", false),
            ("$T", "data,locked,visible", false),
            ("$P", "data,locked,visible", false),
            ("$G", "data,locked,visible", false),
            ("$E", "data,locked,visible", false),
            ("$R", "data,locked,visible", false),
            ("$R2", "data,locked,visible", false),
            ("$B1", "data", false),
            ("$B2", "data", false),
            ("$K1", "data", false),
            ("$K2", "data", false),
            ("$CA", "data", false),
            ("$CK", "data", false),
            ("$C1", "data", false),
            ("$C2", "data", false),
            ("children:$B", "-", false),
            ("children:$CP", "-", false),
            ("children:$CG", "-", false),
            ("children:$G", "-", false),
        ],
    ),
    // artboard.convertToArtboards: each selected root's geometric bounds becomes a new artboard ('Artboard <id>', id from all artboards); the objects are deleted.
    r("artboard.convertToArtboards", r#"{}"#, &["$R"], "selection", &[("$R", "*", false), ("~artboards", "*", false)]),
    r(
        "artboard.convertToArtboards",
        r#"{}"#,
        &["$G"],
        "selection",
        &[("$G", "*", false), ("children:$G", "-", false), ("$C1", "*", false), ("$C2", "*", false), ("~artboards", "*", false)],
    ),
    // artboard.rearrange: lays out all artboards in order from the first's origin; moveArtwork (default true) moves each top-level layer's direct children whose centre lies in an artboard (prefs moveLockedWithArtboard); artboard guides follow.
    // UNSURE: which artboard each object's centre lies in is geometry the fixture does not give.
    r(
        "artboard.rearrange",
        r#"{}"#,
        &[],
        "mode",
        &[
            ("~artboards", "*", false),
            ("drawing", "guides", false),
            ("$L1", "locked,visible", false),
            ("$L2", "locked,visible", false),
            ("children:$L1", "-", false),
            ("children:$L2", "-", false),
            ("$B", "data,locked,visible", false),
            ("$CP", "data,locked,visible", false),
            ("$CG", "data,locked,visible", false),
            ("$TA", "data,locked,visible", false),
            ("$PR", "data,locked,visible", false),
            ("$S", "data,locked,visible", false),
            ("$T", "data,locked,visible", false),
            ("$P", "data,locked,visible", false),
            ("$G", "data,locked,visible", false),
            ("$E", "data,locked,visible", false),
            ("$R", "data,locked,visible", false),
            ("$R2", "data,locked,visible", false),
            ("$B1", "data", false),
            ("$B2", "data", false),
            ("$K1", "data", false),
            ("$K2", "data", false),
            ("$CA", "data", false),
            ("$CK", "data", false),
            ("$C1", "data", false),
            ("$C2", "data", false),
            ("children:$B", "-", false),
            ("children:$CP", "-", false),
            ("children:$CG", "-", false),
            ("children:$G", "-", false),
        ],
    ),
    r("artboard.rearrange", r#"{"moveArtwork":false}"#, &[], "", &[("~artboards", "*", false), ("drawing", "guides", false)]),
    // file.rasterEffectsSettings: copies ppi and the raster settings, applies the given keys, writes both back whole when they differ; colorModel reads the colour mode. No params: query.
    // Pinned after review: C8-vector-engine-2: rasterfx.rs:114-125 now writes only the keys given (resolution: settings.raster_effects_ppi).
    p("file.rasterEffectsSettings", r#"{"resolution":"high"}"#, &[], "", &[("drawing", "settings.raster_effects_ppi", false)]),
    // Pinned after review: C8-vector-engine-2: rasterfx.rs:114-126 now writes only the keys given (colorModel, checked against the colour mode).
    p(
        "file.rasterEffectsSettings",
        r#"{"colorModel":"grayscale"}"#,
        &[],
        "",
        &[("drawing", "settings.raster_effects.colorModel,settings.color_mode", false)],
    ),
    // view.saved.new: appends a view; default name 'View N' from the existing names; at most 25.
    r("view.saved.new", r#"{"center":[100,200],"zoom":2.0}"#, &[], "", &[("drawing", "settings.views", false)]),
    // view.saved.edit: by name: rename or delete (position looked up before the edit); no params: list.
    // UNSURE: the fixture has no saved views: on it this row fails with 'no view named'.
    r("view.saved.edit", r#"{"name":"View 1","newName":"Overview"}"#, &[], "", &[("drawing", "settings.views", false)]),
    r("view.saved.edit", r#"{}"#, &[], "", &[]),
    // select.save: the selected objects' ids under a name (default 'Selection N' from existing names; an existing name is replaced).
    r(
        "select.save",
        r#"{"name":"Pair"}"#,
        &["$R", "$E"],
        "selection",
        &[("drawing", "settings.saved_selections", false), ("$R", "-", false), ("$E", "-", false)],
    ),
    // view.guides.make: selected roots; every non-clipping path in them (groups recursed) gets the guide flag; selection cleared.
    r("view.guides.make", r#"{}"#, &["$P"], "selection", &[("$P", "data", false)]),
    r(
        "view.guides.make",
        r#"{}"#,
        &["$G"],
        "selection",
        &[("$G", "-", false), ("children:$G", "-", false), ("$C1", "data", false), ("$C2", "data", false)],
    ),
    // view.guides.release: selected guide paths, else every editable guide path in the document, back to paths.
    // UNSURE: the fixture has no guide paths: this row fails with 'no guides'; reading: a scan of every path's guide flag and its editability.
    r(
        "view.guides.release",
        r#"{}"#,
        &[],
        "selection",
        &[("$ALL", "data,visible,locked", false), ("$L1", "visible,locked", false), ("$L2", "visible,locked", false)],
    ),
    // view.guides.clear: deletes every ruler guide and every guide path (found by scanning all objects).
    r(
        "view.guides.clear",
        r#"{}"#,
        &[],
        "",
        &[
            ("drawing", "guides", false),
            ("$ALL", "data", false),
            ("children:$L1", "-", false),
            ("children:$L2", "-", false),
            ("children:$B", "-", false),
            ("children:$CP", "-", false),
            ("children:$CG", "-", false),
            ("children:$G", "-", false),
        ],
    ),
    // guide.add: appends a guide; artboard index → that artboard's id; layer = given, else the insertion layer (isolation / current layer if shown and unlocked, else the default layer).
    r("guide.add", r#"{"vertical":true,"pos":200}"#, &[], "active", &[("drawing", "guides", false), ("$L1", "visible,locked", false)]),
    r(
        "guide.add",
        r#"{"vertical":false,"pos":50,"artboard":1,"layer":"$L2"}"#,
        &[],
        "",
        &[("drawing", "guides", false), ("artboard:1", "-", true), ("$L2", "-", false)],
    ),
    // guide.select: selects guides by positional index (selection only).
    // Pinned after review: without `indexes` the command reads the guides selected now (the earlier table check, tests/table_check.rs).
    p("guide.select", r#"{"indexes":[0]}"#, &[], "selection", &[("drawing", "guides", false)]),
    // guide.remove: guide index (positional), else the selected guides; removed from the list.
    r("guide.remove", r#"{"index":0}"#, &[], "", &[("drawing", "guides", false)]),
    // guide.move: index → pos set; else the selected guides moved (copy appends copies).
    r("guide.move", r#"{"index":0,"pos":150}"#, &[], "", &[("drawing", "guides", false)]),
    // UNSURE: the selected-guides path needs a guide selection, which `on` cannot give; on the fixture it fails.
    r("guide.move", r#"{"dx":10}"#, &[], "selection", &[("drawing", "guides", false)]),
    // guide.setLayer: guide index / indexes, else the selected guides, onto layer `layer` (a layer id).
    r("guide.setLayer", r#"{"index":0,"layer":"$L1"}"#, &[], "", &[("drawing", "guides", false), ("$L1", "-", false)]),
    // file.closeAll: closes every open document without asking.
    r(
        "file.closeAll",
        r#"{}"#,
        &[],
        "",
        &[
            ("drawing", "*", false),
            ("~swatches", "*", false),
            ("~swatchGroups", "*", false),
            ("~graphicStyles", "*", false),
            ("~charStyles", "*", false),
            ("~paraStyles", "*", false),
            ("~symbols", "*", false),
            ("~patterns", "*", false),
            ("~artboards", "*", false),
            ("$ALL", "*", false),
            ("$L1", "*", false),
            ("$L2", "*", false),
            ("children:$L1", "-", false),
            ("children:$L2", "-", false),
            ("children:$B", "-", false),
            ("children:$CP", "-", false),
            ("children:$CG", "-", false),
            ("children:$G", "-", false),
            ("$SL", "*", false),
            ("$AS", "*", false),
        ],
    ),
    // file.documentColorMode: compares the colour mode; sets it; convert (default) maps every colour of art, layers, symbols, pattern art and swatches through the session's colour settings.
    r(
        "file.documentColorMode",
        r#"{"mode":"cmyk"}"#,
        &[],
        "mode",
        &[
            ("drawing", "settings.color_mode", false),
            ("$ALL", "appearance,data", false),
            ("$L1", "appearance", false),
            ("$L2", "appearance", false),
            ("~swatches", "*", false),
            ("~symbols", "*", false),
            ("~patterns", "*", false),
        ],
    ),
    r("file.documentColorMode", r#"{"mode":"cmyk","convert":false}"#, &[], "", &[("drawing", "settings.color_mode", false)]),
    // file.setInfo: copies title and metadata, applies the given fields, writes both back whole when they differ. No params: query.
    // Pinned after review: C8-vector-engine-2: fileinfo.rs:58 now writes only the keys given (author: settings.metadata.author).
    p("file.setInfo", r#"{"author":"Ann"}"#, &[], "", &[("drawing", "settings.metadata.author", false)]),
    // artboard.reorder: removes artboard index (positional) and inserts it at `to` (clamped) in the artboards' list.
    r("artboard.reorder", r#"{"index":1,"to":0}"#, &[], "", &[("artboard:1", "*", true), ("~artboards", "*", false)]),
    // artboard.duplicate: index, else the Artboard tool's active artboard (else 0); art, else the tool's moveArt option (default true); a copy right of the rightmost artboard, named '<name>' or '<name> copy…' by the names taken, with copies of its guides and of the art fully inside it (each just above its original).
    r(
        "artboard.duplicate",
        r#"{"index":0,"art":false}"#,
        &[],
        "",
        &[("artboard:0", "*", true), ("~artboards", "*", false), ("drawing", "guides", false)],
    ),
    // UNSURE: art fully inside Artboard 1 assumed to be every top-level object (geometry not in the fixture); the tool's active artboard and moveArt are session state.
    r(
        "artboard.duplicate",
        r#"{}"#,
        &[],
        "mode",
        &[
            ("artboard:0", "*", true),
            ("~artboards", "*", false),
            ("drawing", "guides", false),
            ("$L1", "locked,visible", false),
            ("$L2", "locked,visible", false),
            ("children:$L1", "-", false),
            ("children:$L2", "-", false),
            ("$B", "*", true),
            ("$CP", "*", true),
            ("$CG", "*", true),
            ("$TA", "*", true),
            ("$PR", "*", true),
            ("$S", "*", true),
            ("$T", "*", true),
            ("$P", "*", true),
            ("$G", "*", true),
            ("$E", "*", true),
            ("$R", "*", true),
            ("$R2", "*", true),
            ("$B1", "*", false),
            ("$B2", "*", false),
            ("$K1", "*", false),
            ("$K2", "*", false),
            ("$CA", "*", false),
            ("$CK", "*", false),
            ("$C1", "*", false),
            ("$C2", "*", false),
            ("children:$B", "-", false),
            ("children:$CP", "-", false),
            ("children:$CG", "-", false),
            ("children:$G", "-", false),
        ],
    ),
    // artboard.copy: index/art as artboard.duplicate; the artboard (whole) and the art fully inside it onto the clipboard; the document is not written.
    // Pinned after review: `mode` from the preferences this command reads on a path the reading's rows do not take (tests/preferences.rs, READERS).
    p("artboard.copy", r#"{"index":0,"art":false}"#, &[], "mode", &[("artboard:0", "*", true)]),
    // artboard.cut: artboard.copy, then deletes the artboard, its guides and that art (not the only artboard).
    // Pinned after review: `mode` from the preferences this command reads on a path the reading's rows do not take (tests/preferences.rs, READERS).
    p("artboard.cut", r#"{"index":1,"art":false}"#, &[], "mode", &[("artboard:1", "*", true), ("drawing", "guides", false)]),
    // file.exportPdf: encodes the whole document (preset from prefs; preserveEditing carries the native document); no write to the document.
    r(
        "file.exportPdf",
        r#"{"path":"/tmp/o.pdf"}"#,
        &[],
        "mode",
        &[
            ("drawing", "*", false),
            ("~swatches", "*", false),
            ("~swatchGroups", "*", false),
            ("~graphicStyles", "*", false),
            ("~charStyles", "*", false),
            ("~paraStyles", "*", false),
            ("~symbols", "*", false),
            ("~patterns", "*", false),
            ("~artboards", "*", false),
            ("$ALL", "*", false),
            ("$L1", "*", false),
            ("$L2", "*", false),
            ("children:$L1", "-", false),
            ("children:$L2", "-", false),
            ("children:$B", "-", false),
            ("children:$CP", "-", false),
            ("children:$CG", "-", false),
            ("children:$G", "-", false),
            ("$SL", "*", false),
            ("$AS", "*", false),
        ],
    ),
    // file.setup: copies setup and units, applies the given keys, writes both back whole; script size changes restamp text runs using them and character styles.
    // Pinned after review: C8-vector-engine-2: docsetup.rs:72-74 now writes only the keys given (bleed: settings.setup.bleed).
    p("file.setup", r#"{"bleed":3}"#, &[], "", &[("drawing", "settings.setup.bleed", false)]),
    r(
        "file.setup",
        r#"{"superscript":{"size":50}}"#,
        &[],
        "",
        &[("drawing", "settings.setup,settings.units", false), ("$T", "data", false), ("$TA", "data", false), ("~charStyles", "*", false)],
    ),
    // Pinned after review: C8-vector-engine-2: docsetup.rs:72-74 now writes only the keys given (settings.setup.flattenerPreset).
    p("file.setup", r#"{"flattenerPreset":"Low Resolution"}"#, &[], "mode", &[("drawing", "settings.setup.flattenerPreset", false)]),
    // file.newPresets.save: preset into the preferences (from a preset or default settings); no document.
    r("file.newPresets.save", r#"{"name":"Mine","width":300,"height":300}"#, &[], "mode", &[]),
    // file.newPresets.delete: removes a saved preset from the preferences by name.
    r("file.newPresets.delete", r#"{"name":"Mine"}"#, &[], "mode", &[]),
    // file.place: the file's art at the first artboard's centre (or at/rect), on top of the insertion layer (current layer if shown/unlocked, else default); vector art adopts resources by name; replace swaps the one selected object at its place; template makes a template layer below the current layer.
    r("file.place", r#"{"path":"/tmp/a.png"}"#, &[], "active", &[("artboard:0", "rect", true), ("$L1", "visible,locked", false)]),
    r(
        "file.place",
        r#"{"path":"/tmp/a.svg","at":[100,100]}"#,
        &[],
        "active",
        &[("$L1", "visible,locked", false), ("~swatches", "*", false), ("~symbols", "*", false), ("~patterns", "*", false)],
    ),
    r("file.place", r#"{"path":"/tmp/a.png","replace":true}"#, &["$R"], "selection", &[("$R", "*", true)]),
    r(
        "file.place",
        r#"{"path":"/tmp/a.png","template":true}"#,
        &[],
        "active",
        &[("artboard:0", "rect", true), ("$L1", "-", true), ("$L2", "-", false)],
    ),
    // file.exportDxf: file.export as DXF of the whole document (one drawing per artboard by default); selectedOnly: the selected roots in their layers.
    r(
        "file.exportDxf",
        r#"{"path":"/tmp/o.dxf"}"#,
        &[],
        "",
        &[
            ("drawing", "*", false),
            ("~swatches", "*", false),
            ("~swatchGroups", "*", false),
            ("~graphicStyles", "*", false),
            ("~charStyles", "*", false),
            ("~paraStyles", "*", false),
            ("~symbols", "*", false),
            ("~patterns", "*", false),
            ("~artboards", "*", false),
            ("$ALL", "*", false),
            ("$L1", "*", false),
            ("$L2", "*", false),
            ("children:$L1", "-", false),
            ("children:$L2", "-", false),
            ("children:$B", "-", false),
            ("children:$CP", "-", false),
            ("children:$CG", "-", false),
            ("children:$G", "-", false),
            ("$SL", "*", false),
            ("$AS", "*", false),
        ],
    ),
    r(
        "file.exportDxf",
        r#"{"path":"/tmp/o.dxf","selectedOnly":true}"#,
        &["$R"],
        "selection",
        &[("$R", "*", false), ("$L1", "*", false), ("~artboards", "*", false), ("drawing", "settings.units", false)],
    ),
    // file.exportEps: file.export as EPS of the whole document (carries the native document); prefs flattener presets.
    r(
        "file.exportEps",
        r#"{"path":"/tmp/o.eps"}"#,
        &[],
        "mode",
        &[
            ("drawing", "*", false),
            ("~swatches", "*", false),
            ("~swatchGroups", "*", false),
            ("~graphicStyles", "*", false),
            ("~charStyles", "*", false),
            ("~paraStyles", "*", false),
            ("~symbols", "*", false),
            ("~patterns", "*", false),
            ("~artboards", "*", false),
            ("$ALL", "*", false),
            ("$L1", "*", false),
            ("$L2", "*", false),
            ("children:$L1", "-", false),
            ("children:$L2", "-", false),
            ("children:$B", "-", false),
            ("children:$CP", "-", false),
            ("children:$CG", "-", false),
            ("children:$G", "-", false),
            ("$SL", "*", false),
            ("$AS", "*", false),
        ],
    ),
    // file.package: the document as it is in memory (needs a saved path) with its linked files and fonts.
    // UNSURE: fixture document is never saved: the command fails ('save the document first').
    r(
        "file.package",
        r#"{"folder":"/tmp/pkg"}"#,
        &[],
        "mode",
        &[
            ("drawing", "*", false),
            ("~swatches", "*", false),
            ("~swatchGroups", "*", false),
            ("~graphicStyles", "*", false),
            ("~charStyles", "*", false),
            ("~paraStyles", "*", false),
            ("~symbols", "*", false),
            ("~patterns", "*", false),
            ("~artboards", "*", false),
            ("$ALL", "*", false),
            ("$L1", "*", false),
            ("$L2", "*", false),
            ("children:$L1", "-", false),
            ("children:$L2", "-", false),
            ("children:$B", "-", false),
            ("children:$CP", "-", false),
            ("children:$CG", "-", false),
            ("children:$G", "-", false),
            ("$SL", "*", false),
            ("$AS", "*", false),
        ],
    ),
    // print.setup: the stored print settings with `settings` over them, stored whole when they differ. No settings: query.
    // Pinned after review: C8-vector-engine-2: print.rs:92-98 now stores only the settings given over the stored ones (settings.print_setup.copies).
    p("print.setup", r#"{"settings":{"copies":2}}"#, &[], "", &[("drawing", "settings.print_setup.copies", false)]),
    // file.print: prints the whole document with the stored print settings and `settings` over them; prefs flattener presets.
    r(
        "file.print",
        r#"{"path":"/tmp/p.pdf"}"#,
        &[],
        "mode",
        &[
            ("drawing", "*", false),
            ("~swatches", "*", false),
            ("~swatchGroups", "*", false),
            ("~graphicStyles", "*", false),
            ("~charStyles", "*", false),
            ("~paraStyles", "*", false),
            ("~symbols", "*", false),
            ("~patterns", "*", false),
            ("~artboards", "*", false),
            ("$ALL", "*", false),
            ("$L1", "*", false),
            ("$L2", "*", false),
            ("children:$L1", "-", false),
            ("children:$L2", "-", false),
            ("children:$B", "-", false),
            ("children:$CP", "-", false),
            ("children:$CG", "-", false),
            ("children:$G", "-", false),
            ("$SL", "*", false),
            ("$AS", "*", false),
        ],
    ),
    // file.recovery.save: a recovery copy of every modified open document (prefs recoveryOffForComplex); document unchanged.
    r(
        "file.recovery.save",
        r#"{}"#,
        &[],
        "mode",
        &[
            ("drawing", "*", false),
            ("~swatches", "*", false),
            ("~swatchGroups", "*", false),
            ("~graphicStyles", "*", false),
            ("~charStyles", "*", false),
            ("~paraStyles", "*", false),
            ("~symbols", "*", false),
            ("~patterns", "*", false),
            ("~artboards", "*", false),
            ("$ALL", "*", false),
            ("$L1", "*", false),
            ("$L2", "*", false),
            ("children:$L1", "-", false),
            ("children:$L2", "-", false),
            ("children:$B", "-", false),
            ("children:$CP", "-", false),
            ("children:$CG", "-", false),
            ("children:$G", "-", false),
            ("$SL", "*", false),
            ("$AS", "*", false),
        ],
    ),
    // file.recovery.restore: opens copies from the recovery store as new documents; no open document read.
    r("file.recovery.restore", r#"{}"#, &[], "", &[]),
    // file.recovery.discard: deletes copies from the recovery store.
    r("file.recovery.discard", r#"{}"#, &[], "", &[]),
    // file.exportForWeb: renders artboard 0 (default clipToArtboard) and its slices, File Info as metadata; no write to the document.
    r(
        "file.exportForWeb",
        r#"{"path":"/tmp/w.gif"}"#,
        &[],
        "",
        &[
            ("artboard:0", "rect", true),
            ("$ALL", "*", false),
            ("$L1", "*", false),
            ("$L2", "*", false),
            ("children:$L1", "-", false),
            ("children:$L2", "-", false),
            ("children:$B", "-", false),
            ("children:$CP", "-", false),
            ("children:$CG", "-", false),
            ("children:$G", "-", false),
            ("$SL", "*", false),
            ("drawing", "settings.metadata,settings.title,settings.slices_clip_to_artboard", false),
            ("~swatches", "*", false),
            ("~symbols", "*", false),
            ("~patterns", "*", false),
        ],
    ),
    r(
        "file.exportForWeb",
        r#"{"path":"/tmp/w.gif","slices":"selected"}"#,
        &["$R"],
        "selection",
        &[
            ("artboard:0", "rect", true),
            ("$ALL", "*", false),
            ("$L1", "*", false),
            ("$L2", "*", false),
            ("children:$L1", "-", false),
            ("children:$L2", "-", false),
            ("children:$B", "-", false),
            ("children:$CP", "-", false),
            ("children:$CG", "-", false),
            ("children:$G", "-", false),
            ("$SL", "*", false),
            ("drawing", "settings.metadata,settings.title", false),
        ],
    ),
    // css.export: CSS of ids/id, else the selection (scope selection), or of every layer (scope all): name, kind/geometry, paint, opacity, blend, effects of each object.
    // UNSURE: which non-appearance props CSS reads (opacity, blend) is my best reading of css_rules, which lives in another crate.
    r("css.export", r#"{"path":"/tmp/s.css"}"#, &["$R"], "selection", &[("$R", "name,data,appearance,props.opacity,props.blend", false)]),
    r(
        "css.export",
        r#"{"path":"/tmp/s.css","ids":["$G"]}"#,
        &[],
        "",
        &[
            ("$G", "name,data,appearance,props.opacity,props.blend", false),
            ("children:$G", "-", false),
            ("$C1", "name,data,appearance,props.opacity,props.blend", false),
            ("$C2", "name,data,appearance,props.opacity,props.blend", false),
        ],
    ),
    // print.tiling.set: origin relative to the printed artboard containing it (or `artboard`, else the first) stored in the print settings, written back whole; reset clears it.
    // Pinned after review: C8-vector-engine-2: printtiling.rs:104-108 writes only tileOrigin; the point is placed in the printed region the layout settings (print_regions), the setup and the artboards decide.
    p(
        "print.tiling.set",
        r#"{"origin":[0,0]}"#,
        &[],
        "",
        &[
            (
                "drawing",
                "settings.print_setup.tileOrigin,settings.print_setup.artboards,settings.print_setup.range,settings.print_setup.skipBlank,settings.print_setup.media,settings.print_setup.width,settings.print_setup.height,settings.print_setup.orientation,settings.print_setup.autoRotate,settings.print_setup.transverse,settings.print_setup.printLayers,settings.print_setup.placement,settings.print_setup.scaling,settings.print_setup.scale,settings.print_setup.overlap,settings.print_setup.tileRange,settings.print_setup.margin,settings.print_setup.marks,settings.print_setup.bleed,settings.setup",
                false,
            ),
            ("~artboards", "*", false),
        ],
    ),
    // Pinned after review: C8-vector-engine-2: printtiling.rs:104-108 writes only tileOrigin; the point is placed in the printed region the layout settings and the setup decide.
    p(
        "print.tiling.set",
        r#"{"origin":[0,0],"artboard":1}"#,
        &[],
        "",
        &[
            ("artboard:1", "rect", true),
            (
                "drawing",
                "settings.print_setup.tileOrigin,settings.print_setup.artboards,settings.print_setup.range,settings.print_setup.skipBlank,settings.print_setup.media,settings.print_setup.width,settings.print_setup.height,settings.print_setup.orientation,settings.print_setup.autoRotate,settings.print_setup.transverse,settings.print_setup.printLayers,settings.print_setup.placement,settings.print_setup.scaling,settings.print_setup.scale,settings.print_setup.overlap,settings.print_setup.tileRange,settings.print_setup.margin,settings.print_setup.marks,settings.print_setup.bleed,settings.setup",
                false,
            ),
        ],
    ),
    // Pinned after review: C8-vector-engine-2: printtiling.rs:104-108 writes only tileOrigin.
    p("print.tiling.set", r#"{"reset":true}"#, &[], "", &[("drawing", "settings.print_setup.tileOrigin", false)]),
    // plugin.run: plug-in by its string id; ids (non-integers dropped), else the selection; filter targets are the visible, unlocked, non-guide, non-clipping paths/compounds in them (groups and layers recursed, hidden/locked subtrees skipped); the plug-in replaces/deletes/adds objects (new ones into the insertion layer).
    r(
        "plugin.run",
        r#"{"id":"example.filter","ids":["$R"]}"#,
        &[],
        "active,mode",
        &[("$R", "*", false), ("$L1", "visible,locked", false), ("~artboards", "*", false)],
    ),
    r(
        "plugin.run",
        r#"{"id":"example.filter"}"#,
        &["$G"],
        "active,mode,selection",
        &[
            ("$G", "visible,locked", false),
            ("children:$G", "-", false),
            ("$C1", "*", false),
            ("$C2", "*", false),
            ("$L1", "visible,locked", false),
            ("~artboards", "*", false),
        ],
    ),
    // text.add: new type at top of the insertion parent (isolation, else the current layer if editable, else the topmost editable layer); fill from session paint, Auto Size pref, interface language.
    r("text.add", r#"{"x":10,"y":50,"text":"New"}"#, &[], "active,mode", &[("$L1", "locked,visible", false)]),
    r("text.add", r#"{"x":10,"y":50,"text":"New","area":{"width":100,"height":50}}"#, &[], "active,mode", &[("$L1", "locked,visible", false)]),
    // transparency.set: ids -> object.setProps on each (opacity/blend/isolate/knockout whole fields); selection -> mask-edit object else selection subjects, opacity/blend to the Appearance panel's active item if any.
    r("transparency.set", r#"{"ids":["$R"],"opacity":50}"#, &[], "", &[("$R", "props.opacity", false)]),
    r("transparency.set", r#"{"ids":["$G"],"blend":"multiply","isolate":true}"#, &[], "", &[("$G", "props.blend,props.isolate", false)]),
    r("transparency.set", r#"{"opacity":50}"#, &["$R"], "selection", &[("drawing", "settings.mask_edit", false), ("$R", "props.opacity", false)]),
    r("transparency.set", r#"{"ids":["$E"],"opacity":50,"item":0}"#, &[], "", &[("$E", "appearance", false)]),
    // transparency.makeOpacityMask: ids in paint order (or selection roots); the topmost becomes the mask art (removed whole) of the object(s) below; one object gets an empty mask and enters mask editing; clip/invert default from the panel menu.
    r("transparency.makeOpacityMask", r#"{"ids":["$R","$E"]}"#, &[], "mode", &[("$R", "props.mask", true), ("$E", "*", true)]),
    r(
        "transparency.makeOpacityMask",
        r#"{"ids":["$R","$E","$P"],"clip":true,"invert":false}"#,
        &[],
        "",
        &[("$R", "-", true), ("$E", "-", true), ("$P", "*", true)],
    ),
    r("transparency.makeOpacityMask", r#"{}"#, &["$R", "$E"], "mode,selection", &[("$R", "props.mask", true), ("$E", "*", true)]),
    r("transparency.makeOpacityMask", r#"{"ids":["$E"]}"#, &[], "mode", &[("$E", "props.mask", false)]),
    // transparency.releaseOpacityMask: masked targets (ids, else mask-edit object, else selection subjects); mask art inserted above each object. Fixture has no mask: errors.
    // UNSURE: the fixture has no opacity mask, so every path fails; rows assume $E carries one
    r("transparency.releaseOpacityMask", r#"{"ids":["$E"]}"#, &[], "", &[("$E", "props.mask", true)]),
    r("transparency.releaseOpacityMask", r#"{}"#, &["$E"], "selection", &[("drawing", "settings.mask_edit", false), ("$E", "props.mask", true)]),
    // transparency.disableOpacityMask: masked targets (unmasked silently skipped); sets mask.disabled, other flags kept. Fixture has no mask: errors.
    // UNSURE: the fixture has no opacity mask; rows assume $E carries one
    r("transparency.disableOpacityMask", r#"{"ids":["$E"]}"#, &[], "", &[("$E", "props.mask", false)]),
    r("transparency.disableOpacityMask", r#"{}"#, &["$E"], "selection", &[("drawing", "settings.mask_edit", false), ("$E", "props.mask", false)]),
    // transparency.enableOpacityMask: masked targets (unmasked silently skipped); sets mask.disabled, other flags kept. Fixture has no mask: errors.
    // UNSURE: the fixture has no opacity mask; rows assume $E carries one
    r("transparency.enableOpacityMask", r#"{"ids":["$E"]}"#, &[], "", &[("$E", "props.mask", false)]),
    r("transparency.enableOpacityMask", r#"{}"#, &["$E"], "selection", &[("drawing", "settings.mask_edit", false), ("$E", "props.mask", false)]),
    // transparency.unlinkOpacityMask: masked targets (unmasked silently skipped); sets mask.linked, other flags kept. Fixture has no mask: errors.
    // UNSURE: the fixture has no opacity mask; rows assume $E carries one
    r("transparency.unlinkOpacityMask", r#"{"ids":["$E"]}"#, &[], "", &[("$E", "props.mask", false)]),
    r("transparency.unlinkOpacityMask", r#"{}"#, &["$E"], "selection", &[("drawing", "settings.mask_edit", false), ("$E", "props.mask", false)]),
    // transparency.linkOpacityMask: masked targets (unmasked silently skipped); sets mask.linked, other flags kept. Fixture has no mask: errors.
    // UNSURE: the fixture has no opacity mask; rows assume $E carries one
    r("transparency.linkOpacityMask", r#"{"ids":["$E"]}"#, &[], "", &[("$E", "props.mask", false)]),
    r("transparency.linkOpacityMask", r#"{}"#, &["$E"], "selection", &[("drawing", "settings.mask_edit", false), ("$E", "props.mask", false)]),
    // transparency.setOpacityMask: masked targets; sets the given clip/invert/disabled/linked, keeps the others. Fixture has no mask: errors.
    // UNSURE: the fixture has no opacity mask; rows assume $E carries one
    r("transparency.setOpacityMask", r#"{"ids":["$E"],"clip":false,"invert":true}"#, &[], "", &[("$E", "props.mask", false)]),
    r(
        "transparency.setOpacityMask",
        r#"{"invert":true}"#,
        &["$E"],
        "selection",
        &[("drawing", "settings.mask_edit", false), ("$E", "props.mask", false)],
    ),
    // transparency.togglePageIsolatedBlending: value given or the negation of the current flag; no edit when unchanged.
    r("transparency.togglePageIsolatedBlending", r#"{}"#, &[], "", &[("drawing", "settings.page_isolate", false)]),
    r("transparency.togglePageIsolatedBlending", r#"{"value":true}"#, &[], "", &[("drawing", "settings.page_isolate", false)]),
    // transparency.togglePageKnockoutGroup: value given or the negation of the current flag.
    r("transparency.togglePageKnockoutGroup", r#"{}"#, &[], "", &[("drawing", "settings.page_knockout", false)]),
    r("transparency.togglePageKnockoutGroup", r#"{"value":true}"#, &[], "", &[("drawing", "settings.page_knockout", false)]),
    // text.orientation.vertical: text among ids/selection and their descendants; sets vertical and lays out again (needs a selection even with ids).
    r("text.orientation.vertical", r#"{"ids":["$T"]}"#, &["$T"], "", &[("$T", "data", false)]),
    r("text.orientation.vertical", r#"{}"#, &["$TA"], "selection", &[("$TA", "data", false)]),
    // Pinned after review: the reading gave `ids` and `on` together (the engine refuses `ids` with nothing selected, a defect); a host refuses a step with both, so the row gives `ids` alone.
    p(
        "text.orientation.vertical",
        r#"{"ids":["$L1"]}"#,
        &[],
        "",
        &[
            ("children:$L1", "-", false),
            ("children:$G", "-", false),
            ("children:$CG", "-", false),
            ("children:$CP", "-", false),
            ("children:$B", "-", false),
            ("$T", "data", false),
            ("$TA", "data", false),
        ],
    ),
    // text.orientation.horizontal: text among ids/selection and their descendants; sets vertical and lays out again (needs a selection even with ids).
    r("text.orientation.horizontal", r#"{"ids":["$T"]}"#, &["$T"], "", &[("$T", "data", false)]),
    r("text.orientation.horizontal", r#"{}"#, &["$TA"], "selection", &[("$TA", "data", false)]),
    // Pinned after review: the reading gave `ids` and `on` together (the engine refuses `ids` with nothing selected, a defect); a host refuses a step with both, so the row gives `ids` alone.
    p(
        "text.orientation.horizontal",
        r#"{"ids":["$L1"]}"#,
        &[],
        "",
        &[
            ("children:$L1", "-", false),
            ("children:$G", "-", false),
            ("children:$CG", "-", false),
            ("children:$CP", "-", false),
            ("children:$B", "-", false),
            ("$T", "data", false),
            ("$TA", "data", false),
        ],
    ),
    // text.convertToOutlines: selection roots; each text (or text inside a root) replaced by a group of glyph outlines at its parent/index, carrying opacity/blend/name.
    r("text.convertToOutlines", r#"{}"#, &["$T"], "selection", &[("$T", "*", true)]),
    // text.setText: text among ids/selection (descendants); replaces runs and paragraphs keeping the first run's and paragraph's style.
    r("text.setText", r#"{"ids":["$T"],"text":"Bye"}"#, &[], "", &[("$T", "data", false)]),
    r("text.setText", r#"{"text":"Bye"}"#, &["$TA"], "selection", &[("$TA", "data", false)]),
    // text.areaOptions: area type among ids/selection; the first target's area options are the base written whole onto every target; width/height resize the frame.
    r("text.areaOptions", r#"{"ids":["$TA"],"inset":5}"#, &["$TA"], "", &[("$TA", "data", false)]),
    r("text.areaOptions", r#"{"width":150}"#, &["$TA"], "selection", &[("$TA", "data", false)]),
    // Pinned after review: the reading gave `ids` and `on` together (the engine refuses `ids` with nothing selected, a defect); a host refuses a step with both, so the row gives `ids` alone.
    p("text.areaOptions", r#"{"ids":["$TA","$T"],"columns":2}"#, &[], "", &[("$TA", "data", false), ("$T", "data.type", false)]),
    // text.reshapeArea: area type by id; moves frame anchors, the text reflows.
    r("text.reshapeArea", r#"{"id":"$TA","anchors":[[0,2]],"dx":10,"dy":0}"#, &[], "", &[("$TA", "data,props.perspective", false)]),
    // text.fitHeadline: text among ids/selection (descendants), point type silently skipped; tracks the first paragraph to fill the frame.
    r("text.fitHeadline", r#"{"ids":["$TA"]}"#, &["$TA"], "", &[("$TA", "data", false)]),
    r("text.fitHeadline", r#"{}"#, &["$TA", "$T"], "selection", &[("$TA", "data", false), ("$T", "data.type", false)]),
    // text.setStyle: text among ids/id/selection (descendants); sets the given character attributes (all runs or start..end) and justify of touched paragraphs; prefs missing-glyph protection on font change.
    r("text.setStyle", r#"{"ids":["$T"],"size":24}"#, &[], "", &[("$T", "data", false)]),
    r("text.setStyle", r#"{"font":"Arial"}"#, &["$T"], "mode,selection", &[("$T", "data", false)]),
    r("text.setStyle", r#"{"id":"$TA","start":0,"end":2,"justify":"center"}"#, &[], "", &[("$TA", "data", false)]),
    r(
        "text.setStyle",
        r#"{"ids":["$L1"],"tracking":50}"#,
        &[],
        "",
        &[
            ("children:$L1", "-", false),
            ("children:$G", "-", false),
            ("children:$CG", "-", false),
            ("children:$CP", "-", false),
            ("children:$B", "-", false),
            ("$T", "data", false),
            ("$TA", "data", false),
        ],
    ),
    // text.changeCase: text among ids/selection (descendants); rewrites non-inline run text.
    r("text.changeCase", r#"{"ids":["$T"],"case":"upper"}"#, &["$T"], "", &[("$T", "data", false)]),
    r("text.changeCase", r#"{"case":"lower"}"#, &["$TA"], "selection", &[("$TA", "data", false)]),
    // text.smartPunctuation: nothing selected or scope document -> every text in the document; else the selection's text; quotes from Document Setup.
    r(
        "text.smartPunctuation",
        r#"{}"#,
        &[],
        "selection",
        &[
            ("drawing", "settings.setup", false),
            ("$ALL", "data.type", false),
            ("$T", "data", false),
            ("$TA", "data", false),
            ("children:$L1", "-", false),
            ("children:$L2", "-", false),
            ("children:$G", "-", false),
            ("children:$CG", "-", false),
            ("children:$CP", "-", false),
            ("children:$B", "-", false),
        ],
    ),
    r("text.smartPunctuation", r#"{"quotes":false}"#, &["$T"], "selection", &[("$T", "data", false)]),
    // text.convertToAreaType: point type among ids/selection (others skipped); frame from its layout bounds.
    r("text.convertToAreaType", r#"{"ids":["$T"]}"#, &["$T"], "", &[("$T", "data", false)]),
    r("text.convertToAreaType", r#"{}"#, &["$T"], "selection", &[("$T", "data", false)]),
    // text.convertToPointType: area type among ids/selection (others skipped); soft wraps become breaks, origin from the first line.
    r("text.convertToPointType", r#"{"ids":["$TA"]}"#, &["$TA"], "", &[("$TA", "data", false)]),
    r("text.convertToPointType", r#"{}"#, &["$TA"], "selection", &[("$TA", "data", false)]),
    // text.pathOptions: type on a path among ids/selection (others skipped); sets the given options. Fixture has none: errors.
    // UNSURE: no type on a path in the fixture; the row fails, it only shows what is read
    r("text.pathOptions", r#"{"spacing":5}"#, &["$T"], "selection", &[("$T", "data", false)]),
    // text.fillPlaceholder: text among ids/selection; runs replaced by placeholder text in the first style (area: sized from the frame).
    r("text.fillPlaceholder", r#"{"ids":["$TA"]}"#, &["$TA"], "", &[("$TA", "data", false)]),
    r("text.fillPlaceholder", r#"{}"#, &["$T"], "selection", &[("$T", "data", false)]),
    // text.insert: text among ids/selection; appends the character to the last run (own run after an inline graphic).
    r("text.insert", r#"{"ids":["$T"],"char":"bullet"}"#, &["$T"], "", &[("$T", "data", false)]),
    r("text.insert", r#"{"text":"!"}"#, &["$T"], "selection", &[("$T", "data", false)]),
    // text.editRange: text by id; replaces bytes start..end (inserted text takes the replaced text's style).
    r("text.editRange", r#"{"id":"$T","start":0,"end":1,"insert":"J"}"#, &[], "", &[("$T", "data", false)]),
    // text.setRangeStyle: id, else the one selected text; styles start..end; position/smallCaps sizes from Document Setup; missing-glyph pref.
    r("text.setRangeStyle", r#"{"id":"$T","start":0,"end":2,"size":30}"#, &[], "", &[("$T", "data", false)]),
    r("text.setRangeStyle", r#"{"position":"superscript"}"#, &["$T"], "selection", &[("drawing", "settings.setup", false), ("$T", "data", false)]),
    // text.addInPath: the path by id becomes area/on-path type (removed and replaced at its parent/index, keeping opacity and name); style from session paint, prefs, language.
    r("text.addInPath", r#"{"path":"$P","mode":"onPath","text":"On"}"#, &[], "mode", &[("$P", "*", true)]),
    r("text.addInPath", r#"{"path":"$R","mode":"area","text":"In"}"#, &[], "mode", &[("$R", "*", true)]),
    // text.fitHeadlineSelection: area type directly among ids/selection (no group expansion); tracks the first paragraph to its first line's width.
    r("text.fitHeadlineSelection", r#"{"ids":["$TA"]}"#, &["$TA"], "", &[("$TA", "data", false)]),
    r("text.fitHeadlineSelection", r#"{}"#, &["$TA"], "selection", &[("$TA", "data", false)]),
    // text.step: id (range, default all) else the Type tool's range else the selection's text; increments from preferences.
    r("text.step", r#"{"attribute":"size","id":"$T"}"#, &[], "mode", &[("$T", "data", false)]),
    r("text.step", r#"{"attribute":"tracking","by":-2}"#, &["$TA"], "mode,selection", &[("$TA", "data", false)]),
    // format.increaseFontSize: text.step size without id: Type tool range else the selection's text (ids/id ignored); prefs increment.
    r("format.increaseFontSize", r#"{}"#, &["$T"], "mode,selection", &[("$T", "data", false)]),
    // format.decreaseFontSize: as increase, by -by.
    r("format.decreaseFontSize", r#"{"by":2}"#, &["$T"], "mode,selection", &[("$T", "data", false)]),
    // text.discardEmpty: text by id; reads its length (non-empty: nothing changes; empty: removed or the history rolled back).
    r("text.discardEmpty", r#"{"id":"$T"}"#, &[], "", &[("$T", "data", false)]),
    // text.insertInline: text by id (else the Type tool's), at the given byte (else the tool selection or the end); symbol by name, else the current symbol, else the first one.
    r("text.insertInline", r#"{"id":"$T","symbol":"Sym","at":1}"#, &[], "selection", &[("$T", "data", false), ("symbol:Sym", "*", false)]),
    r(
        "text.insertInline",
        r#"{"id":"$T"}"#,
        &[],
        "selection",
        &[("$T", "data", false), ("drawing", "settings.unknown", false), ("~symbols", "*", false)],
    ),
    // charStyle.new: attrs given (validated) or the selected text's style at start, else Japanese defaults; name given (must be free) or the first free 'Character Style N'.
    r("charStyle.new", r#"{"name":"Big","attrs":{"size":30}}"#, &[], "", &[]),
    r("charStyle.new", r#"{"name":"From"}"#, &["$T"], "selection", &[("$T", "data", false)]),
    r("charStyle.new", r#"{}"#, &[], "mode,selection", &[("~charStyles", "*", false)]),
    // charStyle.apply: style by name; text among ids/selection (descendants) or exactly id; all runs or id+start..end; clearOverrides bases on the Normal style.
    r("charStyle.apply", r#"{"name":"Char 1","ids":["$T"]}"#, &[], "", &[("charStyle:Char 1", "*", false), ("$T", "data", false)]),
    r("charStyle.apply", r#"{"name":"Char 1"}"#, &["$TA"], "selection", &[("charStyle:Char 1", "*", false), ("$TA", "data", false)]),
    r("charStyle.apply", r#"{"name":"Char 1","id":"$T","start":0,"end":2}"#, &[], "", &[("charStyle:Char 1", "*", false), ("$T", "data", false)]),
    // UNSURE: clearOverrides also looks up the Normal style by name (absent until redefined); held as the table
    r(
        "charStyle.apply",
        r#"{"name":"Char 1","ids":["$T"],"clearOverrides":true}"#,
        &[],
        "",
        &[("charStyle:Char 1", "*", false), ("~charStyles", "*", false), ("$T", "data", false)],
    ),
    // charStyle.redefine: the style's attrs replaced by the selected text's (id/start or first selected text); every run of every text still using the old values follows.
    r(
        "charStyle.redefine",
        r#"{"name":"Char 1"}"#,
        &["$T"],
        "selection",
        &[
            ("charStyle:Char 1", "*", false),
            ("$ALL", "data.type", false),
            ("$T", "data", false),
            ("$TA", "data", false),
            ("children:$L1", "-", false),
            ("children:$L2", "-", false),
            ("children:$G", "-", false),
            ("children:$CG", "-", false),
            ("children:$CP", "-", false),
            ("children:$B", "-", false),
        ],
    ),
    r(
        "charStyle.redefine",
        r#"{"name":"Char 1","id":"$T","start":1}"#,
        &[],
        "",
        &[
            ("charStyle:Char 1", "*", false),
            ("$ALL", "data.type", false),
            ("$T", "data", false),
            ("$TA", "data", false),
            ("children:$L1", "-", false),
            ("children:$L2", "-", false),
            ("children:$G", "-", false),
            ("children:$CG", "-", false),
            ("children:$CP", "-", false),
            ("children:$B", "-", false),
        ],
    ),
    // charStyle.setAttrs: the style's attrs replaced by attrs; users in every text follow (overrides kept).
    r(
        "charStyle.setAttrs",
        r#"{"name":"Char 1","attrs":{"size":20}}"#,
        &[],
        "",
        &[
            ("charStyle:Char 1", "*", false),
            ("$ALL", "data.type", false),
            ("$T", "data", false),
            ("$TA", "data", false),
            ("children:$L1", "-", false),
            ("children:$L2", "-", false),
            ("children:$G", "-", false),
            ("children:$CG", "-", false),
            ("children:$CP", "-", false),
            ("children:$B", "-", false),
        ],
    ),
    // charStyle.duplicate: the style's attrs copied under the first free '<name> copy N'.
    r("charStyle.duplicate", r#"{"name":"Char 1"}"#, &[], "", &[("charStyle:Char 1", "*", false), ("~charStyles", "*", false)]),
    // charStyle.rename: the style's name, and style_name of every run using it in every text.
    r(
        "charStyle.rename",
        r#"{"name":"Char 1","to":"Char X"}"#,
        &[],
        "",
        &[
            ("charStyle:Char 1", "*", false),
            ("$ALL", "data.type", false),
            ("$T", "data", false),
            ("$TA", "data", false),
            ("children:$L1", "-", false),
            ("children:$L2", "-", false),
            ("children:$G", "-", false),
            ("children:$CG", "-", false),
            ("children:$CP", "-", false),
            ("children:$B", "-", false),
        ],
    ),
    // charStyle.delete: removes the style; every run using it loses the link (keeps its look).
    r(
        "charStyle.delete",
        r#"{"name":"Char 1"}"#,
        &[],
        "",
        &[
            ("charStyle:Char 1", "*", false),
            ("$ALL", "data.type", false),
            ("$T", "data", false),
            ("$TA", "data", false),
            ("children:$L1", "-", false),
            ("children:$L2", "-", false),
            ("children:$G", "-", false),
            ("children:$CG", "-", false),
            ("children:$CP", "-", false),
            ("children:$B", "-", false),
        ],
    ),
    // paraStyle.new: attrs given or the paragraph at start of the selected text, else Japanese defaults; name given or the first free 'Paragraph Style N'.
    r("paraStyle.new", r#"{"name":"Body","attrs":{"space_after":6}}"#, &[], "", &[]),
    r("paraStyle.new", r#"{"name":"From"}"#, &["$T"], "selection", &[("$T", "data", false)]),
    r("paraStyle.new", r#"{}"#, &[], "mode,selection", &[("~paraStyles", "*", false)]),
    // paraStyle.apply: style by name; text among ids/selection or exactly id; all paragraphs or those id+start..end touches.
    r("paraStyle.apply", r#"{"name":"Para 1","ids":["$T"]}"#, &[], "", &[("paraStyle:Para 1", "*", false), ("$T", "data", false)]),
    r("paraStyle.apply", r#"{"name":"Para 1"}"#, &["$TA"], "selection", &[("paraStyle:Para 1", "*", false), ("$TA", "data", false)]),
    r("paraStyle.apply", r#"{"name":"Para 1","id":"$TA","start":0,"end":1}"#, &[], "", &[("paraStyle:Para 1", "*", false), ("$TA", "data", false)]),
    // paraStyle.redefine: the style's attrs replaced by the selected text's paragraph at start; every paragraph using it follows.
    r(
        "paraStyle.redefine",
        r#"{"name":"Para 1"}"#,
        &["$T"],
        "selection",
        &[
            ("paraStyle:Para 1", "*", false),
            ("$ALL", "data.type", false),
            ("$T", "data", false),
            ("$TA", "data", false),
            ("children:$L1", "-", false),
            ("children:$L2", "-", false),
            ("children:$G", "-", false),
            ("children:$CG", "-", false),
            ("children:$CP", "-", false),
            ("children:$B", "-", false),
        ],
    ),
    // paraStyle.setAttrs: the style's attrs replaced; users follow (overrides kept).
    r(
        "paraStyle.setAttrs",
        r#"{"name":"Para 1","attrs":{"justify":"Center"}}"#,
        &[],
        "",
        &[
            ("paraStyle:Para 1", "*", false),
            ("$ALL", "data.type", false),
            ("$T", "data", false),
            ("$TA", "data", false),
            ("children:$L1", "-", false),
            ("children:$L2", "-", false),
            ("children:$G", "-", false),
            ("children:$CG", "-", false),
            ("children:$CP", "-", false),
            ("children:$B", "-", false),
        ],
    ),
    // paraStyle.duplicate: attrs copied under the first free '<name> copy N'.
    r("paraStyle.duplicate", r#"{"name":"Para 1"}"#, &[], "", &[("paraStyle:Para 1", "*", false), ("~paraStyles", "*", false)]),
    // paraStyle.rename: the style's name and every paragraph's style_name using it.
    r(
        "paraStyle.rename",
        r#"{"name":"Para 1","to":"Para X"}"#,
        &[],
        "",
        &[
            ("paraStyle:Para 1", "*", false),
            ("$ALL", "data.type", false),
            ("$T", "data", false),
            ("$TA", "data", false),
            ("children:$L1", "-", false),
            ("children:$L2", "-", false),
            ("children:$G", "-", false),
            ("children:$CG", "-", false),
            ("children:$CP", "-", false),
            ("children:$B", "-", false),
        ],
    ),
    // paraStyle.delete: removes the style; paragraphs using it lose the link.
    r(
        "paraStyle.delete",
        r#"{"name":"Para 1"}"#,
        &[],
        "",
        &[
            ("paraStyle:Para 1", "*", false),
            ("$ALL", "data.type", false),
            ("$T", "data", false),
            ("$TA", "data", false),
            ("children:$L1", "-", false),
            ("children:$L2", "-", false),
            ("children:$G", "-", false),
            ("children:$CG", "-", false),
            ("children:$CP", "-", false),
            ("children:$B", "-", false),
        ],
    ),
    // text.replaceFont: every text in the layers (or selectionOnly: the selection's); runs whose family (and style) match take the new font.
    r(
        "text.replaceFont",
        r#"{"from":{"family":"Arial"},"to":{"family":"Noto Sans"}}"#,
        &[],
        "",
        &[
            ("$ALL", "data.type", false),
            ("$T", "data", false),
            ("$TA", "data", false),
            ("children:$L1", "-", false),
            ("children:$L2", "-", false),
            ("children:$G", "-", false),
            ("children:$CG", "-", false),
            ("children:$CP", "-", false),
            ("children:$B", "-", false),
        ],
    ),
    r(
        "text.replaceFont",
        r#"{"from":{"family":"Arial"},"to":{"family":"Noto Sans"},"selectionOnly":true}"#,
        &["$T"],
        "selection",
        &[("$T", "data", false)],
    ),
    // text.rescanFonts: rescans system fonts (file system) and lays out every text of every open document again (caches; Auto Size frames).
    r(
        "text.rescanFonts",
        r#"{}"#,
        &[],
        "",
        &[
            ("$ALL", "data.type", false),
            ("$T", "data", false),
            ("$TA", "data", false),
            ("children:$L1", "-", false),
            ("children:$L2", "-", false),
            ("children:$G", "-", false),
            ("children:$CG", "-", false),
            ("children:$CP", "-", false),
            ("children:$B", "-", false),
        ],
    ),
    // text.thread.add: selection roots in paint order (needs 2+); closed paths replaced by empty area frames (same id); existing threads merged; the story reflows.
    r("text.thread.add", r#"{}"#, &["$TA", "$R"], "selection", &[("$TA", "data", true), ("$R", "*", true), ("drawing", "threads", false)]),
    // text.thread.releaseSelection: selection roots taken out of their threads; story into the first kept frame. Fixture has no thread: disabled.
    // UNSURE: no thread in the fixture (the command is disabled); row shows what it would rest on
    r("text.thread.releaseSelection", r#"{}"#, &["$TA"], "selection", &[("drawing", "threads", false), ("$TA", "data", false)]),
    // text.thread.remove: threads containing a selected root dropped. Fixture has no thread: disabled.
    // UNSURE: no thread in the fixture (the command is disabled)
    r("text.thread.remove", r#"{}"#, &["$TA"], "selection", &[("drawing", "threads", false)]),
    // transparency.editOpacityMask: id, else the first selection subject with a mask; copies the mask art to an editing layer, isolates it. Fixture has no mask: errors.
    // UNSURE: the fixture has no opacity mask; rows assume $E carries one
    r("transparency.editOpacityMask", r#"{"id":"$E"}"#, &[], "", &[("$E", "props.mask", false), ("drawing", "settings.mask_edit", false)]),
    r("transparency.editOpacityMask", r#"{}"#, &["$E"], "selection", &[("$E", "props.mask", false), ("drawing", "settings.mask_edit", false)]),
    // transparency.stopEditingOpacityMask: needs mask editing; writes the editing layer's art back into the mask, drops the layer, isolation off, current layer = default layer. Fixture not editing: errors.
    // UNSURE: fixture is not in mask editing; the row fails
    r("transparency.stopEditingOpacityMask", r#"{}"#, &[], "active", &[("drawing", "settings.mask_edit", false)]),
    // transparency.viewOpacityMask: on (default: toggle of the shown mask); id, else the edited object, else the first selected subject with a mask; starts mask editing when not editing.
    // UNSURE: the fixture has no opacity mask; rows assume $E carries one
    r("transparency.viewOpacityMask", r#"{"on":true,"id":"$E"}"#, &[], "", &[("$E", "props.mask", false), ("drawing", "settings.mask_edit", false)]),
    r("transparency.viewOpacityMask", r#"{}"#, &["$E"], "selection", &[("$E", "props.mask", false), ("drawing", "settings.mask_edit", false)]),
    // text.tabs.set: text among ids/selection (descendants); replaces tab stops of every paragraph or those start..end touches.
    r("text.tabs.set", r#"{"ids":["$TA"],"stops":[{"position":36}]}"#, &["$TA"], "", &[("$TA", "data", false)]),
    r("text.tabs.set", r#"{"stops":[{"position":36}],"start":0,"end":1}"#, &["$T"], "selection", &[("$T", "data", false)]),
    // text.tabs.clear: as text.tabs.set with no stops.
    r("text.tabs.clear", r#"{"ids":["$TA"]}"#, &["$TA"], "", &[("$TA", "data", false)]),
    r("text.tabs.clear", r#"{}"#, &["$T"], "selection", &[("$T", "data", false)]),
    // text.setFormat: text directly among ids/id/selection (groups not expanded, others skipped); character attrs (range or all) and paragraph attrs; position/smallCaps sizes from Document Setup.
    r("text.setFormat", r#"{"ids":["$T"],"underline":true}"#, &[], "", &[("$T", "data", false)]),
    r("text.setFormat", r#"{"spaceAfter":6}"#, &["$TA"], "selection", &[("$TA", "data", false)]),
    r("text.setFormat", r#"{"ids":["$T"],"smallCaps":true}"#, &[], "", &[("drawing", "settings.setup", false), ("$T", "data", false)]),
    // graphicStyle.apply: style by name; targets = ids, else the Layers-panel target or the selected roots (contents: their leaves); sets each target's appearance whole, opacity/blend/isolate/knockout and the style link; Override Character Color (prefs) also clears type runs' colours; add: appends items/effects and unlinks; nothing selected: the session's new-art template takes it.
    r(
        "graphicStyle.apply",
        r#"{"name":"Style 1"}"#,
        &["$R"],
        "mode,selection",
        &[
            ("graphicStyle:Style 1", "*", false),
            ("$R", "appearance,props.opacity,props.blend,props.isolate,props.knockout,props.graphic_style", false),
        ],
    ),
    r(
        "graphicStyle.apply",
        r#"{"name":"Style 1","ids":["$R"]}"#,
        &[],
        "mode",
        &[
            ("graphicStyle:Style 1", "*", false),
            ("$R", "appearance,props.opacity,props.blend,props.isolate,props.knockout,props.graphic_style", false),
        ],
    ),
    r(
        "graphicStyle.apply",
        r#"{"name":"Style 1"}"#,
        &["$G"],
        "mode,selection",
        &[
            ("graphicStyle:Style 1", "*", false),
            ("$G", "appearance,props.opacity,props.blend,props.isolate,props.knockout,props.graphic_style", false),
        ],
    ),
    r(
        "graphicStyle.apply",
        r#"{"name":"Style 1","target":"contents"}"#,
        &["$G"],
        "mode,selection",
        &[
            ("graphicStyle:Style 1", "*", false),
            ("children:$G", "-", false),
            ("$C1", "appearance,props.opacity,props.blend,props.isolate,props.knockout,props.graphic_style", false),
            ("$C2", "appearance,props.opacity,props.blend,props.isolate,props.knockout,props.graphic_style", false),
        ],
    ),
    r(
        "graphicStyle.apply",
        r#"{"name":"Style 1"}"#,
        &["$T"],
        "mode,selection",
        &[
            ("graphicStyle:Style 1", "*", false),
            ("$T", "appearance,data,props.opacity,props.blend,props.isolate,props.knockout,props.graphic_style", false),
        ],
    ),
    r(
        "graphicStyle.apply",
        r#"{"name":"Style 1","add":true}"#,
        &["$R"],
        "selection",
        &[("graphicStyle:Style 1", "*", false), ("$R", "appearance,props.graphic_style", false)],
    ),
    r("graphicStyle.apply", r#"{"name":"Style 1"}"#, &[], "selection", &[("graphicStyle:Style 1", "*", false)]),
    // graphicStyle.new: source = `id` or the first selected root; captures its appearance and opacity/blend/isolate/knockout (a group without own paint lends its topmost leaf's, type its first run's colours); links the source.
    r(
        "graphicStyle.new",
        r#"{"id":"$E"}"#,
        &[],
        "",
        &[("$E", "appearance,props.opacity,props.blend,props.isolate,props.knockout,props.graphic_style", false)],
    ),
    r(
        "graphicStyle.new",
        r#"{}"#,
        &["$E"],
        "selection",
        &[("$E", "appearance,props.opacity,props.blend,props.isolate,props.knockout,props.graphic_style", false)],
    ),
    r(
        "graphicStyle.new",
        r#"{"id":"$G"}"#,
        &[],
        "",
        &[
            ("$G", "appearance,props.opacity,props.blend,props.isolate,props.knockout,props.graphic_style", false),
            ("children:$G", "-", false),
            ("$C2", "appearance", false),
        ],
    ),
    r(
        "graphicStyle.new",
        r#"{"id":"$T"}"#,
        &[],
        "",
        &[("$T", "appearance,data,props.opacity,props.blend,props.isolate,props.knockout,props.graphic_style", false)],
    ),
    // graphicStyle.delete: style by name removed; every node whose link is that style's id (all layer art, layers too) is unlinked.
    r(
        "graphicStyle.delete",
        r#"{"name":"Style 1"}"#,
        &[],
        "",
        &[
            ("graphicStyle:Style 1", "*", false),
            ("$ALL", "props.graphic_style", false),
            ("$L1", "props.graphic_style", false),
            ("$L2", "props.graphic_style", false),
        ],
    ),
    // graphicStyle.duplicate: style by name copied whole, inserted right after it in the list.
    r("graphicStyle.duplicate", r#"{"name":"Style 1"}"#, &[], "", &[("graphicStyle:Style 1", "*", true)]),
    // graphicStyle.redefine: source = `id` or first selected root, style = `name` or the source's link; style replaced by the source's look; other linked nodes still in sync with the old look restyled (others unlinked); prefs Override Character Color.
    r(
        "graphicStyle.redefine",
        r#"{"name":"Style 1","id":"$R"}"#,
        &[],
        "mode",
        &[
            ("$R", "appearance,props.opacity,props.blend,props.isolate,props.knockout,props.graphic_style", false),
            ("graphicStyle:Style 1", "*", false),
            ("$ALL", "props.graphic_style", false),
            ("$L1", "props.graphic_style", false),
            ("$L2", "props.graphic_style", false),
            ("$E", "appearance,props.opacity,props.blend,props.isolate,props.knockout,props.graphic_style", false),
        ],
    ),
    r(
        "graphicStyle.redefine",
        r#"{}"#,
        &["$E"],
        "mode,selection",
        &[
            ("$E", "appearance,props.opacity,props.blend,props.isolate,props.knockout,props.graphic_style", false),
            ("graphicStyle:Style 1", "*", false),
            ("$ALL", "props.graphic_style", false),
            ("$L1", "props.graphic_style", false),
            ("$L2", "props.graphic_style", false),
        ],
    ),
    // graphicStyle.breakLink: appearance_targets (ids, else panel target / selected roots; contents: leaves); clears each one's style link.
    r("graphicStyle.breakLink", r#"{}"#, &["$E"], "selection", &[("$E", "props.graphic_style", false)]),
    r("graphicStyle.breakLink", r#"{"ids":["$E"]}"#, &[], "", &[("$E", "props.graphic_style", false)]),
    r(
        "graphicStyle.breakLink",
        r#"{"target":"contents"}"#,
        &["$G"],
        "selection",
        &[("children:$G", "-", false), ("$C1", "props.graphic_style", false), ("$C2", "props.graphic_style", false)],
    ),
    // graphicStyle.rename: style by name gets `to` (existing `to` is an error).
    r("graphicStyle.rename", r#"{"name":"Style 1","to":"Style X"}"#, &[], "", &[("graphicStyle:Style 1", "*", false)]),
    // graphicStyle.sortByName: every style's name and the list order.
    r("graphicStyle.sortByName", r#"{}"#, &[], "", &[("~graphicStyles", "-", false)]),
    // graphicStyle.merge: each named style read whole (items/effects of all, transparency of the first); new style appended. (Fixture has one named style; the name is given twice.)
    r("graphicStyle.merge", r#"{"names":["Style 1","Style 1"]}"#, &[], "", &[("graphicStyle:Style 1", "*", false)]),
    // graphicStyle.move: style by name moved to index `to` (clamped) of the list.
    r("graphicStyle.move", r#"{"name":"Style 1","to":0}"#, &[], "", &[("graphicStyle:Style 1", "-", true), ("~graphicStyles", "-", false)]),
    // swatch.new: color → document colour model (settings.color_mode); colors as given; swatch → copy of that swatch (tint → tint swatch of its base); pattern by name; nothing → the session's current fill.
    r("swatch.new", r##"{"color":"#ff0000"}"##, &[], "", &[("drawing", "settings.color_mode", false)]),
    r("swatch.new", r##"{"colors":["#ff0000"]}"##, &[], "", &[]),
    r("swatch.new", r#"{"swatch":"Brand"}"#, &[], "", &[("swatch:Brand", "*", false)]),
    r("swatch.new", r#"{"swatch":"Brand","tint":50}"#, &[], "", &[("swatch:Brand", "*", false)]),
    r("swatch.new", r#"{"pattern":"Pat"}"#, &[], "", &[("pattern:Pat", "*", false)]),
    r("swatch.new", r#"{}"#, &[], "mode", &[]),
    // swatch.delete: swatch (or colour group) by name removed; a global one's links are dropped everywhere (all art incl. type runs and layers, symbol art, pattern tiles, graphic styles, swatches) and in the session default paints; unlink:false skips that.
    r("swatch.delete", r#"{"name":"Other"}"#, &[], "", &[("swatch:Other", "*", false)]),
    r(
        "swatch.delete",
        r#"{"name":"Brand"}"#,
        &[],
        "mode",
        &[
            ("swatch:Brand", "*", false),
            ("$ALL", "appearance", false),
            ("$T", "appearance,data", false),
            ("$TA", "appearance,data", false),
            ("$L1", "appearance", false),
            ("$L2", "appearance", false),
            ("~symbols", "-", false),
            ("~patterns", "-", false),
            ("~graphicStyles", "-", false),
            ("~swatches", "-", false),
            ("~swatchGroups", "-", false),
        ],
    ),
    r("swatch.delete", r#"{"name":"Brand","unlink":false}"#, &[], "", &[("swatch:Brand", "*", false)]),
    // swatch.newGroup: colors → new swatches; swatches → named solid swatches moved in; fromArtwork → colours of the raw selection's subtrees (linked swatches moved in, unlinked colours new swatches, toGlobal links the selection's matching paints).
    r("swatch.newGroup", r##"{"colors":["#ff0000"]}"##, &[], "", &[]),
    r("swatch.newGroup", r#"{"swatches":["Other"]}"#, &[], "", &[("swatch:Other", "*", false)]),
    r(
        "swatch.newGroup",
        r#"{"fromArtwork":true}"#,
        &["$R"],
        "selection",
        &[("$R", "appearance", false), ("swatch:Brand", "*", false), ("~swatches", "-", false)],
    ),
    r(
        "swatch.newGroup",
        r#"{"fromArtwork":true}"#,
        &["$G"],
        "selection",
        &[
            ("$G", "appearance", false),
            ("children:$G", "-", false),
            ("$C1", "appearance", false),
            ("$C2", "appearance", false),
            ("swatch:Brand", "*", false),
            ("~swatches", "-", false),
        ],
    ),
    // swatch.duplicate: swatch by name copied whole, inserted right after it (in its list or group).
    r("swatch.duplicate", r#"{"name":"Brand"}"#, &[], "", &[("swatch:Brand", "*", true)]),
    // swatch.sortByName: every swatch list (ungrouped and each group) by name.
    r("swatch.sortByName", r#"{}"#, &[], "", &[("~swatches", "-", false), ("~swatchGroups", "-", false)]),
    // swatch.edit: swatch by name; its colour/name/global/spot set; every paint linked to it (all art incl. type and layers, symbol art, pattern tiles, graphic styles, tint swatches) relinked/recoloured at its tint or unlinked; session defaults too.
    // Pinned after review: the reading held the whole swatch table (`~swatches`) for the tint swatches of the edited one, the only premise its vocabulary had (another swatch's rename then refused it, tests/outside.rs s15); the edit rests on the tint swatches linked to it ("Brand 40%" in the fixture), not on the table.
    p(
        "swatch.edit",
        r##"{"name":"Brand","color":"#ff0000"}"##,
        &[],
        "mode",
        &[
            ("swatch:Brand", "*", false),
            ("swatch:Brand 40%", "*", false),
            ("$ALL", "appearance", false),
            ("$T", "appearance,data", false),
            ("$TA", "appearance,data", false),
            ("$L1", "appearance", false),
            ("$L2", "appearance", false),
            ("~symbols", "-", false),
            ("~patterns", "-", false),
            ("~graphicStyles", "-", false),
        ],
    ),
    // Pinned after review: the reading held the whole swatch table (`~swatches`) for the tint swatches of the edited one, the only premise its vocabulary had (another swatch's rename then refused it, tests/outside.rs s15); the edit rests on the tint swatches linked to it ("Brand 40%" in the fixture), not on the table.
    p(
        "swatch.edit",
        r#"{"name":"Brand","newName":"Brand 2"}"#,
        &[],
        "mode",
        &[
            ("swatch:Brand", "*", false),
            ("swatch:Brand 40%", "*", false),
            ("$ALL", "appearance", false),
            ("$T", "appearance,data", false),
            ("$TA", "appearance,data", false),
            ("$L1", "appearance", false),
            ("$L2", "appearance", false),
            ("~symbols", "-", false),
            ("~patterns", "-", false),
            ("~graphicStyles", "-", false),
        ],
    ),
    // Pinned after review: the reading held the whole swatch table (`~swatches`) for the tint swatches of the edited one, the only premise its vocabulary had (another swatch's rename then refused it, tests/outside.rs s15); the edit rests on the tint swatches linked to it ("Brand 40%" in the fixture), not on the table.
    p(
        "swatch.edit",
        r#"{"name":"Brand","paint":{"swatch":"Other"}}"#,
        &[],
        "mode",
        &[
            ("swatch:Brand", "*", false),
            ("swatch:Brand 40%", "*", false),
            ("swatch:Other", "*", false),
            ("$ALL", "appearance", false),
            ("$T", "appearance,data", false),
            ("$TA", "appearance,data", false),
            ("$L1", "appearance", false),
            ("$L2", "appearance", false),
            ("~symbols", "-", false),
            ("~patterns", "-", false),
            ("~graphicStyles", "-", false),
        ],
    ),
    // swatch.move: swatches by name moved before the item now at index `to` of the destination list (ungrouped or `group`).
    r("swatch.move", r#"{"name":"Other","to":0}"#, &[], "", &[("swatch:Other", "*", false), ("~swatches", "-", false)]),
    // swatch.addUsedColors: colours of all art (or the raw selection's subtrees) not held by any solid swatch become swatches; global links the matching unlinked paints.
    r(
        "swatch.addUsedColors",
        r#"{}"#,
        &[],
        "",
        &[
            ("$ALL", "appearance", false),
            ("$T", "appearance,data", false),
            ("$TA", "appearance,data", false),
            ("$L1", "appearance", false),
            ("$L2", "appearance", false),
            ("swatch:Brand", "*", false),
            ("~swatches", "-", false),
            ("~swatchGroups", "-", false),
        ],
    ),
    r(
        "swatch.addUsedColors",
        r#"{"global":true}"#,
        &[],
        "",
        &[
            ("$ALL", "appearance", false),
            ("$T", "appearance,data", false),
            ("$TA", "appearance,data", false),
            ("$L1", "appearance", false),
            ("$L2", "appearance", false),
            ("swatch:Brand", "*", false),
            ("~swatches", "-", false),
            ("~swatchGroups", "-", false),
        ],
    ),
    r(
        "swatch.addUsedColors",
        r#"{"selection":true}"#,
        &["$R"],
        "selection",
        &[("$R", "appearance", false), ("swatch:Brand", "*", false), ("~swatches", "-", false), ("~swatchGroups", "-", false)],
    ),
    // swatch.merge: first named swatch kept (its colour, global, spot); others removed; paints linked to them everywhere take the kept colour and link; session defaults too.
    r(
        "swatch.merge",
        r#"{"names":["Brand","Other"]}"#,
        &[],
        "mode",
        &[
            ("swatch:Brand", "*", false),
            ("swatch:Other", "*", false),
            ("$ALL", "appearance", false),
            ("$T", "appearance,data", false),
            ("$TA", "appearance,data", false),
            ("$L1", "appearance", false),
            ("$L2", "appearance", false),
            ("~symbols", "-", false),
            ("~patterns", "-", false),
            ("~graphicStyles", "-", false),
            ("~swatches", "-", false),
            ("~swatchGroups", "-", false),
        ],
    ),
    // swatch.ungroup: colour group by name; its swatches appended to the ungrouped list.
    // UNSURE: the fixture has no colour group; written for a group "Color Group".
    // Pinned after review: the fixture has no colour group "Color Group": nothing of that name to hold (a colleague giving the name is refused by the name parameter).
    p("swatch.ungroup", r#"{"name":"Color Group"}"#, &[], "", &[]),
    // swatch.sortByKind: every swatch list ordered by kind (paint kind, spot flag).
    r("swatch.sortByKind", r#"{}"#, &[], "", &[("~swatches", "-", false), ("~swatchGroups", "-", false)]),
    // swatch.spotOptions: settings.spot_use_lab; when it changes, paints linked to Lab spot swatches (read from every swatch) are recoloured, session defaults too.
    r(
        "swatch.spotOptions",
        r#"{"useLab":false}"#,
        &[],
        "mode",
        &[("drawing", "settings.spot_use_lab", false), ("~swatches", "-", false), ("~swatchGroups", "-", false)],
    ),
    r("swatch.spotOptions", r#"{}"#, &[], "", &[("drawing", "settings.spot_use_lab", false)]),
    // swatch.editGroup: colour group by name rewritten (swatches take colours by index, extras added, surplus removed); changed/removed globals relinked everywhere.
    // UNSURE: the fixture has no colour group; written for a group "Color Group" (fields of the doc-wide relink only matter when its swatches are global).
    // Pinned after review: the fixture has no colour group "Color Group": nothing of that name to hold (a colleague giving the name is refused by the name parameter).
    p(
        "swatch.editGroup",
        r##"{"group":"Color Group","colors":["#ff0000"]}"##,
        &[],
        "mode",
        &[("~swatches", "-", false), ("~swatchGroups", "-", false)],
    ),
    // effect.apply: appearance_targets (ids; else panel target / selected roots, and the Appearance panel's active item); appends the effect to each one's effect list; a Pathfinder effect on ≥2 loose selected roots groups them first where the topmost was.
    r("effect.apply", r#"{"effect":"stylize.dropShadow"}"#, &["$R"], "selection", &[("$R", "appearance", false)]),
    r("effect.apply", r#"{"effect":"stylize.dropShadow","ids":["$R"]}"#, &[], "", &[("$R", "appearance", false)]),
    r("effect.apply", r#"{"effect":"stylize.dropShadow"}"#, &["$G"], "selection", &[("$G", "appearance", false)]),
    r(
        "effect.apply",
        r#"{"effect":"stylize.dropShadow","target":"contents"}"#,
        &["$G"],
        "selection",
        &[("children:$G", "-", false), ("$C1", "appearance", false), ("$C2", "appearance", false)],
    ),
    r("effect.apply", r#"{"effect":"pathfinder.add"}"#, &["$R", "$E"], "selection", &[("$R", "-", true), ("$E", "-", true)]),
    // effect.remove: the effect at `index` of each target's effect list (object level or an item's).
    r("effect.remove", r#"{"index":0,"ids":["$R"]}"#, &[], "", &[("$R", "appearance", false)]),
    r("effect.remove", r#"{"index":0}"#, &["$R"], "selection", &[("$R", "appearance", false)]),
    // effect.setParams: params merged into the effect at `index` (and visible) on each target.
    r("effect.setParams", r#"{"index":0,"params":{"opacity":50},"ids":["$R"]}"#, &[], "", &[("$R", "appearance", false)]),
    r("effect.setParams", r#"{"index":0,"params":{"opacity":50}}"#, &["$R"], "selection", &[("$R", "appearance", false)]),
    // effect.expandAppearance: each target replaced whole by its expanded art (containers recurse into members; instances read the symbol; brushes from settings.unknown).
    r("effect.expandAppearance", r#"{"ids":["$R"]}"#, &[], "", &[("$R", "*", false), ("drawing", "settings.unknown", false)]),
    r(
        "effect.expandAppearance",
        r#"{}"#,
        &["$G"],
        "selection",
        &[("$G", "*", false), ("children:$G", "-", false), ("$C1", "*", false), ("$C2", "*", false), ("drawing", "settings.unknown", false)],
    ),
    r("effect.expandAppearance", r#"{"ids":["$S"]}"#, &[], "", &[("$S", "*", false), ("symbol:Sym", "*", false)]),
    // effect.duplicate: the effect at `index` copied right after it on each target.
    r("effect.duplicate", r#"{"index":0,"ids":["$R"]}"#, &[], "", &[("$R", "appearance", false)]),
    r("effect.duplicate", r#"{"index":0}"#, &["$R"], "selection", &[("$R", "appearance", false)]),
    // effect.move: effect `from` of the source list moved/copied to `to` of the destination list on each target.
    r("effect.move", r#"{"from":0,"to":1,"ids":["$R"]}"#, &[], "", &[("$R", "appearance", false)]),
    r("effect.move", r#"{"from":0,"to":1}"#, &["$R"], "selection", &[("$R", "appearance", false)]),
    // swatch.setSpot: swatch by name gets spot (and global) when it changes; paints are not touched.
    // Pinned after review: C8-vector-engine-2: swatch.setSpot relinks the art, symbols, patterns, styles, other swatches and the new-art paints linked to the swatch (colormgmt.rs:514, map_solid_paints), reading settings.spot_use_lab.
    p(
        "swatch.setSpot",
        r#"{"name":"Brand"}"#,
        &[],
        "mode",
        &[
            ("swatch:Brand", "*", false),
            ("$ALL", "appearance", false),
            ("$T", "appearance,data", false),
            ("$TA", "appearance,data", false),
            ("$L1", "appearance", false),
            ("$L2", "appearance", false),
            ("~symbols", "*", false),
            ("~patterns", "*", false),
            ("~graphicStyles", "*", false),
            ("~swatches", "*", false),
            ("drawing", "settings.spot_use_lab", false),
        ],
    ),
    // brush.apply: brush by name in the document's library (settings.unknown); targets = paths/compounds under ids (layers dropped) or the selected roots (clipping paths and guides silently skipped); sets the stroke's brush (adding the session stroke if none) and the current brush.
    r(
        "brush.apply",
        r#"{"name":"Charcoal"}"#,
        &["$R"],
        "mode,selection",
        &[("drawing", "settings.unknown", false), ("$R", "appearance,data.type", false)],
    ),
    r(
        "brush.apply",
        r#"{"name":"Charcoal","ids":["$R"]}"#,
        &[],
        "mode",
        &[("drawing", "settings.unknown", false), ("$R", "appearance,data.type", false)],
    ),
    r(
        "brush.apply",
        r#"{"name":"Charcoal"}"#,
        &["$G"],
        "mode,selection",
        &[
            ("drawing", "settings.unknown", false),
            ("children:$G", "-", false),
            ("$C1", "appearance,data.type", false),
            ("$C2", "appearance,data.type", false),
        ],
    ),
    r(
        "brush.apply",
        r#"{"name":"Charcoal"}"#,
        &["$CG"],
        "mode,selection",
        &[("drawing", "settings.unknown", false), ("children:$CG", "-", false), ("$CA", "appearance,data.type", false), ("$CK", "data", false)],
    ),
    // brush.remove: same targets; every stroke item's brush cleared.
    r("brush.remove", r#"{}"#, &["$R"], "selection", &[("$R", "appearance,data.type", false)]),
    r("brush.remove", r#"{"ids":["$R"]}"#, &[], "", &[("$R", "appearance,data.type", false)]),
    // brush.setCurrent: brush by name in the library; the current-brush key of settings.unknown set (or removed with null) outside the undo history.
    r("brush.setCurrent", r#"{"name":"Charcoal"}"#, &[], "", &[("drawing", "settings.unknown", false)]),
    r("brush.setCurrent", r#"{"name":null}"#, &[], "", &[("drawing", "settings.unknown", false)]),
    // brush.new: the library (settings.unknown) read and written whole; art/scatter/pattern brushes take the roots (ids or selected roots) whole as their art.
    r("brush.new", r#"{"type":"calligraphic"}"#, &[], "", &[("drawing", "settings.unknown", false)]),
    r("brush.new", r#"{"type":"art"}"#, &["$R"], "selection", &[("drawing", "settings.unknown", false), ("$R", "*", false)]),
    r("brush.new", r#"{"type":"art","ids":["$R"]}"#, &[], "", &[("drawing", "settings.unknown", false), ("$R", "*", false)]),
    // brush.delete: brush by name removed from the library; every layer-art stroke using it unbrushed; current key cleared if it.
    // Pinned after review: C8-vector-engine-2: the rename/delete now reaches symbol art, pattern tiles, graphic styles and type runs (Document::map_items, doc/src/swatches.rs:160; patterncmds.rs:492,533; brushsym.rs:322).
    p(
        "brush.delete",
        r#"{"name":"Charcoal"}"#,
        &[],
        "",
        &[
            ("drawing", "settings.unknown", false),
            ("$ALL", "appearance", false),
            ("$L1", "appearance", false),
            ("$L2", "appearance", false),
            ("$T", "appearance,data", false),
            ("$TA", "appearance,data", false),
            ("~symbols", "*", false),
            ("~patterns", "*", false),
            ("~graphicStyles", "*", false),
        ],
    ),
    // brush.duplicate: brush by name copied into the library.
    r("brush.duplicate", r#"{"name":"Charcoal"}"#, &[], "", &[("drawing", "settings.unknown", false)]),
    // brush.options: brush by name merged with params; newName renames it on every layer-art stroke and the current key.
    r("brush.options", r#"{"name":"Charcoal","params":{"size":12}}"#, &[], "", &[("drawing", "settings.unknown", false)]),
    // Pinned after review: C8-vector-engine-2: the rename/delete now reaches symbol art, pattern tiles, graphic styles and type runs (Document::map_items, doc/src/swatches.rs:160; patterncmds.rs:492,533; brushsym.rs:322).
    p(
        "brush.options",
        r#"{"name":"Charcoal","newName":"Coal"}"#,
        &[],
        "",
        &[
            ("drawing", "settings.unknown", false),
            ("$ALL", "appearance", false),
            ("$L1", "appearance", false),
            ("$L2", "appearance", false),
            ("$T", "appearance,data", false),
            ("$TA", "appearance,data", false),
            ("~symbols", "*", false),
            ("~patterns", "*", false),
            ("~graphicStyles", "*", false),
        ],
    ),
    // brush.freehand: path.freehand draws a new path (session paint, draw mode, insertion parent = top of the current layer) then gives its stroke the brush (unknown brush silently ignored); extend: continues that path.
    r(
        "brush.freehand",
        r#"{"points":[[0,0],[50,50],[100,0]],"brush":"Charcoal"}"#,
        &[],
        "active,mode,selection",
        &[("drawing", "settings.unknown", false), ("$L1", "-", false)],
    ),
    r(
        "brush.freehand",
        r#"{"points":[[0,0],[50,50],[100,0]],"brush":"Charcoal","extend":{"id":"$P","end":"end"}}"#,
        &[],
        "",
        &[("drawing", "settings.unknown", false), ("$P", "data,appearance", false)],
    ),
    // symbol.new: roots (ids or selected roots) copied whole into a new symbol and replaced by an instance placed after the last (topmost) one; sizes and current symbol in settings.unknown.
    r("symbol.new", r#"{"name":"S2"}"#, &["$R"], "selection", &[("$R", "*", true), ("drawing", "settings.unknown", false)]),
    r("symbol.new", r#"{"name":"S2","ids":["$R"]}"#, &[], "", &[("$R", "*", true), ("drawing", "settings.unknown", false)]),
    r(
        "symbol.new",
        r#"{"name":"S2"}"#,
        &["$G"],
        "selection",
        &[("$G", "*", true), ("children:$G", "-", false), ("$C1", "*", false), ("$C2", "*", false), ("drawing", "settings.unknown", false)],
    ),
    // symbol.place: symbol by name (else current symbol from settings.unknown, else the first); natural size from settings.unknown; centre x,y else artboard 0's; instance at the top of the current layer.
    r(
        "symbol.place",
        r#"{"name":"Sym","x":100,"y":100}"#,
        &[],
        "active",
        &[("symbol:Sym", "*", false), ("drawing", "settings.unknown", false), ("$L1", "-", false)],
    ),
    r(
        "symbol.place",
        r#"{}"#,
        &[],
        "active",
        &[("~symbols", "-", false), ("drawing", "settings.unknown", false), ("artboard:0", "rect", false), ("$L1", "-", false)],
    ),
    // symbol.breakLink: instances under the roots (ids or selected roots) replaced in place by the symbol's art (tinted by the instance fill, with its opacity/blend).
    r("symbol.breakLink", r#"{}"#, &["$S"], "selection", &[("$S", "*", true), ("symbol:Sym", "*", false)]),
    r("symbol.breakLink", r#"{"ids":["$S"]}"#, &[], "", &[("$S", "*", true), ("symbol:Sym", "*", false)]),
    // symbol.edit: the first instance under the roots broken into art in place.
    r("symbol.edit", r#"{}"#, &["$S"], "selection", &[("$S", "*", true), ("symbol:Sym", "*", false)]),
    // symbol.update: symbol by name gets the roots' art (whole); every instance of it rescaled (xf); roots replaced by an instance at the topmost's place; sizes in settings.unknown.
    r(
        "symbol.update",
        r#"{"name":"Sym"}"#,
        &["$R"],
        "selection",
        &[("$R", "*", true), ("symbol:Sym", "*", false), ("drawing", "settings.unknown", false), ("$ALL", "data.type", false), ("$S", "data", false)],
    ),
    // symbol.delete: symbol by name removed; every instance of it (scanned in all art) expanded in place (or deleted); its size and current key removed.
    r(
        "symbol.delete",
        r#"{"name":"Sym"}"#,
        &[],
        "",
        &[("symbol:Sym", "*", false), ("$ALL", "data.type", false), ("$S", "*", true), ("drawing", "settings.unknown", false)],
    ),
    r(
        "symbol.delete",
        r#"{"name":"Sym","expandInstances":false}"#,
        &[],
        "",
        &[("symbol:Sym", "*", false), ("$ALL", "data.type", false), ("$S", "*", false), ("drawing", "settings.unknown", false)],
    ),
    // symbol.duplicate: symbol by name copied with its natural size.
    r("symbol.duplicate", r#"{"name":"Sym"}"#, &[], "", &[("symbol:Sym", "*", false), ("drawing", "settings.unknown", false)]),
    // symbol.replace: instances under the roots get symbol `name`, xf rescaled by the natural sizes (settings.unknown).
    r(
        "symbol.replace",
        r#"{"name":"Sym"}"#,
        &["$S"],
        "selection",
        &[("$S", "data", false), ("symbol:Sym", "*", false), ("drawing", "settings.unknown", false)],
    ),
    r(
        "symbol.replace",
        r#"{"name":"Sym","ids":["$S"]}"#,
        &[],
        "",
        &[("$S", "data", false), ("symbol:Sym", "*", false), ("drawing", "settings.unknown", false)],
    ),
    // symbol.setCurrent: symbol by name (else current, else first) stored as settings.unknown currentSymbol outside history.
    r("symbol.setCurrent", r#"{"name":"Sym"}"#, &[], "", &[("symbol:Sym", "*", false), ("drawing", "settings.unknown", false)]),
    r("symbol.setCurrent", r#"{}"#, &[], "", &[("~symbols", "-", false), ("drawing", "settings.unknown", false)]),
    // symbol.spray: instances of the symbol (name/current/first) along the points into the one selected Symbol Set, else a new set at the top of the current layer; alt removes instances of it near the points (all art scanned).
    r(
        "symbol.spray",
        r#"{"points":[[100,100]],"name":"Sym"}"#,
        &[],
        "active,selection",
        &[("symbol:Sym", "*", false), ("drawing", "settings.unknown", false), ("$L1", "-", false)],
    ),
    // UNSURE: whether $S lies within the radius of the point in the fixture.
    r(
        "symbol.spray",
        r#"{"points":[[100,100]],"name":"Sym","alt":true}"#,
        &[],
        "selection",
        &[("symbol:Sym", "*", false), ("$ALL", "data.type", false), ("$S", "*", false)],
    ),
    // symbol.adjust: instances under the raw selection, else every instance; locked/hidden ones (or ancestors) silently skipped; those within radius of the points moved/scaled/spun (xf), stained/styled (fill items: colour param else session fill; style by name else graphic_styles[1]) or screened (opacity).
    // UNSURE: whether $S lies within the radius of the points in the fixture.
    r(
        "symbol.adjust",
        r#"{"tool":"shift","points":[[100,100],[110,100]]}"#,
        &[],
        "selection",
        &[("$ALL", "data.type", false), ("$S", "data,locked,visible", false), ("$L1", "locked,visible", false)],
    ),
    // UNSURE: as above.
    r(
        "symbol.adjust",
        r#"{"tool":"stain","points":[[100,100]]}"#,
        &[],
        "mode,selection",
        &[("$ALL", "data.type", false), ("$S", "data,appearance,locked,visible", false), ("$L1", "locked,visible", false)],
    ),
    // UNSURE: as above.
    r(
        "symbol.adjust",
        r#"{"tool":"style","style":"Style 1","points":[[100,100]]}"#,
        &[],
        "selection",
        &[
            ("graphicStyle:Style 1", "*", false),
            ("$ALL", "data.type", false),
            ("$S", "data,appearance,locked,visible", false),
            ("$L1", "locked,visible", false),
        ],
    ),
    // pattern.options: pattern by name (else the one being edited, settings.pattern_edit) gets the options; newName renames its swatch, pattern paints in swatches and layer-art fills/strokes.
    r("pattern.options", r#"{"name":"Pat","hSpacing":5}"#, &[], "", &[("pattern:Pat", "*", false)]),
    // Pinned after review: C8-vector-engine-2: the rename/delete now reaches symbol art, pattern tiles, graphic styles and type runs (Document::map_items, doc/src/swatches.rs:160; patterncmds.rs:492,533; brushsym.rs:322).
    p(
        "pattern.options",
        r#"{"name":"Pat","newName":"Pat 2"}"#,
        &[],
        "",
        &[
            ("pattern:Pat", "*", false),
            ("drawing", "settings.pattern_edit", false),
            ("~swatches", "-", false),
            ("~swatchGroups", "-", false),
            ("$ALL", "appearance", false),
            ("$L1", "appearance", false),
            ("$L2", "appearance", false),
            ("$T", "appearance,data", false),
            ("$TA", "appearance,data", false),
            ("~symbols", "*", false),
            ("~patterns", "*", false),
            ("~graphicStyles", "*", false),
        ],
    ),
    // pattern.delete: pattern by name removed with every swatch painting it; layer-art fills/strokes painting it become None.
    // Pinned after review: C8-vector-engine-2: the rename/delete now reaches symbol art, pattern tiles, graphic styles and type runs (Document::map_items, doc/src/swatches.rs:160; patterncmds.rs:492,533; brushsym.rs:322).
    p(
        "pattern.delete",
        r#"{"name":"Pat"}"#,
        &[],
        "",
        &[
            ("pattern:Pat", "*", false),
            ("~swatches", "-", false),
            ("~swatchGroups", "-", false),
            ("$ALL", "appearance", false),
            ("$L1", "appearance", false),
            ("$L2", "appearance", false),
            ("$T", "appearance,data", false),
            ("$TA", "appearance,data", false),
            ("~symbols", "*", false),
            ("~patterns", "*", false),
            ("~graphicStyles", "*", false),
        ],
    ),
    // pattern.transform: targets (ids/id or the raw selection) and their subtrees; every pattern fill/stroke's placement premultiplied.
    r("pattern.transform", r#"{"matrix":[1,0,0,1,10,0],"ids":["$R"]}"#, &[], "", &[("$R", "appearance", false)]),
    r(
        "pattern.transform",
        r#"{"matrix":[1,0,0,1,10,0]}"#,
        &["$G"],
        "selection",
        &[("$G", "appearance", false), ("children:$G", "-", false), ("$C1", "appearance", false), ("$C2", "appearance", false)],
    ),
    // swatch.library.add: library swatches (session library) compared by name/paint with the document's, added under free names (groups by name); apply paints the selection like paint.setFill and the defaults.
    r("swatch.library.add", r#"{"library":"pastels"}"#, &[], "", &[("~swatches", "-", false), ("~swatchGroups", "-", false)]),
    r(
        "swatch.library.add",
        r#"{"library":"pastels","apply":"fill"}"#,
        &["$R"],
        "mode,selection",
        &[("~swatches", "-", false), ("~swatchGroups", "-", false), ("$R", "appearance", false)],
    ),
    // swatch.resetDefaults: defaults of settings.color_mode added where their names are free; replace swaps the tables and drops links to non-default globals everywhere and in the defaults.
    r(
        "swatch.resetDefaults",
        r#"{}"#,
        &[],
        "",
        &[("drawing", "settings.color_mode", false), ("~swatches", "-", false), ("~swatchGroups", "-", false)],
    ),
    r(
        "swatch.resetDefaults",
        r#"{"replace":true}"#,
        &[],
        "mode",
        &[
            ("drawing", "settings.color_mode", false),
            ("$ALL", "appearance", false),
            ("$T", "appearance,data", false),
            ("$TA", "appearance,data", false),
            ("$L1", "appearance", false),
            ("$L2", "appearance", false),
            ("~symbols", "-", false),
            ("~patterns", "-", false),
            ("~graphicStyles", "-", false),
            ("~swatches", "-", false),
            ("~swatchGroups", "-", false),
        ],
    ),
    // swatch.library.save: every swatch/group (or the named ones) written out; library name else the document title.
    r(
        "swatch.library.save",
        r#"{"format":"gpl"}"#,
        &[],
        "",
        &[("~swatches", "-", false), ("~swatchGroups", "-", false), ("drawing", "settings.title", false)],
    ),
    r("swatch.library.save", r#"{"format":"gpl","names":["Brand"],"name":"Lib"}"#, &[], "", &[("swatch:Brand", "*", false)]),
    // swatch.library.load: a library file/data into the session's library list; no document read.
    r("swatch.library.load", r#"{"data":"GIMP Palette\nName: X\n0 0 0 Black\n","name":"x.gpl"}"#, &[], "", &[]),
    // swatch.library.copyToUser: a loaded session library copied to the user folder; no document read.
    r("swatch.library.copyToUser", r#"{"library":"loaded/x.gpl"}"#, &[], "", &[]),
    // graphicStyle.addFromLibrary: library styles compared with the document's style of the same name (same look → existing), added under free names with their patterns; apply runs graphicStyle.apply on ids or the selection.
    // Pinned after review: the drawing has no graphic style "Soft Shadow" yet: nothing of that name to hold (a colleague giving the name is refused by the name parameter).
    p("graphicStyle.addFromLibrary", r#"{"library":"shadows-glows","name":"Soft Shadow"}"#, &[], "", &[]),
    // Pinned after review: the drawing has no graphic style "Soft Shadow" yet: nothing of that name to hold (a colleague giving the name is refused by the name parameter).
    p(
        "graphicStyle.addFromLibrary",
        r#"{"library":"shadows-glows","name":"Soft Shadow","apply":true}"#,
        &["$R"],
        "mode,selection",
        &[("$R", "appearance,props.opacity,props.blend,props.isolate,props.knockout,props.graphic_style", false)],
    ),
    // graphicStyle.saveLibrary: every style (or the named ones) and the patterns they paint with written out; library name else the document title.
    r(
        "graphicStyle.saveLibrary",
        r#"{}"#,
        &[],
        "",
        &[("~graphicStyles", "-", false), ("~patterns", "-", false), ("drawing", "settings.title", false)],
    ),
    r("graphicStyle.saveLibrary", r#"{"names":["Style 1"],"name":"Lib"}"#, &[], "", &[("graphicStyle:Style 1", "*", false)]),
    // graphicStyle.loadLibrary: a file/data into the session's style library list; no document read.
    r("graphicStyle.loadLibrary", r#"{"data":"{}","name":"x.vcstyles"}"#, &[], "", &[]),
    // library.setCurrent: the session's library panel current library; no document.
    r("library.setCurrent", r#"{"library":"My Library"}"#, &[], "", &[]),
    // library.new: a new session library; no document.
    r("library.new", r#"{"name":"Lib"}"#, &[], "", &[]),
    // library.rename: `library` else the panel's current library renamed; no document.
    r("library.rename", r#"{"name":"New"}"#, &[], "mode", &[]),
    r("library.rename", r#"{"library":"My Library","name":"New"}"#, &[], "", &[]),
    // library.delete: `library` else the current library deleted; no document.
    r("library.delete", r#"{}"#, &[], "mode", &[]),
    // library.add: graphic = ids/selection roots copied whole with their resources (swatches, styles…); fillColor = `color`, else the first of ids/raw selection with a fill, else the session fill; charStyle = first selected text's attributes; into `library` else current (else new).
    r("library.add", r#"{"kind":"graphic"}"#, &["$R"], "mode,selection", &[("$R", "*", false), ("swatch:Brand", "*", false)]),
    r("library.add", r#"{"kind":"fillColor"}"#, &["$R"], "mode,selection", &[("$R", "appearance", false)]),
    r("library.add", r##"{"kind":"fillColor","color":"#ff0000"}"##, &[], "mode", &[]),
    r("library.add", r#"{"kind":"charStyle"}"#, &["$T"], "mode,selection", &[("$T", "data", false)]),
    // library.use: colour = paint.setFill/setStroke on the selection (+defaults); charStyle = document style of that name compared (new one if it differs) and applied to the selected text; graphic = pasted at center (else artboard 0's centre) into the current layer.
    r("library.use", r#"{"kind":"fillColor","item":"Red"}"#, &["$R"], "mode,selection", &[("$R", "appearance", false)]),
    r(
        "library.use",
        r#"{"kind":"charStyle","item":"Char 1"}"#,
        &["$T"],
        "mode,selection",
        &[("charStyle:Char 1", "*", false), ("$T", "data", false)],
    ),
    // UNSURE: paste of a library graphic (Paste Remembers Layers, swatch-conflict merge) not followed in full.
    r(
        "library.use",
        r#"{"kind":"graphic","item":"graphic","center":[100,100]}"#,
        &[],
        "active,mode",
        &[("$L1", "-", false), ("drawing", "settings.paste_remembers_layers", false), ("~swatches", "-", false)],
    ),
    // library.removeItem: item of `library` else the current library; no document.
    r("library.removeItem", r#"{"kind":"fillColor","item":"Red"}"#, &[], "mode", &[]),
    // library.addGroup: group in `library` else the current library; no document.
    r("library.addGroup", r#"{}"#, &[], "mode", &[]),
    // library.renameGroup: group in `library` else the current library; no document.
    r("library.renameGroup", r#"{"group":"Group","name":"G2"}"#, &[], "mode", &[]),
    // library.deleteGroup: group in `library` else the current library; no document.
    r("library.deleteGroup", r#"{"group":"Group"}"#, &[], "mode", &[]),
    // library.moveItem: item into a group of `library` else the current library; no document.
    r("library.moveItem", r#"{"kind":"fillColor","item":"Red","group":"Group"}"#, &[], "mode", &[]),
    // colorTheme.save: themes in the session preferences; no document.
    r("colorTheme.save", r##"{"colors":["#ff0000"]}"##, &[], "mode", &[]),
    // colorTheme.delete: theme by name in the preferences; no document.
    r("colorTheme.delete", r#"{"name":"Theme 1"}"#, &[], "mode", &[]),
    // colorTheme.addToSwatches: theme from the preferences → swatch.newGroup with its colours (names created); no document premise.
    r("colorTheme.addToSwatches", r#"{"name":"Theme 1"}"#, &[], "mode", &[]),
    // graph.add: no object resolved; new graph group goes on top of the insertion parent: isolation, else the active layer when it and its ancestors are visible and unlocked, else silently the topmost visible unlocked layer.
    // Pinned after review: `mode` from the preferences this command reads on a path the reading's rows do not take (tests/preferences.rs, READERS).
    p("graph.add", r#"{"type":"column","x":100,"y":100,"width":300,"height":200}"#, &[], "active,mode", &[("$L1", "locked,visible", false)]),
    // graph.setData: `id` (walking up to the graph that holds it) else the selection's first graph; reads the whole graph spec (props.graph) and the current bounds of its generated children; replaces props.graph and regenerates every child.
    // UNSURE: the fixture has no graph; $G stands in for a graph whose generated children are $C1/$C2.
    // Pinned after review: `mode` from the preferences this command reads on a path the reading's rows do not take (tests/preferences.rs, READERS).
    p(
        "graph.setData",
        r#"{"id":"$G","rows":[[1,2],[3,4]]}"#,
        &[],
        "mode",
        &[("$G", "props.graph", false), ("children:$G", "-", false), ("$C1", "*", false), ("$C2", "*", false)],
    ),
    // UNSURE: stand-in as above; selection of a member walks up through its parent to the graph.
    // Pinned after review: the fixture has no graph (`$G` stood in for one): a plain group's member is no member of a graph, so the selected `$C1` alone is acted on.
    p("graph.setData", r#"{"rows":[[1,2],[3,4]]}"#, &["$C1"], "selection", &[("$C1", "-", true), ("$C1", "*", false)]),
    // graph.setType: as setData; also renames the graph when its name ends with " Graph"; without `id`, a selection made only of the graph's series groups (props.series_index) retypes just those series.
    // UNSURE: the fixture has no graph; $G stands in.
    // Pinned after review: `mode` from the preferences this command reads on a path the reading's rows do not take (tests/preferences.rs, READERS).
    p(
        "graph.setType",
        r#"{"id":"$G","type":"line"}"#,
        &[],
        "mode",
        &[("$G", "name,props.graph", false), ("children:$G", "-", false), ("$C1", "*", false), ("$C2", "*", false)],
    ),
    // UNSURE: stand-in; selection path reads every series group's props.series_index to decide series vs whole graph.
    r(
        "graph.setType",
        r#"{"type":"line"}"#,
        &["$G"],
        "selection",
        &[("$G", "name,props.graph", false), ("children:$G", "-", false), ("$C1", "*", false), ("$C2", "*", false)],
    ),
    // variable.define: a name and kind; reads and extends the document's variables (settings.variables).
    r("variable.define", r#"{"name":"V","kind":"text"}"#, &[], "", &[("drawing", "settings.variables", false)]),
    // variable.delete: `names`, else the Variables panel's highlighted row; prunes the variables, their dataset values and bindings.
    // UNSURE: needs a variable "V" defined first (fixture has none).
    r("variable.delete", r#"{"names":["V"]}"#, &[], "", &[("drawing", "settings.variables", false)]),
    // UNSURE: needs "V" defined and highlighted first.
    r("variable.delete", r#"{}"#, &[], "selection", &[("drawing", "settings.variables", false)]),
    // variable.rename: by name; renames the variable, every binding and every dataset's value key.
    // UNSURE: needs "V" defined first.
    r("variable.rename", r#"{"name":"V","newName":"W"}"#, &[], "", &[("drawing", "settings.variables", false)]),
    // variable.makeTextDynamic: selection roots (compound member -> compound); new variable named after the first root's name or text; binds every root.
    r("variable.makeTextDynamic", r#"{}"#, &["$T"], "selection", &[("drawing", "settings.variables", false), ("$T", "name,data", false)]),
    // variable.makeVisibilityDynamic: as above, a visibility variable; name from the first root's name, else its kind label.
    r("variable.makeVisibilityDynamic", r#"{}"#, &["$R"], "selection", &[("drawing", "settings.variables", false), ("$R", "name,data.type", false)]),
    // variable.bind: `ids` (raw, checked to exist) else selection roots; reads the variable's kind and the existing bindings; adds bindings (lock/type checks only refuse).
    // UNSURE: needs a text variable "V" defined first.
    r("variable.bind", r#"{"variable":"V","ids":["$T"]}"#, &[], "", &[("drawing", "settings.variables", false), ("$T", "-", false)]),
    // UNSURE: needs "V" defined first.
    r("variable.bind", r#"{"variable":"V"}"#, &["$T"], "selection", &[("drawing", "settings.variables", false), ("$T", "-", false)]),
    // variable.unbind: `ids` else selection roots; drops the named binding or all of the object's.
    // UNSURE: needs a binding of $T first.
    r("variable.unbind", r#"{"ids":["$T"]}"#, &[], "", &[("drawing", "settings.variables", false), ("$T", "-", false)]),
    // UNSURE: needs "V" defined and bound to $T first.
    r("variable.unbind", r#"{"variable":"V"}"#, &["$T"], "selection", &[("drawing", "settings.variables", false), ("$T", "-", false)]),
    // variable.highlight: panel state only (session variables_highlight); checks the variable exists; no document write.
    r("variable.highlight", r#"{}"#, &[], "", &[]),
    // dataset.new: new row by name; `values` checked against the variables' kinds.
    r("dataset.new", r#"{"name":"D1"}"#, &[], "", &[("drawing", "settings.variables", false)]),
    // dataset.set: dataset by name; its values replaced whole.
    // UNSURE: needs dataset "D1" first.
    r("dataset.set", r#"{"name":"D1","values":{}}"#, &[], "", &[("drawing", "settings.variables", false)]),
    // dataset.rename: by name; the active dataset name follows.
    // UNSURE: needs dataset "D1" first.
    r("dataset.rename", r#"{"name":"D1","newName":"D2"}"#, &[], "", &[("drawing", "settings.variables", false)]),
    // dataset.capture: per variable, the first bound object (lowest id) that is a variable target (own lock, ancestors' lock and visibility: silently skipped) gives its text or visibility; new dataset made active.
    // UNSURE: needs a text variable bound to $T first.
    r(
        "dataset.capture",
        r#"{}"#,
        &[],
        "",
        &[("drawing", "settings.variables", false), ("$T", "data,visible,locked", false), ("$L1", "locked,visible", false)],
    ),
    // dataset.update: as capture, written whole into the active dataset.
    // UNSURE: needs a bound text variable and an active dataset first.
    r(
        "dataset.update",
        r#"{}"#,
        &[],
        "",
        &[("drawing", "settings.variables", false), ("$T", "data,visible,locked", false), ("$L1", "locked,visible", false)],
    ),
    // dataset.delete: `names`, else the active dataset (a document field).
    // UNSURE: needs dataset "D1" first.
    r("dataset.delete", r#"{"names":["D1"]}"#, &[], "", &[("drawing", "settings.variables", false)]),
    // UNSURE: needs an active dataset first.
    r("dataset.delete", r#"{}"#, &[], "", &[("drawing", "settings.variables", false)]),
    // dataset.select: dataset by name; every binding applied in variable order: text objects' text replaced (keeping the replaced text's style), visibility objects' visible set; non-targets (locked, on locked/hidden ancestors) silently skipped; active dataset set.
    // UNSURE: needs a text variable bound to $T and dataset "D1" carrying it.
    r(
        "dataset.select",
        r#"{"name":"D1"}"#,
        &[],
        "",
        &[("drawing", "settings.variables", false), ("$T", "data,visible,locked", false), ("$L1", "locked,visible", false)],
    ),
    // dataset.next: the dataset after the active one in list order (wrapping), then as dataset.select.
    // UNSURE: needs datasets and a bound $T first.
    r(
        "dataset.next",
        r#"{}"#,
        &[],
        "",
        &[("drawing", "settings.variables", false), ("$T", "data,visible,locked", false), ("$L1", "locked,visible", false)],
    ),
    // dataset.prev: the dataset before the active one (wrapping), then as dataset.select.
    // UNSURE: needs datasets and a bound $T first.
    r(
        "dataset.prev",
        r#"{}"#,
        &[],
        "",
        &[("drawing", "settings.variables", false), ("$T", "data,visible,locked", false), ("$L1", "locked,visible", false)],
    ),
    // imageTrace.make: `ids`/`id` else selection roots; the first (bottom-most) root that is an image or Image Trace group; the image is copied whole (pixels from the document's image store) and the target replaced at its index by the trace group; documentLibrary palette reads the document swatches.
    // UNSURE: the fixture has no image; $R stands in for an embedded image.
    r("imageTrace.make", r#"{"id":"$R"}"#, &[], "", &[("$R", "*", true)]),
    // UNSURE: stand-in; selection path, and library "document" reads every swatch.
    r(
        "imageTrace.make",
        r#"{"params":{"mode":"color","palette":"documentLibrary","library":"document","colors":6}}"#,
        &["$R"],
        "selection",
        &[("$R", "*", true), ("drawing", "~swatches", false)],
    ),
    // imageTrace.makeAndExpand: as make, no hidden image kept, group unnamed.
    // UNSURE: the fixture has no image; $R stands in.
    r("imageTrace.makeAndExpand", r#"{"id":"$R"}"#, &[], "", &[("$R", "*", true)]),
    // imageTrace.release: roots filtered silently to Image Trace groups (name, non-clip group, first child an image); the group is replaced at its place by its first child image made visible.
    // UNSURE: the fixture has no trace group; $G stands in, $C1 (its first child) as the source image, $C2 as traced paths.
    r("imageTrace.release", r#"{"id":"$G"}"#, &[], "", &[("$G", "*", true), ("children:$G", "-", false), ("$C1", "*", false), ("$C2", "*", false)]),
    // imageTrace.expand: same filter; clears the group's name and props.trace and drops its image children.
    // UNSURE: stand-in as for release.
    r(
        "imageTrace.expand",
        r#"{}"#,
        &["$G"],
        "selection",
        &[("$G", "name,data.type,props.trace", false), ("children:$G", "-", false), ("$C1", "data.type", false), ("$C2", "data.type", false)],
    ),
    // imageTrace.setView: `id`/`ids`/selection, same filter; sets props.trace's view (no-op when all already have it).
    // UNSURE: stand-in as for release.
    r(
        "imageTrace.setView",
        r#"{"id":"$G","view":"outlines"}"#,
        &[],
        "",
        &[("$G", "name,data.type,props.trace", false), ("children:$G", "-", false), ("$C1", "data.type", false)],
    ),
    // linkedFile.update: no ids -> every linked image/placed document in the layers (walk of every node's kind and link), probed on disk; modified or previewing ones re-read and placed per their placement options (clip may wrap in a clip group at their index). Fixture: nothing linked, no write.
    r("linkedFile.update", r#"{}"#, &[], "", &[("$ALL", "data", false)]),
    // UNSURE: the fixture has no linked image; $R stands in (non-image ids are silently ignored).
    r("linkedFile.update", r#"{"ids":["$R"]}"#, &[], "", &[("$R", "data", true)]),
    // linkedFile.updateChanged: all linked files' stamps vs the session's last-seen stamps; changed ones go through linkedFile.update {ids}.
    r("linkedFile.updateChanged", r#"{}"#, &[], "mode", &[("$ALL", "data", false)]),
    // linkedFile.relink: `ids` (each must be image/placed doc) else selected ones; path or folder (by link name); install keeps bounds per placement options.
    // UNSURE: the fixture has no image; $R stands in.
    r("linkedFile.relink", r#"{"ids":["$R"],"path":"/tmp/x.png"}"#, &[], "", &[("$R", "data", true)]),
    // folder with nothing selected: every missing link in the document.
    r("linkedFile.relink", r#"{"folder":"/tmp"}"#, &[], "selection", &[("$ALL", "data", false)]),
    // linkedFile.goTo: `id` else the first selected image/placed doc; changes only the selection.
    // UNSURE: the fixture has no image; $R stands in.
    r("linkedFile.goTo", r#"{"id":"$R"}"#, &[], "", &[("$R", "-", false)]),
    // linkedFile.embed: `ids` else selected linked ones; previewing images re-read from file, then the link dropped; placed documents expanded in place.
    // UNSURE: the fixture has no linked image; $R stands in.
    // Pinned after review: `mode` from the preferences this command reads on a path the reading's rows do not take (tests/preferences.rs, READERS).
    p("linkedFile.embed", r#"{"ids":["$R"]}"#, &[], "mode", &[("$R", "data", false)]),
    // UNSURE: stand-in; selection path.
    r("linkedFile.embed", r#"{}"#, &["$R"], "selection", &[("$R", "data", false)]),
    // linkedFile.unembed: `id` else first selected image; writes its pixels to path and links the image to it (data: link, key, size, xf).
    // UNSURE: the fixture has no embedded image; $R stands in.
    // Pinned after review: `mode` from the preferences this command reads on a path the reading's rows do not take (tests/preferences.rs, READERS).
    p("linkedFile.unembed", r#"{"id":"$R","path":"/tmp/x.png"}"#, &[], "mode", &[("$R", "data", false)]),
    // linkedFile.placementOptions: `ids` else selected images/placed docs; sets preserve/align/clip (others kept).
    // UNSURE: the fixture has no image; $R stands in.
    r("linkedFile.placementOptions", r#"{"ids":["$R"],"preserve":"fit"}"#, &[], "", &[("$R", "data", false)]),
    // UNSURE: stand-in; selection path.
    r("linkedFile.placementOptions", r#"{"clip":true}"#, &["$R"], "selection", &[("$R", "data", false)]),
    // asset.add: `ids` (checked) mapped to roots in paint order (compound member -> compound, descendants of listed ancestors and layers dropped), else selection roots; reuses an asset with exactly that node list; name from the single object's name, else "Asset N" after every asset's name.
    r("asset.add", r#"{"ids":["$R"]}"#, &[], "", &[("$R", "name", false), ("$AS", "*", false)]),
    r("asset.add", r#"{}"#, &["$R"], "selection", &[("$R", "name", false), ("$AS", "*", false)]),
    // $E already has asset $AS: reused, no write.
    r("asset.add", r#"{"ids":["$E"]}"#, &[], "", &[("$E", "-", false), ("$AS", "*", false)]),
    // a compound member acts as its compound.
    r("asset.add", r#"{"ids":["$K1"]}"#, &[], "", &[("$K1", "-", true), ("$CP", "name,data.type", false), ("$AS", "*", false)]),
    // multiple:false: one asset, nodes in stacking order.
    r("asset.add", r#"{"ids":["$R","$P"],"multiple":false}"#, &[], "", &[("$R", "-", true), ("$P", "-", true), ("$AS", "*", false)]),
    // asset.remove: asset ids; drops them from the asset list.
    r("asset.remove", r#"{"assets":["$AS"]}"#, &[], "", &[("$AS", "*", false)]),
    // asset.rename: asset id; sets its name.
    r("asset.rename", r#"{"asset":"$AS","name":"Hero"}"#, &[], "", &[("$AS", "*", false)]),
    // asset.export: `assets` else all; document export settings under the params; each asset's art cloned whole into an isolated document and rendered; no document write.
    r(
        "asset.export",
        r#"{}"#,
        &[],
        "",
        &[("drawing", "settings.export_settings,settings.title", false), ("$AS", "*", false), ("$E", "*", false), ("swatch:Brand", "*", false)],
    ),
    // asset.settings.set: the document's export settings overlaid by the params and stored whole (not an undo step).
    r("asset.settings.set", r#"{"formats":[{"format":"png","scale":"2x"}]}"#, &[], "", &[("drawing", "settings.export_settings", false)]),
];
