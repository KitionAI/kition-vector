# Architecture

> **Last reviewed:** 2026-11-17 · **Last updated:** 2026-11-17 · **Target:** Adobe Illustrator 2026 (30.x)

How the Vector engine is built. Day-to-day rules (clean-room, never crash, quality gates) are in
[`AGENTS.md`](../AGENTS.md) and [development.md](development.md).

## Shape

A Cargo workspace of engine crates under `crates/`, the headless kit executable `bindings/headless` and
`xtask`, pure Rust (no handwritten JS/TS). `bindings/wasm` compiles the same engine to
WebAssembly for Kition's in-page editor (`web/`); `bindings/headless` serves the same engine headless, over
Kition's Kit Protocol, for Kition's agent.

## Layers

`cargo xtask layers` (part of `cargo xtask ci`) enforces that a crate depends only on lower layers, and that
nothing below L6 depends on a UI toolkit (egui, eframe, winit, rfd). Table: `tools/xtask/src/layers.rs`.

| Layer | Crates | Role |
|---|---|---|
| L0 | `geom`, `color`; `affinity` (standalone, no workspace deps) | curves and transforms (on `kurbo`), colour models, ICC and conversions; the Affinity reader |
| L1 | `doc` | the document model: artboards, layers, objects, appearance, text, symbols; pure data + serde, structural sharing |
| L2 | `pathops`, `brush`, `trace`, `text`, `effects`, `plugins`, `three-d` | booleans/Shape Builder/offset/outline; brushes; Image Trace; fonts, shaping (HarfRust, skrifa), bidi and layout; live effects and the pixel-filter pipeline; sandboxed WebAssembly plug-ins (wasmi); 3D surfaces of revolution, projection and shading (`three-d`, on `geom` only) |
| L3 | `render`, `svg`, `pdf`, `format`, `eps`, `cad`, `metafile` | document to pixels (`vello_cpu`, multithreaded); SVG, PDF, native JSON, EPS/PostScript (with its own interpreter), DXF, EMF/WMF readers and writers |
| L4 | `tools` | tool state machines: pointer events in, commands and overlays out (Pen, Direct Selection, smart guides…) |
| L4 | `kdm-model` | Kition's own: the drawing as a `kit.vector` KDM replica (layer tree, per-object registers, named lists, settings, image blobs), its reader, diff writer and post-merge repair, and `VectorSession` for the engine's command bridge |
| L5 | `engine` | sessions, history (unlimited undo by structural sharing), selection, the command registry (~710 commands), file I/O, the panic guard |
| L6 | `kition-vector-kit` | Kition's in-page Vector engine, compiled to WebAssembly |
| L7 | `bindings/headless`, `xtask` | the headless kit executable for Kition's agent; build tooling |

`testkit` holds test helpers (fixtures, generators, render comparison) and is only a dev-dependency.

## Everything is a command

Every user-visible action is a `CommandSpec` in `crates/engine/engine/src/cmd/*` (id, label, menu path, shortcut,
params doc, `enabled`, `run`, journal flag). Tools emit commands (begin, preview, commit). The editor and
Kition's agent both reach the same registry, so anything a user can do an agent can do. `command.batch` runs
several as one undo step.

## Rendering

The canvas is rasterized on the CPU by `vello_cpu`, with caches per object and per effect: multithreaded in
native builds, single-threaded in the WebAssembly build, which the editor runs in a Web Worker, off the page's
thread. The editor draws the rendered pixels on a 2D canvas (`@kition/sdk/editor`). Raster effects are rendered
offscreen per object and cached while panning. See [hardware-parity.md](parity/hardware-parity.md).

## Files

The native `.kvector` format is versioned, compressed JSON with embedded images, saved atomically, readable
down to v1. The `FORMATS` table in `crates/engine/engine/src/cmd/fileio/mod.rs`
lists every format read or written; importers are fuzzed (`crates/engine/engine/tests/import_fuzz.rs`). See
[file-format-parity.md](parity/file-format-parity.md).

## Agent control

Kition's agent drives the same engine as the editor through `bindings/headless`, the headless kit executable,
over Kition's Kit Protocol (see `docs/kits.md` in the Kition repository and kition-sdk's `docs/platform/`).

## Robustness

No panics in shipped code (workspace clippy lints ban `unwrap`, `expect`, `panic!` and friends outside tests);
`kition_vector_engine::guard` catches a panic at every entry point and rolls the document back. See
[development.md](development.md#never-crash-no-panics-in-shipped-code).

## Revision history

| Date | Change | Summary |
|---|---|---|
| 2026-11-17 | major | First public version |
