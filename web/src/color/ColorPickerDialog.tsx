import { useEffect, useRef, useState, type KeyboardEvent as ReactKeyboardEvent, type PointerEvent as ReactPointerEvent } from 'react'
import { Button, PreviewDialog, useKitEditor } from '@kition/sdk/editor'
import { DIALOG_BUTTONS, useDialogPlace } from '../dialogs/DialogFrame'
import { fieldsOf, fieldsOfCmyk, fieldsOfHsb, hexToRgb, normalHex, squarePoint, squareValue, stripHue, withField, type ColorField, type ColorFields } from './colorMath'
import { pushEscape } from './escapeStack'
import type { Picked } from './paint'

/** The picker's test id; its parts add `-square`, `-hue`, `-new`, `-current`, `-hex`, `-none` and one per field (`-h`, `-s`, `-b`, `-r`, `-g`, `-bl`, `-c`, `-m`, `-y`, `-k`). */
export const COLOR_DIALOG = 'vector-color-dialog'

type FieldRow = { field: ColorField; label: string; unit: string; min: number; max: number; read: (fields: ColorFields) => number }

const round = (value: number, places = 0) => Math.round(value * 10 ** places) / 10 ** places
const HSB: readonly FieldRow[] = [
  { field: 'h', label: 'H', unit: '°', min: 0, max: 360, read: (f) => Math.round(f.hsb.h) % 360 },
  { field: 's', label: 'S', unit: '%', min: 0, max: 100, read: (f) => Math.round(f.hsb.s) },
  { field: 'b', label: 'B', unit: '%', min: 0, max: 100, read: (f) => Math.round(f.hsb.b) },
]
const RGB: readonly FieldRow[] = [
  { field: 'r', label: 'R', unit: '', min: 0, max: 255, read: (f) => f.rgb[0] },
  { field: 'g', label: 'G', unit: '', min: 0, max: 255, read: (f) => f.rgb[1] },
  { field: 'bl', label: 'B', unit: '', min: 0, max: 255, read: (f) => f.rgb[2] },
]
const CMYK: readonly FieldRow[] = [
  { field: 'c', label: 'C', unit: '%', min: 0, max: 100, read: (f) => round(f.cmyk.c, f.typedCmyk ? 1 : 0) },
  { field: 'm', label: 'M', unit: '%', min: 0, max: 100, read: (f) => round(f.cmyk.m, f.typedCmyk ? 1 : 0) },
  { field: 'y', label: 'Y', unit: '%', min: 0, max: 100, read: (f) => round(f.cmyk.y, f.typedCmyk ? 1 : 0) },
  { field: 'k', label: 'K', unit: '%', min: 0, max: 100, read: (f) => round(f.cmyk.k, f.typedCmyk ? 1 : 0) },
]

const startFields = (value: Picked): ColorFields => {
  if (value.kind === 'none') return fieldsOf([255, 255, 255])
  if (value.cmyk) return fieldsOfCmyk(value.cmyk)
  return fieldsOf(hexToRgb(value.hex) ?? [0, 0, 0])
}
const pickedOf = (fields: ColorFields): Picked => ({ kind: 'color', hex: fields.hex, ...(fields.typedCmyk ? { cmyk: fields.cmyk } : {}) })
const sameAs = (a: Picked, b: Picked) => (a.kind === 'none' ? b.kind === 'none' : b.kind === 'color' && a.hex === b.hex && JSON.stringify(a.cmyk ?? null) === JSON.stringify(b.cmyk ?? null))

/**
 * The Color Picker (plan 2026-11-25, stage 3): a saturation and brightness square with a hue strip
 * beside it, the new colour over the current one (a click on the current one goes back to it), and
 * HSB, RGB, CMYK and hex fields that follow each other; None when the colour may be none (a fill
 * or a stroke). `onLive` gets each colour as the person moves through them, and the starting one
 * back on Cancel; OK answers the chosen colour, Cancel (or Escape) null. It sits over the dialog
 * that opened it and takes Escape before that dialog can.
 */
export function ColorPickerDialog({ title, value, allowNone, onLive, onDone }: { title: string; value: Picked; allowNone: boolean; onLive?: (picked: Picked) => void; onDone: (picked: Picked | null) => void }) {
  const { t } = useKitEditor()
  const current = value.kind === 'color' ? { ...value, hex: normalHex(value.hex) ?? '#000000' } : value
  const [fields, setFieldsState] = useState<ColorFields>(() => startFields(current))
  const [none, setNoneState] = useState(current.kind === 'none')
  const fieldsNow = useRef(fields)
  fieldsNow.current = fields
  const noneNow = useRef(none)
  noneNow.current = none
  const [typing, setTyping] = useState<{ field: ColorField | 'hex'; text: string } | null>(null)
  const onLiveNow = useRef(onLive)
  onLiveNow.current = onLive
  const place = useDialogPlace()
  const chosen = (): Picked => (noneNow.current ? { kind: 'none' } : pickedOf(fieldsNow.current))
  const tell = (before: Picked) => {
    const after = chosen()
    if (!sameAs(before, after)) onLiveNow.current?.(after)
  }
  const setFields = (next: ColorFields) => {
    const before = chosen()
    fieldsNow.current = next
    noneNow.current = false
    setFieldsState(next)
    setNoneState(false)
    tell(before)
  }
  const setNone = () => {
    const before = chosen()
    noneNow.current = true
    setNoneState(true)
    tell(before)
  }
  const doneNow = useRef(onDone)
  doneNow.current = onDone
  const cancel = () => {
    if (!sameAs(chosen(), current)) onLiveNow.current?.(current)
    doneNow.current(null)
  }
  const cancelNow = useRef(cancel)
  cancelNow.current = cancel
  useEffect(() => pushEscape(() => cancelNow.current()), [])

  const dragTo = (event: ReactPointerEvent<HTMLElement>, part: 'square' | 'hue') => {
    const rect = event.currentTarget.getBoundingClientRect()
    const x = (event.clientX - rect.left) / Math.max(1, rect.width)
    const y = (event.clientY - rect.top) / Math.max(1, rect.height)
    const hsb = fieldsNow.current.hsb
    setTyping(null)
    setFields(part === 'square' ? fieldsOfHsb({ h: hsb.h, ...squareValue(x, y) }) : fieldsOfHsb({ ...hsb, h: stripHue(y) }))
  }
  const pressOn = (part: 'square' | 'hue') => ({
    onPointerDown: (event: ReactPointerEvent<HTMLElement>) => {
      if (event.button !== 0) return
      event.currentTarget.setPointerCapture?.(event.pointerId)
      dragTo(event, part)
    },
    onPointerMove: (event: ReactPointerEvent<HTMLElement>) => {
      if (event.buttons & 1 && event.currentTarget.hasPointerCapture?.(event.pointerId)) dragTo(event, part)
    },
  })
  const nudge = (part: 'square' | 'hue') => (event: ReactKeyboardEvent<HTMLElement>) => {
    const step = event.shiftKey ? 10 : 1
    const hsb = fieldsNow.current.hsb
    const moves: Record<string, () => ColorFields> =
      part === 'square'
        ? { ArrowLeft: () => fieldsOfHsb({ ...hsb, s: hsb.s - step }), ArrowRight: () => fieldsOfHsb({ ...hsb, s: hsb.s + step }), ArrowUp: () => fieldsOfHsb({ ...hsb, b: hsb.b + step }), ArrowDown: () => fieldsOfHsb({ ...hsb, b: hsb.b - step }) }
        : { ArrowUp: () => fieldsOfHsb({ ...hsb, h: (hsb.h + step) % 360 }), ArrowDown: () => fieldsOfHsb({ ...hsb, h: (hsb.h - step + 360) % 360 }) }
    const move = moves[event.key]
    if (!move) return
    event.preventDefault()
    event.stopPropagation()
    setTyping(null)
    setFields(move())
  }

  const point = squarePoint(fields.hsb)
  const pure = fieldsOfHsb({ h: fields.hsb.h, s: 100, b: 100 }).hex
  const hexShown = typing?.field === 'hex' ? typing.text : none ? '' : fields.hex.slice(1)
  const group = (rows: readonly FieldRow[], name: string) => (
    <div className="vector-color-group" data-group={name}>
      {rows.map(({ field, label, unit, min, max, read }) => {
        const id = `${COLOR_DIALOG}-${field}`
        return (
          <label key={field} className="vector-color-field" htmlFor={id} data-field={field}>
            <span className="vector-color-field-label">{label}</span>
            <input
              id={id}
              className="kui-field tabular-nums"
              type="number"
              min={min}
              max={max}
              value={typing?.field === field ? typing.text : none ? '' : read(fields)}
              onChange={(event) => {
                const text = event.target.value
                setTyping({ field, text })
                const number = Number(text)
                if (text.trim() !== '' && Number.isFinite(number)) setFields(withField(fieldsNow.current, field, Math.min(max, Math.max(min, number))))
              }}
              onBlur={() => setTyping(null)}
              data-testid={id}
            />
            <span className="vector-color-field-unit">{unit}</span>
          </label>
        )
      })}
    </div>
  )
  const newSwatch = none ? undefined : fields.hex
  return (
    <PreviewDialog title={title} position={place.position} onMove={place.onMove} onOk={() => doneNow.current(chosen())} onCancel={cancel} ids={DIALOG_BUTTONS} testId={COLOR_DIALOG}>
      <div className="vector-color-dialog" data-color={none ? 'none' : fields.hex}>
        <div
          className="vector-color-square"
          role="slider"
          tabIndex={0}
          aria-label={t('color.square')}
          aria-valuetext={`${Math.round(fields.hsb.s)}%, ${Math.round(fields.hsb.b)}%`}
          style={{ background: `linear-gradient(to top, #000, transparent), linear-gradient(to right, #fff, ${pure})` }}
          onKeyDown={nudge('square')}
          {...pressOn('square')}
          data-testid={`${COLOR_DIALOG}-square`}
        >
          {none ? null : <span className="vector-color-square-mark" style={{ left: `${point.x * 100}%`, top: `${point.y * 100}%` }} />}
        </div>
        <div
          className="vector-color-hue"
          role="slider"
          tabIndex={0}
          aria-label={t('color.hue')}
          aria-valuemin={0}
          aria-valuemax={360}
          aria-valuenow={Math.round(fields.hsb.h)}
          onKeyDown={nudge('hue')}
          {...pressOn('hue')}
          data-testid={`${COLOR_DIALOG}-hue`}
        >
          <span className="vector-color-hue-mark" style={{ top: `${(1 - fields.hsb.h / 360) * 100}%` }} />
        </div>
        <div className="vector-color-side">
          <div className="vector-color-compare">
            <span className="vector-color-compare-label">{t('color.new')}</span>
            <span className="vector-color-compare-new vector-chip-paint" style={{ background: newSwatch }} data-none={none || undefined} data-color={none ? 'none' : fields.hex} data-testid={`${COLOR_DIALOG}-new`} />
            <button
              type="button"
              className="vector-color-compare-current vector-chip-paint"
              style={{ background: current.kind === 'color' ? current.hex : undefined }}
              data-none={current.kind === 'none' || undefined}
              title={t('color.backToCurrent')}
              aria-label={t('color.backToCurrent')}
              onClick={() => {
                setTyping(null)
                if (current.kind === 'none') setNone()
                else setFields(startFields(current))
              }}
              data-color={current.kind === 'color' ? current.hex : 'none'}
              data-testid={`${COLOR_DIALOG}-current`}
            />
            <span className="vector-color-compare-label">{t('color.current')}</span>
          </div>
          {allowNone ? (
            <Button variant="secondary" onClick={setNone} disabled={none} data-testid={`${COLOR_DIALOG}-none`}>
              {t('color.none')}
            </Button>
          ) : null}
          <div className="vector-color-fields">
            {group(HSB, 'hsb')}
            {group(RGB, 'rgb')}
            {group(CMYK, 'cmyk')}
            <label className="vector-color-field vector-color-field-hex" htmlFor={`${COLOR_DIALOG}-hex`}>
              <span className="vector-color-field-label">#</span>
              <input
                id={`${COLOR_DIALOG}-hex`}
                className="kui-field tabular-nums"
                type="text"
                spellCheck={false}
                maxLength={7}
                value={hexShown}
                placeholder={none ? t('color.none') : undefined}
                onChange={(event) => {
                  const text = event.target.value
                  setTyping({ field: 'hex', text })
                  const rgb = /^#?[0-9a-f]{6}$/i.test(text.trim()) ? hexToRgb(text) : null
                  if (rgb) setFields(fieldsOf(rgb, fieldsNow.current.hsb.h))
                }}
                onBlur={() => setTyping(null)}
                data-testid={`${COLOR_DIALOG}-hex`}
              />
            </label>
          </div>
        </div>
      </div>
    </PreviewDialog>
  )
}
