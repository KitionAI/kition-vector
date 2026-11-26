import { useState } from 'react'
import { LinkIcon as Link, UnlinkIcon as Unlink } from '@kition/tessera/icons/react'
import { IconButton, PanelSection, useKitEditor, useReadOnly } from '@kition/sdk/editor'
import { boundsParams, referencePoint, type BoundsKey } from '../canvas/bounds'
import { fromPoints, pointsPer, toPoints, unitAbbreviation, unitDecimals } from '../dialogs/units'
import type { VectorInfo } from '../vector'
import { PanelNumber } from './fields'
import { useLive } from './useLive'

const FIELDS: readonly { key: BoundsKey; label: string }[] = [
  { key: 'x', label: 'X' },
  { key: 'y', label: 'Y' },
  { key: 'width', label: 'W' },
  { key: 'height', label: 'H' },
]

/** The bounding box's nine points to measure from (0 top left, 4 centre, 8 bottom right). */
function ReferenceGrid({ value, onChange }: { value: number; onChange: (value: number) => void }) {
  const { t } = useKitEditor()
  return (
    <div className="vector-reference" role="radiogroup" aria-label={t('dialog.reference')} title={t('dialog.reference')} data-testid="vector-reference">
      {Array.from({ length: 9 }, (_, index) => (
        <button key={index} type="button" role="radio" aria-checked={value === index} aria-label={t(`dialog.reference.${index}`)} className="vector-reference-point" onClick={() => onChange(index)} data-testid={`vector-reference-${index}`} />
      ))}
    </div>
  )
}

/**
 * Properties › Transform (plan 2026-11-25, stage 3): the selection's X, Y, W and H measured from
 * the reference point chosen on the grid, in the document's units; the link keeps W and H in
 * proportion; the rotation turns the selection about that point. Every field reads arithmetic, Tab
 * moves on to the next, and a label dragged sideways scrubs the value as one history step.
 */
export function TransformSection({ info }: { info: VectorInfo }) {
  const { run, phrase: p, t } = useKitEditor()
  const readOnly = useReadOnly()
  const live = useLive()
  const [reference, setReference] = useState(0)
  const [locked, setLocked] = useState(false)
  const bounds = info.selectionBounds
  const unit = info.units
  const at = bounds ? referencePoint(bounds, reference) : null
  const shown = (key: BoundsKey) => (!bounds || !at ? null : fromPoints(key === 'x' ? at.x : key === 'y' ? at.y : bounds[key], unit))
  const params = (key: BoundsKey, value: number) => {
    const base = boundsParams(key, toPoints(value, unit), reference)
    return base && locked && (key === 'width' || key === 'height') ? { ...base, proportional: true } : base
  }
  const rotateParams = (angle: number) => ({ angle, absolute: true, ...(at ? { origin: [at.x, at.y] } : {}) })
  return (
    <PanelSection title={p('Transform')} testId="vector-transform">
      {bounds ? (
        <div className="vector-transform">
          <ReferenceGrid value={reference} onChange={setReference} />
          <div className="vector-fields">
            {FIELDS.map(({ key, label }) => (
              <PanelNumber
                key={key}
                label={label}
                title={t(`transform.${key}`)}
                value={shown(key)}
                unit={unitAbbreviation(unit)}
                decimals={unitDecimals(unit)}
                min={key === 'width' || key === 'height' ? 0 : -Infinity}
                lengths
                pointsPerUnit={pointsPer(unit)}
                onSet={(value) => {
                  const next = params(key, value)
                  if (next) void run('object.setBounds', next)
                  return next !== null
                }}
                onScrub={(value, final) => {
                  const next = params(key, value)
                  if (!next) return
                  if (final) void live.commit('object.setBounds', next)
                  else live.set('object.setBounds', next)
                }}
                testId={`vector-bounds-${label}`}
              />
            ))}
            <PanelNumber
              label="∠"
              title={t('transform.rotation')}
              value={Math.round(info.rotation * 100) / 100}
              unit="°"
              decimals={2}
              onSet={(angle) => void run('object.rotate', rotateParams(angle))}
              onScrub={(angle, final) => (final ? void live.commit('object.rotate', rotateParams(angle)) : live.set('object.rotate', rotateParams(angle)))}
              testId="vector-rotation"
            />
            <IconButton icon={locked ? Link : Unlink} label={t('transform.proportions')} pressed={locked} disabled={readOnly} onClick={() => setLocked((on) => !on)} data-testid="vector-lock-proportions" />
          </div>
        </div>
      ) : (
        <p className="vector-empty kui-muted">{t('noSelection')}</p>
      )}
    </PanelSection>
  )
}
