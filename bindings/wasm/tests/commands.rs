//! Every command says what it does and every read command an agent is offered says what it
//! answers (kition-sdk `testing::check_commands`, KHP A1, A5).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use kition_kit::KitDocument;
use kition_vector_kit::VectorDocument;

/// Read commands an agent is offered that answer nothing worth a schema, with why.
const NO_OUTPUT: &[(&str, &str)] = &[];

fn doc() -> VectorDocument {
    VectorDocument::create("Drawing.kvector", 800, 600).unwrap()
}

#[test]
fn every_command_says_what_it_does() {
    let commands = doc().commands();
    if let Ok(path) = std::env::var("KITION_COMMANDS_DUMP") {
        std::fs::write(path, serde_json::to_string(&commands).unwrap()).unwrap();
    }
    if let Err(problems) = kition_kit::testing::check_commands(&commands, NO_OUTPUT) {
        panic!("{} problems:\n{}", problems.len(), problems.join("\n"));
    }
}

/// With `KITION_COMMANDS_OBSERVE` set to a file, runs every offered read command with no
/// parameters on the conformance drawing (everything selected) and writes what each answered:
/// the evidence the output schemas in `agent-commands.json` are written from.
#[test]
fn observe_read_commands() {
    let Ok(path) = std::env::var("KITION_COMMANDS_OBSERVE") else { return };
    let bytes = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../conformance/basic.kvector")).unwrap();
    let mut seen = serde_json::Map::new();
    let reads: Vec<String> = VectorDocument::open("basic.kvector", &bytes)
        .unwrap()
        .commands()
        .into_iter()
        .filter(|c| c.schema.is_some() && !c.mutates)
        .map(|c| c.id)
        .collect();
    for id in reads {
        let mut doc = VectorDocument::open("basic.kvector", &bytes).unwrap();
        let _ = doc.execute("select.all", serde_json::json!({}));
        let answer = match doc.execute(&id, serde_json::json!({})) {
            Ok(value) => value,
            Err(error) => serde_json::json!({ "__error": error }),
        };
        seen.insert(id, answer);
    }
    std::fs::write(path, serde_json::to_string(&seen).unwrap()).unwrap();
}

/// Every entry of `agent-commands.json` names a command, types only parameters the command's
/// notation has, and gives output schemas the SDK accepts.
#[test]
fn agent_text_names_real_commands_and_parameters() {
    let table: serde_json::Map<String, serde_json::Value> = serde_json::from_str(kition_vector_kit::agent_text::SOURCE).unwrap();
    let notation: std::collections::HashMap<String, Option<serde_json::Value>> = doc()
        .commands()
        .into_iter()
        .map(|c| (c.id.clone(), c.params.as_deref().and_then(|p| kition_kit::schema::from_kit_params(p).ok()).map(|c| c.schema)))
        .collect();
    let mut problems = Vec::new();
    for (id, entry) in &table {
        let Some(schema) = notation.get(id) else {
            problems.push(format!("{id}: not a command"));
            continue;
        };
        for key in entry.get("params").and_then(|p| p.as_object()).map(|p| p.keys().cloned().collect::<Vec<_>>()).unwrap_or_default() {
            if schema.as_ref().and_then(|s| s.get(&key)).is_none() {
                problems.push(format!("{id}: types `{key}`, which its notation doesn't have"));
            }
        }
        if let Some(output) = entry.get("output")
            && let Err(error) = kition_kit::schema::validate_output(output)
        {
            problems.push(format!("{id}: output schema: {error}"));
        }
    }
    for command in doc().commands() {
        if let Some(schema) = &command.schema
            && let Err(error) = kition_kit::schema::validate(schema)
        {
            problems.push(format!("{}: parameter schema: {error}", command.id));
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

/// What a read command answers on the conformance drawing fits the output schema it declares.
#[test]
fn read_commands_answer_what_their_output_schemas_say() {
    let bytes = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../conformance/basic.kvector")).unwrap();
    let mut problems = Vec::new();
    for command in VectorDocument::open("basic.kvector", &bytes).unwrap().commands() {
        let (Some(output), true) = (command.output_schema.as_ref(), command.schema.is_some() && !command.mutates) else { continue };
        let mut doc = VectorDocument::open("basic.kvector", &bytes).unwrap();
        let _ = doc.execute("select.all", serde_json::json!({}));
        if let Ok(answer) = doc.execute(&command.id, serde_json::json!({}))
            && let Err(error) = fits(&answer, output, &command.id)
        {
            problems.push(error);
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

fn fits(value: &serde_json::Value, schema: &serde_json::Value, at: &str) -> Result<(), String> {
    use serde_json::Value;
    if let Some(branches) = schema.get("oneOf").and_then(Value::as_array) {
        return if branches.iter().any(|b| fits(value, b, at).is_ok()) { Ok(()) } else { Err(format!("{at}: {value} fits no branch")) };
    }
    let kind = schema.get("type").and_then(Value::as_str).unwrap_or("json");
    let ok = match kind {
        "json" => true,
        "null" => value.is_null(),
        "string" => value.is_string(),
        "number" => value.is_number(),
        "integer" => value.is_i64() || value.is_u64(),
        "boolean" => value.is_boolean(),
        "array" => value.is_array(),
        "object" => value.is_object(),
        _ => true,
    };
    if !ok {
        return Err(format!("{at}: expected {kind}, got {value}"));
    }
    if let (Some(items), Some(list)) = (schema.get("items"), value.as_array()) {
        for (i, item) in list.iter().enumerate() {
            fits(item, items, &format!("{at}[{i}]"))?;
        }
    }
    if let (Some(props), Some(object)) = (schema.get("properties").and_then(Value::as_object), value.as_object()) {
        for (key, inner) in props {
            if let Some(v) = object.get(key)
                && !v.is_null()
            {
                fits(v, inner, &format!("{at}.{key}"))?;
            }
        }
    }
    Ok(())
}
