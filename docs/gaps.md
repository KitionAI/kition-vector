# Gaps

> **Last updated:** 2026-11-17 · **Source:** `features.yaml` (updated 2026-11-17) and the engine's command list

Every known shortfall of Kition Vector, from the feature catalogue: features that are not yet `has`,
then the differences recorded on features that work. Update the feature in `features.yaml` when you
land work, then this list. The comparison with the target application is in
[parity/target-app-parity.md](parity/target-app-parity.md); the order of work is in [roadmap.md](roadmap.md).

## Features not done

- `vector.agent.premises` (P0, partial): Explicit object identities, editor context and write premises.
- `vector.artboard.manage` (P1, partial): Artboards. The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen.
- `vector.clipboard.formats` (P1, partial): Clipboard in SVG, PDF, PNG and text. The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen.
- `vector.color.recolor` (P1, partial): Recolor Artwork, reduce and randomize colours. The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen.
- `vector.document.inspect` (P1, partial): Document setup, units, colour mode and inspection. The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen.
- `vector.edit.colors` (P1, partial): Edit Colors: blend, invert, adjust and convert artwork colours. The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen.
- `vector.edit.history` (P1, partial): Undo, redo, cut, copy, paste and clear. The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen.
- `vector.edit.misc` (P1, partial): Find and replace, spelling, keyboard shortcuts and other Edit menu commands. The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen.
- `vector.effect.live` (P1, partial): Live effects. The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen.
- `vector.file.export` (P1, partial): Export to PDF, DXF, EPS, screens, web and Office; format, PDF and print settings; package. File › Export writes SVG, PDF and PNG through the host (web/tests/flow.mjs). Export for Screens, Export Selection, Save for Office and Save as Template have no editor dialog that writes through the host, so the editor's menus leave them out; agents still run them.
- `vector.file.templates` (P1, partial): Templates, new-document presets, placed files, file info and recovery data. The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen.
- `vector.graph.data` (P1, partial): Graphs. The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen.
- `vector.guide.manage` (P1, partial): Guides. The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen.
- `vector.image.trace` (P1, partial): Image Trace. The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen.
- `vector.layer.panel` (P1, partial): Layers and sublayers. The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen.
- `vector.links.placed` (P1, partial): Placed images and links. The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen.
- `vector.object.arrange` (P1, partial): Arrange objects in stacking order. The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen.
- `vector.object.blend` (P1, partial): Blends. The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen.
- `vector.object.clipping-mask` (P1, partial): Clipping masks. The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen.
- `vector.object.envelope` (P1, partial): Envelope distort. The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen.
- `vector.object.mesh` (P1, partial): Gradient meshes. The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen.
- `vector.object.path` (P1, partial): Object ▸ Path: join, average, outline stroke, offset, simplify and clean up. The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen.
- `vector.object.pathfinder` (P1, partial): Pathfinder: unite, minus front, intersect, exclude, divide and the other shape modes. The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen.
- `vector.object.pattern` (P1, partial): Pattern editing mode. The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen.
- `vector.object.repeat` (P1, partial): Repeat: radial, grid and mirror. The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen.
- `vector.object.slice` (P1, partial): Slices. The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen.
- `vector.object.text-wrap` (P1, partial): Text wrap around objects. The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen.
- `vector.object.transform` (P1, partial): Transform, group, lock, hide, expand and other Object menu commands. The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen.
- `vector.output.web` (P1, partial): Save for Web presets, asset export and CSS. The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen.
- `vector.paint.appearance` (P1, partial): Appearance panel: multiple fills and strokes. The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen.
- `vector.paint.brushes` (P1, partial): Brushes. The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen.
- `vector.paint.color` (P1, partial): Colour guide, colour profiles, colour conversion and gamut warning. The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen.
- `vector.paint.fill-stroke` (P1, partial): Fill and stroke, swatches applied to them, and freeform gradients. The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen.
- `vector.paint.graphic-styles` (P1, partial): Graphic styles. The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen.
- `vector.paint.patterns` (P1, partial): Pattern swatches. The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen.
- `vector.paint.region-paint` (P1, partial): Shape Builder and Region Paint. The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen.
- `vector.paint.stroke` (P1, partial): Stroke options and variable width profiles. The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen.
- `vector.paint.swatches` (P1, partial): Swatches, colour groups and swatch libraries. The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen.
- `vector.paint.transparency` (P1, partial): Opacity, blending modes and opacity masks. The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen.
- `vector.path.edit` (P1, partial): Draw paths with the Pen and edit anchors and direction handles. The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen.
- `vector.perspective.grid` (P1, partial): Perspective grid and drawing on perspective planes. The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen.
- `vector.select.basic` (P1, partial): Select all, none, inverse, next above or below, and saved selections. The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen.
- `vector.select.object` (P1, partial): Select objects by kind (brush strokes, clipping masks, stray points, text objects …). The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen.
- `vector.select.same` (P1, partial): Select objects with the same fill, stroke, appearance or other attribute. The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen.
- `vector.shape.primitives` (P1, partial): Draw rectangles, ellipses, polygons, stars, flares and line segments. The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen.
- `vector.symbol.manage` (P1, partial): Symbols. The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen.
- `vector.text.edit` (P1, partial): Create and edit point, area and path type; character and paragraph formats. The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen.
- `vector.text.styles` (P1, partial): Character and paragraph styles. The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen.
- `vector.text.tabs` (P1, partial): Tab stops. The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen.
- `vector.text.thread` (P1, partial): Threaded text frames. The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen.
- `vector.type.convert` (P1, partial): Type orientation, outlines, case, smart punctuation and type conversion. The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen.
- `vector.view.display` (P1, partial): View modes, proofing, overprint and separations previews, views. The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen.
- `vector.app.preferences` (P2, partial): Preferences, batch commands and plug-ins. The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen.
- `vector.data.variables` (P2, partial): Variables and data sets. The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen.
- `vector.library.assets` (P2, partial): Libraries. The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen.
- `vector.object.attributes` (P2, partial): Object attributes (overprint, image map, URL). The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen.
- `vector.output.flattener` (P2, partial): Transparency flattener presets. The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen.
- `vector.output.pdf-presets` (P2, partial): PDF presets. The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen.
- `vector.output.print` (P2, partial): Print setup, preview and presets. The engine has these commands; no case yet shows the area working end to end in the editor and surviving save and reopen.

## Catalogue

The catalogue has 66 features: 7 `has`, 59 not yet.
