import { useState, type ReactNode } from 'react'
import { Checkbox, Choice, GradientSlider, parseHex, rgbToHex, useKitEditor } from '@kition/sdk/editor'
import { useColorPicker } from '../color/ColorPickerHost'

/**
 * The dialogs' controls, on kit-editor's: a slider row (label, track, value to type), a drop-down,
 * a check box, a colour, a number and a text field. Each reports every change at once; the preview
 * waits for the values to settle (`useCommandPreview`).
 */

export function SliderRow({ label, value, min, max, softMax, decimals = 0, fallback, onChange, testId }: { label: string; value: number; min: number; max: number; softMax?: number; decimals?: number; fallback?: number; onChange: (value: number) => void; testId?: string }) {
  return <GradientSlider label={label} value={value} min={min} max={max} softMax={softMax} decimals={decimals} default={fallback} onChange={(next) => onChange(next)} testId={testId} />
}

export function ChoiceRow<V extends string>({ label, value, choices, onChange, disabled, testId }: { label: string; value: V; choices: readonly (readonly [V, string])[]; onChange: (value: V) => void; disabled?: boolean; testId?: string }) {
  return (
    <div className="vector-dialog-row">
      <Choice label={label} value={value} choices={choices} onChange={onChange} disabled={disabled} testId={testId} />
    </div>
  )
}

export function CheckRow({ label, checked, onChange, disabled, testId }: { label: string; checked: boolean; onChange: (checked: boolean) => void; disabled?: boolean; testId?: string }) {
  return (
    <div className="vector-dialog-row">
      <Checkbox label={label} checked={checked} onChange={onChange} disabled={disabled} testId={testId} />
    </div>
  )
}

/** A labelled row around any field. */
export function FieldRow({ label, children, htmlFor }: { label: string; children: ReactNode; htmlFor?: string }) {
  return (
    <div className="vector-dialog-row">
      <label className="vector-dialog-label" htmlFor={htmlFor}>
        {label}
      </label>
      {children}
    </div>
  )
}

/** A colour: its swatch opens the Color Picker (over the dialog), and its hex code can be typed. */
export function ColorRow({ label, value, onChange, testId }: { label: string; value: string; onChange: (value: string) => void; testId?: string }) {
  const { t } = useKitEditor()
  const pick = useColorPicker()
  const [typing, setTyping] = useState<string | null>(null)
  return (
    <FieldRow label={label}>
      <span className="vector-dialog-color">
        <button
          type="button"
          className="vector-dialog-swatch"
          style={{ background: value }}
          aria-label={t('dialog.pickColor')}
          onClick={() => {
            const hex = parseHex(value) ? rgbToHex(parseHex(value) ?? [0, 0, 0]) : '#000000'
            void pick?.({ title: label, value: { kind: 'color', hex } }).then((picked) => {
              if (picked?.kind === 'color') onChange(picked.hex)
            })
          }}
          data-testid={testId ? `${testId}-swatch` : undefined}
        />
        <input
          className="kui-field tabular-nums"
          type="text"
          value={typing ?? value}
          onBlur={() => setTyping(null)}
          onChange={(event) => {
            setTyping(event.target.value)
            const next = parseHex(event.target.value)
            if (next) onChange(rgbToHex(next))
          }}
          data-testid={testId}
        />
      </span>
    </FieldRow>
  )
}

/** A number to type, or empty (`null`, the engine's default). */
export function NumberRow({ label, value, min, max, decimals, unit, placeholder, onChange, testId }: { label: string; value: number | null; min: number; max: number; decimals: number; unit?: string; placeholder?: string; onChange: (value: number | null) => void; testId?: string }) {
  const id = testId ? `${testId}-input` : undefined
  // The text being typed, kept while the field has focus ("2." is a number on its way).
  const [typing, setTyping] = useState<string | null>(null)
  return (
    <FieldRow label={label} htmlFor={id}>
      <span className="vector-dialog-number">
        <input
          id={id}
          className="kui-field tabular-nums"
          type="number"
          value={typing ?? (value ?? '')}
          min={min}
          max={max}
          step={10 ** -decimals}
          placeholder={placeholder}
          onBlur={() => setTyping(null)}
          onChange={(event) => {
            const text = event.target.value
            setTyping(text)
            if (text.trim() === '') return onChange(null)
            const next = Number(text)
            // Kept as typed (a first digit may be under the minimum); the engine clamps what it runs.
            if (Number.isFinite(next)) onChange(next)
          }}
          data-testid={testId}
        />
        {unit ? <span className="vector-dialog-unit">{unit}</span> : null}
      </span>
    </FieldRow>
  )
}

export function TextRow({ label, value, placeholder, onChange, testId }: { label: string; value: string; placeholder?: string; onChange: (value: string) => void; testId?: string }) {
  const id = testId ? `${testId}-input` : undefined
  return (
    <FieldRow label={label} htmlFor={id}>
      <input id={id} className="kui-field" type="text" value={value} placeholder={placeholder} onChange={(event) => onChange(event.target.value)} data-testid={testId} />
    </FieldRow>
  )
}
