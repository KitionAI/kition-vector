import type { KitModuleManifest } from '@kition/kit-editor'

/** The engine, from this package's own folder. */
export function engineManifest(): KitModuleManifest {
  return {
    kitId: 'kit.vector',
    base: new URL('./engine/', document.baseURI).href,
    module: 'kition_vector_kit',
    documentClass: 'VectorDocument',
  }
}
