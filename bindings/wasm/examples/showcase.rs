//! Builds one showcase sample (KitionAI/kition-showcase) with the kit's own commands and renders
//! its thumbnails with the kit's own engine. The sample is a JSON request, written by the
//! showcase's `tools/build.mjs` from its `showcase.yaml`:
//!
//! ```json
//! { "create": { "name": "x.kvector", "width": 1200, "height": 800 },
//!   "fonts": ["/abs/DroidSansFallbackFull.ttf"],
//!   "steps": [{ "command": "layer.new.layer", "params": { "name": "Ink" }, "as": "ink" },
//!             { "command": "layer.setProps", "params": { "layer": "$ink.layer", "opacity": 0.5 } }],
//!   "save": "/abs/out/x.kvector",
//!   "renders": [{ "file": "/abs/out/thumb.png", "max": 640, "area": null,
//!                 "steps": [] }],
//!   "inspect": "/abs/out/inspect.json" }
//! ```
//!
//! A string parameter `"$name.path.0"` is the value at that path in the result of the step saved
//! `as` `name` (`"$$…"` is a literal `$…`); a render's `area` may also name `$inspect.…`. The report (named results, size, diagnostics, timings)
//! is printed on stdout as JSON; `--commands` prints the command catalogue instead. `cargo run --release -p kition-vector-kit --example showcase -- req.json`

use std::path::Path;
use std::time::Instant;

use kition_kit::KitDocument;
use kition_vector_kit::VectorDocument as Doc;
use serde_json::{Map, Value, json};

/// Adds a sample's font to the engine's font database (a face already there counts as added).
fn add_font(bytes: &[u8]) -> usize {
    kition_vector_text::FontDb::global().add_font(bytes.to_vec());
    1
}

fn resolve(value: &Value, results: &Map<String, Value>) -> Result<Value, String> {
    Ok(match value {
        Value::String(text) if text.starts_with("$$") => Value::String(text[1..].to_owned()),
        Value::String(text) if text.starts_with('$') => {
            let mut path = text[1..].split('.');
            let name = path.next().unwrap_or_default();
            let mut at = results.get(name).ok_or_else(|| format!("{text}: no step was saved as {name}"))?;
            for key in path {
                at = match key.parse::<usize>() {
                    Ok(index) if at.is_array() => at.get(index),
                    _ => at.get(key),
                }
                .ok_or_else(|| format!("{text}: the result of {name} has no {key}: {at}"))?;
            }
            at.clone()
        }
        Value::Array(items) => Value::Array(items.iter().map(|item| resolve(item, results)).collect::<Result<_, _>>()?),
        Value::Object(fields) => {
            Value::Object(fields.iter().map(|(key, item)| Ok((key.clone(), resolve(item, results)?))).collect::<Result<_, String>>()?)
        }
        other => other.clone(),
    })
}

fn run_steps(doc: &mut Doc, steps: Option<&Value>, results: &mut Map<String, Value>) -> Result<usize, String> {
    let steps = steps.and_then(Value::as_array).map(Vec::as_slice).unwrap_or_default();
    for (index, step) in steps.iter().enumerate() {
        let command = step["command"].as_str().ok_or_else(|| format!("step {index}: no command"))?;
        let params = resolve(step.get("params").unwrap_or(&json!({})), results)?;
        let result = doc.execute(command, params).map_err(|error| format!("step {index} ({command}): {error}"))?;
        if let Some(name) = step.get("as").and_then(Value::as_str) {
            results.insert(name.to_owned(), result);
        }
    }
    Ok(steps.len())
}

fn write(path: &str, bytes: &[u8]) -> Result<(), String> {
    if let Some(dir) = Path::new(path).parent() {
        std::fs::create_dir_all(dir).map_err(|error| format!("{}: {error}", dir.display()))?;
    }
    std::fs::write(path, bytes).map_err(|error| format!("{path}: {error}"))
}

/// The document, or its `area` ([x, y, width, height] in document pixels), at most `max` pixels
/// wide and high, as PNG at `path`.
fn render(doc: &Doc, max: u32, area: Option<&Value>, path: &str) -> Result<[u32; 2], String> {
    let [x, y, width, height] = match area {
        Some(area) => {
            // [x, y, width, height] or { x, y, width, height }, as kits answer page and slide areas.
            let side = |index: usize, key: &str| {
                area.get(index).or_else(|| area.get(key)).and_then(Value::as_f64).ok_or_else(|| format!("area: [x, y, width, height], got {area}"))
            };
            [side(0, "x")?, side(1, "y")?, side(2, "width")?.max(1.0), side(3, "height")?.max(1.0)]
        }
        None => [0.0, 0.0, f64::from(doc.width().max(1)), f64::from(doc.height().max(1))],
    };
    let scale = (f64::from(max) / width.max(height)).min(1.0);
    let (w, h) = (((width * scale).round() as u32).max(1), ((height * scale).round() as u32).max(1));
    let rgba = doc.render(x, y, scale, w, h)?;
    let picture = image::RgbaImage::from_raw(w, h, rgba).ok_or("a render of the wrong size")?;
    let mut png = Vec::new();
    picture.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png).map_err(|error| error.to_string())?;
    write(path, &png)?;
    Ok([w, h])
}

fn build(request: &Value) -> Result<Value, String> {
    let started = Instant::now();
    for font in request["fonts"].as_array().map(Vec::as_slice).unwrap_or_default() {
        let path = font.as_str().ok_or("fonts: a path")?;
        let bytes = std::fs::read(path).map_err(|error| format!("{path}: {error}"))?;
        if add_font(&bytes) == 0 {
            return Err(format!("{path}: this kit takes no extra fonts"));
        }
    }
    let mut doc = if let Some(path) = request["open"].as_str() {
        let bytes = std::fs::read(path).map_err(|error| format!("{path}: {error}"))?;
        Doc::open(path, &bytes)?
    } else {
        let create = &request["create"];
        let dimension = |key: &str| create[key].as_u64().and_then(|n| u32::try_from(n).ok()).ok_or_else(|| format!("create.{key}: a size"));
        Doc::create(create["name"].as_str().ok_or("create.name: a file name")?, dimension("width")?, dimension("height")?)?
    };
    let mut results = Map::new();
    let steps = run_steps(&mut doc, request.get("steps"), &mut results)?;
    let built_ms = started.elapsed().as_millis();

    let mut saved = Value::Null;
    if let Some(path) = request["save"].as_str() {
        let name = Path::new(path).file_name().and_then(|name| name.to_str());
        let bytes = doc.save(name)?;
        write(path, &bytes)?;
        // The saved file must open again: a sample nobody can reopen is no sample.
        Doc::open(path, &bytes).map_err(|error| format!("reopening {path}: {error}"))?;
        saved = json!({ "path": path, "bytes": bytes.len() });
    }

    let mut renders = Vec::new();
    for entry in request["renders"].as_array().map(Vec::as_slice).unwrap_or_default() {
        run_steps(&mut doc, entry.get("steps"), &mut results)?;
        let path = entry["file"].as_str().ok_or("renders: a file")?;
        // The area may name a place in the inspection: "$inspect.pages.0.area".
        results.insert("inspect".to_owned(), doc.inspect()?);
        let area = entry.get("area").filter(|area| !area.is_null()).map(|area| resolve(area, &results)).transpose()?;
        results.remove("inspect");
        let size = render(&doc, entry["max"].as_u64().unwrap_or(640) as u32, area.as_ref(), path)?;
        renders.push(json!({ "file": path, "size": size }));
    }

    let mut frames = 0;
    if let Some(spec) = request.get("frames").filter(|spec| spec.is_object()) {
        let dir = spec["dir"].as_str().ok_or("frames.dir: a directory")?;
        let seek = spec["seek"].as_str().ok_or("frames.seek: a command")?;
        // The parameter the seek command takes the time in, in seconds.
        let param = spec["param"].as_str().unwrap_or("seconds");
        let (from, to, fps) =
            (spec["from"].as_f64().unwrap_or(0.0), spec["to"].as_f64().ok_or("frames.to: seconds")?, spec["fps"].as_f64().unwrap_or(10.0));
        let max = spec["max"].as_u64().unwrap_or(320) as u32;
        while from + f64::from(frames) / fps < to {
            let seconds = from + f64::from(frames) / fps;
            let mut params = Map::new();
            params.insert(param.to_owned(), json!(seconds));
            doc.execute(seek, Value::Object(params)).map_err(|error| format!("{seek} {seconds}: {error}"))?;
            render(&doc, max, None, &format!("{dir}/{frames:04}.png"))?;
            frames += 1;
        }
    }

    let inspect = doc.inspect()?;
    if let Some(path) = request["inspect"].as_str() {
        write(path, serde_json::to_string_pretty(&inspect).map_err(|error| error.to_string())?.as_bytes())?;
    }
    let diagnostics: Vec<Value> = doc.check().iter().map(|diagnostic| serde_json::to_value(diagnostic).unwrap_or(Value::Null)).collect();
    Ok(json!({
        "width": doc.width(),
        "height": doc.height(),
        "summary": doc.summary(),
        "steps": steps,
        "results": results,
        "saved": saved,
        "renders": renders,
        "frames": frames,
        "diagnostics": diagnostics,
        "built_ms": built_ms,
        "total_ms": started.elapsed().as_millis(),
    }))
}

fn main() {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: showcase <request.json> | --commands");
        std::process::exit(2);
    };
    let outcome = if path == "--commands" {
        // The command catalogue with each command's parameters, for writing samples.
        Doc::create("commands.kvector", 64, 64).and_then(|doc| serde_json::to_string_pretty(&doc.commands()).map_err(|error| error.to_string()))
    } else {
        std::fs::read(&path)
            .map_err(|error| format!("{path}: {error}"))
            .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).map_err(|error| format!("{path}: {error}")))
            .and_then(|request| build(&request))
            .map(|report| report.to_string())
    };
    match outcome {
        Ok(text) => println!("{text}"),
        Err(error) => {
            println!("{}", json!({ "error": error }));
            std::process::exit(1);
        }
    }
}
