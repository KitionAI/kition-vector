//! A story's characters as one rich `text` field: a type object's own (`Node.text`), or a
//! threaded story's (`Thread.text`, which the thread's area-type frames share).
//!
//! The engine keeps a story as runs ([`TextRun`]: characters sharing one [`CharStyle`], or one
//! inline graphic) and one [`ParaStyle`] per paragraph. The replica's text is the plain text plus
//! one final `\n`, so every paragraph, the last included, ends with a break that carries its
//! attributes:
//! - every character carries its run's character attributes, one mark per attribute as
//!   [`CharStyle`] serializes it (`font_family`, `size`, `fill`, …), all `expand: "after"`: text
//!   typed after a character takes its look, and two people formatting overlapping ranges with
//!   different attributes both keep theirs;
//! - each break carries its paragraph's attributes, one mark per attribute as [`ParaStyle`]
//!   serializes it, prefixed `para_` (`para_justify`, …), `expand: "none"`;
//! - an inline graphic is its object replacement character ([`INLINE_CHAR`]) with the `inline`
//!   mark holding its [`InlineArt`] (`expand: "none"`).
//!
//! Swatch and symbol names inside mark values (a fill's swatch, an inline graphic's symbol) are
//! written as `{ref, name}` ([`crate::refs`]). A read gives adjacent characters with the same look
//! one run, and the final break's look, when it differs from the text before it, an empty last
//! run (the look typing at the end takes).

use std::collections::BTreeMap;

use kition_kit::session::TextRun as Delta;
use kition_vector_doc::text::INLINE_CHAR;
use kition_vector_doc::{CharStyle, InlineArt, ParaStyle, TextRun};
use serde_json::{Map, Value};

use crate::refs::{Locals, Names, names_in, names_out};
use crate::{KdmResult, kdm_err};

/// The inline graphic mark.
pub(crate) const INLINE: &str = "inline";
/// What paragraph marks' keys start with.
pub(crate) const PARA: &str = "para_";

/// The character marks: [`CharStyle`]'s fields as it serializes them.
pub(crate) const CHAR_KEYS: &[&str] = &[
    "font_family",
    "font_style",
    "font_version",
    "size",
    "leading",
    "tracking",
    "kerning",
    "baseline_shift",
    "h_scale",
    "v_scale",
    "rotation",
    "fill",
    "stroke",
    "stroke_width",
    "underline",
    "strikethrough",
    "all_caps",
    "features",
    "style_name",
    "overprint_fill",
    "overprint_stroke",
    "stroke_cap",
    "stroke_join",
    "stroke_miter_limit",
    "stroke_dash",
    "position",
    "small_caps",
    "charAlign",
];

/// The paragraph attributes ([`ParaStyle`]'s fields as it serializes them), each the mark
/// [`PARA`]` + key`.
pub(crate) const PARA_KEYS: &[&str] = &[
    "justify",
    "justify_auto",
    "left_indent",
    "right_indent",
    "first_line_indent",
    "space_before",
    "space_after",
    "hyphenate",
    "composer",
    "tabs",
    "style_name",
    "mojikumi",
    "direction",
    "leading_model",
    "burasagari",
    "kinsoku",
];

type Marks = BTreeMap<String, Value>;

fn object_of(v: Value) -> Map<String, Value> {
    match v {
        Value::Object(map) => map,
        _ => Map::new(),
    }
}

/// `style`'s character marks.
fn char_marks(style: &CharStyle, names: &Names) -> KdmResult<Marks> {
    let mut v = serde_json::to_value(style).map_err(kdm_err)?;
    names_out(&mut v, names);
    Ok(object_of(v).into_iter().filter(|(_, v)| !v.is_null()).collect())
}

/// `para`'s paragraph marks.
fn para_marks(para: &ParaStyle, names: &Names) -> KdmResult<Marks> {
    let mut v = serde_json::to_value(para).map_err(kdm_err)?;
    names_out(&mut v, names);
    Ok(object_of(v).into_iter().filter(|(_, v)| !v.is_null()).map(|(k, v)| (format!("{PARA}{k}"), v)).collect())
}

/// Builds runs, merging neighbours with the same marks.
#[derive(Default)]
struct Runs {
    out: Vec<Delta>,
}

impl Runs {
    fn push(&mut self, text: &str, attributes: &Marks) {
        if text.is_empty() {
            return;
        }
        if let Some(last) = self.out.last_mut()
            && last.attributes == *attributes
        {
            last.insert.push_str(text);
            return;
        }
        self.out.push(Delta { insert: text.to_owned(), attributes: attributes.clone() });
    }
}

/// The text field of the story `runs` with one paragraph style per paragraph (`paras`; the last
/// one stands for any paragraph past its end, none for the default).
pub(crate) fn encode(runs: &[TextRun], paras: &[ParaStyle], names: &Names) -> KdmResult<Vec<Delta>> {
    let mut out = Runs::default();
    let mut para_cache: Vec<Option<Marks>> = vec![None; paras.len().max(1)];
    let mut para = |i: usize| -> KdmResult<Marks> {
        let at = i.min(para_cache.len().saturating_sub(1));
        if let Some(Some(marks)) = para_cache.get(at) {
            return Ok(marks.clone());
        }
        let marks = para_marks(&paras.get(at).cloned().unwrap_or_default(), names)?;
        if let Some(slot) = para_cache.get_mut(at) {
            *slot = Some(marks.clone());
        }
        Ok(marks)
    };
    let mut paragraph = 0usize;
    let mut last: Option<Marks> = None;
    for run in runs {
        let marks = char_marks(&run.style, names)?;
        let mut body = marks.clone();
        if let Some(art) = &run.inline {
            let mut v = serde_json::to_value(art).map_err(kdm_err)?;
            names_out(&mut v, names);
            body.insert(INLINE.to_owned(), v);
        }
        for piece in run.text.split_inclusive('\n') {
            match piece.strip_suffix('\n') {
                Some(text) => {
                    out.push(text, &body);
                    let mut brk = marks.clone();
                    brk.extend(para(paragraph)?);
                    paragraph += 1;
                    out.push("\n", &brk);
                }
                None => out.push(piece, &body),
            }
        }
        last = Some(marks);
    }
    let mut brk = match last {
        Some(marks) => marks,
        None => char_marks(&CharStyle::default(), names)?,
    };
    brk.extend(para(paragraph)?);
    out.push("\n", &brk);
    Ok(out.out)
}

/// A story read back from its text field.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Story {
    pub runs: Vec<TextRun>,
    /// One per paragraph.
    pub paras: Vec<ParaStyle>,
}

/// Reads a text field back (see the module docs). An empty field is an empty story with the
/// default look.
pub(crate) fn decode(delta: &[Delta], locals: &Locals) -> KdmResult<Story> {
    let base_char = object_of(serde_json::to_value(CharStyle::default()).map_err(kdm_err)?);
    let base_para = object_of(serde_json::to_value(ParaStyle::default()).map_err(kdm_err)?);
    let mut runs: Vec<TextRun> = Vec::new();
    let mut paras: Vec<ParaStyle> = Vec::new();
    for d in delta {
        let mut chars = base_char.clone();
        let mut para = base_para.clone();
        let mut inline = None;
        for (key, value) in &d.attributes {
            if value.is_null() {
                continue;
            }
            let mut value = value.clone();
            names_in(&mut value, locals);
            if key == INLINE {
                inline = serde_json::from_value::<InlineArt>(value).ok();
            } else if let Some(field) = key.strip_prefix(PARA).filter(|field| PARA_KEYS.contains(field)) {
                para.insert(field.to_owned(), value);
            } else if CHAR_KEYS.contains(&key.as_str()) {
                chars.insert(key.clone(), value);
            }
        }
        // A value no build of this kit wrote (another program's batch) gives the default look
        // rather than no drawing.
        let style: CharStyle = serde_json::from_value(Value::Object(chars)).unwrap_or_default();
        let para: ParaStyle = serde_json::from_value(Value::Object(para)).unwrap_or_default();
        let breaks = d.insert.matches('\n').count();
        paras.extend(std::iter::repeat_n(para, breaks));
        let push = |runs: &mut Vec<TextRun>, text: &str| {
            if text.is_empty() {
                return;
            }
            match runs.last_mut() {
                Some(last) if last.inline.is_none() && last.style == style => last.text.push_str(text),
                _ => runs.push(TextRun::new(text, style.clone())),
            }
        };
        match &inline {
            Some(art) => {
                let mut start = 0;
                for (at, c) in d.insert.char_indices() {
                    if c == INLINE_CHAR {
                        push(&mut runs, d.insert.get(start..at).unwrap_or_default());
                        runs.push(TextRun::inline(art.clone(), style.clone()));
                        start = at + c.len_utf8();
                    }
                }
                push(&mut runs, d.insert.get(start..).unwrap_or_default());
            }
            None => push(&mut runs, &d.insert),
        }
    }
    // The final break is the replica's, not the story's.
    if let Some(last) = runs.last_mut()
        && last.inline.is_none()
        && last.text.ends_with('\n')
    {
        last.text.pop();
    }
    // An emptied last run stays only as the look typing at the end takes, when it differs.
    if runs.len() > 1
        && let [.., before, last] = runs.as_slice()
        && last.text.is_empty()
        && last.style == before.style
    {
        runs.pop();
    }
    if runs.is_empty() {
        runs.push(TextRun::new("", CharStyle::default()));
    }
    Ok(Story { runs, paras })
}

/// `runs` as a read gives them back (see [`decode`]): neighbours with the same look merged, empty
/// runs dropped but for an empty last one whose look differs from the run before it, and one
/// empty run of the default look for none. For comparing views ([`crate::normalize`]).
pub(crate) fn canonical_runs(runs: &mut Vec<Value>) {
    let mut out: Vec<Value> = Vec::with_capacity(runs.len());
    let count = runs.len();
    for (i, run) in std::mem::take(runs).into_iter().enumerate() {
        let text = run.get("text").and_then(Value::as_str).unwrap_or_default().to_owned();
        let inline = run.get("inline").is_some_and(|v| !v.is_null());
        if text.is_empty() && !inline && i + 1 < count {
            continue;
        }
        if let Some(last) = out.last_mut()
            && !inline
            && last.get("inline").is_none_or(Value::is_null)
            && last.get("style") == run.get("style")
            && let Some(Value::String(t)) = last.get_mut("text")
        {
            t.push_str(&text);
            continue;
        }
        out.push(run);
    }
    if out.len() > 1
        && let [.., before, last] = out.as_slice()
        && last.get("text").and_then(Value::as_str) == Some("")
        && last.get("inline").is_none_or(Value::is_null)
        && last.get("style") == before.get("style")
    {
        out.pop();
    }
    if out.is_empty()
        && let Ok(run) = serde_json::to_value(TextRun::new("", CharStyle::default()))
    {
        out.push(run);
    }
    *runs = out;
}

#[cfg(test)]
mod tests {
    use super::*;
    use kition_vector_doc::color::{Color, Paint};
    use kition_vector_doc::{Justify, TextObject};

    fn round_trip(runs: &[TextRun], paras: &[ParaStyle]) -> Story {
        let delta = encode(runs, paras, &Names::default()).unwrap();
        assert!(delta.last().unwrap().insert.ends_with('\n'), "every paragraph ends with a break");
        decode(&delta, &Locals::default()).unwrap()
    }

    fn bold() -> CharStyle {
        CharStyle { font_style: "Bold".into(), fill: Paint::solid(Color::rgb(0.8, 0.1, 0.1)), ..CharStyle::default() }
    }

    #[test]
    fn runs_and_paragraphs_read_back() {
        let runs =
            vec![TextRun::new("Hello ", CharStyle::default()), TextRun::new("world\nSecond", bold()), TextRun::new(" para", CharStyle::default())];
        let centred = ParaStyle { justify: Justify::Center, left_indent: 12.0, ..ParaStyle::default() };
        let paras = vec![ParaStyle::default(), centred.clone()];
        let back = round_trip(&runs, &paras);
        assert_eq!(back.runs, runs);
        assert_eq!(back.paras, paras);
    }

    #[test]
    fn neighbours_with_one_look_become_one_run() {
        let runs = vec![TextRun::new("ab", bold()), TextRun::new("", CharStyle::default()), TextRun::new("cd", bold())];
        let back = round_trip(&runs, &[]);
        assert_eq!(back.runs, vec![TextRun::new("abcd", bold())]);
    }

    #[test]
    fn an_empty_story_keeps_its_look_and_a_trailing_look_stays() {
        let back = round_trip(&[TextRun::new("", bold())], &[]);
        assert_eq!(back.runs, vec![TextRun::new("", bold())]);
        assert_eq!(back.paras.len(), 1);
        let runs = vec![TextRun::new("abc", CharStyle::default()), TextRun::new("", bold())];
        assert_eq!(round_trip(&runs, &[]).runs, runs, "the look typing at the end takes");
        let back = decode(&[], &Locals::default()).unwrap();
        assert_eq!(back.runs, vec![TextRun::new("", CharStyle::default())]);
    }

    #[test]
    fn inline_graphics_are_one_character_each() {
        let art = InlineArt::new("Star");
        let runs = vec![
            TextRun::new("a", CharStyle::default()),
            TextRun::inline(art.clone(), CharStyle::default()),
            TextRun::inline(art.clone(), CharStyle::default()),
            TextRun::new("b", CharStyle::default()),
        ];
        let delta = encode(&runs, &[], &Names::default()).unwrap();
        let inline: Vec<&str> = delta.iter().filter(|d| d.attributes.contains_key(INLINE)).map(|d| d.insert.as_str()).collect();
        assert_eq!(inline.concat().chars().count(), 2);
        assert_eq!(decode(&delta, &Locals::default()).unwrap().runs, runs);
    }

    #[test]
    fn paragraph_marks_sit_on_breaks_only() {
        let runs = vec![TextRun::new("ab\ncd", CharStyle::default())];
        let delta = encode(&runs, &[], &Names::default()).unwrap();
        for d in &delta {
            let para = d.attributes.keys().any(|k| k.starts_with(PARA));
            assert_eq!(para, d.insert.chars().all(|c| c == '\n'), "{d:?}");
        }
    }

    #[test]
    fn every_attribute_is_a_declared_mark() {
        let schema: Value = serde_json::from_str(crate::SCHEMA).unwrap();
        let mut t = TextObject::point(kition_vector_doc::geom::Point::ZERO, "x", CharStyle::default());
        // Every optional attribute set, so it serializes.
        let style = &mut t.runs[0].style;
        style.font_version = Some("1.0".into());
        style.leading = Some(14.0);
        style.kerning = Some(10.0);
        style.features = vec!["dlig".into()];
        style.style_name = Some("Emphasis".into());
        style.overprint_fill = true;
        style.overprint_stroke = true;
        style.stroke_dash = Some(Default::default());
        style.small_caps = Some(70.0);
        let char_keys: Vec<String> = object_of(serde_json::to_value(&t.runs[0].style).unwrap()).keys().cloned().collect();
        let mut sorted: Vec<&str> = CHAR_KEYS.to_vec();
        sorted.sort_unstable();
        let mut found: Vec<&str> = char_keys.iter().map(String::as_str).collect();
        found.sort_unstable();
        for key in &found {
            assert!(sorted.contains(key), "character attribute `{key}` has no mark");
        }
        for (ty, field) in [("Node", "text"), ("Thread", "text")] {
            let marks = schema["types"][ty]["fields"][field]["marks"].as_object().cloned().unwrap();
            for key in CHAR_KEYS {
                assert_eq!(marks.get(*key).and_then(|m| m["expand"].as_str()), Some("after"), "{ty}.{field} {key}");
            }
            for key in PARA_KEYS {
                assert_eq!(marks.get(&format!("{PARA}{key}")).and_then(|m| m["expand"].as_str()), Some("none"), "{ty}.{field} {key}");
            }
            assert_eq!(marks.get(INLINE).and_then(|m| m["expand"].as_str()), Some("none"));
            assert_eq!(marks.len(), CHAR_KEYS.len() + PARA_KEYS.len() + 1, "{ty}.{field}");
        }
        let para = ParaStyle { justify: Justify::Auto, direction: Some(kition_vector_doc::ParaDirection::RightToLeft), ..ParaStyle::default() };
        for key in object_of(serde_json::to_value(&para).unwrap()).keys() {
            assert!(PARA_KEYS.contains(&key.as_str()), "paragraph attribute `{key}` has no mark");
        }
    }
}
