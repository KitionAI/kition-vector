import { useMemo, useState } from 'react'
import { useKitEditor } from '@kition/sdk/editor'
import { ChoiceRow, NumberRow, TextRow } from '../dialogs/controls'
import { DialogFrame } from '../dialogs/DialogFrame'
import type { NewPreset, NewPresets } from '../dialogs/NewDocumentDialog'
import { fromPoints, toPoints, unitAbbreviation, unitDecimals } from '../dialogs/units'

/** An artboard as the dialog starts from it (the drawing's points). */
export type ArtboardStart = { index: number; name: string; x: number; y: number; width: number; height: number }

const CUSTOM = 'custom'

/** The presets of `file.newPresets` a size can be picked from (each once by name), Print first. */
export function sizePresets(presets: NewPresets | null): NewPreset[] {
  const seen = new Set<string>()
  const order = ['Print', 'Web', 'Mobile', 'Film & Video', 'Art & Illustration']
  const categories = [...(presets?.categories ?? [])].sort((a, b) => (order.indexOf(a.name) + 1 || 99) - (order.indexOf(b.name) + 1 || 99))
  return categories.flatMap((category) => category.presets).filter((preset) => preset.width > 0 && preset.height > 0 && !seen.has(preset.name) && seen.add(preset.name) !== undefined)
}

/**
 * Artboard Options (plan 2026-11-25, stage 4), from the Artboards panel, its menu, or the Artboard
 * tool's double-click: the name, a preset size, width and height, the orientation and the position,
 * in the document's unit, previewed on the canvas and one `artboard.setProps` step on OK.
 */
export function ArtboardOptionsDialog({ start, presets, units, onClose }: { start: ArtboardStart; presets: NewPresets | null; units: string; onClose: () => void }) {
  const { t, phrase } = useKitEditor()
  const sizes = useMemo(() => sizePresets(presets), [presets])
  const [name, setName] = useState(start.name)
  const [x, setX] = useState(start.x)
  const [y, setY] = useState(start.y)
  const [width, setWidth] = useState(start.width)
  const [height, setHeight] = useState(start.height)
  const matching = (w: number, h: number) => sizes.find((preset) => (Math.abs(preset.width - w) < 0.01 && Math.abs(preset.height - h) < 0.01) || (Math.abs(preset.width - h) < 0.01 && Math.abs(preset.height - w) < 0.01))
  const [preset, setPreset] = useState(() => matching(start.width, start.height)?.name ?? CUSTOM)
  const orientation = width > height ? 'landscape' : 'portrait'
  const unit = unitAbbreviation(units)
  const decimals = unitDecimals(units)
  const valid = width >= 1 && height >= 1 && width <= 16_383 && height <= 16_383 && name.trim().length > 0
  const params = { index: start.index, name: name.trim() || start.name, x, y, width, height }
  return (
    <DialogFrame command="artboard.setProps" title={phrase('Artboard Options')} params={params} previewing onClose={onClose} okDisabled={!valid}>
      <div className="vector-dialog-fields" data-command="artboard.setProps">
        <TextRow label={phrase('Name')} value={name} onChange={setName} testId="vector-artboard-name" />
        <ChoiceRow
          label={t('dialog.preset')}
          value={preset}
          choices={[...sizes.map((item) => [item.name, item.size ? `${item.name} (${item.size})` : item.name] as const), [CUSTOM, t('dialog.preset.custom')] as const]}
          onChange={(next) => {
            setPreset(next)
            const chosen = sizes.find((item) => item.name === next)
            if (!chosen) return
            // A preset keeps the orientation the artboard has.
            const [w, h] = orientation === 'landscape' ? [Math.max(chosen.width, chosen.height), Math.min(chosen.width, chosen.height)] : [Math.min(chosen.width, chosen.height), Math.max(chosen.width, chosen.height)]
            setWidth(w)
            setHeight(h)
          }}
          testId="vector-artboard-preset"
        />
        <NumberRow label={t('dialog.width')} value={fromPoints(width, units)} min={0} max={1_000_000} decimals={decimals} unit={unit} onChange={(next) => { setWidth(toPoints(next ?? 0, units)); setPreset(matching(toPoints(next ?? 0, units), height)?.name ?? CUSTOM) }} testId="vector-artboard-width" />
        <NumberRow label={t('dialog.height')} value={fromPoints(height, units)} min={0} max={1_000_000} decimals={decimals} unit={unit} onChange={(next) => { setHeight(toPoints(next ?? 0, units)); setPreset(matching(width, toPoints(next ?? 0, units))?.name ?? CUSTOM) }} testId="vector-artboard-height" />
        <ChoiceRow
          label={t('dialog.orientation')}
          value={orientation}
          choices={[['portrait', t('dialog.portrait')], ['landscape', t('dialog.landscape')]] as const}
          onChange={(next) => {
            if (next === orientation) return
            setWidth(height)
            setHeight(width)
          }}
          testId="vector-artboard-orientation"
        />
        <NumberRow label="X" value={fromPoints(x, units)} min={-1_000_000} max={1_000_000} decimals={decimals} unit={unit} onChange={(next) => setX(toPoints(next ?? 0, units))} testId="vector-artboard-x" />
        <NumberRow label="Y" value={fromPoints(y, units)} min={-1_000_000} max={1_000_000} decimals={decimals} unit={unit} onChange={(next) => setY(toPoints(next ?? 0, units))} testId="vector-artboard-y" />
        {valid ? null : <div className="vector-dialog-warning">{t('dialog.badSize')}</div>}
      </div>
    </DialogFrame>
  )
}
