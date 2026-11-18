# Asset attribution

Every non-code asset in this repository (icons, images, fonts, example art, colour profiles, presets) is listed here with its author, source and licence. `cargo xtask assets` (part of `cargo xtask ci`) fails if an asset file is missing from this table.

**Policy (mandatory):** Kition Vector contains **no Adobe iconography, images, artwork, presets, swatch/brush/symbol/pattern libraries or ICC profiles**. Every asset is either original work by Kition Vector contributors, upstream material credited in NOTICE (the `dc-*.svg` icons, the examples and the translation catalogs), or third-party material under an open licence (OSI open source, public domain / CC0, or Creative Commons that allows redistribution), and it is attributed below. Screenshots of Adobe software are never committed.

Generated-in-code art is original and has no file to list. This covers the default swatches, brushes, symbols, patterns, graphic styles and image-trace presets.

**Font files are never committed here.** Fonts live in [KitionAI/kition-fonts](https://github.com/KitionAI/kition-fonts). The small Latin UI and document fonts below are the exception: the engine compiles them in.

**Optional build input: kition-fonts.** Native builds made with `KITION_FONTS_DIR=<kition-fonts checkout>` embed every font in its `fonts/manifest.txt` (per-file authors and licences in its [`ATTRIBUTION.md`](https://github.com/KitionAI/kition-fonts/blob/main/ATTRIBUTION.md)): today Droid Sans Fallback (Apache-2.0) and, under the SIL Open Font License 1.1, Inter, Lora, JetBrains Mono, Bricolage Grotesque, Carlito, Caladea, Tinos, Cousine, Arimo, Noto Color Emoji, Ma Shan Zheng, BIZ UDPGothic, BIZ UDMincho and Noto Sans Arabic. Web builds made with it embed only BIZ UDPGothic Regular. They are not files in this repository; kition-fonts' `ATTRIBUTION.md` lists them. Builds without `KITION_FONTS_DIR` embed none of them.

| Asset | Author | Source | Licence | Notes |
|---|---|---|---|---|
| `assets/fonts/Inter-Medium.ttf` | Rasmus Andersson / The Inter Project Authors | https://github.com/rsms/inter | OFL-1.1 (`assets/fonts/OFL-Inter.txt`) | Open-source typeface, not product iconography or artwork |
| `assets/fonts/Inter-Regular.ttf` | Rasmus Andersson / The Inter Project Authors | https://github.com/rsms/inter | OFL-1.1 (`assets/fonts/OFL-Inter.txt`) | Open-source typeface, not product iconography or artwork |
| `assets/fonts/Inter-SemiBold.ttf` | Rasmus Andersson / The Inter Project Authors | https://github.com/rsms/inter | OFL-1.1 (`assets/fonts/OFL-Inter.txt`) | Open-source typeface, not product iconography or artwork |
| `assets/fonts/JetBrainsMono-Regular.ttf` | The JetBrains Mono Project Authors | https://github.com/JetBrains/JetBrainsMono | OFL-1.1 (`assets/fonts/OFL-JetBrainsMono.txt`) | Open-source typeface, not product iconography or artwork |
| `assets/fonts/SourceSans3-Bold.ttf` | Paul D. Hunt / Adobe (released as open source) | https://github.com/adobe-fonts/source-sans | OFL-1.1 (`assets/fonts/OFL-SourceSans3.txt`) | Open-source typeface, not product iconography or artwork |
| `assets/fonts/SourceSans3-It.ttf` | Paul D. Hunt / Adobe (released as open source) | https://github.com/adobe-fonts/source-sans | OFL-1.1 (`assets/fonts/OFL-SourceSans3.txt`) | Open-source typeface, not product iconography or artwork |
| `assets/fonts/SourceSans3-Regular.ttf` | Paul D. Hunt / Adobe (released as open source) | https://github.com/adobe-fonts/source-sans | OFL-1.1 (`assets/fonts/OFL-SourceSans3.txt`) | Open-source typeface, not product iconography or artwork |
| `assets/fonts/SourceSans3-Semibold.ttf` | Paul D. Hunt / Adobe (released as open source) | https://github.com/adobe-fonts/source-sans | OFL-1.1 (`assets/fonts/OFL-SourceSans3.txt`) | Open-source typeface, not product iconography or artwork |
| `assets/fonts/SourceSerif4-Regular.ttf` | Frank Grießhammer / Adobe (released as open source) | https://github.com/adobe-fonts/source-serif | OFL-1.1 (`assets/fonts/OFL-SourceSerif4.txt`) | Open-source typeface, not product iconography or artwork |
| `assets/fonts/OFL-Inter.txt` | (licence text) | upstream project | — |  |
| `assets/fonts/OFL-JetBrainsMono.txt` | (licence text) | upstream project | — |  |
| `assets/fonts/OFL-SourceSans3.txt` | (licence text) | upstream project | — |  |
| `assets/fonts/OFL-SourceSerif4.txt` | (licence text) | upstream project | — |  |
| `assets/icons/LICENSE-lucide.txt` | (licence text) | upstream project | — |  |
| `assets/icons/align-center-horizontal.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/align-center-vertical.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/align-end-horizontal.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/align-end-vertical.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/align-horizontal-justify-center.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/align-horizontal-justify-end.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/align-horizontal-justify-start.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/align-start-horizontal.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/align-start-vertical.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/align-vertical-justify-center.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/align-vertical-justify-end.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/align-vertical-justify-start.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/arrow-left-right.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/arrow-up-down.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/blend.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/brush.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/chart-column.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/chevron-down.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC; derived from Feather, also MIT (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/chevron-left.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC; derived from Feather, also MIT (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/chevron-right.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC; derived from Feather, also MIT (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/chevrons-left.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC; derived from Feather, also MIT (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/chevrons-right.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC; derived from Feather, also MIT (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/circle.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC; derived from Feather, also MIT (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/clipboard-paste.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/cloud.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/combine.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/copy.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/dc-actions.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-al-bottom.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-al-hcenter.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-al-left.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-al-right.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-al-top.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-al-vcenter.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-align.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-alignto-artboard.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-alignto-key.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-alignto-selection.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-anchor.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-anchor-smooth.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-appearance.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-arc.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-arrow-down.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-arrow-extend.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-arrow-tip.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-arrow-up.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-artboard-options.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-artboards.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-blend.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-bloat.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-cap-butt.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-cap-round.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-cap-square.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-center-hide.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-center-show.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-clear.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-color-guide.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-crystallize.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-cube.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-dash-align.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-dash-exact.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-dir-off.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-dir-on.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-direct.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-dist-bottom.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-dist-hcenter.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-dist-hspace.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-dist-left.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-dist-right.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-dist-top.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-dist-vcenter.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-dist-vspace.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-draw-behind.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-draw-inside.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-draw-normal.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-ellipse.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-fill-none.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-flare.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-folder.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-free-transform.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-fx.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-gamut.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-grad-freeform.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-grad-linear.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-grad-radial.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-grad-stroke-across.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-grad-stroke-along.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-grad-stroke-within.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-gradient.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-graphic-styles.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-grid-view.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-group-select.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-join-bevel.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-join-miter.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-join-round.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-join.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-knife.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-line-cut.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-line.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-list-view.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-live-bucket.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-live-select.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-mask-none.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-measure.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-mesh.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-mirror-cut.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-new-fill.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-new-item.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-new-stroke.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-options.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-para-center.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-para-justify-all.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-para-justify-center.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-para-justify-left.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-para-justify-right.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-para-left.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-para-ltr.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-para-right.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-para-rtl.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-path-eraser.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-pathfinder.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-perspective.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-pf-crop.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-pf-divide.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-pf-exclude.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-pf-intersect.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-pf-merge.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-pf-minus-back.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-pf-minus-front.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-pf-outline.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-pf-trim.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-pf-unite.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-place-symbol.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-polar-grid.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-polygon.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-pucker.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-pin-warp.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-rearrange.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-rect-cut.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-rect-grid.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-reference-point.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-remove-brush.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-reshape.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-reverse.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-rotate-view.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-rounded-rect.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-rule-evenodd.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-rule-nonzero.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-scallop.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-screen-mode.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-selection.svg` | upstream contributors (see NOTICE); filled by the Kition authors | The upstream icon set, modified | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-shape-builder.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-shear.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-smooth.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-stroke-center.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-stroke-inside.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-stroke-outside.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-stroke.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-swap.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-swatch-kinds.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-symbol-sprayer.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-touch-type.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-transform-panel.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-transparency.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-twirl.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-type-area.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-type-path.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-type-vertical.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-width-profile.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-width.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-wrinkle.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-zoom-large.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/dc-zoom-small.svg` | upstream contributors (see NOTICE) | The upstream icon set, byte-identical | MIT ([NOTICE](../NOTICE)) |  |
| `assets/icons/ellipsis.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/eraser.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/eye-off.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/eye.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/file-plus.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/flip-horizontal-2.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/flip-vertical-2.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/folder-open.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/frame.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/grid-3x3.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/group.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/hand.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/hexagon.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/history.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/house.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/image.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/info.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC; derived from Feather, also MIT (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/lasso.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/layers.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/layout-grid.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/library.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/link-2-off.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/link.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC; derived from Feather, also MIT (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/external-link.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC; derived from Feather, also MIT (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/git-branch.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/globe.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/message-circle.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/lock-open.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/lock.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC; derived from Feather, also MIT (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/map.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/maximize-2.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/menu.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/minus.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC; derived from Feather, also MIT (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/monitor.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC; derived from Feather, also MIT (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/move-diagonal-2.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/move.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC; derived from Feather, also MIT (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/paint-bucket.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/paintbrush.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/palette.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/pen-tool.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/pen-tool-add.svg` | Lucide Icons and Contributors; badge by Kition Vector contributors | https://lucide.dev (lucide-static v1.49.0 `pen-tool`; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) | Lucide's pen-tool with a plus badge |
| `assets/icons/pen-tool-delete.svg` | Lucide Icons and Contributors; badge by Kition Vector contributors | https://lucide.dev (lucide-static v1.49.0 `pen-tool`; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) | Lucide's pen-tool with a minus badge |
| `assets/icons/pencil.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/pilcrow.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/pin.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/pipette.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/plus.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC; derived from Feather, also MIT (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/printer.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/redo-2.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/rotate-3d.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/rotate-ccw.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/rotate-cw.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/ruler.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/save.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/scaling.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/scissors.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/search.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC; derived from Feather, also MIT (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/settings.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/shapes.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/share-2.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/slice.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/sparkles.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/spline-pointer.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/spline.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/spray-can.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/square-dashed.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/square.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC; derived from Feather, also MIT (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/squares-exclude.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/squares-intersect.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/squares-subtract.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/squares-unite.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/squircle.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/star.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/sun.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/swatch-book.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/text-cursor-input.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/tornado.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/trash-2.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC; derived from Feather, also MIT (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/triangle.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC; derived from Feather, also MIT (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/type.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC; derived from Feather, also MIT (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/undo-2.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/ungroup.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/wand-sparkles.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/waves.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/x.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC; derived from Feather, also MIT (`assets/icons/LICENSE-lucide.txt`) |  |
| `assets/icons/zoom-in.svg` | Lucide Icons and Contributors | https://lucide.dev (lucide-static v1.49.0; header kept in file) | ISC; derived from Feather, also MIT (`assets/icons/LICENSE-lucide.txt`) |  |
| `examples/dusk-poster.kvector` | upstream contributors (see NOTICE) | The upstream example artwork, converted to `.kvector` by the Kition authors (product name in its text replaced) | MIT ([NOTICE](../NOTICE)) |  |
| `examples/dusk-poster.svg` | upstream contributors (see NOTICE) | The upstream SVG export of the above (product name in its text replaced) | MIT ([NOTICE](../NOTICE)) |  |
| `examples/neon-drive.kvector` | upstream contributors (see NOTICE) | The upstream example artwork, converted to `.kvector` by the Kition authors (product name in its text replaced) | MIT ([NOTICE](../NOTICE)) |  |
| `examples/ribbons.kvector` | upstream contributors (see NOTICE) | The upstream example artwork, converted to `.kvector` by the Kition authors (product name in its text replaced) | MIT ([NOTICE](../NOTICE)) |  |
| `examples/feature-sheet.kvector` | upstream contributors (see NOTICE) | The upstream example artwork, converted to `.kvector` by the Kition authors (product name in its text replaced) | MIT ([NOTICE](../NOTICE)) |  |
| `examples/perspective-city.kvector` | upstream contributors (see NOTICE) | The upstream example artwork, converted to `.kvector` by the Kition authors (product name in its text replaced) | MIT ([NOTICE](../NOTICE)) |  |
| `web/src/i18n/upstream/cs.tsv` | upstream contributors (see NOTICE) | The upstream Czech interface catalog: original translations of the upstream English labels, edited by the Kition authors | MIT ([NOTICE](../NOTICE)) | Czech UI strings |
| `web/src/i18n/upstream/de.tsv` | upstream contributors (see NOTICE) | The upstream German interface catalog: original translations of the upstream English labels, edited by the Kition authors | MIT ([NOTICE](../NOTICE)) | German UI strings |
| `web/src/i18n/upstream/es.tsv` | upstream contributors (see NOTICE) | The upstream Spanish interface catalog: original translations of the upstream English labels, edited by the Kition authors | MIT ([NOTICE](../NOTICE)) | Spanish UI strings |
| `web/src/i18n/upstream/fr.tsv` | upstream contributors (see NOTICE) | The upstream French interface catalog: original translations of the upstream English labels, edited by the Kition authors | MIT ([NOTICE](../NOTICE)) | French UI strings |
| `web/src/i18n/upstream/it.tsv` | upstream contributors (see NOTICE) | The upstream Italian interface catalog: original translations of the upstream English labels, edited by the Kition authors | MIT ([NOTICE](../NOTICE)) | Italian UI strings |
| `web/src/i18n/upstream/ja.tsv` | upstream contributors (see NOTICE) | The upstream Japanese interface catalog: original translations of the upstream English labels, edited by the Kition authors | MIT ([NOTICE](../NOTICE)) | Japanese UI strings |
| `web/src/i18n/upstream/pt-br.tsv` | upstream contributors (see NOTICE) | The upstream Brazilian Portuguese interface catalog: original translations of the upstream English labels, edited by the Kition authors | MIT ([NOTICE](../NOTICE)) | Brazilian Portuguese UI strings |
| `web/src/i18n/upstream/ru.tsv` | upstream contributors (see NOTICE) | The upstream Russian interface catalog: original translations of the upstream English labels, edited by the Kition authors | MIT ([NOTICE](../NOTICE)) | Russian UI strings |
| `web/src/i18n/upstream/uk.tsv` | upstream contributors (see NOTICE) | The upstream Ukrainian interface catalog: original translations of the upstream English labels, edited by the Kition authors | MIT ([NOTICE](../NOTICE)) | Ukrainian UI strings |
| `web/src/i18n/upstream/zh-hans.tsv` | upstream contributors (see NOTICE) | The upstream Simplified Chinese interface catalog: original translations of the upstream English labels, edited by the Kition authors | MIT ([NOTICE](../NOTICE)) | Simplified Chinese UI strings |
| `web/src/i18n/upstream/zh-hant.tsv` | upstream contributors (see NOTICE) | The upstream Traditional Chinese interface catalog: original translations of the upstream English labels, edited by the Kition authors | MIT ([NOTICE](../NOTICE)) | Traditional Chinese UI strings |
| `web/public/sample/Sample.kvector` | Kition Vector contributors (generated) | Written by `web/scripts/sample.mjs` through the engine's own commands | Apache-2.0 |  |
| `conformance/basic.kvector` | Kition Vector contributors | Generated by the engine (`KITION_WRITE_CONFORMANCE=1 cargo test -p kition-vector-kit conformance`) | Apache-2.0 | KHP conformance fixture |
| `conformance/check-empty-group.kvector` | Kition Vector contributors | Generated by the engine (`KITION_WRITE_CONFORMANCE=1 cargo test -p kition-vector-kit conformance`) | Apache-2.0 | KHP conformance fixture |
| `conformance/check-empty-text.kvector` | Kition Vector contributors | Generated by the engine (`KITION_WRITE_CONFORMANCE=1 cargo test -p kition-vector-kit conformance`) | Apache-2.0 | KHP conformance fixture |
| `conformance/check-font-substituted.kvector` | Kition Vector contributors | Generated by the engine (`KITION_WRITE_CONFORMANCE=1 cargo test -p kition-vector-kit conformance`) | Apache-2.0 | KHP conformance fixture |
| `conformance/check-missing-glyphs.kvector` | Kition Vector contributors | Generated by the engine (`KITION_WRITE_CONFORMANCE=1 cargo test -p kition-vector-kit conformance`) | Apache-2.0 | KHP conformance fixture |
| `conformance/check-missing-graphic-style.kvector` | Kition Vector contributors | Generated by the engine (`KITION_WRITE_CONFORMANCE=1 cargo test -p kition-vector-kit conformance`) | Apache-2.0 | KHP conformance fixture |
| `conformance/check-missing-image.kvector` | Kition Vector contributors | Generated by the engine (`KITION_WRITE_CONFORMANCE=1 cargo test -p kition-vector-kit conformance`) | Apache-2.0 | KHP conformance fixture |
| `conformance/check-missing-link.kvector` | Kition Vector contributors | Generated by the engine (`KITION_WRITE_CONFORMANCE=1 cargo test -p kition-vector-kit conformance`) | Apache-2.0 | KHP conformance fixture |
| `conformance/check-missing-symbol.kvector` | Kition Vector contributors | Generated by the engine (`KITION_WRITE_CONFORMANCE=1 cargo test -p kition-vector-kit conformance`) | Apache-2.0 | KHP conformance fixture |
| `conformance/check-no-artboard.kvector` | Kition Vector contributors | Generated by the engine (`KITION_WRITE_CONFORMANCE=1 cargo test -p kition-vector-kit conformance`) | Apache-2.0 | KHP conformance fixture |
| `conformance/check-off-artboard.kvector` | Kition Vector contributors | Generated by the engine (`KITION_WRITE_CONFORMANCE=1 cargo test -p kition-vector-kit conformance`) | Apache-2.0 | KHP conformance fixture |
| `conformance/check-stray-point.kvector` | Kition Vector contributors | Generated by the engine (`KITION_WRITE_CONFORMANCE=1 cargo test -p kition-vector-kit conformance`) | Apache-2.0 | KHP conformance fixture |
| `conformance/check-truncated.kvector` | Kition Vector contributors | Generated by the engine (`KITION_WRITE_CONFORMANCE=1 cargo test -p kition-vector-kit conformance`) | Apache-2.0 | KHP conformance fixture |
| `conformance/golden/basic-scale-2.png` | Kition Vector contributors | Generated by the engine (`KITION_WRITE_CONFORMANCE=1 cargo test -p kition-vector-kit conformance`) | Apache-2.0 | KHP conformance fixture |
| `conformance/golden/basic.png` | Kition Vector contributors | Generated by the engine (`KITION_WRITE_CONFORMANCE=1 cargo test -p kition-vector-kit conformance`) | Apache-2.0 | KHP conformance fixture |
| `cases/vector.agent.act/back-artboard/input/shapes.kvector` | Kition Vector contributors | Copied from the conformance fixture the engine generates (`KITION_WRITE_CONFORMANCE=1 cargo test -p kition-vector-kit conformance`) | Apache-2.0 | eval case input |
| `cases/vector.agent.draft/stroked-box/input/shapes.kvector` | Kition Vector contributors | Copied from the conformance fixture the engine generates (`KITION_WRITE_CONFORMANCE=1 cargo test -p kition-vector-kit conformance`) | Apache-2.0 | eval case input |
| `cases/vector.agent.observe/wider-shape/input/shapes.kvector` | Kition Vector contributors | Copied from the conformance fixture the engine generates (`KITION_WRITE_CONFORMANCE=1 cargo test -p kition-vector-kit conformance`) | Apache-2.0 | eval case input |
| `cases/vector.agent.recover/off-artboard/input/stray.kvector` | Kition Vector contributors | Copied from the conformance fixture the engine generates (`KITION_WRITE_CONFORMANCE=1 cargo test -p kition-vector-kit conformance`) | Apache-2.0 | eval case input |
| `assets/readme/hero.png` | The Kition authors | Screenshot of Kition Vector's editor, taken by `web/scripts/readme-shots.mjs` on the showcase sample alpine-lake-sunrise | Apache-2.0 | A README picture |
| `assets/readme/see-1.png` | The Kition authors | Screenshot of Kition Vector's editor, taken by `web/scripts/readme-shots.mjs` on the showcase sample alpine-lake-sunrise | Apache-2.0 | A README picture |
| `assets/readme/see-2.png` | The Kition authors | Screenshot of Kition Vector's editor, taken by `web/scripts/readme-shots.mjs` on the showcase sample alpine-lake-sunrise | Apache-2.0 | A README picture |
| `assets/readme/see-3.png` | The Kition authors | Screenshot of Kition Vector's editor, taken by `web/scripts/readme-shots.mjs` on the showcase sample alpine-lake-sunrise | Apache-2.0 | A README picture |
| `assets/readme/see-4.png` | The Kition authors | Screenshot of Kition Vector's editor, taken by `web/scripts/readme-shots.mjs` on the showcase sample weather-icon-set (exported as SVG and opened again) | Apache-2.0 | A README picture |
| `assets/readme/made-1.png` | The Kition authors | Rendered by Kition Vector from the showcase sample isometric-corner-bookshop (its `thumb.png` in kition-showcase) | Apache-2.0 | A README picture |
| `assets/readme/made-2.png` | The Kition authors | Rendered by Kition Vector from the showcase sample reading-owl-mascot (its `thumb.png` in kition-showcase) | Apache-2.0 | A README picture |
| `assets/readme/made-3.png` | The Kition authors | Rendered by Kition Vector from the showcase sample saltmarsh-bay-trail-map (its `thumb.png` in kition-showcase) | Apache-2.0 | A README picture |
| `assets/readme/made-4.png` | The Kition authors | Rendered by Kition Vector from the showcase sample retro-diner-lettering (its `thumb.png` in kition-showcase) | Apache-2.0 | A README picture |
