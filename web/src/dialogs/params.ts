/**
 * A command's parameters as the generic dialog's fields (plan 2026-11-25, stage 1): a slider for a
 * number with a range, a number field for one without, a drop-down for a choice, a check box, a
 * colour, or a text field. The keys a person never sets in a dialog (which objects, an appearance
 * item, a seed, a file, raw JSON, the alternatives an agent may give instead) are left out, so the
 * engine's defaults apply to them. Effects get theirs from the effect catalogue (`effect.list`):
 * its defaults give each value's kind, its notes the ranges and choices. Pure, so `node --test`
 * checks it.
 */

type SchemaNode = {
  type?: string
  description?: string
  enum?: unknown[]
  default?: unknown
  minimum?: number
  maximum?: number
  oneOf?: SchemaNode[]
}

/** A command as the engine lists it (`KitCommand`'s fields this module reads). */
export type ParamsCommand = { id: string; params?: string | null; schema?: unknown }

/** Keys a dialog never asks: which objects, where, the undo coalescing, a seed, files and data. */
export const SHARED_KEYS: ReadonlySet<string> = new Set(['id', 'ids', 'item', 'coalesce', 'at', 'origin', 'matrix', 'seed', 'path', 'data', 'dataBase64', 'document', 'file', 'dir', 'user', 'on'])

/** What a schema note says of a key a person does not set: another name for a listed key, or one of several ways to give the same value. */
const ALTERNATIVE = /read by the command beside its listed parameters|one of several alternative parameters/

export type ParamField =
  | { key: string; kind: 'slider'; min: number; max: number; softMax: number; decimals: number; initial: number }
  | { key: string; kind: 'number'; min: number; max: number; decimals: number; initial: number | null }
  | { key: string; kind: 'choice'; options: readonly string[]; initial: string }
  | { key: string; kind: 'check'; initial: boolean }
  | { key: string; kind: 'color'; initial: string }
  | { key: string; kind: 'text'; initial: string }

const NUMBER = String.raw`-?\d+(?:\.\d+)?`

/** The range a note gives ("-100 to 100; %", "0–1000", "0..100 (5)"), if any. */
export function rangeFromText(text: string | undefined): { min: number; max: number } | null {
  if (!text) return null
  const match = new RegExp(String.raw`(${NUMBER})\s*(?:\.\.|to|–|…)\s*(${NUMBER})`).exec(text)
  if (!match) return null
  const [min, max] = [Number(match[1]), Number(match[2])]
  return min < max ? { min, max } : null
}

/** The default a schema note gives as a bare number between semicolons ("points; 12", "1 to 1000; 255"), if any. */
export function defaultFromDescription(description: string | undefined): number | null {
  if (!description) return null
  for (const part of description.split(';')) {
    const text = part.trim()
    if (new RegExp(`^${NUMBER}$`).test(text)) return Number(text)
  }
  return null
}

/** Decimals a value keeps: integers none; a narrow range two, a wide one one. */
function decimalsFor(integer: boolean, min: number, max: number) {
  if (integer) return 0
  return max - min <= 20 ? 2 : 1
}

const clamp = (value: number, min: number, max: number) => Math.min(max, Math.max(min, value))

/** The slider's own range: a very wide one (Revolve's offset to 100,000 pt) slides to 250, and the typed value still reaches the maximum. */
export function softMaxFor(min: number, max: number) {
  return max - min > 500 && min >= 0 ? Math.min(max, 250) : max
}

/** The blend modes, as the engine names them, in the Transparency panel's order. */
export const BLEND_MODES: readonly string[] = ['normal', 'darken', 'multiply', 'colorBurn', 'lighten', 'screen', 'colorDodge', 'overlay', 'softLight', 'hardLight', 'difference', 'exclusion', 'hue', 'saturation', 'color', 'luminosity']

const isBlendKey = (key: string) => key === 'mode' || key === 'blend' || key === 'blending' || key === 'blendMode'

/** A choice's options: every string in `values` but templates ("cmyk:<profile>"), which name no option. */
const realOptions = (values: readonly unknown[]) => values.filter((value): value is string => typeof value === 'string' && value !== '' && !/[<#]/.test(value))

/**
 * A number field: a slider for a range with a known starting value; else a field to type, empty
 * (the engine's own default) until a value is typed.
 */
function numberField(key: string, integer: boolean, range: { min: number; max: number } | null, fallback: number | null): ParamField {
  if (range && fallback !== null) {
    const initial = clamp(fallback, range.min, range.max)
    return { key, kind: 'slider', min: range.min, max: range.max, softMax: Math.max(initial, softMaxFor(range.min, range.max)), decimals: decimalsFor(integer, range.min, range.max), initial }
  }
  return { key, kind: 'number', min: range?.min ?? -1_000_000, max: range?.max ?? 1_000_000, decimals: integer ? 0 : 2, initial: fallback }
}

/**
 * Starting values the schema does not give (plan 2026-11-25, stage 1): Offset Path's distance has
 * no default (the engine refuses it bare), and Edit Colors' scope boxes are on when omitted.
 */
const COLOR_SCOPE = { fill: true, stroke: true, includeImages: true, includePatterns: true }
const STARTING: Readonly<Record<string, Readonly<Record<string, number | boolean>>>> = {
  'object.path.offsetPath': { offset: 10 },
  'edit.colors.saturate': COLOR_SCOPE,
  'edit.colors.adjustBalance': COLOR_SCOPE,
}

/** A boolean's default as the engine's notes give it (`horizontal?: bool (true)`, `clip?: true`), if any. */
export function booleanFromNotes(notes: string | null | undefined, key: string): boolean | null {
  const note = notes ? noteOf(notes, key) : ''
  const match = /^\s*(?:bool\s*)?\(?\s*(true|false)\b/.exec(note)
  return match ? match[1] === 'true' : null
}

/** One key's field of `command`, or null when the dialog does not ask for it. */
export function fieldFor(command: ParamsCommand, key: string, node: SchemaNode): ParamField | null {
  if (SHARED_KEYS.has(key) || ALTERNATIVE.test(node.description ?? '')) return null
  if (node.enum?.length) {
    const options = realOptions(node.enum)
    if (options.length < 2) return null
    const initial = typeof node.default === 'string' && options.includes(node.default) ? node.default : options[0]
    return { key, kind: 'choice', options, initial }
  }
  const starting = STARTING[command.id]?.[key]
  if (node.type === 'boolean') {
    const initial = typeof starting === 'boolean' ? starting : typeof node.default === 'boolean' ? node.default : booleanFromNotes(command.params, key)
    return { key, kind: 'check', initial: initial === true }
  }
  if (node.type === 'number' || node.type === 'integer') {
    const range = rangeFromText(node.description) ?? (node.minimum !== undefined && node.maximum !== undefined && node.minimum < node.maximum ? { min: node.minimum, max: node.maximum } : null)
    const fallback = typeof starting === 'number' ? starting : typeof node.default === 'number' ? node.default : defaultFromDescription(node.description)
    return numberField(key, node.type === 'integer', range, fallback)
  }
  if (node.type === 'string') {
    if (/colou?r$/i.test(key)) return { key, kind: 'color', initial: typeof node.default === 'string' && /^#[0-9a-f]{6}$/i.test(node.default) ? node.default : '#000000' }
    if (isBlendKey(key)) return { key, kind: 'choice', options: BLEND_MODES, initial: typeof node.default === 'string' && BLEND_MODES.includes(node.default) ? node.default : 'normal' }
    // A string whose notes list its words (`background?: transparent|white|black`) is a choice of them.
    const listed = command.params ? choicesFromNote(noteOf(command.params, key)) : []
    if (listed.length >= 2) return { key, kind: 'choice', options: listed, initial: typeof node.default === 'string' && listed.includes(node.default) ? node.default : listed[0] }
    return { key, kind: 'text', initial: typeof node.default === 'string' ? node.default : '' }
  }
  // Arrays, objects, JSON and either-or values keep the engine's defaults.
  return null
}

/**
 * `fields` in the order the engine's notes name their keys (Offset Path's distance, then its joins
 * and miter limit, as its dialog reads; the schema lists keys alphabetically); keys the notes do not
 * name keep their place after them.
 */
export function inNotesOrder(fields: ParamField[], notes: string | null | undefined): ParamField[] {
  if (!notes) return fields
  const at = (key: string) => {
    const match = new RegExp(String.raw`(?:^|[{,|(]\s*)${key}\??(?=[:,}|\s])`).exec(notes)
    return match ? match.index : Number.MAX_SAFE_INTEGER
  }
  return fields
    .map((field, index) => ({ field, index, at: at(field.key) }))
    .sort((a, b) => a.at - b.at || a.index - b.index)
    .map((item) => item.field)
}

/** The fields the generic dialog shows for `command`, in the order its notes name them. */
export function paramFields(command: ParamsCommand): ParamField[] {
  const schema = (command.schema ?? {}) as Record<string, SchemaNode>
  const fields = Object.entries(schema).flatMap(([key, node]) => {
    const field = fieldFor(command, key, node)
    return field ? [field] : []
  })
  return inNotesOrder(fields, command.params)
}

/** One effect of the catalogue (`effect.list`'s `catalog`). */
export type EffectEntry = { id: string; label: string; menu: readonly string[]; params: string; defaults: Record<string, unknown> | null; raster?: boolean }

/** The part of an effect's notes that speaks of `key`: from `key:` to the next key at the same level. */
function noteOf(notes: string, key: string) {
  const start = new RegExp(String.raw`(?:^|[{,]\s*)${key}\??:`).exec(notes)
  if (!start) return ''
  const rest = notes.slice(start.index + start[0].length)
  // The note ends at the next `, word:` outside brackets.
  let depth = 0
  for (let i = 0; i < rest.length; i += 1) {
    const char = rest[i]
    if (char === '(' || char === '[' || char === '{') depth += 1
    else if (char === ')' || char === ']' || char === '}') {
      if (depth === 0) return rest.slice(0, i)
      depth -= 1
    } else if (char === ',' && depth === 0 && /^\s*[A-Za-z]+\??:/.test(rest.slice(i + 1))) return rest.slice(0, i)
  }
  return rest
}

/** The quoted choices a note lists (`"smooth"|"corner"`, `left|right`), if any. */
export function choicesFromNote(note: string): string[] {
  const quoted = [...note.matchAll(/"([A-Za-z][\w-]*)"/g)].map((match) => match[1])
  if (note.includes('|') && quoted.length >= 2) return [...new Set(quoted)]
  const bare = /(?:^|\s)([A-Za-z][\w-]*(?:\|[A-Za-z][\w-]*)+)/.exec(note)
  return bare ? bare[1].split('|') : []
}

/** The fields of an effect's dialog: one per default value a person sets, in the order its notes name them. */
export function effectFields(effect: EffectEntry): ParamField[] {
  const defaults = effect.defaults ?? {}
  const fields = Object.entries(defaults).flatMap(([key, value]): ParamField[] => {
    if (SHARED_KEYS.has(key)) return []
    const note = noteOf(effect.params, key)
    if (typeof value === 'boolean') return [{ key, kind: 'check', initial: value }]
    if (typeof value === 'number') {
      // An opacity is a percentage even where the note gives no range.
      const range = rangeFromText(note) ?? (key === 'opacity' ? { min: 0, max: 100 } : null)
      return [numberField(key, Number.isInteger(value) && /integer|int\b/.test(note), range, value)]
    }
    if (typeof value === 'string') {
      if (/^#[0-9a-f]{6}$/i.test(value)) return [{ key, kind: 'color', initial: value }]
      if (isBlendKey(key)) return [{ key, kind: 'choice', options: BLEND_MODES, initial: BLEND_MODES.includes(value) ? value : 'normal' }]
      const options = choicesFromNote(note)
      if (options.length >= 2) return [{ key, kind: 'choice', options: options.includes(value) ? options : [value, ...options], initial: value }]
      return [{ key, kind: 'text', initial: value }]
    }
    // Lists and objects (Free Distort's corners, a curve's points) need a control of their own.
    return []
  })
  return inNotesOrder(fields, effect.params)
}

/** Whether an effect's dialog can ask everything it has: false when a value needs a control of its own. */
export function effectAsksAll(effect: EffectEntry) {
  return Object.entries(effect.defaults ?? {}).every(([key, value]) => SHARED_KEYS.has(key) || typeof value !== 'object' || value === null)
}

export type ParamValues = Record<string, string | number | boolean | null>

/** The fields' starting values: the remembered ones where they still fit, else each field's own. */
export function initialValues(fields: readonly ParamField[], remembered: Readonly<Record<string, unknown>> = {}): ParamValues {
  return Object.fromEntries(
    fields.map((field) => {
      const kept = remembered[field.key]
      return [field.key, fits(field, kept) ? (kept as string | number | boolean | null) : field.initial]
    }),
  )
}

/** Whether `value` is one `field` can show. */
export function fits(field: ParamField, value: unknown): boolean {
  switch (field.kind) {
    case 'slider':
    case 'number':
      return (value === null && field.kind === 'number') || (typeof value === 'number' && Number.isFinite(value) && value >= field.min && value <= field.max)
    case 'choice':
      return typeof value === 'string' && field.options.includes(value)
    case 'check':
      return typeof value === 'boolean'
    case 'color':
      return typeof value === 'string' && /^#[0-9a-f]{6}$/i.test(value)
    case 'text':
      return typeof value === 'string'
  }
}

/** The command's parameters from the fields' values: an empty text or number leaves its key out (the engine's default). */
export function paramsOf(fields: readonly ParamField[], values: ParamValues): Record<string, unknown> {
  const out: Record<string, unknown> = {}
  for (const field of fields) {
    const value = values[field.key]
    if (value === null || value === undefined) continue
    if (field.kind === 'text' && value === '') continue
    out[field.key] = value
  }
  return out
}

/**
 * Words for a key or a choice in English Title Case ("cellSize" → "Cell Size", "gridByRow" →
 * "Grid By Row"), the form upstream's translation catalogs list them in.
 */
export function humanize(word: string) {
  const spaced = word
    .replace(/([a-z0-9])([A-Z])/g, '$1 $2')
    .replace(/[-_]+/g, ' ')
    .trim()
  return spaced.replace(/(^|\s)([a-z])/g, (_, space: string, letter: string) => `${space}${letter.toUpperCase()}`)
}
