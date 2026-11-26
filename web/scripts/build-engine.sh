#!/bin/sh
# Builds the Vector engine (bindings/wasm) to WebAssembly into public/engine, where the
# editor loads it from its own package. Needs wasm-pack.
set -eu
web="$(cd "$(dirname "$0")/.." && pwd)"
repo="$(cd "$web/.." && pwd)"
if toolchain="$(rustc --print sysroot 2>/dev/null)" && [ -d "$toolchain/lib" ]; then
  export DYLD_FALLBACK_LIBRARY_PATH="$toolchain/lib${DYLD_FALLBACK_LIBRARY_PATH:+:$DYLD_FALLBACK_LIBRARY_PATH}"
fi
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$repo/target}"
cd "$repo"
wasm-pack build bindings/wasm --target web --release --no-typescript --out-dir "$web/public/engine"
rm -f "$web/public/engine/.gitignore" "$web/public/engine/package.json"
# Fonts (plan 2026-11-25, stage 5), from the kition-fonts checkout next to the repository (or
# KITION_FONTS_DIR); never committed here. Each folder is copied whole (the font files with their
# licence beside them) into public/fonts, and its lines of kition-fonts' manifest become
# public/fonts/manifest.txt, which the font menu reads (web/src/type/served.ts): a family loads when
# a person picks it or a drawing names it. Droid Sans Fallback is also the engine's lazy font
# (web/src/engine.ts): it loads once text has a character no other face has (Chinese).
kition_fonts="${KITION_FONTS_DIR:-$repo/../kition-fonts}"
served="droid-sans-fallback lora bricolage-grotesque arimo tinos cousine ma-shan-zheng"
rm -rf "$web/public/fonts"
if [ -f "$kition_fonts/fonts/manifest.txt" ]; then
  mkdir -p "$web/public/fonts"
  {
    echo "# The font menu's served fonts, from kition-fonts' manifest (written by scripts/build-engine.sh)."
    for family in $served; do
      if [ -d "$kition_fonts/fonts/$family" ]; then
        cp -R "$kition_fonts/fonts/$family" "$web/public/fonts/"
        grep -F " | fonts/$family/" "$kition_fonts/fonts/manifest.txt" || true
      else
        echo "warning: kition-fonts has no fonts/$family; the font menu leaves it out" >&2
      fi
    done
  } > "$web/public/fonts/manifest.txt"
else
  echo "warning: no kition-fonts checkout at $kition_fonts; Chinese type shows as boxes and the font menu has only the engine's own families" >&2
fi
