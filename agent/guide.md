# Vector kit

The Vector kit opens, edits and checks vector drawings (`.kvector`; SVG, AI and EPS files open as
imports). An agent can draw shapes and paths, set type, paint fills and strokes, align and arrange
objects, work with layers, symbols and artboards, check the drawing for missing images, missing
symbols and art off the artboards, and export it to SVG, PDF or PNG.

## Tools

The kit's prefix is `vector`. Its tools are:

| Tool | Does |
|---|---|
| `vector_inspect` | The drawing as JSON (artboards, layers, and objects with ids, bounds, fill and stroke), with a preview on request |
| `vector_query` | Reads objects of the drawing's object model by id, or by following fields from the root |
| `vector_commands` | Finds commands by keywords, each with its parameter schema |
| `vector_run` | Runs a list of command steps |
| `vector_batch` | Runs one batch of object-model operations |
| `vector_check` | What is wrong with the drawing, as diagnostics |
| `vector_create` | Makes a new `.kvector` drawing |
| `vector_export` | Writes the drawing to a new SVG, PDF or PNG file |
| `vector_draft_ready`, `vector_draft_reopen` | Asks the person to review your draft, or takes it back to edit |

Every tool takes `path`, the drawing's path in the workspace. With `draft: true` your edits go to
your draft of the drawing and leave the file as it is; only the person merges or discards a draft.

## Addressing and coordinates

- A drawing has **artboards** (the pages that export and print) and **layers**. Layers hold
  **objects**: paths and shapes (`Rectangle`, `Ellipse`, `Path`), type (`Type`), groups, images and
  symbol instances.
- Coordinates are **points** in the drawing's space: `x` grows to the right, `y` grows **down**, and
  `(0, 0)` is the top-left corner of the first artboard. `vector_inspect` lists each artboard's `x`,
  `y`, `width` and `height`.
- Every object has a numeric **`id`** (an integer, never a name). Read ids from `vector_inspect`
  (`details.layers[].children[].id`) or from the `result` of the command that made the object
  ([`shape.rectangle`](/kits/vector/commands/shape.rectangle/) → `{id}`).
- Diagnostics name objects as `target.path` `object:<id>`; pass the number in `ids`.
- A command that takes `ids` (paint, stroke, appearance, transparency, `object.setProps`, the
  Layers panel's `layer.*`) acts on them; give the ids as numbers, or as the step's `on`, which
  fills `ids`. Commands without `ids` ([`object.move`](/kits/vector/commands/object.move/),
  [`arrange.group`](/kits/vector/commands/arrange.group/),
  [`object.align`](/kits/vector/commands/object.align/), Pathfinder, clipping and compound paths)
  act on **the selection**: give the step `on: [ids]`, which becomes the selection for that step
  only; the person's selection, current layer and new-art paint are put back after the call.
- Other things have ids too: artboards (`artboard:<n>` in receipts; commands take an artboard's
  index from 0), slices and export assets (numbers). Swatches, colour groups, graphic styles,
  character and paragraph styles, symbols and patterns have ids of their own, saved with the file
  and kept when they are renamed (`swatch:3f2a9c01b2d4`, `charStyle:…` in receipts and
  `vector_query`); commands take their names (`{"swatch": "Brand"}`). A name is once in its table
  (swatches and colour groups share one): a file holding one twice opens with the later ones
  renamed `Brand 2`, `Brand 3`, so a write by name names one object. The built-in `[None]` and
  `[Registration]` swatches are read-only.

## What a write rests on (with `basis`)

A write with `basis` is refused (`conflict`) only when what it rests on changed since. What each
of the 508 write commands rests on was read from its handler and the engine operations behind it
(all 508 read; per family: object 123, path 32, edit 23, layer 21, text 19, swatch 19,
perspective 18, paint 18, file 17, type 14, transparency 13, graphic style 13, appearance 13,
artboard 12, the other 31 families 153):

- a property write (`object.setProps`, `layer.setProps`, `transparency.set`): the fields it sets
  (a rename is not refused for a colleague's move or fill);
- a fill, stroke or effect: the painted objects' appearance (type: also its characters), and, for a
  group or layer, which objects it holds; not the graphic style link;
- a move, align or distribute: the geometry and appearance of what it moves, and everything a
  group holds; a rotate, scale, reflect or shear also its rotated box; with `copy`, every field of
  each and where it is; aligned to the artboard, the artboards;
- a group, blend, envelope or clipping mask: where its objects are (one moved into another group
  or restacked refuses it), not what they are (a colleague's rename, fill or move of one is kept
  in the group); a compound path or clipping path also the geometry and paint it takes; Pathfinder, Shape Builder and Region Paint: every field of each object and where the
  frontmost is (the result goes there);
- a write on a group, compound path, blend or clip group (expand, release, outline, rasterize, a
  path edit inside it): what it holds; on a member of a compound path, blend or clip group: the
  whole and its other members;
- an arrange command: where its object is and which objects its parent holds;
- a delete or ungroup: every field of its objects and of what they hold, not where they are;
- Hide, Lock or Outline Others, Show All, Unlock All: the other layers' visibility or lock;
- an edit of a global swatch: the tint swatches made from it, which follow its colour;
- a write by name: that swatch, colour group, graphic style, character or paragraph style, symbol
  or pattern (renamed away, another one taking the name, or an edit of it, refuses; another
  style's edit, or a reorder of the table, does not; a duplicate or move of a style or swatch also
  its place in the table);
- what its objects refer to: an instance's symbol (rasterize, expand, break link), the graphic
  style a redefinition re-applies to every object linked to it, a symbol's instances when it is
  deleted, the swatches a paste relinks;
- a settings edit (`file.setup`, `file.setInfo`, `file.rasterEffectsSettings`, `print.setup`,
  `print.tiling.set`): the settings it is given (another program's change of another setting of
  the same group is kept);
- the drawing's own settings and tables the handler reads: the guides (slices from guides,
  artboard and layer writes that carry guides), the variables (data sets), the colour mode (a colour
  given), document setup (type), the perspective grid, the raster effects resolution, the tables
  a document-wide write rewrites;
- a write by artboard index: the artboard at that index (another board put there refuses); moving,
  resizing, duplicating or rearranging an artboard with its art: the art on it;
- new art: the layer it goes into staying shown and unlocked (an agent's new art or paste into a
  locked or hidden layer is refused: show and unlock it, or make another layer current); a new layer: the current
  layer's name and place;
- a batch (`command.batch`): what each of its steps rests on.

Preferences that change a write's result when its parameters leave them out (Scale Strokes &
Effects, Scale Corners, Transform Patterns, Use Preview Bounds, Move Locked Artwork with Artboard,
the Keyboard Increment, the interface language) are no part of any premise: the commands reading
them (transform, align, distribute, cuts, artboard move and resize, new type and new text styles)
declare the mode. Give the parameter (`strokes`,
`corners`, `patterns`) or set the preference with `prefs.set` earlier in the same run; the
person's preferences are put back after the call.

Where the guard is blind or too strict: a row moved by `node.move` to an index others filled
since is applied (waiting for the SDK); a structural write that depends on where its objects are
is also refused when an object below them in the same list was deleted or added (their index
shifted, their order did not; waiting for the SDK); a step that acts on the selection an earlier
step of the same run made rests on what the steps before it leave, except after a step that reads
or writes files, libraries or runs other commands (`file.*`, `document.*`, `links.*`,
`command.batch`, …: give the later step `on`); 60
commands hold more on some of their paths than those paths read (each path is listed in
`bindings/wasm/tests/declarations_independent.rs`, `HELD_MORE`): a colleague's change of what such a
path does not read refuses it, so inspect again.

Commands that pick objects by a point or a stroke when nothing is selected (`path.knife`,
`path.eraseRegion`, `path.joinScrub`, `path.blob`, `shaper.select`, `shaper.scribble`,
`symbol.adjust`, `object.liquify`) rest on what lies under it: a colleague's object moved, reshaped, shown or
unlocked into it, or the object under it moved out, refuses them; a change elsewhere does not.
With nothing selected, a colleague's delete of an object anywhere refuses them too (the kit
cannot tell whether it lay under the point): give them `on`, or inspect again.
- Colours are `"#rrggbb"` strings; CMYK, grey and Lab objects are accepted too (see each command's
  parameters).

## Workflow

1. `vector_inspect {path}`: artboards, layers, and objects with ids, bounds, fill and stroke. Add
   `preview: true` for a picture.
2. `vector_commands {path, query}`: find commands by keywords (`"rectangle"`, `"align"`,
   `"setFill"`). Each result has the command `id` and its parameter schema; the `description` says
   what it returns. Use the exact id it returns and follow its parameter schema; never guess a
   command id. An unknown id fails and lists the nearest commands.
3. `vector_run {path, steps: [{command, params}], save?}` a few steps at a time. Steps run in order;
   a failing step stops the call and the kit undoes the steps before it, so the file is as it was
   before the call and nothing is saved: read `error.index` for the failing step, fix it, and send
   the whole run again. A call saves the file when it succeeds (`save` defaults
   to `true`; pass `save: false` to try steps first; a draft is never saved).
4. `vector_check {path}`: fix what it reports, then check again.
5. `vector_inspect {path, preview: true}` to look at the result.

## Example 1: a badge with centred type

```json
vector_run {"path": "Badge.kvector", "steps": [
  {"command": "shape.rectangle", "params": {"x": 10, "y": 10, "width": 180, "height": 80, "radius": 12}},
  {"command": "text.add", "params": {"x": 100, "y": 58, "text": "Kition", "size": 28, "color": "#ffffff"}}
]}
```

Answer: `completed: [{command: "shape.rectangle", result: {id: 4}}, {command: "text.add",
result: {id: 5}}]`. Use those ids:

```json
vector_run {"path": "Badge.kvector", "steps": [
  {"command": "select.set", "params": {"ids": [4]}},
  {"command": "paint.setFill", "params": {"color": "#5645d4"}},
  {"command": "select.set", "params": {"ids": [5]}},
  {"command": "object.align", "params": {"horizontal": "center", "vertical": "center", "to": "artboard"}}
]}
vector_check {"path": "Badge.kvector"}
```

The check answers `{"diagnostics": []}`; `vector_inspect {path, preview: true}` shows the purple
badge with the word centred on the artboard.

## Example 2: fix what the check reports

```json
vector_check {"path": "Poster.kvector"}
```

```json
{"diagnostics": [{"code": "vector.off-artboard", "severity": "warning",
  "target": {"path": "object:4"},
  "message": "Object 4 lies outside every artboard, so exports and prints leave it out."}]}
```

No `fix` is given, so move the object yourself (its bounds are in `vector_inspect`):

```json
vector_run {"path": "Poster.kvector", "steps": [
  {"command": "select.set", "params": {"ids": [4]}},
  {"command": "object.move", "params": {"dx": -560, "dy": 40}}
]}
```

A diagnostic with a fix is applied as given:

```json
vector_check {"path": "Icons.kvector"}
→ {"diagnostics": [{"code": "vector.empty-group", "severity": "info", "target": {"path": "object:1004"},
    "fix": {"command": "edit.delete", "params": {"ids": [1004]}}}]}
vector_run {"path": "Icons.kvector", "steps": [{"command": "edit.delete", "params": {"ids": [1004]}}]}
vector_check {"path": "Icons.kvector"}
→ {"diagnostics": []}
```

## Pitfalls

- **Ids are integers.** Strings, names or `"$n"` references are refused.
- **Results of earlier steps:** `vector_run` steps cannot refer to an earlier step's result
  (`"$0.id"` is refused). Make the objects in one call, read their ids from `completed[].result`,
  then use them in the next call.
- **Select first:** [`paint.setFill`](/kits/vector/commands/paint.setFill/) with nothing selected
  changes nothing.
- **`y` grows down:** a rectangle at `y: 10` sits near the top.
- **Type position:** the `x, y` of [`text.add`](/kits/vector/commands/text.add/) is the start
  of the first baseline, not the top-left corner of the box.
- **Files:** commands that name files take workspace-relative paths of the stated type; paths
  outside the workspace answer `not-allowed`.
- **Undo:** your undo ([`edit.undo`](/kits/vector/commands/edit.undo/)) takes back your own steps
  only; the person's edits stay, and their undo never takes back yours.

## Checks

`vector_check` lists what is wrong with the drawing. Each diagnostic has a stable `code`, a
`severity` (`error`, `warning` or `info`), a `message`, a `target` and, for some, a `fix`: a
command step to run as it is. Act on the code, not on the message. A `fix` is a suggestion: run it
only when it is what the person asked for, and never delete the person's art without saying so.
Every code the kit reports is on the [checks page](/kits/vector/checks/).

| Code | Severity | What to do |
|---|---|---|
| `vector.missing-image` | error | The image's pixels are gone: place the picture again or delete the object |
| `vector.missing-symbol` | error | The instance names a symbol the drawing lacks: create the symbol again or delete the instance |
| `vector.no-artboard` | warning | Add an artboard around the art |
| `vector.missing-graphic-style` | warning | Apply an existing graphic style or clear the link |
| `vector.off-artboard` | warning | Select the object and [`object.move`](/kits/vector/commands/object.move/) it onto an artboard, or delete it |
| `vector.font-substituted` | info | The font is not installed here; choose an available family ([`text.fontList`](/kits/vector/commands/text.fontList/)) |
| `vector.empty-group`, `vector.empty-text`, `vector.stray-point` | info | `fix`: [`edit.delete`](/kits/vector/commands/edit.delete/) `{ids}` |
| `vector.truncated` | info | More of one code than listed: fix the listed ones and check again |
