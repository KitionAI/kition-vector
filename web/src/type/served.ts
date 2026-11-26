import { useEffect, useSyncExternalStore } from 'react'
import { parseManifest, type FontCatalog, type ServedFile } from './catalog'

/**
 * The served fonts at run time (plan 2026-11-25, stage 5, after Photo's): the catalog
 * (`fonts/manifest.txt`, fetched once per page), and loading a family: its files fetched once per
 * page, given to the engine document's `addFace` once (it lays the drawing's type out again in
 * them), and registered with the page too, so the font menu shows the family in its own face.
 */

/** An engine document as the editor holds it (`session.doc`): `refreshInfo` after its type was laid out again. */
export type FontEngine = { invoke<T = unknown>(name: string, ...args: unknown[]): Promise<T>; refreshInfo?: () => Promise<unknown> }

type Snapshot = { catalog: FontCatalog; loading: ReadonlySet<string>; added: number }

let snapshot: Snapshot = { catalog: new Map(), loading: new Set(), added: 0 }
let catalogLoad: Promise<FontCatalog> | null = null
const listeners = new Set<() => void>()
const files = new Map<string, Promise<Uint8Array | null>>()
const given = new WeakMap<FontEngine, Map<string, Promise<boolean>>>()
const pageFaces = new Set<string>()

const update = (next: Partial<Snapshot>) => {
  snapshot = { ...snapshot, ...next }
  for (const listener of listeners) listener()
}

/** The catalog, fetched once (an editor served without fonts has an empty one). */
export function loadCatalog(): Promise<FontCatalog> {
  catalogLoad ??= fetch(new URL('./fonts/manifest.txt', document.baseURI))
    .then((answer) => (answer.ok ? answer.text() : ''))
    .catch(() => '')
    .then((text) => {
      const catalog = parseManifest(text)
      update({ catalog })
      return catalog
    })
  return catalogLoad
}

/** The served catalog, the families loading now, and a count that moves when a face was added (the engine's family list changed). */
export function useServedFonts(): Snapshot {
  useEffect(() => void loadCatalog(), [])
  return useSyncExternalStore(
    (listener) => {
      listeners.add(listener)
      return () => listeners.delete(listener)
    },
    () => snapshot,
  )
}

const read = (file: string) => {
  let bytes = files.get(file)
  if (!bytes) {
    bytes = fetch(new URL(`./${file}`, document.baseURI))
      .then(async (answer) => (answer.ok ? new Uint8Array(await answer.arrayBuffer()) : null))
      .catch(() => null)
    files.set(file, bytes)
  }
  return bytes
}

/** A face's CSS descriptors from its manifest style (a variable file covers every weight). */
function descriptors(file: ServedFile): FontFaceDescriptors {
  const style = file.style.toLowerCase()
  const weight = /variable/i.test(file.file) ? '100 900' : style.includes('bold') ? '700' : '400'
  return { weight, style: style.includes('italic') ? 'italic' : 'normal' }
}

/** Registers a face with the page (once), so the font menu draws the family in its own face. */
function addToPage(family: string, key: string, bytes: Uint8Array, faceDescriptors: FontFaceDescriptors = {}) {
  if (pageFaces.has(key) || typeof FontFace === 'undefined') return
  pageFaces.add(key)
  try {
    const face = new FontFace(family, bytes.slice().buffer, faceDescriptors)
    void face.load().then(
      (loaded) => document.fonts.add(loaded),
      () => undefined,
    )
  } catch {
    // The page draws it in a fallback; the engine has it all the same.
  }
}

/** Gives the engine a font file's bytes: the families it added (none when it is not a font). */
async function giveEngine(engine: FontEngine, bytes: Uint8Array): Promise<string[]> {
  const answer = JSON.parse(await engine.invoke<string>('addFace', bytes).catch(() => '{}')) as { faces?: number; families?: string[] }
  if ((answer.faces ?? 0) > 0) {
    // The drawing's type was laid out again: the page asks the engine for the picture anew.
    await engine.refreshInfo?.().catch(() => undefined)
    update({ added: snapshot.added + 1 })
  }
  return answer.families ?? []
}

/**
 * Gives `engine` the served family `family` (every file of it), once per engine: true when the
 * engine has it now. Families the editor does not serve answer false at once.
 */
export async function ensureFamily(engine: FontEngine, family: string): Promise<boolean> {
  const catalog = await loadCatalog()
  const entries = catalog.get(family)
  if (!entries) return false
  let mine = given.get(engine)
  if (!mine) {
    mine = new Map()
    given.set(engine, mine)
  }
  let done = mine.get(family)
  if (!done) {
    update({ loading: new Set([...snapshot.loading, family]) })
    done = (async () => {
      let added = false
      for (const entry of entries) {
        const bytes = await read(entry.file)
        if (!bytes) continue
        addToPage(entry.family, entry.file, bytes, descriptors(entry))
        added = (await giveEngine(engine, bytes)).length > 0 || added
      }
      return added
    })().finally(() => {
      const loading = new Set(snapshot.loading)
      loading.delete(family)
      update({ loading })
    })
    mine.set(family, done)
    // A file that failed to load can be tried again next time.
    void done.then((ok) => {
      if (!ok) mine.delete(family)
    })
  }
  return done
}

/**
 * A font file the person chose (Type › Load Font File…): given to the engine and to the page;
 * answers the families it added (empty when it is not a font the engine reads).
 */
export async function loadFontFile(engine: FontEngine, name: string, bytes: Uint8Array): Promise<string[]> {
  const families = await giveEngine(engine, bytes)
  for (const family of families) addToPage(family, `file:${name}:${family}`, bytes)
  return families
}
