import { useCallback } from 'react'
import { useEngineQuery, useKitEditor } from '@kition/sdk/editor'
import { useColorPicker } from '../color/ColorPickerHost'
import { pickedOf, pickedParams, type EnginePaint, type Proxies } from '../color/paint'
import type { VectorInfo } from '../vector'
import { useLive } from './useLive'

/** What the panels re-read on: the selection and the proxy in front (the revision is the query's own). */
const stateKey = (info: VectorInfo) => `${info.selection.join(',')}|${info.paint.fillActive}`

/** The Fill and Stroke proxies' paints (`paint.proxies`): the selection's, else what new art gets. */
export function useProxies(info: VectorInfo): Proxies | null {
  return useEngineQuery<Proxies>('paint.proxies', {}, stateKey(info))
}

/** The selection's opacity (0 to 100) and blending mode (`transparency.info`; null where they differ). */
export function useTransparency(info: VectorInfo): { opacity: number | null; blend: string | null } | null {
  return useEngineQuery<{ opacity: number | null; blend: string | null }>('transparency.info', {}, stateKey(info))
}

/**
 * Opens the Color Picker on the fill or the stroke (`which`), from `paint`: the canvas shows each
 * colour as the person moves through them, and OK leaves one history step (`useLive`).
 */
export function usePaintPicker() {
  const pick = useColorPicker()
  const live = useLive()
  const { t } = useKitEditor()
  return useCallback(
    (which: 'fill' | 'stroke', paint: EnginePaint | null | undefined) => {
      const command = which === 'fill' ? 'paint.setFill' : 'paint.setStroke'
      if (!pick) return
      void pick({
        title: t(which === 'fill' ? 'color.fillTitle' : 'color.strokeTitle'),
        value: pickedOf(paint),
        allowNone: true,
        onLive: (picked) => live.set(command, pickedParams(picked)),
      }).then((picked) => {
        if (picked) void live.commit(command, pickedParams(picked))
        else void live.cancel()
      })
    },
    [pick, live, t],
  )
}
