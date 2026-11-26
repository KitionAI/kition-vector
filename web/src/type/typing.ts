/**
 * Type in the editor (plan 2026-11-25, stage 5): what the engine's `inspect()` reports of the Type
 * tool's editing (`typing`), and the commands the Character and Paragraph panels and the control
 * bar send: the selected characters while some are selected in the type being edited, otherwise the
 * whole type object (edited, or every one selected); with no type at all, what new type is set in.
 * Pure, so `node --test` checks it.
 */
import type { EnginePaint } from '../color/paint.ts'

/** The character attributes the Character panel shows; null where the characters differ. */
export type CharacterStyle = {
  font: string | null
  style: string | null
  size: number | null
  /** Points, or 'auto' (120 % of the size). */
  leading: number | 'auto' | null
  tracking: number | null
  /** 1/1000 em, or 'auto' (the font's kerning). */
  kerning: number | 'auto' | null
  baselineShift: number | null
  hScale: number | null
  vScale: number | null
  rotation: number | null
  /** The paints as the engine labels them (`#rrggbb`, `None`, `<swatch> (#rrggbb)`, `… gradient`). */
  fill: string | null
  stroke: string | null
  strokeWidth: number | null
  underline: boolean | null
  strikethrough: boolean | null
  allCaps: boolean | null
  smallCaps: boolean | null
  position: 'normal' | 'superscript' | 'subscript' | null
}

/** The paragraph attributes the Paragraph panel shows; null where the paragraphs differ. */
export type ParagraphStyle = {
  justify: string | null
  leftIndent: number | null
  rightIndent: number | null
  firstLineIndent: number | null
  spaceBefore: number | null
  spaceAfter: number | null
  hyphenate: boolean | null
}

/** What the Type tool edits now and what the type panels act on. */
export type Typing = {
  /** The type object edited, or null. */
  editing: number | null
  /** The selected characters, bytes of its text (equal: a caret). */
  start: number
  end: number
  /** An input method composes text now. */
  composing: boolean
  /** The caret line (or the start of the clause an input method converts) in document pixels: [ax, ay, bx, by]. */
  caret: readonly [number, number, number, number] | null
  /** What new type is set in; null each for the engine's default. */
  newType: { font: string | null; style: string | null; size: number | null }
  /** The attributes of what the panels act on, or null when no type is edited or selected. */
  character: CharacterStyle | null
  paragraph: ParagraphStyle | null
}

export const NO_TYPING: Typing = { editing: null, start: 0, end: 0, composing: false, caret: null, newType: { font: null, style: null, size: null }, character: null, paragraph: null }

const num = (value: unknown) => (typeof value === 'number' && Number.isFinite(value) ? value : null)
const bool = (value: unknown) => (typeof value === 'boolean' ? value : null)
const str = (value: unknown) => (typeof value === 'string' ? value : null)
const numOrAuto = (value: unknown) => (value === 'auto' ? 'auto' : num(value))

function characterOf(raw: unknown): CharacterStyle | null {
  if (!raw || typeof raw !== 'object') return null
  const r = raw as Record<string, unknown>
  const position = r.position === 'normal' || r.position === 'superscript' || r.position === 'subscript' ? r.position : null
  return {
    font: str(r.font),
    style: str(r.style),
    size: num(r.size),
    leading: numOrAuto(r.leading),
    tracking: num(r.tracking),
    kerning: numOrAuto(r.kerning),
    baselineShift: num(r.baselineShift),
    hScale: num(r.hScale),
    vScale: num(r.vScale),
    rotation: num(r.rotation),
    fill: str(r.fill),
    stroke: str(r.stroke),
    strokeWidth: num(r.strokeWidth),
    underline: bool(r.underline),
    strikethrough: bool(r.strikethrough),
    allCaps: bool(r.allCaps),
    smallCaps: bool(r.smallCaps),
    position,
  }
}

function paragraphOf(raw: unknown): ParagraphStyle | null {
  if (!raw || typeof raw !== 'object') return null
  const r = raw as Record<string, unknown>
  return {
    justify: str(r.justify),
    leftIndent: num(r.leftIndent),
    rightIndent: num(r.rightIndent),
    firstLineIndent: num(r.firstLineIndent),
    spaceBefore: num(r.spaceBefore),
    spaceAfter: num(r.spaceAfter),
    hyphenate: bool(r.hyphenate),
  }
}

/** `typing` as the engine's `inspect()` reports it. */
export function typingOf(raw: unknown): Typing {
  if (!raw || typeof raw !== 'object') return NO_TYPING
  const r = raw as Record<string, unknown>
  const caret = Array.isArray(r.caret) && r.caret.length === 4 && r.caret.every((v) => typeof v === 'number' && Number.isFinite(v)) ? (r.caret as [number, number, number, number]) : null
  const fresh = (r.newType && typeof r.newType === 'object' ? r.newType : {}) as Record<string, unknown>
  return {
    editing: typeof r.editing === 'number' ? r.editing : null,
    start: num(r.start) ?? 0,
    end: num(r.end) ?? 0,
    composing: r.composing === true,
    caret,
    newType: { font: str(fresh.font), style: str(fresh.style), size: num(fresh.size) },
    character: characterOf(r.character),
    paragraph: paragraphOf(r.paragraph),
  }
}

/** Whether characters are selected in the type being edited (not just a caret). */
export const hasRange = (typing: Typing) => typing.editing !== null && typing.end > typing.start

/** The character attributes `text.setStyle` takes for every selected type object; `text.setFormat` takes the others. */
const SET_STYLE = new Set(['font', 'style', 'size', 'leading', 'tracking', 'fill'])

/** A command and its parameters. */
export type Step = { command: string; params: Record<string, unknown> }

/**
 * What sets character attributes `values` (keys as `text.setRangeStyle` names them): the selected
 * characters of the type being edited; with only a caret in it, all its text; with type objects
 * selected, every one of them; with none, what new type is set in (family, style and size only:
 * null, `request` the Type tool's request then).
 */
export function characterStep(typing: Typing, selectedText: boolean, values: Record<string, unknown>): Step | { request: Record<string, unknown> } | null {
  if (typing.editing !== null) {
    const range = hasRange(typing) ? { start: typing.start, end: typing.end } : {}
    return { command: 'text.setRangeStyle', params: { id: typing.editing, ...range, ...values } }
  }
  if (selectedText) {
    const keys = Object.keys(values)
    if (keys.every((key) => SET_STYLE.has(key))) return { command: 'text.setStyle', params: values }
    if (keys.every((key) => !SET_STYLE.has(key))) return { command: 'text.setFormat', params: values }
    return null
  }
  const fresh: Record<string, unknown> = {}
  for (const key of ['font', 'style', 'size'] as const) if (key in values) fresh[key] = values[key]
  return Object.keys(fresh).length ? { request: { newType: fresh } } : null
}

/**
 * What sets paragraph attributes `values`: the paragraphs the selection (or the caret) of the type
 * being edited touches, else every paragraph of the selected type objects. `justify` goes through
 * `text.setStyle`, the others through `text.setFormat`.
 */
export function paragraphStep(typing: Typing, values: Record<string, unknown>): Step {
  const command = 'justify' in values ? 'text.setStyle' : 'text.setFormat'
  if (typing.editing !== null) return { command, params: { id: typing.editing, start: typing.start, end: typing.end, ...values } }
  return { command, params: values }
}

/** A paint as the engine labels it, as the paint chips read paints (null: mixed). */
export function labelPaint(label: string | null): EnginePaint | null {
  if (label === null) return null
  if (label === 'None') return { type: 'none' }
  const hex = /#[0-9a-f]{6}/i.exec(label)?.[0]
  if (/gradient$/i.test(label)) return { type: 'gradient', gradient: { kind: /radial/i.test(label) ? 'radial' : 'linear', stops: [] } }
  if (label.startsWith('pattern ')) return { type: 'pattern', pattern: label.slice('pattern '.length) }
  return hex ? { type: 'solid', color: hex as never } : null
}

const encoder = new TextEncoder()
const decoder = new TextDecoder()

/** The characters of `text` between byte offsets `start` and `end` (UTF-8, as the engine counts them). */
export function byteSlice(text: string, start: number, end: number): string {
  const bytes = encoder.encode(text)
  return decoder.decode(bytes.slice(Math.max(0, Math.min(start, end)), Math.max(0, Math.max(start, end))))
}

/** The Type tool's requests the page sends as keys (the binding's `Type:` keys). */
export const typeRequest = (request: Record<string, unknown>) => `Type:${JSON.stringify(request)}`

/** The leading choices the Character panel offers (points), as the reference's list. */
export const SIZES = [6, 7, 8, 9, 10, 11, 12, 14, 18, 21, 24, 36, 48, 60, 72] as const

/** The Paragraph panel's seven alignments, in the reference's order, with `text.setStyle`'s `justify` names. */
export const ALIGNMENTS = ['left', 'center', 'right', 'justifyLeft', 'justifyCenter', 'justifyRight', 'justifyAll'] as const

/** Change Case's choices (`text.changeCase`'s `mode`). */
export const CASES = ['upper', 'lower', 'title', 'sentence'] as const
