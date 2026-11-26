/**
 * The font menu's families beyond the engine's own (plan 2026-11-25, stage 5): the families the
 * editor serves next to itself (`public/fonts/manifest.txt`, which `scripts/build-engine.sh` writes
 * from the kition-fonts checkout, in its manifest format), loaded when a person picks one or a
 * drawing uses it. Pure, so `node --test` checks it without a browser.
 */

/** One served font file: its family and style, the file (relative to the editor's folder), and the scripts it is for. */
export type ServedFile = { family: string; style: string; file: string; scripts: readonly string[] }

/** The served families, each with its files in manifest order. */
export type FontCatalog = ReadonlyMap<string, readonly ServedFile[]>

/** A relative path inside the editor's folder: no scheme, no leading `/`, no `..`, no query. */
const inside = (path: string) => path.length > 0 && !path.startsWith('/') && !/[\\?#:]/.test(path) && path.split('/').every((part) => part !== '' && part !== '.' && part !== '..')

/**
 * The catalog in a manifest's text: one file per line, `family | style | file | scripts | …`
 * (kition-fonts' format); comments (`#`), blank lines and lines without a usable file are skipped.
 */
export function parseManifest(text: string): FontCatalog {
  const out = new Map<string, ServedFile[]>()
  for (const raw of text.split('\n')) {
    const line = raw.trim()
    if (!line || line.startsWith('#')) continue
    const [family, style, file, scripts = ''] = line.split(' | ').map((field) => field.trim())
    if (!family || !style || !file || !inside(file)) continue
    const entry = { family, style, file, scripts: scripts.split(',').map((script) => script.trim()).filter(Boolean) }
    out.set(family, [...(out.get(family) ?? []), entry])
  }
  return out
}

/** How a family shows in a font list: loaded (the engine has it), to load on choosing, loading, or missing (a document names it, nothing has it: drawn with a substitute). */
export type FontState = 'loaded' | 'served' | 'loading' | 'missing'

/**
 * The font list: every family the engine has or the editor serves, by name (case-insensitive), with
 * the `current` family kept even when nothing has it; each with its state.
 */
export function fontList(engine: readonly string[], catalog: FontCatalog, loading: ReadonlySet<string>, current: string): { family: string; state: FontState }[] {
  const names = new Set([...engine, ...catalog.keys()])
  const sorted = [...names].sort((a, b) => a.localeCompare(b, 'en', { sensitivity: 'base' }))
  const stateOf = (family: string): FontState => (loading.has(family) ? 'loading' : engine.includes(family) ? 'loaded' : catalog.has(family) ? 'served' : 'missing')
  const list = sorted.map((family) => ({ family, state: stateOf(family) }))
  return current && !names.has(current) ? [{ family: current, state: 'missing' }, ...list] : list
}

/** The served families a document's type names that the engine does not have yet (to load when it opens). */
export function familiesToLoad(named: Iterable<string>, engine: readonly string[], catalog: FontCatalog): string[] {
  const out: string[] = []
  for (const family of named) if (family && catalog.has(family) && !engine.includes(family) && !out.includes(family)) out.push(family)
  return out
}

/** The recently used families after `family` was used: it first, then the others, `max` at most. */
export function withRecent(recent: readonly string[], family: string, max = 5): string[] {
  return [family, ...recent.filter((name) => name.toLowerCase() !== family.toLowerCase())].slice(0, max)
}

/** A stored list of recent families as the editor reads it, or undefined when it is not one. */
export function recentOf(raw: unknown): string[] | undefined {
  return Array.isArray(raw) && raw.every((name) => typeof name === 'string') ? (raw as string[]).slice(0, 5) : undefined
}
