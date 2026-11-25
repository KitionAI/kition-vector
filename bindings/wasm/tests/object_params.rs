//! C8-land-11-vector: declarations describe the resolver, never integer spelling alone.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

use std::collections::BTreeSet;

use kition_kit::{KitCommand, KitDocument, testing};
use kition_vector_kit::VectorDocument;
use serde_json::{Value, json};

fn commands() -> Vec<KitCommand> {
    let commands = VectorDocument::create("Ids.kvector", 320, 200).unwrap().commands();
    std::fs::create_dir_all("../../target/land11").unwrap();
    std::fs::write("../../target/land11/commands.json", serde_json::to_vec_pretty(&commands).unwrap()).unwrap();
    commands
}

/// The commands the SDK's scanner flags, with its problems.
fn flagged(commands: &[KitCommand]) -> (BTreeSet<String>, Vec<String>) {
    match testing::check_id_params(commands, address_allow()) {
        Ok(()) => (BTreeSet::new(), Vec::new()),
        Err(problems) => (problems.iter().filter_map(|problem| problem.split(':').next().map(str::to_owned)).collect(), problems),
    }
}

/// Integer parameters whose name or description says "id" and that name no object, each with why.
fn address_allow() -> &'static [(&'static str, &'static str)] {
    &[]
}

#[test]
fn every_id_parameter_is_declared() {
    let commands = commands();
    let (_, problems) = flagged(&commands);
    assert!(problems.is_empty(), "integer id parameters not declared:\n{}", problems.join("\n"));
    let targets = commands.iter().filter(|command| command.targets.is_some()).count();
    let objects: usize = commands.iter().map(|command| command.objects.len()).sum();
    println!("C8 SDK scanner: {targets} targets, {objects} other object parameters, 0 allowed, 0 pending");
}

/// Integer parameters that name no object, each with what the handler reads it as (from the
/// per-handler reading; `command<TAB>parameter<TAB>reason`).
const VALUES: &str = include_str!("object_params_values.txt");

/// Every integer parameter, at any depth (`a`, `a.b`, `a[].b`), that the command does not declare
/// as its targets, an object, a place or a name.
fn undeclared_integers(commands: &[KitCommand]) -> BTreeSet<(String, String)> {
    fn integer(node: &Value) -> bool {
        node.get("type").and_then(Value::as_str) == Some("integer")
            || node.get("oneOf").and_then(Value::as_array).is_some_and(|branches| branches.iter().any(integer))
            || node.get("type").and_then(Value::as_str) == Some("array")
                && node.get("items").is_some_and(|items| items.get("properties").is_none() && integer(items))
    }
    fn walk(command: &KitCommand, path: &str, node: &Value, out: &mut BTreeSet<(String, String)>) {
        let declared = command.targets == Some(path)
            || command.objects.contains(&path)
            || command.places.contains(&path)
            || command.name_params.iter().any(|(name, _)| *name == path);
        if integer(node) && !declared {
            out.insert((command.id.clone(), path.to_owned()));
        }
        for (key, child) in node.get("properties").and_then(Value::as_object).into_iter().flatten() {
            walk(command, &format!("{path}.{key}"), child, out);
        }
        if let Some(items) = node.get("items").filter(|items| items.get("properties").is_some()) {
            for (key, child) in items.get("properties").and_then(Value::as_object).into_iter().flatten() {
                walk(command, &format!("{path}[].{key}"), child, out);
            }
        }
    }
    let mut out = BTreeSet::new();
    for command in commands {
        for (key, node) in command.schema.as_ref().and_then(Value::as_object).into_iter().flatten() {
            walk(command, key, node, &mut out);
        }
    }
    out
}

#[test]
fn every_integer_parameter_names_an_object_or_says_what_it_is() {
    let commands = commands();
    let found = undeclared_integers(&commands);
    let mut allowed = BTreeSet::new();
    for line in VALUES.lines().filter(|line| !line.trim().is_empty()) {
        let mut fields = line.splitn(3, '\t');
        let (Some(id), Some(param), Some(reason)) = (fields.next(), fields.next(), fields.next()) else { panic!("bad line {line:?}") };
        assert!(reason.trim().len() > 8, "{id} {param}: give the reason");
        allowed.insert((id.to_owned(), param.to_owned()));
    }
    let undeclared: Vec<_> = found.difference(&allowed).collect();
    assert!(undeclared.is_empty(), "integer parameters neither declared nor explained: {undeclared:?}");
    let stale: Vec<_> = allowed.difference(&found).collect();
    assert!(stale.is_empty(), "explained but declared now (or gone): {stale:?}");
    let declared: usize = commands.iter().map(|command| command.object_types.len() + command.places.len()).sum();
    println!("C8 integer scanner: {declared} declared (typed objects and places); {} values explained", allowed.len());
}

#[test]
fn the_scanner_reaches_nested_members_arrays_and_union_branches() {
    let command = KitCommand {
        id: "synthetic.nested".to_owned(),
        schema: Some(json!({"rules":{"type":"array","items":{"type":"object","properties":{
            "sourceId":{"oneOf":[{"type":"string"},{"type":"integer"}]},
            "nested":{"type":"object","properties":{"ids":{"type":"array","items":{"type":"integer"}}}}
        }}}})),
        ..Default::default()
    };
    let found: Vec<_> = undeclared_integers(&[command]).into_iter().map(|(_, path)| path).collect();
    assert_eq!(found, ["rules[].nested.ids", "rules[].sourceId"]);
}

/// String parameters that look a name up (or say they take one) and are no object-model name,
/// each with why (`command<TAB>parameter<TAB>reason`).
const NAMES: &str = include_str!("object_params_names.txt");

/// String parameters, at any depth, whose name or description speaks of a named noun (a swatch, a
/// symbol, a style, a pattern, a brush, a name) and that the command does not declare by name.
fn undeclared_names(commands: &[KitCommand]) -> BTreeSet<(String, String)> {
    const NOUNS: &[&str] = &["swatch", "symbol", "style", "pattern", "brush", "name"];
    fn walk(command: &KitCommand, path: &str, key: &str, node: &Value, out: &mut BTreeSet<(String, String)>) {
        let text = format!("{key} {}", node.get("description").and_then(Value::as_str).unwrap_or_default()).to_lowercase();
        let string = node.get("type").and_then(Value::as_str) == Some("string")
            || node.get("type").and_then(Value::as_str) == Some("array") && node.pointer("/items/type").and_then(Value::as_str) == Some("string");
        let declared = command.name_params.iter().any(|(name, _)| *name == path) || command.targets == Some(path) && command.name_field.is_some();
        if string && node.get("enum").is_none() && NOUNS.iter().any(|noun| text.contains(noun)) && !declared {
            out.insert((command.id.clone(), path.to_owned()));
        }
        for (child, value) in node.get("properties").and_then(Value::as_object).into_iter().flatten() {
            walk(command, &format!("{path}.{child}"), child, value, out);
        }
        for (child, value) in node.pointer("/items/properties").and_then(Value::as_object).into_iter().flatten() {
            walk(command, &format!("{path}[].{child}"), child, value, out);
        }
    }
    let mut out = BTreeSet::new();
    for command in commands {
        for (key, node) in command.schema.as_ref().and_then(Value::as_object).into_iter().flatten() {
            walk(command, key, key, node, &mut out);
        }
    }
    out
}

#[test]
fn every_name_parameter_looks_up_a_declared_type_or_says_what_it_is() {
    let commands = commands();
    let found = undeclared_names(&commands);
    let mut allowed = BTreeSet::new();
    for line in NAMES.lines().filter(|line| !line.trim().is_empty()) {
        let mut fields = line.splitn(3, '\t');
        let (Some(id), Some(param), Some(reason)) = (fields.next(), fields.next(), fields.next()) else { panic!("bad line {line:?}") };
        assert!(reason.trim().len() > 8, "{id} {param}: give the reason");
        allowed.insert((id.to_owned(), param.to_owned()));
    }
    let undeclared: Vec<_> = found.difference(&allowed).collect();
    assert!(undeclared.is_empty(), "name parameters neither declared nor explained: {undeclared:?}");
    let stale: Vec<_> = allowed.difference(&found).collect();
    assert!(stale.is_empty(), "explained but declared now (or gone): {stale:?}");
    let declared: usize = commands.iter().map(|command| command.name_params.len()).sum();
    println!("C8 name scanner: {declared} declared; {} explained", allowed.len());
}

#[test]
fn node_ids_survive_replica_and_file_reopens() {
    use kition_kit::session::model::Exchange;
    let mut doc = VectorDocument::create("Ids.kvector", 320, 200).unwrap();
    let a = doc.execute("shape.rectangle", json!({"x":10,"y":10,"width":20,"height":20})).unwrap()["id"].clone();
    let b = doc.execute("shape.rectangle", json!({"x":60,"y":10,"width":20,"height":20})).unwrap()["id"].clone();
    doc.execute("edit.delete", json!({"ids":[a]})).unwrap();
    let saved = doc.save(None).unwrap();
    let mut reopened = VectorDocument::open("Ids.kvector", &saved).unwrap();
    let mut exchange = Exchange::new();
    let loaded = exchange.batch(&mut reopened, &json!({"ops":[],"load":[{"target":b.to_string(),"fields":["id","name"]}]}), false).unwrap();
    assert_eq!(loaded["loaded"][0]["fields"]["id"], b);
    let ids: Vec<Value> = reopened.inspect().unwrap()["layers"].as_array().unwrap().iter().map(|n| n["id"].clone()).collect();
    assert!(!ids.is_empty());
}

#[test]
fn tree_property_calls_hold_only_written_fields_and_moves_hold_the_gap() {
    let mut doc = VectorDocument::create("Tree.kvector", 320, 200).unwrap();
    let id = doc.execute("shape.rectangle", json!({"x":10,"y":10,"width":20,"height":20})).unwrap()["id"].clone();
    let property = doc.under("object.setProps", &json!({"ids":[id],"name":"New"})).unwrap().unwrap();
    assert_eq!(property.premises[0].fields, Some(vec!["name".into()]));
    assert_eq!(property.premises[0].positions, Some(vec![]));
    let layer = doc.inspect().unwrap()["layers"][0]["id"].clone();
    let moved = doc.under("node.move", &json!({"id":id,"parent":layer,"index":0})).unwrap().unwrap();
    assert_eq!(moved.within[0].parent, layer);
    assert_eq!((moved.within[0].from, moved.within[0].to), (0, 0));
    assert_eq!(moved.premises[0].positions, Some(vec!["nodes".into()]));
}

fn two_rectangles() -> (VectorDocument, Value, Value) {
    let mut doc = VectorDocument::create("Targets.kvector", 320, 200).unwrap();
    let a = doc.execute("shape.rectangle", json!({"x":10,"y":10,"width":20,"height":20})).unwrap()["id"].clone();
    let b = doc.execute("shape.rectangle", json!({"x":60,"y":10,"width":20,"height":20})).unwrap()["id"].clone();
    doc.execute("select.set", json!({"ids":[a]})).unwrap();
    (doc, a, b)
}

fn node(doc: &VectorDocument, id: &Value) -> Value {
    doc.execute_query("file.node", json!({"id": id}))
}

trait Query {
    fn execute_query(&self, command: &str, params: Value) -> Value;
}

impl Query for VectorDocument {
    fn execute_query(&self, command: &str, params: Value) -> Value {
        let bytes = self.save(None).unwrap();
        VectorDocument::open("Query.kvector", &bytes).unwrap().execute(command, params).unwrap()
    }
}

/// A declared target is what the handler acts on: given, it wins over the person's selection.
#[test]
fn declared_targets_act_on_the_given_objects_not_the_selection() {
    let commands = commands();
    for (command, params) in [
        ("paint.setFill", json!({"color":"#ff0000"})),
        ("object.setProps", json!({"name":"Given"})),
        ("transparency.set", json!({"opacity":40})),
        ("stroke.set", json!({"weight":7})),
    ] {
        let declared = commands.iter().find(|c| c.id == command).unwrap();
        let (mut doc, a, b) = two_rectangles();
        let target = declared.targets.unwrap();
        let mut params = params;
        params[target] = if target == "ids" { json!([b]) } else { b.clone() };
        let (a_before, b_before) = (node(&doc, &a), node(&doc, &b));
        doc.execute(command, params).unwrap();
        assert_eq!(node(&doc, &a), a_before, "{command} changed the selected object instead of its target");
        assert_ne!(node(&doc, &b), b_before, "{command} left its target {b} unchanged");
        assert_eq!(declared.object_types.iter().find(|(name, _)| *name == target).unwrap().1, &["Node"]);
        assert!(declared.reads.contains(&kition_kit::EditorState::Selection), "{command} falls back to the selection");
    }
}

/// A command whose handler always reads the selection says so, and takes no target to fill.
#[test]
fn selection_only_commands_read_the_selection() {
    let commands = commands();
    for id in ["object.move", "arrange.group", "object.compoundPath.make", "object.pathfinder.unite", "object.clippingMask.make"] {
        let command = commands.iter().find(|c| c.id == id).unwrap();
        assert!(command.reads.contains(&kition_kit::EditorState::Selection), "{id}");
        assert_eq!(command.targets, None, "{id}");
    }
    let (mut doc, a, b) = two_rectangles();
    let (a_before, b_before) = (node(&doc, &a), node(&doc, &b));
    doc.execute("object.move", json!({"dx":5,"dy":0})).unwrap();
    assert_eq!(node(&doc, &b), b_before);
    assert_ne!(node(&doc, &a), a_before, "object.move moves the selected object");
    let creates = commands.iter().find(|c| c.id == "shape.rectangle").unwrap();
    assert!(creates.reads.contains(&kition_kit::EditorState::Active), "new art goes to the current layer");
}

/// Artboards, slices and export assets keep their numbers as tool ids across a save and a reopen.
#[test]
fn artboards_slices_and_assets_have_tool_ids() {
    let schema: Value = serde_json::from_str(include_str!("../../../crates/kdm/kdm-model/model/schema.json")).unwrap();
    assert_eq!(schema["types"]["Node"]["toolId"]["field"], "id");
    assert_eq!(schema["types"]["Slice"]["toolId"]["field"], "id");
    assert_eq!(schema["types"]["Asset"]["toolId"]["field"], "id");
    assert_eq!(schema["types"]["Artboard"]["toolId"]["prefix"], "artboard:");
    for name in ["Swatch", "SwatchGroup", "GraphicStyle", "CharStyle", "ParaStyle", "Symbol", "Pattern"] {
        assert_eq!(schema["types"][name]["label"], "name", "{name} is looked up by name");
    }
}
