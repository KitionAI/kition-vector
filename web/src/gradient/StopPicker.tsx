import { useLayoutEffect, type RefObject } from 'react'
import { useKitEditor } from '@kition/sdk/editor'
import { useColorPicker } from '../color/ColorPickerHost'
import { colorParam } from '../color/paint'
import { useLive } from '../panels/useLive'

/** Opens the Color Picker on stop `index` of the gradient being edited, starting from `hex`. */
export type PickStop = (index: number, hex: string | null) => void

/**
 * The canvas's gradient annotator asks for a stop's colour (a double-click on a stop, the Gradient
 * tool's request): the Color Picker opens on it, the stop takes each colour live (`paint.sampleColor`
 * with `stop`, which recolours that stop and keeps the gradient), and OK leaves one history step.
 * Drawn inside the editor's providers; App, outside them, reaches it through `api`.
 */
export function StopPicker({ api }: { api: RefObject<PickStop | null> }) {
  const pick = useColorPicker()
  const live = useLive()
  const { t } = useKitEditor()
  useLayoutEffect(() => {
    api.current = (index, hex) => {
      if (!pick) return
      void pick({
        title: t('gradient.stopColor'),
        value: { kind: 'color', hex: hex ?? '#000000' },
        onLive: (picked) => {
          if (picked.kind === 'color') live.set('paint.sampleColor', { color: colorParam(picked), stop: index })
        },
      }).then((picked) => {
        if (picked?.kind === 'color') void live.commit('paint.sampleColor', { color: colorParam(picked), stop: index })
        else void live.cancel()
      })
    }
  })
  return null
}
