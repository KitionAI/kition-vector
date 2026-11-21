# Hardware parity

> **Last reviewed:** 2026-11-17 · **Last updated:** 2026-11-17 · **Target:** Adobe Illustrator 2026 (30.x)

What hardware Illustrator uses against what the kit uses: the editor in `web/`, which runs in a browser, and the
engine. Illustrator's side is from its public documentation (GPU Performance, touch workspace, pen pressure pages);
ours from the code (`crates/render/render`, `bindings/wasm`, and the editor's canvas: `KitCanvas` and
`rasterViewport` in `@kition/sdk/editor`). Part of [target-app-parity.md](target-app-parity.md).

## Summary

**~35% (estimated), 25–45 h.** Kition Vector rasterizes the canvas on the CPU
(`vello_cpu`, single-threaded in the WebAssembly engine the editor runs) and the editor draws the pixels on a 2D
canvas, so there is no GPU-accelerated preview on huge documents. Pen input is pressure only, from the browser's
pointer events.

| Feature | Illustrator | Kition Vector | Status / hours |
|---|---|---|---|
| GPU-accelerated canvas (GPU Performance, GPU Preview, GPU anti-aliasing) | Metal, DirectX | CPU raster, drawn on a 2D canvas | partial; a GPU renderer spike is pending (M5): 15–25 h |
| Multithreaded rendering off the UI thread | partial | off the page's thread (the engine runs in a Web Worker), single-threaded in WebAssembly; multithreaded in native builds | beyond Illustrator |
| Pen pressure | yes (Wacom, Windows Ink, macOS tablet events) | from the browser's pointer events (`@kition/sdk/editor` passes a pen's pressure to the engine) | partial: 4–8 h |
| Pen tilt, bearing, barrel rotation (6D Art Pen) | yes, drives brushes | no | missing: 4–8 h after the input layer |
| Pen eraser end | yes | no | missing |
| Touch gestures (pinch zoom, two-finger pan, rotate view, Touch workspace) | yes (Windows touch, Surface) | a finger acts as a pointer; no multi-touch gestures | partial: touch workspace and rotate gesture missing, 3–5 h |
| Trackpad (smooth scroll, pinch) | yes | the wheel pans; Cmd/Ctrl with the wheel, or a pinch, zooms | done |
| HiDPI / Retina | yes | renders at the browser's device pixel ratio | done |
| Display colour profile (proof and display through the monitor's ICC) | yes | no | missing: 3–5 h |
| 3D GPU rendering (Materials, ray tracing) | yes | — | blocked on 3D |

## Revision history

| Date | Change | Summary |
|---|---|---|
| 2026-11-17 | major | First public version |
