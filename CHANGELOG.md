# Changelog

Changes to Kition Vector, newest first. A version is the kit package version in `kit.json`;
each package is published with a `package-v<version>` tag.

## 0.1.2

- A host asks what document a drawing file is (`document.identity`) and copies one as a new
  document (`document.fork`), Kit Process Protocol section 5.6 (kition-sdk 6672663). A
  `.kvector` embeds no replica (the kit makes one under a new id each time it opens a file), so a
  drawing carries no document id and its fork is the same bytes.
- The headless program speaks Kit Process Protocol version 1 (`headless.protocol` is
  `kition-kit/1`), as hosts on kition-sdk from bb1b07b require.

## 0.1.1

- `kit.json` declares the engine's editor state as `engine.editorState`, the same list as the
  engine's `EDITOR_STATE`, so a host that does not run the engine reads `inspect()` without it.
- The package is built and published on GitHub with Kition's `kit-package.yml`.
- The README gives the kit to an agent with `kition`, no build: `npm install -g @kition/cli` and
  `kition agent add`; it links KHP at `kitharness/kitharness`.

## 0.1.0

First public release of the source, under the Apache License 2.0. It contains:

- The Rust engine: artboards, layers, paths and shapes, type, fill and stroke, swatches and
  gradients, brushes, Pathfinder, live effects and Image Trace, every action a command; `.svg`,
  `.ai` and `.eps` files open for viewing, and drawings export to SVG, PDF and PNG.
- The kit engine (`kition-vector-kit`), native and WebAssembly: the drawing is a KDM replica, and
  replicas merge concurrent edits. A check reports what is wrong with a drawing (a missing image or
  symbol, art off the artboards) with the commands that fix it.
- The headless kit (`kition-vector`) for agents over the Kit Protocol: it reads every layer and
  object, works with the editor's own commands, runs the same check and works in drafts. A write
  names the revision it read and is refused when someone else changed what it rests on. The host
  owns every file; the kit reaches none by path.
- The kit's editor (`web/`): the menu bar, the tool rail, the ribbon, the artboard canvas and the
  Transform, Appearance, Swatches, Align, Pathfinder and Layers panels, in kit-editor's frame,
  with Classic, Single line and Focus display modes.
- NOTICE and docs/attribution.md list the upstream work, the fonts, the icons and every asset, and
  Loro (MIT), bundled by the editor and linked by the engine.
