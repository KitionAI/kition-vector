//! Type on the canvas for the editor (plan 2026-11-25, stage 5): what the Type tool edits and the
//! attributes the Character and Paragraph panels show (`typing` in `inspect()`), and the requests
//! the page gives the Type tool beyond keys and typed text (`Type:` keys, [`REQUEST`]): an input
//! method's marked text, a new click count, a styled copy, and what new type is set in.

use kition_vector_doc::{CharStyle, Document, Justify, NodeId, NodeKind, ParaStyle, Selection, TextObject};
use kition_vector_engine::Session;
use kition_vector_geom::Point;
use serde_json::{Map, Value, json};

/// A key the page gives the Type tool that is a request, not a key: `Type:` and a JSON object
/// (`{"preedit": {text, from?, to?}}`, `{"resetClicks": true}`, `{"copy": true}`,
/// `{"newType": {font?, style?, size?}}`).
pub const REQUEST: &str = "Type:";

/// The Type tools (the tool the options of new type live in is `type`; the others share them).
const TYPE_TOOL: &str = "type";

/// What the Type tool edits now, kept after every tool event and command (its caret line needs
/// the session mutably, which `inspect()` does not have).
#[derive(Clone, Debug, Default)]
pub struct Editing {
    /// The type object edited, with its selected byte range and whether an input method composes.
    pub object: Option<(NodeId, usize, usize, bool)>,
    /// The caret line, or the start of the clause an input method converts (points).
    pub caret: Option<(Point, Point)>,
    /// What new type is set in: `{font, style, size}`, each null for the engine's default.
    pub new_type: Value,
}

/// What the Type tool edits in `session` now.
pub fn editing(session: &mut Session, view: kition_vector_engine::ViewInfo) -> Editing {
    let options = session.tool_options();
    let id = options.get("editing").and_then(Value::as_u64).map(NodeId);
    let offset = |key: &str| options.get(key).and_then(Value::as_u64).and_then(|v| usize::try_from(v).ok()).unwrap_or(0);
    let object = id.map(|id| (id, offset("start"), offset("end"), options.get("composing").and_then(Value::as_bool).unwrap_or(false)));
    let caret = if object.is_some() { session.tool_ime_caret(view) } else { None };
    let new_options = if options.get("newFont").is_some() { options } else { session.tool_options_of(TYPE_TOOL) };
    let new_type = json!({ "font": new_options["newFont"], "style": new_options["newStyle"], "size": new_options["newSize"] });
    Editing { object, caret, new_type }
}

/// `typing` in `inspect()`: `{editing, start, end, composing, caret: [ax, ay, bx, by] | null,
/// newType, character, paragraph}` with offsets in bytes of the edited text, the caret line in
/// document pixels (less `origin`), and the character and paragraph attributes of what the
/// panels act on: the selected characters (or the one before the caret) while editing, else every
/// selected type object; each attribute null where they differ (`mixed` names those).
pub fn report(doc: &Document, selection: &Selection, editing: &Editing, origin: Point) -> Value {
    let (character, paragraph) = attributes(doc, selection, editing);
    let caret = editing.caret.map(|(a, b)| json!([a.x - origin.x, a.y - origin.y, b.x - origin.x, b.y - origin.y]));
    let mut out = match editing.object {
        Some((id, start, end, composing)) => {
            let len = text_of(doc, id).map_or(0, |t| t.plain_text().len());
            json!({ "editing": id.0, "start": start.min(len), "end": end.min(len), "composing": composing })
        }
        None => json!({ "editing": null }),
    };
    out["caret"] = caret.unwrap_or(Value::Null);
    out["newType"] = editing.new_type.clone();
    out["character"] = character;
    out["paragraph"] = paragraph;
    out
}

fn text_of(doc: &Document, id: NodeId) -> Option<&TextObject> {
    match &doc.node(id)?.kind {
        NodeKind::Text(t) => Some(t),
        _ => None,
    }
}

/// The type objects the selection holds, groups looked into.
fn selected_texts<'a>(doc: &'a Document, selection: &Selection) -> Vec<&'a TextObject> {
    let mut out: Vec<&'a TextObject> = Vec::new();
    for id in &selection.objects {
        let Some(node) = doc.node(*id) else { continue };
        node.walk(&mut |n: &'a kition_vector_doc::Node| {
            if let NodeKind::Text(t) = &n.kind {
                out.push(t);
            }
        });
    }
    out
}

/// The character attributes and the paragraph attributes the panels show (null when no type is
/// edited or selected).
fn attributes(doc: &Document, selection: &Selection, editing: &Editing) -> (Value, Value) {
    let mut chars: Vec<CharStyle> = Vec::new();
    let mut paras: Vec<ParaStyle> = Vec::new();
    match editing.object.and_then(|(id, start, end, _)| Some((text_of(doc, id)?, start, end))) {
        Some((t, start, end)) => {
            let text = t.plain_text();
            let (a, b) = (start.min(end).min(text.len()), start.max(end).min(text.len()));
            let (a, b) = if a < b {
                (a, b)
            } else {
                // A caret: the character before it styles what is typed there (the first one at the start).
                let before = text.get(..a).and_then(|head| head.char_indices().next_back()).map(|(i, _)| i);
                match before {
                    Some(i) => (i, a),
                    None => (0, text.get(a..).and_then(|tail| tail.chars().next()).map_or(0, char::len_utf8)),
                }
            };
            chars.extend(kition_vector_text::edit::slice_runs(&t.runs, a, b).into_iter().map(|run| run.style));
            if chars.is_empty() {
                chars.push(t.first_style());
            }
            paras.extend(t.paragraphs_in(start, end).map(|i| t.para_at(i).clone()));
        }
        None => {
            for t in selected_texts(doc, selection) {
                chars.extend(t.runs.iter().filter(|run| !run.text.is_empty()).map(|run| run.style.clone()));
                if t.runs.iter().all(|run| run.text.is_empty()) {
                    chars.push(t.first_style());
                }
                paras.extend(t.paragraph_styles());
            }
        }
    }
    (common(chars.iter().map(character)), common(paras.iter().map(paragraph)))
}

/// The attributes all of `rows` share, null where they differ (named in `mixed`); null for none.
fn common(rows: impl Iterator<Item = Map<String, Value>>) -> Value {
    let mut out: Option<Map<String, Value>> = None;
    let mut mixed: Vec<String> = Vec::new();
    for row in rows {
        match out.as_mut() {
            None => out = Some(row),
            Some(all) => {
                for (key, value) in all.iter_mut() {
                    if !value.is_null() && row.get(key) != Some(value) {
                        *value = Value::Null;
                        mixed.push(key.clone());
                    }
                }
            }
        }
    }
    let Some(mut all) = out else { return Value::Null };
    all.insert("mixed".into(), json!(mixed));
    Value::Object(all)
}

/// A character style as the Character panel names its fields (`text.setRangeStyle`'s parameters).
fn character(style: &CharStyle) -> Map<String, Value> {
    let auto = |value: Option<f64>| value.map_or(json!("auto"), |v| json!(v));
    let json = json!({
        "font": style.font_family,
        "style": style.font_style,
        "size": style.size,
        "leading": auto(style.leading),
        "tracking": style.tracking,
        "kerning": auto(style.kerning),
        "baselineShift": style.baseline_shift,
        "hScale": style.h_scale,
        "vScale": style.v_scale,
        "rotation": style.rotation,
        "fill": style.fill.label(),
        "stroke": style.stroke.label(),
        "strokeWidth": style.stroke_width,
        "underline": style.underline,
        "strikethrough": style.strikethrough,
        "allCaps": style.all_caps,
        "smallCaps": style.small_caps.is_some(),
        "position": style.position.id(),
    });
    json.as_object().cloned().unwrap_or_default()
}

/// A paragraph style as the Paragraph panel names its fields (`text.setFormat` and `text.setStyle`'s `justify`).
fn paragraph(style: &ParaStyle) -> Map<String, Value> {
    let justify = match style.justify {
        Justify::Auto => "auto",
        Justify::Left => "left",
        Justify::Center => "center",
        Justify::Right => "right",
        Justify::JustifyLeft => "justifyLeft",
        Justify::JustifyCenter => "justifyCenter",
        Justify::JustifyRight => "justifyRight",
        Justify::JustifyAll => "justifyAll",
    };
    let json = json!({
        "justify": justify,
        "leftIndent": style.left_indent,
        "rightIndent": style.right_indent,
        "firstLineIndent": style.first_line_indent,
        "spaceBefore": style.space_before,
        "spaceAfter": style.space_after,
        "hyphenate": style.hyphenate,
    });
    json.as_object().cloned().unwrap_or_default()
}

/// Runs a [`REQUEST`] (`body`, its JSON) on the Type tool; answers what the tool asks of the page.
pub fn request(session: &mut Session, body: &str, view: kition_vector_engine::ViewInfo) -> Result<Vec<kition_vector_engine::UiRequest>, String> {
    let request: Value = serde_json::from_str(body).map_err(|error| format!("invalid-params: a Type request is JSON: {error}"))?;
    if let Some(preedit) = request.get("preedit") {
        // An input method's marked text (characters `from..to` are the clause it converts).
        let text = preedit.get("text").and_then(Value::as_str).unwrap_or("");
        let at = |key: &str| preedit.get(key).and_then(Value::as_u64).and_then(|v| usize::try_from(v).ok());
        let active = match (at("from"), at("to")) {
            (Some(from), Some(to)) if from <= to => Some(from..to),
            _ => None,
        };
        return session.tool_preedit(text, active, view).map_err(|error| error.to_string());
    }
    if request.get("resetClicks").and_then(Value::as_bool) == Some(true) {
        session.set_tool_option("resetClicks", &json!(true));
    }
    if request.get("copy").and_then(Value::as_bool) == Some(true) {
        // The styled copy of the selected characters: pasting the same text keeps their style.
        let options = session.tool_options();
        let id = options.get("editing").and_then(Value::as_u64).map(NodeId);
        let offset = |key: &str| options.get(key).and_then(Value::as_u64).and_then(|v| usize::try_from(v).ok()).unwrap_or(0);
        if let Some(t) = id.and_then(|id| session.active().and_then(|state| text_of(&state.doc, id)).cloned()) {
            let runs = kition_vector_text::edit::slice_runs(&t.runs, offset("start"), offset("end"));
            session.set_tool_option("copy", &serde_json::to_value(runs).unwrap_or(Value::Null));
        }
    }
    if let Some(new_type) = request.get("newType").and_then(Value::as_object) {
        let mut values = Map::new();
        for (key, option) in [("font", "newFont"), ("style", "newStyle"), ("size", "newSize")] {
            if let Some(value) = new_type.get(key) {
                values.insert(option.into(), value.clone());
            }
        }
        session.set_tool_options(Some(TYPE_TOOL), &values);
    }
    Ok(Vec::new())
}
