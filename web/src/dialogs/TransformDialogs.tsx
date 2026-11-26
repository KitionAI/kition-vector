import { useState } from 'react'
import { useKitEditor } from '@kition/sdk/editor'
import { DialogFrame } from './DialogFrame'
import { CheckRow, ChoiceRow, NumberRow, SliderRow } from './controls'
import { fromPoints, toPoints, unitAbbreviation, unitDecimals } from './units'

/**
 * Object › Transform's dialogs (plan 2026-11-25, stage 1): Move, Rotate, Scale, Reflect, Shear and
 * Transform Each, each with its values, the transform options, Preview, and Copy (the command on a
 * copy, as the reference's Copy button). Lengths show in the document's units; the engine takes
 * points. Each opens with the values it was last OK'd with, over what it is given (a tool's
 * request, the preferences' option boxes).
 */

/** What a transform dialog starts from: the preferences' options and anything a tool hands it. */
export type TransformStart = Record<string, unknown>

type Close = () => void
type Props = { title: string; units: string; start?: TransformStart; onClose: Close }

/** Each transform dialog's last OK'd values, for the session. */
const last = new Map<string, Record<string, number | boolean | string>>()

const num = (value: unknown, fallback: number) => (typeof value === 'number' && Number.isFinite(value) ? value : fallback)
const bool = (value: unknown, fallback = false) => (typeof value === 'boolean' ? value : fallback)
const round = (value: number, decimals = 2) => Math.round(value * 10 ** decimals) / 10 ** decimals

/** The dialog's starting values: its last OK'd over its own, then what it is given over both. */
function useStart<T extends Record<string, number | boolean | string>>(id: string, own: T, start: TransformStart | undefined): [T, (key: keyof T, value: T[keyof T]) => void] {
  const [values, setValues] = useState<T>(() => {
    const merged: Record<string, unknown> = { ...own, ...last.get(id) }
    for (const [key, value] of Object.entries(start ?? {})) if (key in own) merged[key] = value
    return merged as T
  })
  return [values, (key, value) => setValues((current) => ({ ...current, [key]: value }))]
}

/** Keys a tool hands a dialog that the dialog passes on as they are (the point it transforms about). */
const passed = (start: TransformStart | undefined) => (start && Array.isArray(start.origin) ? { origin: start.origin } : {})

/** The option boxes every transform dialog shares. */
function Options({ values, set, strokes = false }: { values: Record<string, unknown>; set: (key: string, value: boolean) => void; strokes?: boolean }) {
  const { t } = useKitEditor()
  return (
    <div className="vector-dialog-options">
      {strokes ? (
        <>
          <CheckRow label={t('dialog.scaleCorners')} checked={values.corners === true} onChange={(next) => set('corners', next)} testId="vector-dialog-corners" />
          <CheckRow label={t('dialog.scaleStrokes')} checked={values.strokes === true} onChange={(next) => set('strokes', next)} testId="vector-dialog-strokes" />
        </>
      ) : null}
      <CheckRow label={t('dialog.transformPatterns')} checked={values.patterns === true} onChange={(next) => set('patterns', next)} testId="vector-dialog-patterns" />
    </div>
  )
}

/** Object › Transform › Move…: horizontal and vertical, or distance and angle (one pair follows the other). */
export function MoveDialog({ title, units, start, onClose }: Props) {
  const { t } = useKitEditor()
  // Kept in points; shown in the document's units.
  const [values, set] = useStart('move', { dx: 0, dy: 0, patterns: bool(start?.patterns) }, start)
  const unit = unitAbbreviation(units)
  const decimals = unitDecimals(units)
  const distance = Math.hypot(values.dx, values.dy)
  // Counter-clockwise from the right, as the reference measures it: the document's y grows downwards.
  const angle = distance === 0 ? 0 : round((Math.atan2(-values.dy, values.dx) * 180) / Math.PI, 3)
  const polar = (length: number, degrees: number) => {
    const radians = (degrees * Math.PI) / 180
    set('dx', round(length * Math.cos(radians), 6))
    set('dy', round(-length * Math.sin(radians), 6))
  }
  return (
    <DialogFrame
      command="object.move"
      title={title}
      params={{ ...passed(start), dx: values.dx, dy: values.dy, patterns: values.patterns }}
      previewing
      copy
      onClose={(committed) => {
        if (committed) last.set('move', values)
        onClose()
      }}
    >
      <div className="vector-dialog-fields">
        <NumberRow label={t('dialog.horizontal')} value={fromPoints(values.dx, units)} min={-1_000_000} max={1_000_000} decimals={decimals} unit={unit} onChange={(next) => set('dx', toPoints(next ?? 0, units))} testId="vector-dialog-dx" />
        <NumberRow label={t('dialog.vertical')} value={fromPoints(values.dy, units)} min={-1_000_000} max={1_000_000} decimals={decimals} unit={unit} onChange={(next) => set('dy', toPoints(next ?? 0, units))} testId="vector-dialog-dy" />
        <NumberRow label={t('dialog.distance')} value={fromPoints(distance, units)} min={0} max={1_000_000} decimals={decimals} unit={unit} onChange={(next) => polar(toPoints(next ?? 0, units), angle)} testId="vector-dialog-distance" />
        <NumberRow label={t('dialog.angle')} value={angle} min={-360} max={360} decimals={2} unit="°" onChange={(next) => polar(distance, next ?? 0)} testId="vector-dialog-angle" />
      </div>
      <Options values={values} set={(key, value) => set(key as 'patterns', value)} />
    </DialogFrame>
  )
}

/** Object › Transform › Rotate…: an angle, counter-clockwise. */
export function RotateDialog({ title, start, onClose }: Props) {
  const { t } = useKitEditor()
  const [values, set] = useStart('rotate', { angle: 0, patterns: bool(start?.patterns) }, start)
  return (
    <DialogFrame
      command="object.rotate"
      title={title}
      params={{ ...passed(start), angle: values.angle, patterns: values.patterns }}
      previewing
      copy
      onClose={(committed) => {
        if (committed) last.set('rotate', values)
        onClose()
      }}
    >
      <div className="vector-dialog-fields">
        <NumberRow label={t('dialog.angle')} value={values.angle} min={-360} max={360} decimals={2} unit="°" onChange={(next) => set('angle', next ?? 0)} testId="vector-dialog-angle" />
      </div>
      <Options values={values} set={(key, value) => set(key as 'patterns', value)} />
    </DialogFrame>
  )
}

/** Object › Transform › Scale…: uniform, or horizontal and vertical apart; strokes, corners and patterns by the preferences. */
export function ScaleDialog({ title, start, onClose }: Props) {
  const { t } = useKitEditor()
  const [values, set] = useStart('scale', { uniform: true, scale: 100, sx: 100, sy: 100, strokes: bool(start?.strokes), corners: bool(start?.corners), patterns: bool(start?.patterns) }, start)
  const sx = values.uniform ? values.scale : values.sx
  const sy = values.uniform ? values.scale : values.sy
  return (
    <DialogFrame
      command="object.scale"
      title={title}
      params={{ ...passed(start), sx, sy, strokes: values.strokes, corners: values.corners, patterns: values.patterns }}
      previewing
      copy
      okDisabled={!(sx > 0 && sy > 0)}
      onClose={(committed) => {
        if (committed) last.set('scale', values)
        onClose()
      }}
    >
      <div className="vector-dialog-fields">
        <ChoiceRow label={t('dialog.scaleKind')} value={values.uniform ? 'uniform' : 'nonUniform'} choices={[['uniform', t('dialog.uniform')], ['nonUniform', t('dialog.nonUniform')]] as const} onChange={(next) => set('uniform', next === 'uniform')} testId="vector-dialog-uniform" />
        {values.uniform ? (
          <NumberRow label={t('dialog.scale')} value={values.scale} min={-100_000} max={100_000} decimals={2} unit="%" onChange={(next) => set('scale', next ?? 100)} testId="vector-dialog-scale" />
        ) : (
          <>
            <NumberRow label={t('dialog.horizontal')} value={values.sx} min={-100_000} max={100_000} decimals={2} unit="%" onChange={(next) => set('sx', next ?? 100)} testId="vector-dialog-sx" />
            <NumberRow label={t('dialog.vertical')} value={values.sy} min={-100_000} max={100_000} decimals={2} unit="%" onChange={(next) => set('sy', next ?? 100)} testId="vector-dialog-sy" />
          </>
        )}
      </div>
      <Options values={values} set={(key, value) => set(key as 'patterns', value)} strokes />
    </DialogFrame>
  )
}

/** Object › Transform › Reflect…: about the horizontal or vertical axis, or one at an angle. */
export function ReflectDialog({ title, start, onClose }: Props) {
  const { t } = useKitEditor()
  const [values, set] = useStart('reflect', { axis: 'vertical', angle: 90, patterns: bool(start?.patterns) }, start)
  const axis = values.axis === 'angle' ? values.angle : values.axis
  return (
    <DialogFrame
      command="object.reflect"
      title={title}
      params={{ ...passed(start), axis, patterns: values.patterns }}
      previewing
      copy
      onClose={(committed) => {
        if (committed) last.set('reflect', values)
        onClose()
      }}
    >
      <div className="vector-dialog-fields">
        <ChoiceRow label={t('dialog.axis')} value={values.axis as 'horizontal' | 'vertical' | 'angle'} choices={[['horizontal', t('dialog.horizontal')], ['vertical', t('dialog.vertical')], ['angle', t('dialog.angle')]] as const} onChange={(next) => set('axis', next)} testId="vector-dialog-axis" />
        {values.axis === 'angle' ? <NumberRow label={t('dialog.angle')} value={values.angle} min={-360} max={360} decimals={2} unit="°" onChange={(next) => set('angle', next ?? 0)} testId="vector-dialog-angle" /> : null}
      </div>
      <Options values={values} set={(key, value) => set(key as 'patterns', value)} />
    </DialogFrame>
  )
}

/** Object › Transform › Shear…: an angle along the horizontal or vertical axis. */
export function ShearDialog({ title, start, onClose }: Props) {
  const { t } = useKitEditor()
  const [values, set] = useStart('shear', { angle: 0, axis: 'horizontal', patterns: bool(start?.patterns) }, start)
  return (
    <DialogFrame
      command="object.shear"
      title={title}
      params={{ ...passed(start), angle: values.angle, axis: values.axis, patterns: values.patterns }}
      previewing
      copy
      onClose={(committed) => {
        if (committed) last.set('shear', values)
        onClose()
      }}
    >
      <div className="vector-dialog-fields">
        <NumberRow label={t('dialog.shearAngle')} value={values.angle} min={-359} max={359} decimals={2} unit="°" onChange={(next) => set('angle', next ?? 0)} testId="vector-dialog-angle" />
        <ChoiceRow label={t('dialog.axis')} value={values.axis as 'horizontal' | 'vertical'} choices={[['horizontal', t('dialog.horizontal')], ['vertical', t('dialog.vertical')]] as const} onChange={(next) => set('axis', next)} testId="vector-dialog-axis" />
      </div>
      <Options values={values} set={(key, value) => set(key as 'patterns', value)} />
    </DialogFrame>
  )
}

/** The bounding box's nine points, as Transform Each and the Transform panel choose one (0 top left, 4 centre, 8 bottom right). */
export function ReferencePoint({ value, onChange }: { value: number; onChange: (value: number) => void }) {
  const { t } = useKitEditor()
  return (
    <div className="vector-dialog-row">
      <span className="vector-dialog-label">{t('dialog.reference')}</span>
      <div className="vector-reference" role="radiogroup" aria-label={t('dialog.reference')} data-testid="vector-dialog-reference">
        {Array.from({ length: 9 }, (_, index) => (
          <button key={index} type="button" role="radio" aria-checked={value === index} aria-label={t(`dialog.reference.${index}`)} className="vector-reference-point" onClick={() => onChange(index)} data-testid={`vector-dialog-reference-${index}`} />
        ))}
      </div>
    </div>
  )
}

/** Object › Transform › Transform Each…: scale, move, rotate and reflect each selected object about its own reference point, optionally by random amounts. */
export function TransformEachDialog({ title, units, start, onClose }: Props) {
  const { t } = useKitEditor()
  const [values, set] = useStart(
    'transformEach',
    { scaleH: 100, scaleV: 100, moveH: 0, moveV: 0, rotate: 0, reflectX: false, reflectY: false, random: false, reference: 4, strokes: bool(start?.strokes), corners: bool(start?.corners), patterns: bool(start?.patterns) },
    start,
  )
  const unit = unitAbbreviation(units)
  const decimals = unitDecimals(units)
  const { reference, ...rest } = values
  return (
    <DialogFrame
      command="object.transformEach"
      title={title}
      params={{ ...rest, reference: num(reference, 4) }}
      previewing
      copy
      onClose={(committed) => {
        if (committed) last.set('transformEach', values)
        onClose()
      }}
    >
      <div className="vector-dialog-fields">
        <SliderRow label={t('dialog.scaleHorizontal')} value={values.scaleH} min={0} max={200} decimals={0} fallback={100} onChange={(next) => set('scaleH', next)} testId="vector-dialog-scaleH" />
        <SliderRow label={t('dialog.scaleVertical')} value={values.scaleV} min={0} max={200} decimals={0} fallback={100} onChange={(next) => set('scaleV', next)} testId="vector-dialog-scaleV" />
        <NumberRow label={t('dialog.moveHorizontal')} value={fromPoints(values.moveH, units)} min={-1_000_000} max={1_000_000} decimals={decimals} unit={unit} onChange={(next) => set('moveH', toPoints(next ?? 0, units))} testId="vector-dialog-moveH" />
        <NumberRow label={t('dialog.moveVertical')} value={fromPoints(values.moveV, units)} min={-1_000_000} max={1_000_000} decimals={decimals} unit={unit} onChange={(next) => set('moveV', toPoints(next ?? 0, units))} testId="vector-dialog-moveV" />
        <NumberRow label={t('dialog.angle')} value={values.rotate} min={-360} max={360} decimals={2} unit="°" onChange={(next) => set('rotate', next ?? 0)} testId="vector-dialog-rotate" />
        <ReferencePoint value={num(reference, 4)} onChange={(next) => set('reference', next)} />
      </div>
      <div className="vector-dialog-options">
        <CheckRow label={t('dialog.reflectX')} checked={values.reflectX} onChange={(next) => set('reflectX', next)} testId="vector-dialog-reflectX" />
        <CheckRow label={t('dialog.reflectY')} checked={values.reflectY} onChange={(next) => set('reflectY', next)} testId="vector-dialog-reflectY" />
        <CheckRow label={t('dialog.random')} checked={values.random} onChange={(next) => set('random', next)} testId="vector-dialog-random" />
      </div>
      <Options values={values} set={(key, value) => set(key as 'patterns', value)} strokes />
    </DialogFrame>
  )
}
