import type { KitModuleManifest } from '@kition/sdk/editor'

/**
 * The engine, from this package's own folder. The web build embeds no CJK face: the Chinese one
 * (`fonts/`, copied from kition-fonts by `scripts/build-engine.sh`) is given to the engine once its
 * text wants a face it does not have (`lazyFonts`, the engine's `wantsFonts`), so drawings without
 * Chinese never load it. Without it Chinese draws as missing-glyph boxes.
 */
export function engineManifest(): KitModuleManifest {
  return {
    kitId: 'kit.vector',
    base: new URL('./engine/', document.baseURI).href,
    module: 'kition_vector_kit',
    documentClass: 'VectorDocument',
    lazyFonts: [new URL('./fonts/droid-sans-fallback/DroidSansFallbackFull.ttf', document.baseURI).href],
  }
}
