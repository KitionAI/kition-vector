import { useEffect } from 'react'
import type { KitEngine } from '@kition/sdk/editor'
import { ensureFamily, loadCatalog } from './served'

/**
 * The fonts a drawing needs (plan 2026-11-25, stage 5): when the editor's lazy font (the Chinese
 * face, `engine.ts`) comes in, the drawing's type is laid out again in it; when a drawing opens
 * naming a family the editor serves but the engine does not have yet, that family loads.
 */
export function useDrawingFonts(doc: KitEngine | null) {
  useEffect(() => {
    if (!doc) return undefined
    let live = true
    const stop = doc.onFonts(() => {
      void doc
        .invoke<boolean>('refreshType')
        .then(() => doc.refreshInfo())
        .catch(() => undefined)
    })
    void (async () => {
      const answer = JSON.parse((await doc.execute('text.missingFonts', {}).catch(() => 'null')) || 'null') as { fonts?: Array<{ family?: string }> } | null
      const catalog = await loadCatalog()
      for (const font of answer?.fonts ?? []) {
        if (!live) return
        if (font.family && catalog.has(font.family)) await ensureFamily(doc, font.family)
      }
    })()
    return () => {
      live = false
      stop()
    }
  }, [doc])
}
