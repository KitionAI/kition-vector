/**
 * The engine's errors as a person reads them (plan 2026-11-25, stage 1): the engine answers in
 * English with command ids (``invalid parameters for `text.areaOptions`: no text objects
 * selected``); the editor shows a sentence of its own catalog instead (`error.<kind>`, en-US and
 * zh-CN in `strings.ts`), and the engine's own text after a general sentence when it does not know
 * the error, for a report. Pure, so `node --test` checks it.
 */

/** The kinds of error the editor words itself, each with what in the engine's text gives it away (first match wins). */
const KINDS: ReadonlyArray<readonly [string, RegExp]> = [
  ['cancelled', /^cancel+ed\b|\bwas cancel+ed\b/i],
  ['saveAsExport', /saving as \.\w+ is an export/i],
  ['clipboardEmpty', /clipboard is empty|nothing (?:was )?copied|nothing to paste/i],
  ['noDocument', /no (?:active )?document|no document (?:is )?open/i],
  ['noFileSystem', /no folder|local folder|file system|native only|not available (?:on the web|in the browser)|needs the desktop/i],
  ['locked', /\blocked\b/i],
  ['noText', /no text objects? (?:is |are )?selected|select (?:a |the )?(?:text|type) objects?|select text|no text is being edited|is not type\b|select (?:area|point) type|no type selected/i],
  ['noOpacityMask', /opacity mask/i],
  ['noKeyObject', /no key object/i],
  ['wrongKind', /select (?:a|an) (?:clip group|region paint group|image trace|image|shaper group|graph|repeat|blend|envelope|mesh|symbol|wrap object|slice)|select an image|not a (?:clip group|blend|envelope|mesh)/i],
  ['emptyResult', /produced an empty result|empty result/i],
  ['nothingToDo', /nothing to (?:\w+)|nothing is hidden|no previous transform|no guides|no (?:datasets|data sets|variables?)\b|no saved|(?:document|object|selection) has no \w+|nothing (?:was )?changed/i],
  ['openPaths', /already closed|needs open paths|join needs open/i],
  ['needsTwo', /select at least two|two or more|at least two objects/i],
  ['needsSelection', /\bselect (?:a|an|the|one|exactly|objects?|some|paths?|rows?|closed|open|stroked|\d)\b|\bnothing (?:is )?selected|\bno [\w\s-]+ (?:is |are )?selected\b|selection is empty|without a selection|needs? (?:a |an )?selection/i],
  ['tooLarge', /too large|too big|limited to|exceeds|out of memory|larger than/i],
  ['unreadable', /cannot read|could not read|not a valid|unsupported (?:file )?format|unrecognized|corrupt|is not valid base64/i],
  ['badValue', /invalid parameters|must be|missing|unknown|out of range|not a number|\bgive\b|expected/i],
  ['notNow', /is not available right now|not available now|is disabled/i],
  ['denied', /permission|not allowed|denied|read-only|view only/i],
]

export type ExplainedError = {
  /** The catalog key of the sentence to show (`error.<kind>`, `error.generic` when unknown). */
  key: string
  /** The engine's own text; null when there is none. */
  detail: string | null
  /** Whether the editor knows the error (its sentence says what went wrong). */
  known: boolean
}

/** The text of `error` (an Error, a string, or a JSON-RPC style `{message}`), without the `Error: ` the worker may put before it. */
export function errorText(error: unknown): string {
  const text = error instanceof Error ? error.message : typeof error === 'string' ? error : error && typeof error === 'object' && typeof (error as { message?: unknown }).message === 'string' ? (error as { message: string }).message : String(error)
  return text.replace(/^Error:\s*/, '')
}

/** What to tell the person about `error`. */
export function explainError(error: unknown): ExplainedError {
  const text = errorText(error).trim()
  const kind = KINDS.find(([, pattern]) => pattern.test(text))
  return { key: kind ? `error.${kind[0]}` : 'error.generic', detail: text || null, known: kind != null }
}

/** The kinds `explainError` words (each needs `error.<kind>` in both catalogs). */
export const ERROR_KINDS: readonly string[] = [...KINDS.map(([kind]) => kind), 'generic']

/** The sentence to show for `error` in the editor's words `t`: its own for a known error, the general one and the engine's text for another. */
export function errorMessage(t: (key: string) => string, error: unknown): string {
  const explained = explainError(error)
  return explained.known || !explained.detail ? t(explained.key) : `${t(explained.key)}\n${explained.detail}`
}
