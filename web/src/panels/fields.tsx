import { useEffect, useRef, useState, type KeyboardEvent as ReactKeyboardEvent, type MouseEvent as ReactMouseEvent, type PointerEvent as ReactPointerEvent, type ReactNode } from 'react'
import { Grid3x3Icon as Grid3x3 } from '@kition/tessera/icons/react'
import { useKitEditor, useReadOnly } from '@kition/sdk/editor'
import { normalHex } from '../color/colorMath'
import type { PaintView } from '../color/paint'
import { evaluate, formatNumber } from './arithmetic'

/** The editor's workspace, where the keyboard goes back to once a field is done. */
export const toWorkspace = (from: Element) => (from.closest('.kui-app') as HTMLElement | null)?.focus({ preventScroll: true })

/**
 * A panel's number field (plan 2026-11-25, stage 3): it reads arithmetic (`50+25`) and lengths with
 * a unit (`10 mm`), sets on Enter, on Tab (the keyboard moves on to the next field, as it should)
 * and when it loses focus; Escape puts the value back. Up and Down step it (Shift ten times). Its
 * label, when the field has `onScrub`, drags the value left and right, live on the canvas and one
 * history step on release. `value` null shows the field empty (a mixed selection).
 *
 * `onSet` answers false when the value is refused (a width of 0): the field shows its value again.
 */
export function PanelNumber({
  label,
  title,
  value,
  unit,
  decimals = 2,
  min = -Infinity,
  max = Infinity,
  step = 1,
  pointsPerUnit = 1,
  lengths = false,
  placeholder,
  onSet,
  onClear,
  onScrub,
  disabled,
  wide,
  testId,
}: {
  label: ReactNode
  title?: string
  value: number | null
  unit?: string
  decimals?: number
  min?: number
  max?: number
  step?: number
  /** Points in one of the field's units, for the lengths typed with a unit. */
  pointsPerUnit?: number
  /** Whether the field is a length (units may be typed). */
  lengths?: boolean
  placeholder?: string
  onSet: (value: number) => boolean | void
  /** Called when the field is emptied (a field whose empty value means "automatic"). */
  onClear?: () => void
  onScrub?: (value: number, final: boolean) => void
  disabled?: boolean
  wide?: boolean
  testId?: string
}) {
  const readOnly = useReadOnly()
  const off = disabled || readOnly
  const [typing, setTyping] = useState<string | null>(null)
  const [scrubbing, setScrubbing] = useState<number | null>(null)
  const typingNow = useRef(typing)
  typingNow.current = typing
  // The value the last Up or Down asked for, until the drawing answers with it: a second press
  // before then steps on from it, not from the value shown (re-walk 2026-11-26: two quick presses
  // stepped once and left two history steps).
  const stepped = useRef<number | null>(null)
  useEffect(() => {
    stepped.current = null
  }, [value])
  const clampTo = (next: number) => Math.min(max, Math.max(min, next))
  const shown = scrubbing ?? value
  const text = typing ?? (shown === null ? '' : formatNumber(shown, decimals))

  const set = (next: number) => {
    const clamped = clampTo(next)
    if (value !== null && Math.abs(clamped - value) < 10 ** -(decimals + 2)) return
    onSet(clamped)
  }
  /** Sets what was typed; false when it is not a number. */
  const commit = () => {
    const typed = typingNow.current
    if (typed === null) return true
    setTyping(null)
    typingNow.current = null
    if (typed.trim() === '') {
      onClear?.()
      return true
    }
    const next = evaluate(typed, pointsPerUnit, lengths)
    if (next === null) return false
    set(next)
    return true
  }
  const keyDown = (event: ReactKeyboardEvent<HTMLInputElement>) => {
    if (event.key === 'Enter') {
      commit()
      toWorkspace(event.currentTarget)
    } else if (event.key === 'Escape') {
      event.preventDefault()
      event.stopPropagation()
      setTyping(null)
      typingNow.current = null
      toWorkspace(event.currentTarget)
    } else if ((event.key === 'ArrowUp' || event.key === 'ArrowDown') && value !== null) {
      event.preventDefault()
      setTyping(null)
      typingNow.current = null
      const base = stepped.current ?? value
      const next = clampTo(base + (event.key === 'ArrowUp' ? 1 : -1) * step * (event.shiftKey ? 10 : 1))
      if (Math.abs(next - base) < 10 ** -(decimals + 2)) return
      stepped.current = next
      onSet(next)
    }
  }

  // The label's scrub: two screen pixels a step, Shift ten steps; one history step on release.
  const scrub = onScrub && !off && value !== null
  const pressLabel = (event: ReactPointerEvent<HTMLSpanElement>) => {
    if (!scrub || event.button !== 0 || value === null) return
    event.preventDefault()
    const handle = event.currentTarget
    handle.setPointerCapture?.(event.pointerId)
    const startX = event.clientX
    const start = value
    let last = start
    const move = (moved: PointerEvent) => {
      const steps = Math.round((moved.clientX - startX) / 2)
      const next = clampTo(start + steps * step * (moved.shiftKey ? 10 : 1))
      if (next === last) return
      last = next
      setScrubbing(next)
      onScrub(next, false)
    }
    const up = () => {
      handle.removeEventListener('pointermove', move)
      handle.removeEventListener('pointerup', up)
      handle.removeEventListener('pointercancel', up)
      setScrubbing(null)
      if (last !== start) onScrub(last, true)
    }
    handle.addEventListener('pointermove', move)
    handle.addEventListener('pointerup', up)
    handle.addEventListener('pointercancel', up)
  }
  return (
    <label className="vector-field" data-wide={wide || undefined} title={title}>
      <span className="vector-field-label" data-scrub={scrub || undefined} onPointerDown={pressLabel} data-testid={testId ? `${testId}-label` : undefined}>
        {label}
      </span>
      <input
        className="kui-field tabular-nums"
        type="text"
        inputMode="decimal"
        spellCheck={false}
        value={text}
        placeholder={placeholder}
        disabled={off}
        aria-label={title}
        onChange={(event) => setTyping(event.target.value)}
        onKeyDown={keyDown}
        onBlur={() => {
          commit()
        }}
        data-testid={testId}
      />
      {unit ? <span className="vector-field-unit">{unit}</span> : null}
    </label>
  )
}

/**
 * A paint as a chip: its colour, a gradient's ramp, None's slash, a pattern's mark, a mixed
 * selection's "?". A click opens it (`onOpen`), or with `onClick` does that and a double-click opens it.
 */
export function PaintChip({ view, label, title = label, onOpen, onClick, pressed, disabled, stroke, testId }: { view: PaintView; label: string; title?: string; onOpen?: () => void; onClick?: (event: ReactMouseEvent<HTMLButtonElement>) => void; pressed?: boolean; disabled?: boolean; stroke?: boolean; testId?: string }) {
  const background = view.kind === 'color' ? view.hex : view.kind === 'gradient' ? view.css : undefined
  return (
    <button
      type="button"
      className="vector-chip vector-chip-paint"
      data-kind={view.kind}
      data-none={view.kind === 'none' || undefined}
      data-stroke={stroke || undefined}
      style={{ background }}
      title={title}
      aria-label={label}
      aria-pressed={pressed}
      disabled={disabled}
      onClick={(event) => (onClick ? onClick(event) : onOpen?.())}
      onDoubleClick={onClick ? onOpen : undefined}
      data-testid={testId}
    >
      {view.kind === 'mixed' ? '?' : view.kind === 'pattern' ? <Grid3x3 size={12} aria-hidden="true" /> : null}
    </button>
  )
}

/** The words a colour field shows instead of a hex code: None, a gradient, a pattern, mixed. */
export function paintWords(view: PaintView, t: (key: string) => string) {
  switch (view.kind) {
    case 'none':
      return t('color.none')
    case 'mixed':
      return t('paint.mixed')
    case 'gradient':
      return t(view.radial ? 'paint.radial' : 'paint.linear')
    case 'pattern':
      return view.name ? `${t('paint.pattern')} · ${view.name}` : t('paint.pattern')
    default:
      return ''
  }
}

/**
 * A fill or stroke: its chip (a click opens the Color Picker) and its hex code, which can be typed
 * (set as soon as it is six digits, or on Enter). None, a gradient, a pattern or a mixed selection
 * show as words in the field, never as a colour they are not.
 */
export function PaintField({ label, view, onPick, onHex, disabled, testId }: { label: string; view: PaintView; onPick: () => void; onHex: (hex: string) => void; disabled?: boolean; testId: string }) {
  const { t } = useKitEditor()
  const readOnly = useReadOnly()
  const [typing, setTyping] = useState<string | null>(null)
  const hex = view.kind === 'color' ? view.hex : ''
  const set = (text: string, loose: boolean) => {
    const full = /^#?[0-9a-f]{6}$/i.test(text.trim())
    const next = full || loose ? normalHex(text) : null
    if (!next) return false
    setTyping(null)
    if (next !== hex) onHex(next)
    return true
  }
  return (
    <div className="vector-field vector-paint-field" data-wide>
      <span className="vector-field-label">{label}</span>
      <PaintChip view={view} label={label} onOpen={onPick} disabled={disabled || readOnly} stroke={testId.includes('stroke')} testId={`${testId}-chip`} />
      <input
        className="kui-field tabular-nums"
        type="text"
        spellCheck={false}
        maxLength={7}
        value={typing ?? hex}
        placeholder={paintWords(view, t)}
        disabled={disabled || readOnly}
        aria-label={label}
        onChange={(event) => {
          setTyping(event.target.value)
          set(event.target.value, false)
        }}
        onKeyDown={(event) => {
          if (event.key === 'Enter') {
            if (typing !== null) set(typing, true)
            setTyping(null)
            toWorkspace(event.currentTarget)
          } else if (event.key === 'Escape') {
            setTyping(null)
            toWorkspace(event.currentTarget)
          }
        }}
        onBlur={() => {
          if (typing !== null) set(typing, true)
          setTyping(null)
        }}
        data-kind={view.kind}
        data-testid={testId}
      />
    </div>
  )
}
