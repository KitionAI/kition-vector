import { useState } from 'react'
import { useKitEditor } from '@kition/sdk/editor'
import { DialogFrame } from './DialogFrame'
import { CheckRow, ChoiceRow, NumberRow } from './controls'
import { fromPoints, toPoints, unitAbbreviation, unitDecimals, UNITS } from './units'

/** What `file.setup {}` answers: the document's setup (lengths in points). */
export type DocumentSetup = {
  units?: string
  bleed?: number[]
  outlineImages?: boolean
  highlightSubstitutedFonts?: boolean
  highlightSubstitutedGlyphs?: boolean
  gridSize?: string
  gridColorsName?: string
  simulatePaper?: boolean
  typographersQuotes?: boolean
  exportText?: string
  backgroundContents?: string
}

const GRID_SIZES = ['small', 'medium', 'large'] as const
const GRID_COLORS = ['Light', 'Medium', 'Dark', 'Red', 'Orange', 'Green', 'Blue', 'Purple'] as const
const SIDES = ['top', 'bottom', 'left', 'right'] as const

/**
 * File › Document Setup… (plan 2026-11-25, stage 1): the document's units, bleed, the transparency
 * grid, the paper, and the type options, as the engine's `file.setup` sets them in one undo step.
 * It opens with the document's current setup; lengths show in the units chosen.
 */
export function DocumentSetupDialog({ setup, title, onClose }: { setup: DocumentSetup; title: string; onClose: () => void }) {
  const { t } = useKitEditor()
  const [units, setUnits] = useState(setup.units ?? 'Points')
  const [bleed, setBleed] = useState<number[]>(() => SIDES.map((_, index) => setup.bleed?.[index] ?? 0))
  const [gridSize, setGridSize] = useState(setup.gridSize ?? 'medium')
  const [gridColors, setGridColors] = useState(setup.gridColorsName ?? 'Light')
  const [checks, setChecks] = useState({
    outlineImages: setup.outlineImages === true,
    highlightSubstitutedFonts: setup.highlightSubstitutedFonts === true,
    highlightSubstitutedGlyphs: setup.highlightSubstitutedGlyphs === true,
    simulatePaper: setup.simulatePaper === true,
    typographersQuotes: setup.typographersQuotes !== false,
  })
  const [exportText, setExportText] = useState(setup.exportText ?? 'editable')
  const [background, setBackground] = useState(setup.backgroundContents ?? 'transparent')
  const unit = unitAbbreviation(units)
  const params = { units, bleed, gridSize, gridColors, exportText, backgroundContents: background, ...checks }
  return (
    <DialogFrame command="file.setup" title={title} params={params} previewing={false} onClose={() => onClose()}>
      <div className="vector-dialog-fields">
        <ChoiceRow label={t('dialog.units')} value={units} choices={UNITS.map((item) => [item, t(`unit.${item}`)] as const)} onChange={setUnits} testId="vector-dialog-units" />
        {SIDES.map((side, index) => (
          <NumberRow
            key={side}
            label={t(`dialog.bleed.${side}`)}
            value={fromPoints(bleed[index] ?? 0, units)}
            min={0}
            max={fromPoints(72, units)}
            decimals={unitDecimals(units)}
            unit={unit}
            onChange={(next) => setBleed((current) => current.map((value, at) => (at === index ? Math.min(72, Math.max(0, toPoints(next ?? 0, units))) : value)))}
            testId={`vector-dialog-bleed-${side}`}
          />
        ))}
        <ChoiceRow label={t('dialog.gridSize')} value={gridSize} choices={GRID_SIZES.map((item) => [item, t(`dialog.gridSize.${item}`)] as const)} onChange={setGridSize} testId="vector-dialog-gridSize" />
        <ChoiceRow label={t('dialog.gridColors')} value={gridColors} choices={GRID_COLORS.map((item) => [item, t(`dialog.gridColors.${item}`)] as const)} onChange={setGridColors} testId="vector-dialog-gridColors" />
        <ChoiceRow label={t('dialog.exportText')} value={exportText} choices={[['editable', t('dialog.exportText.editable')], ['appearance', t('dialog.exportText.appearance')]] as const} onChange={setExportText} testId="vector-dialog-exportText" />
        <ChoiceRow label={t('dialog.backgroundContents')} value={background} choices={[['transparent', t('dialog.background.transparent')], ['white', t('dialog.background.white')]] as const} onChange={setBackground} testId="vector-dialog-backgroundContents" />
      </div>
      <div className="vector-dialog-options">
        {(Object.keys(checks) as (keyof typeof checks)[]).map((key) => (
          <CheckRow key={key} label={t(`dialog.${key}`)} checked={checks[key]} onChange={(next) => setChecks((current) => ({ ...current, [key]: next }))} testId={`vector-dialog-${key}`} />
        ))}
      </div>
    </DialogFrame>
  )
}
