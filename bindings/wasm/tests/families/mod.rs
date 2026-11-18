//! What `saved_families.rs` and `svg_families.rs` share: the fixtures, one write of each
//! command family, and diffs named by tool id.
#![allow(dead_code)]

use kition_kit::KitDocument;
use kition_vector_kit::VectorDocument;
use serde_json::{Value, json};

pub fn fixtures() -> Vec<std::path::PathBuf> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut paths: Vec<_> = ["conformance", "examples"]
        .iter()
        .flat_map(|dir| std::fs::read_dir(root.join(dir)).unwrap().map(|p| p.unwrap().path()))
        .filter(|p| p.extension().is_some_and(|e| e == "kvector"))
        .collect();
    paths.sort();
    paths
}

/// One write: the command, its parameters, and the objects selected for it (`None`: as left).
pub type Write = (&'static str, Value, Option<Vec<Value>>);

/// The objects the writes act on, made in `doc`, and one write of each command family.
pub fn writes(doc: &mut VectorDocument) -> Vec<Write> {
    let mut run = |command: &str, params: Value| doc.execute(command, params).unwrap_or_else(|e| panic!("setup {command}: {e}"));
    run("select.none", json!({}));
    let a = run("shape.rectangle", json!({"x":10,"y":10,"width":40,"height":30}))["id"].clone();
    let b = run("shape.ellipse", json!({"x":30,"y":20,"width":40,"height":40}))["id"].clone();
    let c = run("shape.rectangle", json!({"x":120,"y":10,"width":30,"height":30}))["id"].clone();
    let d = run("shape.star", json!({"cx":200,"cy":60,"radius1":20,"radius2":10}))["id"].clone();
    let open = run("path.create", json!({"anchors":[{"x":10,"y":100},{"x":60,"y":130},{"x":110,"y":100}]}))["id"].clone();
    let text = run("text.create", json!({"x":10,"y":200,"text":"Saved text"}))["id"].clone();
    let area = run("text.create", json!({"x":150,"y":150,"text":"Area one","area":{"width":80,"height":40}}))["id"].clone();
    let area2 = run("text.create", json!({"x":250,"y":150,"text":"Area two","area":{"width":80,"height":40}}))["id"].clone();
    run("swatch.new", json!({"name":"Saved","color":"#336699","global":true}));
    run("charStyle.new", json!({"name":"Saved char"}));
    run("paraStyle.new", json!({"name":"Saved para"}));
    let g1 = run("shape.rectangle", json!({"x":300,"y":10,"width":20,"height":20}))["id"].clone();
    let g2 = run("shape.rectangle", json!({"x":330,"y":10,"width":20,"height":20}))["id"].clone();
    run("select.set", json!({"ids":[g1.clone(), g2.clone()]}));
    let group = run("object.group", json!({}))["id"].clone();
    let s = run("shape.rectangle", json!({"x":400,"y":10,"width":20,"height":20}))["id"].clone();
    run("symbol.new", json!({"name":"Saved symbol","ids":[s]}));
    run("graphicStyle.new", json!({"name":"Saved style","id":a.clone()}));
    // An artboard of its own (a file may have none), the last.
    let board = run("artboard.new", json!({"x":2000,"y":0,"width":100,"height":100,"name":"Setup board"}))["index"].clone();
    run("select.none", json!({}));
    let sel = |ids: &[&Value]| Some(ids.iter().map(|id| (*id).clone()).collect::<Vec<_>>());
    vec![
        ("object.setProps", json!({"ids":[a],"name":"Renamed","opacity":37}), None),
        ("object.move", json!({"dx":3.33333,"dy":-1.7}), sel(&[&a])),
        ("object.rotate", json!({"angle":12.5}), sel(&[&b])),
        ("object.scale", json!({"sx":1.25,"sy":0.8}), sel(&[&c])),
        ("object.transform", json!({"ids":[d],"matrix":[1,0.1,0,1,2,3]}), None),
        ("object.lock", json!({}), sel(&[&c])),
        ("object.hide", json!({}), sel(&[&c])),
        ("object.group", json!({}), sel(&[&a, &b])),
        ("object.ungroup", json!({}), sel(&[&group])),
        ("object.arrange.bringForward", json!({}), sel(&[&a])),
        ("object.align", json!({"horizontal":"left","to":"selection"}), sel(&[&a, &c])),
        ("object.pathfinder.unite", json!({}), sel(&[&a, &b])),
        ("object.pathfinder.minusFront", json!({}), sel(&[&a, &b])),
        ("object.pathfinder.divide", json!({}), sel(&[&a, &b])),
        ("object.clippingMask.make", json!({}), sel(&[&a, &b])),
        ("object.compoundPath.make", json!({}), sel(&[&a, &c])),
        ("object.blend.make", json!({"ids":[a, c],"steps":3}), None),
        ("object.envelope.makeWithWarp", json!({"style":"arc","bend":30}), sel(&[&c])),
        ("object.mesh.create", json!({"rows":2,"cols":2}), sel(&[&c])),
        ("object.expand", json!({}), sel(&[&d])),
        ("object.rasterize", json!({"ppi":72}), sel(&[&c])),
        ("object.slice.make", json!({}), sel(&[&c])),
        ("object.pattern.make", json!({"name":"Saved pattern","edit":false}), sel(&[&c])),
        ("object.textWrap.make", json!({}), sel(&[&c])),
        ("path.join", json!({}), sel(&[&open])),
        ("edit.clear", json!({"ids":[b]}), None),
        ("edit.duplicate", json!({"dx":5,"dy":5}), sel(&[&a])),
        ("node.move", json!({"id":a,"parent":group,"index":0}), None),
        ("layer.new", json!({"name":"Saved layer"}), None),
        ("paint.setFill", json!({"ids":[a],"color":"#123456"}), None),
        ("paint.setFill", json!({"ids":[a],"swatch":"Saved"}), None),
        ("paint.setStroke", json!({"ids":[a],"color":{"c":10,"m":20,"y":30,"k":40}}), None),
        ("stroke.set", json!({"ids":[a],"weight":2.345,"dash":[3,2]}), None),
        ("transparency.set", json!({"ids":[a],"opacity":33,"blend":"Multiply"}), None),
        ("transparency.makeOpacityMask", json!({"ids":[a, b]}), None),
        ("appearance.addFill", json!({"ids":[a]}), None),
        ("appearance.setItem", json!({"ids":[a],"index":0,"opacity":50}), None),
        ("effect.apply", json!({"ids":[a],"effect":"stylize.dropShadow","params":{"x":3,"y":3}}), None),
        ("graphicStyle.apply", json!({"name":"Saved style","ids":[c]}), None),
        ("attributes.set", json!({"overprintFill":true}), sel(&[&a])),
        ("swatch.new", json!({"name":"Another","color":"#abcdef"}), None),
        ("swatch.edit", json!({"name":"Saved","color":"#ff0000"}), None),
        ("swatch.delete", json!({"names":["Saved"]}), None),
        ("swatch.newGroup", json!({"name":"Saved group","colors":["#010203"]}), None),
        ("text.editRange", json!({"id":text,"start":0,"end":5,"insert":"Changed"}), None),
        ("text.setStyle", json!({"ids":[text],"size":17.5,"tracking":25}), None),
        ("text.setFormat", json!({"ids":[text],"baselineShift":1.5}), None),
        ("text.thread.create", json!({}), sel(&[&area, &area2])),
        ("type.createOutlines", json!({}), sel(&[&text])),
        ("charStyle.setAttrs", json!({"name":"Saved char","attrs":{"size":13.25}}), None),
        ("charStyle.apply", json!({"name":"Saved char","ids":[text]}), None),
        ("paraStyle.setAttrs", json!({"name":"Saved para","attrs":{"space_before":3.25}}), None),
        ("symbol.place", json!({"name":"Saved symbol","x":50,"y":300}), None),
        ("artboard.new", json!({"x":600,"y":0,"width":200,"height":100,"name":"Saved board"}), None),
        ("artboard.setProps", json!({"index":board,"name":"Renamed board","width":333.3}), None),
        ("guide.add", json!({"vertical":true,"pos":123.456}), None),
        ("livePaint.make", json!({"ids":[a, c]}), None),
        ("dataset.new", json!({"name":"Saved data"}), None),
        ("recolor.apply", json!({"map":{"#000000":"#ff00ff"}}), sel(&[&a])),
        ("document.setup", json!({"bleed":4.25}), None),
        ("shape.polygon", json!({"cx":500,"cy":300,"radius":20,"sides":7}), None),
        ("perspective.grid.set", json!({"cell":37.5,"horizon":211.25}), None),
    ]
}

/// `diff` with each object id of `states` replaced by what names the object: its tool id field
/// (`id`, `key`) with its type, else its type and name.
pub fn labelled(diff: Value, states: &[&Value]) -> Value {
    let mut names = std::collections::HashMap::new();
    for state in states {
        for (id, object) in state["objects"].as_object().into_iter().flatten() {
            let ty = object["$type"].as_str().unwrap_or("?");
            let own = ["key", "id", "name"]
                .iter()
                .find_map(|field| object.get(*field).filter(|v| !v.is_null()))
                .map_or_else(|| "?".to_owned(), Value::to_string);
            names.entry(id.clone()).or_insert_with(|| format!("{ty}:{own}"));
        }
    }
    fn walk(value: Value, names: &std::collections::HashMap<String, String>) -> Value {
        match value {
            Value::String(text) => Value::String(names.get(&text).cloned().unwrap_or(text)),
            Value::Array(items) => Value::Array(items.into_iter().map(|item| walk(item, names)).collect()),
            Value::Object(map) => {
                Value::Object(map.into_iter().map(|(key, value)| (names.get(&key).cloned().unwrap_or(key), walk(value, names))).collect())
            }
            other => other,
        }
    }
    let mut out = walk(diff, &names);
    // Lists come sorted by object id; the replicas' ids differ, so sort by what names them.
    for key in ["created", "changed", "deleted", "moved", "texts"] {
        if let Some(items) = out.get_mut(key).and_then(Value::as_array_mut) {
            items.sort_by_key(|item| format!("{}{}", item["id"], item["field"]));
        }
    }
    out
}

/// The first place `a` and `b` differ, as a path and both values (cut short).
pub fn first_difference(a: &Value, b: &Value, path: &str) -> String {
    match (a, b) {
        (Value::Object(x), Value::Object(y)) => {
            for key in x.keys().chain(y.keys()) {
                let (u, v) = (x.get(key).unwrap_or(&Value::Null), y.get(key).unwrap_or(&Value::Null));
                if u != v {
                    return first_difference(u, v, &format!("{path}/{key}"));
                }
            }
            String::new()
        }
        (Value::Array(x), Value::Array(y)) if x.len() == y.len() => {
            for (i, (u, v)) in x.iter().zip(y).enumerate() {
                if u != v {
                    return first_difference(u, v, &format!("{path}/{i}"));
                }
            }
            String::new()
        }
        _ => {
            let cut = |v: &Value| v.to_string().chars().take(160).collect::<String>();
            format!("{path}: {} != {}", cut(a), cut(b))
        }
    }
}
