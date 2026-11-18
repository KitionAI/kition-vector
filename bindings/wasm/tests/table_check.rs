//! C8-vector-4 item 1: an independent check of the declaration table (`src/address/table.rs`).
//!
//! The rows were written from sub-agent readings of the handlers. Here a seeded sample of 80
//! commands (seed `0x9e3779b97f4a7c15`, xorshift as `bindings/headless/tests/outside.rs`, over
//! every command id in byte order) and every command of the families most likely to be wrong
//! were read again, handler by handler, and what each handler resolves is written out below,
//! without the table's comments. The test compares each with what the kit declares.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

use std::collections::BTreeSet;

use kition_kit::{EditorState, KitCommand, KitDocument};
use kition_vector_kit::VectorDocument;

/// The sample, in the order the seed draws it (listed before any row was read).
const SAMPLE: [&str; 80] = [
    "edit.colorSettings",
    "document.formats",
    "select.key",
    "type.fillPlaceholder",
    "text.missingFonts",
    "select.same.symbolInstance",
    "layer.template",
    "view.printTiling",
    "effect.apply",
    "object.clippingMask.release",
    "layer.collectInNew",
    "transparency.releaseOpacityMask",
    "object.transform",
    "colorTheme.delete",
    "links.placementOptions",
    "text.fontList",
    "transparency.stopEditingOpacityMask",
    "shapeBuilder.merge",
    "swatch.library.get",
    "type.createOutlines",
    "appearance.targetContents",
    "webExport.presets.list",
    "object.slice.select",
    "document.find",
    "file.newFromTemplate",
    "brush.new",
    "guide.select",
    "type.step",
    "text.discardEmpty",
    "symbol.replace",
    "stroke.widthProfile.reset",
    "library.rename",
    "text.editRange",
    "layer.releaseToLayers",
    "path.appendAnchor",
    "clipboard.importEmf",
    "path.eraseSegments",
    "edit.colors.adjustBalance",
    "text.fonts",
    "variable.bind",
    "recolor.reduce",
    "object.rasterize",
    "object.setProps",
    "swatch.new",
    "perspective.grid.set",
    "graphicStyle.saveLibrary",
    "transparency.togglePageIsolatedBlending",
    "dataset.set",
    "colorTheme.list",
    "file.print",
    "file.close",
    "edit.redo",
    "color.gamutCheck",
    "swatch.delete",
    "swatch.library.save",
    "object.textWrap.make",
    "file.recovery.list",
    "shape.rectangularGrid",
    "links.embed",
    "stroke.widthPoint.remove",
    "brush.freehand",
    "graphicStyle.library",
    "object.ungroup",
    "dataset.next",
    "text.setFormat",
    "edit.colors.blendVertically",
    "library.moveItem",
    "file.place.queue",
    "select.same.textStrokeColor",
    "symbol.list",
    "file.info",
    "clipboard.conflicts",
    "livePaint.make",
    "transparency.enableOpacityMask",
    "document.exportForScreens",
    "object.lock",
    "layer.unlockAll",
    "document.pdfInfo",
    "document.node",
    "command.batch",
];

/// The families most likely to be wrong, read whole: Pathfinder and Shape Builder, clipping and
/// compound paths, type on a path and threads, symbols, blends, envelopes, meshes, Region Paint, the
/// appearance stack by index, artboards.
const FAMILIES: &[&str] = &[
    "object.pathfinder.",
    "shapeBuilder.",
    "object.clippingMask.",
    "layer.clippingMask.",
    "object.compoundPath.",
    "text.thread.",
    "type.pathOptions",
    "symbol.",
    "select.same.symbolInstance",
    "object.blend.",
    "object.envelope.",
    "object.mesh.",
    "livePaint.",
    "appearance.",
    "artboard.",
];

fn xorshift(seed: &mut u64) -> u64 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 7;
    *seed ^= *seed << 17;
    *seed
}

fn commands() -> Vec<KitCommand> {
    VectorDocument::create("Table.kvector", 320, 200).unwrap().commands()
}

#[test]
fn the_sample_is_the_seeds() {
    let mut ids: Vec<String> = commands().into_iter().map(|command| command.id).collect();
    ids.sort();
    assert_eq!(ids.len(), 721);
    let mut seed = 0x9e37_79b9_7f4a_7c15_u64;
    let mut drawn: Vec<&str> = Vec::new();
    while drawn.len() < 80 {
        let at = usize::try_from(xorshift(&mut seed) % ids.len() as u64).unwrap();
        if !drawn.contains(&ids[at].as_str()) {
            drawn.push(&ids[at]);
        }
    }
    assert_eq!(drawn, SAMPLE);
}

/// What each checked handler resolves, one command a line:
/// `id | targets | objects | path:Type+Type… | places | name parameters | reads` (`-`: none). Read
/// from the handlers (C8-vector-4); a line the table disagreed with is listed in
/// [`CORRECTED`].
const EXPECTED: &str = include_str!("table_check_expected.txt");

/// The rows the check found wrong, with what was wrong (all fixed in `src/address/table.rs`).
const CORRECTED: &[(&str, &str)] = &[
    ("brush.freehand", "runs path.freehand, which draws behind the selection in Draw Behind: reads Selection"),
    ("swatch.new", "`swatch` is looked up as a swatch or a pattern; `gradient.stops[].swatch` names swatches (paint_from)"),
    ("shapeBuilder.merge", "`fill.gradient.stops[].swatch` names swatches (fill_param → paint_from)"),
    ("livePaint.fill", "`gradient.stops[].swatch` names swatches (paint_from)"),
    ("livePaint.strokeEdge", "`swatch` is a swatch or a pattern; `gradient.stops[].swatch` names swatches (paint_from)"),
    ("artboard.copy", "declared a place `artboard` the handler never reads (board_and_art reads `index`)"),
    ("paint.setFill", "`swatch` is looked up as a swatch or a pattern (paint_from), as paint.setStroke declares"),
    ("swatch.edit", "`paint.swatch` is a swatch or a pattern; `paint.gradient.stops[].swatch` names swatches (paint_from)"),
];

fn split(field: &str) -> BTreeSet<String> {
    field.split_whitespace().filter(|word| *word != "-").map(str::to_owned).collect()
}

fn reads(states: &[EditorState]) -> BTreeSet<String> {
    states.iter().map(|state| format!("{state:?}")).collect()
}

#[test]
fn the_checked_rows_say_what_their_handlers_resolve() {
    let commands = commands();
    let mut checked = Vec::new();
    let mut wrong = Vec::new();
    for line in EXPECTED.lines().filter(|line| !line.trim().is_empty()) {
        let fields: Vec<&str> = line.split(" | ").map(str::trim).collect();
        let [id, targets, objects, types, places, names, read] = fields[..] else { panic!("bad line {line:?}") };
        let command = commands.iter().find(|command| command.id == id).unwrap_or_else(|| panic!("no command {id}"));
        checked.push(id);
        let mut want_targets = (targets != "-").then_some(targets);
        let mut want_objects = split(objects);
        if command.schema.is_none() {
            // A command without a schema declares its targets among its objects (`address::apply`).
            want_objects.extend(want_targets.take().map(str::to_owned));
        }
        let got_types: BTreeSet<String> = command.object_types.iter().map(|(path, types)| format!("{path}:{}", types.join("+"))).collect();
        let got = (
            command.targets,
            command.objects.iter().map(|object| (*object).to_owned()).collect::<BTreeSet<_>>(),
            got_types,
            command.places.iter().map(|place| (*place).to_owned()).collect::<BTreeSet<_>>(),
            command
                .name_params
                .iter()
                .map(|(name, field)| {
                    assert_eq!(*field, "name", "{id} {name}");
                    (*name).to_owned()
                })
                .collect::<BTreeSet<_>>(),
            reads(command.reads),
        );
        let want = (want_targets, want_objects, split(types), split(places), split(names), split(read));
        if got != want {
            wrong.push(format!("{id}:\n  declared {got:?}\n  handler  {want:?}"));
        }
    }
    for id in SAMPLE {
        assert!(checked.contains(&id), "sampled {id} not checked");
    }
    for command in &commands {
        let in_family = FAMILIES.iter().any(|family| command.id == *family || family.ends_with('.') && command.id.starts_with(family));
        assert!(!in_family || checked.contains(&command.id.as_str()), "{} of a family not checked", command.id);
    }
    for (id, _) in CORRECTED {
        assert!(checked.contains(id), "corrected {id} is checked");
    }
    let sample_wrong = CORRECTED.iter().filter(|(id, _)| SAMPLE.contains(id)).count();
    println!("C8-vector-4 table check: {} rows checked, {} found wrong ({sample_wrong} of the 80 sampled)", checked.len(), CORRECTED.len());
    assert!(wrong.is_empty(), "{} rows disagree with their handlers:\n{}", wrong.len(), wrong.join("\n"));
}
