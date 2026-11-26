import { useMemo, useState } from 'react'
import { KitEngine, PreviewDialog, toast, useKitEditor } from '@kition/sdk/editor'
import { engineManifest } from '../engine'
import { ChoiceRow, NumberRow, TextRow } from './controls'
import { DIALOG_BUTTONS, DIALOG_TEST_ID, useDialogPlace } from './DialogFrame'
import { fromPoints, toPoints, unitAbbreviation, unitDecimals, UNITS } from './units'

/** One preset of `file.newPresets` (sizes in points). */
export type NewPreset = { name: string; size?: string; width: number; height: number; units: string; orientation?: string; artboards?: number }
/** `file.newPresets`'s answer: the presets by category (Recent, Saved, Mobile, Web, Print, …). */
export type NewPresets = { categories: Array<{ name: string; presets: NewPreset[] }> }

/** A file name without characters a file system refuses, never empty, ending in `.kvector`. */
export function documentFileName(name: string, fallback: string) {
  const clean = name.replace(/[\\/:*?"<>|]+/g, ' ').trim() || fallback
  return /\.kvector$/i.test(clean) ? clean : `${clean}.kvector`
}

const CUSTOM = 'custom'

/**
 * File › New… (plan 2026-11-25, stage 1): a name, a preset from the engine's New Document presets
 * (by category), the size in a unit, the orientation and the number of artboards. The editor shows
 * the one drawing its host opened, so OK makes the new drawing in an engine of its own
 * (`file.new` from the preset with the sizes chosen) and writes it beside this one through the
 * host (`host/write`, as an export), and says where; the person opens it from the workspace.
 */
export function NewDocumentDialog({ presets, onClose }: { presets: NewPresets | null; onClose: () => void }) {
  const { t, phrase, host } = useKitEditor()
  // Categories with presets, Recent and Saved only when they have some.
  const categories = useMemo(() => (presets?.categories ?? []).filter((category) => category.presets.length > 0), [presets])
  const first = categories.find((category) => category.name === 'Print') ?? categories[0]
  const [name, setName] = useState(() => t('dialog.untitled'))
  const [category, setCategory] = useState(first?.name ?? '')
  const [preset, setPreset] = useState(first?.presets[0]?.name ?? CUSTOM)
  const chosen = categories.find((item) => item.name === category)?.presets.find((item) => item.name === preset) ?? null
  const [units, setUnits] = useState(chosen?.units ?? 'Points')
  const [width, setWidth] = useState(chosen?.width ?? 612)
  const [height, setHeight] = useState(chosen?.height ?? 792)
  const [artboards, setArtboards] = useState(chosen?.artboards ?? 1)
  const [busy, setBusy] = useState(false)
  const valid = width >= 1 && height >= 1 && width <= 16_383 && height <= 16_383 && Number.isInteger(artboards) && artboards >= 1 && artboards <= 1000
  const choose = (next: NewPreset | null) => {
    setPreset(next?.name ?? CUSTOM)
    if (!next) return
    setWidth(next.width)
    setHeight(next.height)
    setUnits(next.units)
    setArtboards(next.artboards ?? 1)
  }
  const presetsOf = (name: string) => categories.find((item) => item.name === name)?.presets ?? []
  const orientation = width > height ? 'landscape' : 'portrait'
  const unit = unitAbbreviation(units)
  const decimals = unitDecimals(units)
  const create = async () => {
    const file = documentFileName(name, t('dialog.untitled'))
    setBusy(true)
    const engine = await KitEngine.create(engineManifest(), file, width, height)
    try {
      await engine.execute('file.new', { ...(chosen ? { preset: chosen.name } : {}), name: file.replace(/\.kvector$/i, ''), width, height, units, orientation, artboards })
      const path = await host.exportFile(file, await engine.save())
      toast('success', t('dialog.created', { path }))
    } catch (error) {
      toast('error', t('dialog.createFailed'), error instanceof Error ? error.message : String(error))
      throw error
    } finally {
      engine.dispose()
      setBusy(false)
    }
  }
  const place = useDialogPlace()
  return (
    <PreviewDialog
      position={place.position}
      onMove={place.onMove}
      title={t('dialog.newDocument')}
      okLabel={t('dialog.create')}
      okDisabled={!valid || busy}
      onOk={async () => {
        await create()
        onClose()
      }}
      onCancel={onClose}
      ids={DIALOG_BUTTONS}
      testId={DIALOG_TEST_ID}
    >
      <div className="vector-dialog vector-dialog-fields" data-command="kit:file.newDocument">
        <TextRow label={t('dialog.name')} value={name} onChange={setName} testId="vector-dialog-name" />
        {categories.length ? (
          <ChoiceRow
            label={t('dialog.category')}
            value={category}
            choices={categories.map((item) => [item.name, phrase(item.name)] as const)}
            onChange={(next) => {
              setCategory(next)
              choose(presetsOf(next)[0] ?? null)
            }}
            testId="vector-dialog-category"
          />
        ) : null}
        <ChoiceRow
          label={t('dialog.preset')}
          value={preset}
          choices={[...presetsOf(category).map((item) => [item.name, item.size ? `${item.name} (${item.size})` : item.name] as const), [CUSTOM, t('dialog.preset.custom')] as const]}
          onChange={(next) => choose(presetsOf(category).find((item) => item.name === next) ?? null)}
          testId="vector-dialog-preset"
        />
        <NumberRow label={t('dialog.width')} value={fromPoints(width, units)} min={0} max={1_000_000} decimals={decimals} unit={unit} onChange={(next) => { setPreset(CUSTOM); setWidth(toPoints(next ?? 0, units)) }} testId="vector-dialog-width" />
        <NumberRow label={t('dialog.height')} value={fromPoints(height, units)} min={0} max={1_000_000} decimals={decimals} unit={unit} onChange={(next) => { setPreset(CUSTOM); setHeight(toPoints(next ?? 0, units)) }} testId="vector-dialog-height" />
        <ChoiceRow label={t('dialog.units')} value={units} choices={UNITS.map((item) => [item, t(`unit.${item}`)] as const)} onChange={setUnits} testId="vector-dialog-units" />
        <ChoiceRow
          label={t('dialog.orientation')}
          value={orientation}
          choices={[['portrait', t('dialog.portrait')], ['landscape', t('dialog.landscape')]] as const}
          onChange={(next) => {
            if (next === orientation) return
            setPreset(CUSTOM)
            setWidth(height)
            setHeight(width)
          }}
          testId="vector-dialog-orientation"
        />
        <NumberRow label={t('dialog.artboards')} value={artboards} min={1} max={1000} decimals={0} onChange={(next) => setArtboards(Math.round(next ?? 1))} testId="vector-dialog-artboards" />
        {valid ? null : <div className="vector-dialog-warning">{t('dialog.badSize')}</div>}
      </div>
    </PreviewDialog>
  )
}
