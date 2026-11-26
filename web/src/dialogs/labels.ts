// Imports carry `.ts` so `node --test` loads this module as it is.
import { humanize } from './params.ts'

/** The editor's strings and upstream's phrases (`useKitEditor`'s `t` and `phrase`). */
export type Words = { t: (key: string, values?: Record<string, string | number>) => string; phrase: (english: string) => string }

/** The editor's own string `key`, or null when its catalogs lack it (the editor's `t` answers the key itself then). */
function own({ t }: Words, key: string) {
  const text = t(key)
  return text === key ? null : text
}

/**
 * A parameter's label: the editor's own for the command or effect `scope` (`param.<scope>.<key>`,
 * then `param.<family>.<key>` for its family, `warp` for every warp), then for the key
 * (`param.<key>` in `strings.ts`), else its name in Title Case through upstream's phrases
 * ("Radius" → "半径"). Never the raw key.
 */
export function paramLabel(words: Words, key: string, scope?: string) {
  const family = scope?.split('.')[0]
  return (scope ? own(words, `param.${scope}.${key}`) : null) ?? (family ? own(words, `param.${family}.${key}`) : null) ?? own(words, `param.${key}`) ?? words.phrase(humanize(key))
}

/** A choice's label: the editor's own (`option.<value>`), else the value in Title Case through upstream's phrases. */
export function optionLabel(words: Words, value: string) {
  return own(words, `option.${value}`) ?? words.phrase(humanize(value))
}

/** A command's title for its dialog: upstream's phrase of it, without the trailing "…". */
export function dialogTitle(words: Words, title: string) {
  return words.phrase(title).replace(/\s*(…|\.\.\.)\s*$/, '')
}
