# UI and interaction parity

> **Last reviewed:** 2026-11-17 · **Last updated:** 2026-11-26 · **Target:** Adobe Illustrator 2026 (30.x)

How Kition Vector's tools, handles, snapping and panels behave next to Illustrator's. A vector illustrator's
trust is in the details: where a handle lands, what Shift and Alt do mid-drag, what a smart guide snaps to,
how far an arrow key nudges. This checklist is the work list for the **interaction-fidelity pass**
(see [gaps.md](../gaps.md)). Part of [target-app-parity.md](target-app-parity.md).

**How it is measured:** from the tool state machines' own documentation and tests (`crates/engine/tools/src/*.rs`,
the `tests_*` files) against Illustrator's public documentation
(helpx tool pages and the keyboard-shortcut reference). Never against the running app: the clean-room rule
forbids launching or measuring Illustrator, so there is **no side-by-side session**. Every "done" below is
"behaves as the documentation describes", which is weaker than "a power user can't tell". Status:
**done** (documented behaviour implemented and tested), **partial**, **missing**.

## Summary

| Area | % | Remaining (h) | Kind |
|---|---:|---:|---|
| Pen, Curvature and anchor tools | 85% | 6–10 | estimated |
| Direct Selection, handles and anchors | 85% | 5–8 | estimated |
| Smart guides and snapping | 70% | 8–12 | estimated |
| Transform handles, bounding box, Free Transform | 80% | 5–8 | estimated |
| Nudging, numeric entry, precision, units | 85% | 3–5 | estimated |
| Modifier keys and temporary tools | 80% | 5–8 | estimated |
| Keyboard shortcuts (default set) | 70% | 6–10 | estimated |
| Cursors | 75% | 3–5 | estimated |
| Panels, Control bar, Properties, context menus | 70% | 15–25 | estimated |
| Accessibility (screen reader, keyboard-only canvas) | 20% | 15–25 | estimated: canvas and its objects not described to screen readers, never tested with a screen reader |
| **UI/UX overall** | **~55%** | **~70–110** (overlaps the feature areas) | estimated |

The overall figure is lower than the rows because what users notice first (small differences across many
tools, never checked side by side) isn't captured row by row.

## Pen, Curvature and anchor tools

| Behaviour (Illustrator, public docs) | Status | Notes |
|---|---|---|
| Click: corner anchor; drag: smooth anchor with symmetric handles | done | `pen.rs` |
| Shift constrains segments and handles to 45° | done | Illustrator's Constrain Angle preference is honoured (`constrainAngle`) |
| Alt mid-drag splits the handles (only the outgoing follows) | done | keeps the curve already shaped |
| Space mid-drag repositions the anchor, handles and all | done | |
| Click the first anchor to close; the drag shapes the closing curve | done | |
| Click the last anchor to retract its outgoing handle; drag from it to pull a new one | done | |
| Alt over a handle, anchor or segment acts as the Anchor Point tool | done | |
| Cmd/Ctrl lends the last-used selection tool | done | the path being drawn continues afterwards |
| Click an open path's end to continue it; click another path's end to join | done | |
| Rubber Band preview | done | Preferences option |
| Auto Add/Delete on a selected path; Shift or the preference disables it | done | |
| Pen cursors (new path ×, continue /, close o, add +, delete −, convert ^) | partial | the web editor draws all seven, and the cursors follow Shift, Alt and Cmd (`bindings/wasm/src/cursors.rs`, 2026-11-25); not every state verified by hand |
| Each anchor snaps to Smart Guides and construction angles | done | `DrawSnap` |
| Curvature tool: click/drag points, double-click or Alt-click for a corner, temporary Direct Selection, point conversion | partial | modifiers requested |
| Add/Delete/Anchor Point tools; Anchor Point tool drags segments to reshape | done | |
| Pencil: Fidelity, Keep selected, Alt to Smooth, edit selected paths, close by proximity | partial | Keep selected and Alt to Smooth are missing |
| Join tool scrubbing, Average, Join with corner/smooth choice | done | |
| Simplify (slider, auto, corner angle threshold) | done | |

## Direct Selection, anchors and handles

| Behaviour | Status | Notes |
|---|---|---|
| Click/Shift-click anchors; marquee toggles with Shift | done | |
| Drag a curved segment to bend it, a straight one to move its anchors | done | |
| Handles shown are the handles that drag; tolerance from Selection & Anchor Display | done | |
| Drag a handle with Shift (45° about its anchor), Alt (move one handle alone) | done | handles snap to smart guides |
| Show handles when multiple anchors are selected; Highlight anchors on mouse over; anchor and handle size | done | Preferences › Selection & Anchor Display |
| Convert selected anchors to corner/smooth from the Control bar and Properties | done | |
| Cut path at selected anchors, Remove Anchor Points, Delete removes segments | done | |
| Corner widgets (radius readout, Alt-click cycles kind, double-click the Corners dialog) | done | clicking a widget without pre-selecting the anchor missing |
| Corners between curved sides, Relative rounding | missing | |
| Isolation mode by double-click, breadcrumbs, Esc to exit | done | |
| "Magnetic" moves of selected anchors with a modifier (requested) | missing | not an Illustrator feature as documented; under discussion |

## Smart guides and snapping

| Behaviour | Status | Notes |
|---|---|---|
| Snap to anchors, centres, edges, artboards, bleed and ruler guides with magenta lines | done | `guides.rs` kinds: Anchor, Center, Edge, Artboard, Bleed, Guide |
| Anchor/Path labels ("anchor", "center", "path", "align") | done | |
| "intersect" label and snapping to path intersections | missing | |
| Construction guides at preference angles while drawing | done | |
| Measurement labels while drawing, moving and resizing | partial | sizes and offsets; angle and distance readouts while drawing missing |
| Spacing guides (equal distances) | partial | for moves; not for resizing or drawing |
| Snapping tolerance preference | done | |
| Snap to Point, Snap to Grid, Snap to Pixel; Pixel Preview | done | |
| Snap to Glyph | missing | |
| Tangent and perpendicular snapping for line endpoints (new in 30.0) | missing | |
| Snapping panel/popover consolidating the snap options (30.0) | missing | |
| Customisable rotation snap angle | missing | Illustrator: Constrain Angle and construction angles only |
| Ruler guides: drag out, select, move, delete, lock, Make/Release Guides | done | |
| Ruler guides as Layers rows: shown, hidden, locked and deleted with their layer; rows dragged to another layer | done | `guide.setLayer`; guides from older files go on a layer |
| Alt swaps a ruler guide's orientation while dragging | missing | |
| Global and video rulers | missing | |

## Transform handles, bounding box, Free Transform

| Behaviour | Status | Notes |
|---|---|---|
| Bounding-box handles: Shift proportional, Alt from centre, rotate outside the corners | done | |
| Rotated objects keep a rotated bounding box and their angle; Reset Bounding Box | done | |
| Free Transform, Scale and Reflect along a rotated box | missing | |
| Free Transform widget: constrain, free distort, perspective distort; the key-held variants | done | |
| Transform tools: click sets the reference point (snaps to anchors/centres), Alt-click opens the dialog, Alt-release copies | done | |
| Transform Again, Transform Each, Scale Strokes & Effects, Scale Corners | done | Transform Again after a type-area resize scales the type (gap) |
| Reference point locator (nine squares) in Control bar and Transform panel | done | |

## Nudging, numeric entry and precision

| Behaviour | Status | Notes |
|---|---|---|
| Arrow keys nudge by Keyboard Increment (0.001–1296 pt); Shift ×10; Alt copies | done | |
| Number fields: Up/Down step, Shift ×10, Cmd/Ctrl ÷10, wheel over a focused field | done | Shift+arrow in the Stroke weight field doesn't step by 10 |
| Arithmetic in fields (`+ - * /`) and unit suffixes (`3 mm`, `1in`, `2p6`) | done | `Unit::parse`; mixed units in one expression not verified |
| Units: pt, pc, in, mm, cm, px, ft-in, m, yd, ft; General/Stroke/Type units | done | units preference reverting to points on SVG output reported |
| Field precision: 3 decimals for steps; values kept as f64 | done | |
| Dialogs open with the first field focused and selected; Enter confirms | done | |

## Modifier keys and temporary tools

| Behaviour | Status | Notes |
|---|---|---|
| Space: Hand; Cmd+Space / Cmd+Alt+Space: zoom in/out; middle button pans | done | |
| Cmd/Ctrl: last-used selection tool; Cmd+Alt: Group Selection inside Direct Selection | partial | |
| Alt-drag duplicates (objects, artboards with their art) | done | |
| Shape tools: Shift constrains, Alt from centre, Space moves while dragging, arrows change sides/points/rows | done | |
| Tool double-click opens its options | done | |
| Per-platform mapping (Cmd ↔ Ctrl, Option ↔ Alt) | done | |
| Every tool's modifiers checked against the shortcut reference | missing | this is the fidelity pass |

## Keyboard shortcuts

~110 distinct modified shortcuts on menu commands (each command's `shortcut`) plus single-key tool shortcuts for the
default toolbar (`crates/engine/tools/src/catalog.rs`). Not yet diffed item by item
against Illustrator's published default shortcut list; estimated ~70% of it. Remaining: the panel shortcuts,
the type shortcuts (tracking/kerning/leading/baseline steps with Alt/Shift), and selection shortcuts in the
Layers panel.

## Panels, Control bar, Properties, context menus

The editor in `web/` shows a dock of tabbed panel groups on the right (`web/src/panels/dock.tsx`): Properties
(Transform, Appearance, Stroke, Gradient, quick actions, Align, Pathfinder) with Align and Pathfinder tabs, Layers with
Artboards, Swatches, and History, which the Window menu shows and hides (2026-11-26, plan stage 4); a control bar in the
frame's options row (`web/src/panels/ControlBar.tsx`) and its own Color Picker (`web/src/color/`), in the host's frame
(2026-11-25, plan stage 3).

| Behaviour | Status | Notes |
|---|---|---|
| Properties panel per context (every selection type's Quick Actions) | partial | Transform, Appearance, Stroke, Gradient and a few quick actions for any selection; not per selection type |
| Color Picker: field and hue strip, HSB, RGB, CMYK, hex, None, new over current | done | live on the canvas, one history step on OK; no Lab, web-safe colours or canvas eyedropper |
| Gradient panel: type, angle, stops (add, drag, delete, colour, opacity, location), midpoints, reverse | done | aspect ratio only through the Gradient tool |
| Stroke panel: weight, cap, corner, limit, align, dashes and gaps, arrowheads, width profile | partial | no arrowhead scale or swap |
| Swatches panel: previews, apply to fill or stroke, new, options, colour groups, delete, global mark | partial | pattern swatches show a mark, not their tile; no list view or libraries |
| Align panel: align, distribute, distribute spacing, Align To (selection, key object, artboard) | done | |
| Control bar: fill, stroke, weight, opacity, align | partial | the active tool's options are not shown |
| Panel dock: tabbed groups, collapse, resize, Window menu toggles with check marks | partial | docked on the right only; no floating panels or saved workspaces |
| Layers panel: tree, eye, lock, target, selection square, rename, drag (restack, into a layer or group, Alt copies), Shift/Cmd selection, panel and row menus | partial | no thumbnails; only the rows in view are drawn |
| Artboards panel: list, rename, new, duplicate, delete, reorder, Artboard Options (preset, size, orientation, position); status bar artboard navigation | partial | no Rename Artboards for several, Switch Orientation or Delete Empty Artboards commands |
| History panel: steps done and undone, go back or forward | partial | steps back by undoing one at a time (no jump in the engine) |
| Blend panel (30.7) | missing | `blend.options` exists |
| SVG Interactivity, Retype, Comments, Version History panels | missing | the last two are cloud features (out of scope) |
| Preferences: every option acts | partial | ~17 stored but unused |

## Look and feel

The editor in `web/` takes its look from the host's theme tokens and `@kition/sdk/editor`'s components; no
measurement of it is recorded here.

## Revision history

| Date | Change | Summary |
|---|---|---|
| 2026-11-17 | major | First public version |
