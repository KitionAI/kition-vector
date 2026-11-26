import { useState } from 'react'
import { useKitEditor } from '@kition/sdk/editor'
import { DialogFrame } from './DialogFrame'
import { CheckRow, ChoiceRow, SliderRow } from './controls'

const MODES = { rgb: ['r', 'g', 'b'], cmyk: ['c', 'm', 'y', 'k'], gray: ['gray'] } as const
type Mode = keyof typeof MODES

/** The last OK'd values, for the session. */
let last: { mode: Mode; channels: Record<string, number>; convert: boolean; fill: boolean; stroke: boolean } | null = null

/**
 * Edit › Edit Colors › Adjust Color Balance… (plan 2026-11-25, stage 1): a colour mode, a slider
 * per channel of it (-100 to 100 %, added to each colour), whether the results stay in that mode,
 * and whether fills and strokes change, previewed on the selection.
 */
export function ColorBalanceDialog({ title, onClose }: { title: string; onClose: () => void }) {
  const { t } = useKitEditor()
  const [mode, setMode] = useState<Mode>(last?.mode ?? 'rgb')
  const [channels, setChannels] = useState<Record<string, number>>(last?.channels ?? {})
  const [convert, setConvert] = useState(last?.convert ?? false)
  const [fill, setFill] = useState(last?.fill ?? true)
  const [stroke, setStroke] = useState(last?.stroke ?? true)
  const params = { mode, ...Object.fromEntries(MODES[mode].map((channel) => [channel, channels[channel] ?? 0])), convert, fill, stroke }
  return (
    <DialogFrame
      command="edit.colors.adjustBalance"
      title={title}
      params={params}
      previewing
      onReset={() => setChannels({})}
      onClose={(committed) => {
        if (committed) last = { mode, channels, convert, fill, stroke }
        onClose()
      }}
    >
      <div className="vector-dialog-fields">
        <ChoiceRow label={t('dialog.colorMode')} value={mode} choices={(Object.keys(MODES) as Mode[]).map((item) => [item, t(`dialog.colorMode.${item}`)] as const)} onChange={setMode} testId="vector-dialog-mode" />
        {MODES[mode].map((channel) => (
          <SliderRow key={channel} label={t(`dialog.channel.${channel}`)} value={channels[channel] ?? 0} min={-100} max={100} fallback={0} onChange={(next) => setChannels((current) => ({ ...current, [channel]: next }))} testId={`vector-dialog-${channel}`} />
        ))}
      </div>
      <div className="vector-dialog-options">
        <CheckRow label={t('dialog.convert')} checked={convert} onChange={setConvert} testId="vector-dialog-convert" />
        <CheckRow label={t('fill')} checked={fill} onChange={setFill} testId="vector-dialog-fill" />
        <CheckRow label={t('stroke')} checked={stroke} onChange={setStroke} testId="vector-dialog-stroke" />
      </div>
    </DialogFrame>
  )
}
