# Development

> **Last updated:** 2026-11-17

How to build and test this repository. The rules that come first are in [`AGENTS.md`](../AGENTS.md) and
[`quality.md`](quality.md); how the crates fit together is in [architecture.md](architecture.md).

## Repository layout

| Path | What |
|---|---|
| `crates/` | The engine (Rust), in groups: `engine/`, `formats/`, `render/`, `support/` and `kdm/` |
| `bindings/wasm` | `kition-vector-kit`: the engine compiled to WebAssembly for the in-page editor |
| `bindings/headless` | `kition-vector`: the headless kit executable that Kition's agent drives |
| `web/` | The kit's editor (React, on `@kition/kit-editor`), which a host shows in an iframe |
| `tools/` | Repository tooling: `tools/xtask` (`cargo xtask <command>`) |
| `examples/` | Sample `.kvector` drawings |
| `cases/`, `features.yaml` | The feature catalogue and its cases ([`quality.md`](quality.md)) |
| `conformance/` | The agent conformance fixtures the cases refer to |
| `agent/` | The agent guide (`guide.md`) and the check catalogue (`checks.json`) |
| `assets/` | Fonts, icons and the README's pictures (`assets/readme/`), each attributed in [`attribution.md`](attribution.md) |
| `docs/` | This documentation: the entry documents at the top, target-app parity checklists in `parity/`, feature notes in `features/` ([plug-ins](features/plugins.md), [Revolve](features/revolve.md), [Shaper](features/shaper.md), [Affinity validation](features/affinity-validation.md)) |
| `kit.json` | The kit manifest (KHP) |

## Tools

Rust 1.95 or later with rustfmt, clippy and the `wasm32-unknown-unknown` target, wasm-pack, Node.js 22 and
pnpm; [`.github/CONTRIBUTING.md`](../.github/CONTRIBUTING.md) gives the exact versions.

## The engine and the bindings

```sh
cargo test --workspace                    # every crate, both bindings and xtask
cargo build --release -p kition-vector    # the headless kit executable (bindings/headless)
cargo xtask ci                            # the gate: run it before every commit
```

`cargo xtask ci` runs these steps in order and stops at the first failure:

| Step | Command |
|---|---|
| fmt | `cargo fmt --all -- --check` |
| clippy | `cargo clippy --workspace --all-targets -- -D warnings` |
| test | `cargo test --workspace` |
| assets | `cargo xtask assets`: every icon, image and font has a row in `docs/attribution.md` |
| layers | `cargo xtask layers`: a crate depends only on lower layers ([architecture.md](architecture.md#layers)) |
| wasm | `cargo xtask wasm`: `cargo check --target wasm32-unknown-unknown` for every crate up to L5 |

Other `cargo xtask` commands (`tools/xtask/src/main.rs`):

| Command | What it does |
|---|---|
| `corpus [--download]` | Says where the test corpora live (`corpus/`, git-ignored); `--download` fetches PngSuite |
| `stats [--exact]` | Counts tests and lines per crate |
| `version [set X.Y.Z[-pre]]` | Prints or sets the workspace version |
| `eval [options]` | The real-model `layer: eval` cases. Only a maintainer starts it; `--dry-run` calls no model |

The headless kit executable is `bindings/headless/src/main.rs`: it serves the same document as the in-page
editor (`kition-vector-kit`) over Kition's Kit Protocol through kition-sdk's `kition-headless`, as `kit.json`
declares it. The host owns every file; the kit reaches none by path. Its tests are in
`bindings/headless/tests/`.

Environment variables the tests read:

| Variable | Effect |
|---|---|
| `PROPTEST_CASES` | Number of cases the property tests run |
| `AFFINITY_CORPUS_REQUIRED` | `1`: the Affinity corpus tests fail instead of skipping when the corpus is absent ([affinity-validation.md](features/affinity-validation.md)) |
| `KITION_WASM_ENGINE` | Enables the native-against-WebAssembly parity test in `bindings/wasm/tests/convergence.rs`, which is skipped when it is unset |

## The editor (`web/`)

```sh
cd web
pnpm install
pnpm engine      # builds bindings/wasm into web/public/engine with wasm-pack
pnpm dev         # Vite dev server on port 5185; open dev.html, the development host
```

The editor only runs inside a host: `web/index.html` opened on its own says so after three seconds.
`web/dev.html` is the development host, which shows the editor in an iframe as Kition does.

| Script (`web/package.json`) | What it does |
|---|---|
| `pnpm engine` | `scripts/build-engine.sh`: `wasm-pack build bindings/wasm --target web --release` into `public/engine` |
| `pnpm dev` | The Vite dev server |
| `pnpm build` | `tsc --noEmit`, then `vite build` into `web/dist` |
| `pnpm typecheck` | `tsc --noEmit` |
| `pnpm test:unit` | `node --test src/exporting.test.mjs` |
| `pnpm test:flow` | `tests/flow.mjs` in Chromium and WebKit (Playwright): the editor's flow through the development host |
| `pnpm test:modes` | `tests/modes.mjs` in Chromium and WebKit |
| `pnpm test:focus` | `tests/focus.mjs` in Chromium and WebKit |

The browser tests start the Vite dev server themselves; they need the engine built (`pnpm engine`) and
Playwright's browsers installed.

By default the editor's dependencies come from npm: `pnpm install` installs the published `@kition/*`
packages at the versions the committed `web/pnpm-lock.yaml` pins, so no other checkout is needed. To
develop against local checkouts instead, run `KITION_LINK=1 pnpm install` in `web/`: `web/.pnpmfile.cjs`
then links `@kition/kit-editor`, `@kition/kit-host`, `@kition/kit-ui-protocol` and `@kition/kit-dev-host`
from a kition-sdk checkout, and `@kition/tessera-icons` from a kition-tessera checkout (by default
`../kition-sdk` and `../kition-tessera` beside this repository; `KITION_SDK` and `KITION_TESSERA` name
others). A package whose folder is not there stays on npm. The linked install rewrites
`web/pnpm-lock.yaml`; a lockfile changed that way must not be committed.

## Run the headless kit for an agent

`kition-vector` (`bindings/headless`) serves one drawing at a time over the Kit Protocol;
kition-sdk's `kition-kits-mcp` turns it into MCP tools (`vector_inspect`, `vector_run`,
`vector_check`, ...) on a workspace folder for any MCP client:

```sh
cargo build --release -p kition-vector   # target/release/kition-vector
# in a kition-sdk checkout:
cargo build --release -p kition-kits --bin kition-kits-mcp
target/release/kition-kits-mcp --workspace <folder> --kit <kition-vector>/target/release/kition-vector
```

How an agent uses the tools is in [agent/guide.md](../agent/guide.md); kition-sdk's
[agent guide](https://github.com/KitionAI/kition-sdk/blob/main/docs/sdk/agent.md) covers the server.

## Take the README pictures

[`web/scripts/readme-shots.mjs`](../web/scripts/readme-shots.mjs) takes the README's editor pictures
(`assets/readme/hero.png`, `see-1.png` … `see-4.png`) with kition-sdk's shared runner
(`tools/readme-shots/`). It opens the showcase sample `vector/alpine-lake-sunrise` (and, for the
SVG picture, `vector/weather-icon-set`, exported as SVG and opened again) in the development host
(`web/dev.html`) at 1440 × 900, device scale 2, in Classic mode, the light theme and English, and
works the editor as a person would: a click on the drawing, the ribbon, the zoom buttons and the
Export dialog. It needs the engine built in `web/public/` (`pnpm engine`), and kition-sdk and
kition-showcase checkouts beside this repository (or `KITION_SDK` and `KITION_SHOWCASE` naming
them):

```sh
cd web
node scripts/readme-shots.mjs            # every picture
node scripts/readme-shots.mjs see-2      # only the named ones
```

Run it with no HTTP proxy set; `CHROME` may name a Chrome or Chromium executable to use instead of
Playwright's. The click on the sun is measured on the drawing as it opens in a 1440 × 900 window,
so a change to the editor's layout may need it measured again. When `pngquant` is installed the
script shrinks each picture with `pngquant --quality 70-90`; a README picture is at most 400 KB.
`made-1.png` … `made-4.png` are copies of showcase samples' `thumb.png` (isometric-corner-bookshop,
reading-owl-mascot, saltmarsh-bay-trail-map, retro-diner-lettering). Every picture has a row in
[attribution.md](attribution.md).

## Publish

Kition ships the kit's package, `kition-vector-<version>.tgz` (the `vector-ui/` folder), a release
asset of this repository tagged `package-v<version>`. It is built on GitHub, never on a developer
machine: the Package workflow (`.github/workflows/package.yml`, which calls Kition's reusable
`kit-package.yml`) builds the commit it is started on, with `gh workflow run package.yml -R
KitionAI/kition-vector`, and publishes the Release with the package's lock entry,
`kition-vector-<version>.json`. Kition pins it with `node scripts/pin-kits.mjs vector` (see
`docs/kits.md` there). A version is published once: bump `version` in `kit.json` to publish a
change.

## Fonts: kition-fonts (optional build input)

Font files are never committed to this repository. Fonts live in
[KitionAI/kition-fonts](https://github.com/KitionAI/kition-fonts). To add
a font, add it there. (The Latin fonts in `assets/fonts/` predate the rule and stay.)

Kition Vector builds and runs without kition-fonts (for the tests, see below). To embed its fonts, point the `KITION_FONTS_DIR`
build option at a checkout:

```sh
git clone https://github.com/KitionAI/kition-fonts ../kition-fonts
KITION_FONTS_DIR="$PWD/../kition-fonts" cargo xtask ci
```

- `crates/support/text/build.rs` reads the checkout's `fonts/manifest.txt` and embeds every font it lists as
  `kition_vector_text::KITION_FONTS` (empty without `KITION_FONTS_DIR`). Give it an absolute path:
  build scripts run in the crate's directory. Nothing is downloaded, and kition-fonts is never a
  `Cargo.toml` dependency. A bad path is a build warning, or an error with `KITION_FONTS_REQUIRED=1`.
- A `wasm32` build embeds only BIZ UDPGothic Regular, to keep the `.wasm` small.
- The text crate's tests that need the kition-fonts glyphs skip when built without it.
  `bindings/wasm/tests/kit_fonts.rs` is the exception: at run time it reads a font from the checkout
  (`KITION_FONTS_DIR`, else `../kition-fonts` beside this repository) and fails without one.

## Localisation

The editor's text comes from three places, all in `web/src`:

- `i18n/upstream/<code>.tsv`: one catalog per language for the engine's English labels and command titles
  (`cs`, `de`, `es`, `fr`, `it`, `ja`, `pt-br`, `ru`, `uk`, `zh-hans`, `zh-hant`). The format is documented in
  the header of `zh-hant.tsv`.
- `PHRASES` in `strings.ts`: translations for labels the catalogs lack.
- `CATALOGS` in `strings.ts`: the editor's own strings (`en-US` and `zh-CN`).

The host gives the locale (`host.init.locale`); `main.tsx` loads the matching catalog. Command ids and
parameters are never translated, so agents always see the English ids.

Translations are clean-room: written from the meaning of the English text in ordinary vocabulary, never from
another product's localisation resources.

## Never crash: no panics in shipped code

A crash takes the user's unsaved work with it, and much of what the engine reads is untrusted: SVG,
PDF, `.ai` and `.kvector` files, pasted data, Kit Protocol requests, command
parameters. Shipped code therefore never panics. Anything that can fail returns `Result` (or
`Option`), the caller handles or propagates it, and the user sees an error message.

### Enforced by lints

`Cargo.toml` denies these clippy lints for the whole workspace, and `cargo xtask ci` runs clippy with
`-D warnings`. The gate targets the current stable Rust (`rust-version` in `Cargo.toml` is only the
minimum that builds the workspace, and no toolchain is pinned): when a new stable adds lints, they are fixed
in the code:

| Banned in shipped code | Use instead |
|---|---|
| `x.unwrap()`, `x.expect("…")` | `x?`, `x.ok_or(err)?`, `let Some(v) = x else { return … }`, `if let`, `unwrap_or` / `unwrap_or_default` / `unwrap_or_else` |
| `v.last().unwrap()`, `v.pop().unwrap()` | `if let Some(l) = v.last_mut()`, `let Some((last, rest)) = v.split_last() else { … }` |
| `panic!`, `unreachable!()` in a match arm | return an error (`EngineError::Other`, `bad(C, "…")`) or a sensible default |
| `todo!`, `unimplemented!` | don't merge unfinished paths; return an error that says what isn't supported |

`.clippy.toml` allows `unwrap`, `expect` and `panic` inside `#[test]` and `#[cfg(test)]` code.
Integration-test files under `tests/` and `examples/`, and the `testkit` crate, opt out with
`#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]`. Tests use `panic!("…")`
instead of `unreachable!()`.

### Not caught by lints: avoid implicit panics too

- **Indexing and slicing:** `v[i]` and `&s[a..b]` panic when out of range. When the index comes from
  data (a file, a parameter, another document), use `v.get(i)` / `s.get(a..b)`. Slicing a `&str` also
  panics off a UTF-8 boundary.
- **Arithmetic:** integer division or `%` by zero, `usize` subtraction that can go below zero (write
  `i + 1 == n` instead of `i == n - 1`, or use `saturating_sub`), and `as` casts from non-finite floats.
- **Unbounded work:** cap counts, sizes and allocations read from input. Image dimensions, repeat
  counts, line breaks and recursion depth all need a limit.
- **Encoders and decoders:** propagate their errors. Writing an empty file is a silent failure, not a fix.

Don't silence the lint by discarding errors. `let _ = …` and `.ok()` are for failures that really
don't matter, with a comment saying why.

### The safety net

`kition_vector_engine::guard::catch_panic` wraps the engine's entry points:

- `Session::execute` restores the active document, selection and interaction as they were before
  the top-level command and returns `EngineError::Internal`.
- Tool events (`Session::pointer`, `tool_key`, `tool_text`) reset the tool and cancel its drag.

The net exists for bugs, not as a substitute for `Result`. On wasm a panic aborts, so the
in-page engine has no net at all.

### Fuzzing

Untrusted input has property tests that must never panic:

- `crates/engine/engine/tests/import_fuzz.rs`: garbage, hostile and mutated SVG and PDF, and swatch
  (`.vcswatches`, `.gpl`, `.ase`), graphic style (`.vcstyles`) and flattener preset (`.vcflattener`)
  libraries, loaded and used, then rendered and exported.
- `crates/engine/engine/tests/command_sweep.rs`: every command with junk parameters.
- `crates/formats/format/tests/prop_format.rs`: garbage and mutated `.kvector` files.

Before touching an importer, run a deeper search, e.g.
`PROPTEST_CASES=20000 cargo test --release -p kition-vector-engine --test import_fuzz`. When it finds a
panic, fix the code and add the input as a regular test.

## Bidirectional type

Point, area and path type lay out Hebrew and Arabic with the Unicode bidirectional algorithm
(`unicode-bidi`, in `crates/support/text/src/layout.rs`). Each paragraph's base direction is its
`ParaStyle::direction` (`text.setFormat {direction}`), else its first strong character. Text is shaped in logical order, right-to-left runs right to left (`shape.rs` splits
segments by bidi level and script), lines are broken in logical order and then each line's whole
shaping clusters are put in visual order. Stored text, undo and style ranges stay logical; the
caret, selection, hit testing and arrow keys follow the visual order. A paragraph with nothing
right to left in it skips the algorithm, so plain text pays nothing for it.

Alignment is physical (`Justify::Left` is left in either direction). New type
(`text.create`, the Type tools) is aligned `Justify::Auto`, the start of each paragraph's direction:
Hebrew grows to the left of a point type's anchor. Imported text keeps its own alignment, and SVG
import pins the direction SVG implies (left to right unless `direction: rtl`).

Vertical type keeps its logical order down the column. SVG export outlines text with right-to-left
lines (with a warning) until it is written with `direction`/`unicode-bidi`; PDF and raster exports
use the laid-out glyphs. No bundled font covers these scripts, and the kit never draws with the
machine's installed fonts, only with its bundled fonts and those its host adds
(`bindings/wasm/tests/kit_fonts.rs`): until the host adds a font with these glyphs, `text.fonts` counts
them as missing and the check reports `vector.missing-glyphs`.
