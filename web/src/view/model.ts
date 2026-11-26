/**
 * The View menu (plan 2026-11-25, stage 6): its toggles, its rows (Kition's own `kit:view.*` rows
 * among the engine's view commands) and their keys. Pure, so `node --test` checks it.
 *
 * The toggles are the editing session's, not the drawing's: the page keeps them (all but Outline
 * and Pixel Preview across sessions, in the kit's store) and gives them to the engine with a
 * `View:` request, which neither dirties the drawing nor adds an undo step; `inspect().view`
 * reports them to an agent.
 */
import type { MenuRow } from '@kition/sdk/editor'

export const TOGGLES = ['outline', 'pixelPreview', 'smartGuides', 'snapToGrid', 'snapToPoint', 'snapToPixel', 'boundingBox', 'edges', 'cornerWidget', 'guides', 'grid', 'rulers'] as const
export type ViewToggle = (typeof TOGGLES)[number]
export type Toggles = Readonly<Record<ViewToggle, boolean>>

/** As a new drawing opens: Preview, Smart Guides and Snap to Point on, the box, edges, corner widgets and guides shown, no grid or rulers. */
export const DEFAULT_TOGGLES: Toggles = {
  outline: false,
  pixelPreview: false,
  smartGuides: true,
  snapToGrid: false,
  snapToPoint: true,
  snapToPixel: false,
  boundingBox: true,
  edges: true,
  cornerWidget: true,
  guides: true,
  grid: false,
  rulers: false,
}

/** The toggles that change how the art is drawn (the canvas renders again). */
export const RENDERED: ReadonlySet<ViewToggle> = new Set(['outline', 'pixelPreview'])

/** The toggles kept across sessions: not Outline and Pixel Preview, which a drawing opens without. */
export const KEPT: readonly ViewToggle[] = TOGGLES.filter((name) => !RENDERED.has(name))

/** The kept toggles in the kit's store, checked (a stored value that is not a boolean is dropped). */
export function keptOf(value: unknown): Partial<Toggles> | null {
  if (!value || typeof value !== 'object') return null
  const record = value as Record<string, unknown>
  return Object.fromEntries(KEPT.filter((name) => typeof record[name] === 'boolean').map((name) => [name, record[name]]))
}

/** The kept toggles of `toggles`, for the store. */
export function keptPart(toggles: Toggles): Partial<Toggles> {
  return Object.fromEntries(KEPT.map((name) => [name, toggles[name]]))
}

/** The prefix of the View menu's own rows. */
export const VIEW_ROW = 'kit:view.'

/** The zoom rows (they act on the canvas, not on a toggle). */
export const ZOOMS = ['zoomIn', 'zoomOut', 'fitArtboard', 'fitAll', 'actualSize'] as const
export type ViewZoom = (typeof ZOOMS)[number]

/** A View row of Kition's own: a toggle or a zoom, with its label (the reference's English, phrased) and key. */
type OwnRow = { name: ViewToggle | ViewZoom; label: string; shortcut: string | null; path?: readonly string[] }

const OWN: Readonly<Record<string, OwnRow>> = {
  outline: { name: 'outline', label: 'Outline', shortcut: 'Cmd+Y' },
  pixelPreview: { name: 'pixelPreview', label: 'Pixel Preview', shortcut: 'Cmd+Alt+Y' },
  zoomIn: { name: 'zoomIn', label: 'Zoom In', shortcut: 'Cmd+=' },
  zoomOut: { name: 'zoomOut', label: 'Zoom Out', shortcut: 'Cmd+-' },
  fitArtboard: { name: 'fitArtboard', label: 'Fit Artboard in Window', shortcut: 'Cmd+0' },
  fitAll: { name: 'fitAll', label: 'Fit All in Window', shortcut: 'Cmd+Alt+0' },
  actualSize: { name: 'actualSize', label: 'Actual Size', shortcut: 'Cmd+1' },
  edges: { name: 'edges', label: 'Show Edges', shortcut: 'Cmd+H' },
  cornerWidget: { name: 'cornerWidget', label: 'Show Corner Widget', shortcut: null },
  boundingBox: { name: 'boundingBox', label: 'Show Bounding Box', shortcut: 'Cmd+Shift+B' },
  smartGuides: { name: 'smartGuides', label: 'Smart Guides', shortcut: 'Cmd+U' },
  rulers: { name: 'rulers', label: 'Show Rulers', shortcut: 'Cmd+R', path: ['View', 'Rulers'] },
  guides: { name: 'guides', label: 'Show Guides', shortcut: 'Cmd+;', path: ['View', 'Guides'] },
  grid: { name: 'grid', label: 'Show Grid', shortcut: "Cmd+'" },
  snapToGrid: { name: 'snapToGrid', label: 'Snap to Grid', shortcut: "Cmd+Shift+'" },
  snapToPixel: { name: 'snapToPixel', label: 'Snap to Pixel', shortcut: null },
  snapToPoint: { name: 'snapToPoint', label: 'Snap to Point', shortcut: "Cmd+Alt+'" },
}

/**
 * The View menu in the reference's order: Kition's own rows (`kit:view.<name>`) and the engine's
 * view commands (by id), each where it belongs. An engine row the menu does not list here keeps
 * its place after them.
 */
export const VIEW_ORDER: readonly string[] = [
  'outline',
  'view.overprintPreview',
  'pixelPreview',
  'view.proofSetup',
  'view.proofColors',
  'zoomIn',
  'zoomOut',
  'fitArtboard',
  'fitAll',
  'actualSize',
  'edges',
  'cornerWidget',
  'view.slices.hide',
  'view.slices.lock',
  'boundingBox',
  'view.transparencyGrid',
  'smartGuides',
  'rulers',
  'guides',
  'view.guides.lock',
  'view.guides.make',
  'view.guides.release',
  'view.guides.clear',
  'grid',
  'snapToGrid',
  'snapToPixel',
  'snapToPoint',
]

/** The View menu's rows: `engineRows` (the engine's, as `commandMenuTabs` made them) with Kition's own in `VIEW_ORDER`. */
export function viewRows(engineRows: readonly MenuRow[]): MenuRow[] {
  const out: MenuRow[] = []
  const placed = new Set<string>()
  for (const item of VIEW_ORDER) {
    const own = OWN[item]
    if (own) {
      out.push({ path: own.path ?? ['View'], label: own.label, shortcut: own.shortcut, command: `${VIEW_ROW}${own.name}` })
      continue
    }
    for (const row of engineRows.filter((candidate) => candidate.command === item)) {
      out.push(row)
      placed.add(row.command)
    }
  }
  return [...out, ...engineRows.filter((row) => !placed.has(row.command))]
}

/** The toggle or zoom a `kit:view.*` row stands for, or null for any other command. */
export function viewRowOf(command: string): ViewToggle | ViewZoom | null {
  if (!command.startsWith(VIEW_ROW)) return null
  const name = command.slice(VIEW_ROW.length)
  return OWN[name]?.name ?? null
}

export const isToggle = (name: string): name is ViewToggle => (TOGGLES as readonly string[]).includes(name)

/** What the engine's `inspect().view` reports of the View menu's engine rows. */
export type EngineViewState = {
  guidesLocked: boolean
  slicesHidden: boolean
  slicesLocked: boolean
  transparencyGrid: boolean
  proofColors: boolean
  overprintPreview: boolean
}

const ENGINE_CHECKS: Readonly<Record<string, keyof EngineViewState>> = {
  'view.guides.lock': 'guidesLocked',
  'view.slices.hide': 'slicesHidden',
  'view.slices.lock': 'slicesLocked',
  'view.transparencyGrid': 'transparencyGrid',
  'view.proofColors': 'proofColors',
  'view.overprintPreview': 'overprintPreview',
}

/** The View menu's check mark for `command`: a toggle's, or an engine view command's state; undefined for a row that is no toggle. */
export function viewChecked(command: string, toggles: Toggles, engine: EngineViewState | null): boolean | undefined {
  const own = viewRowOf(command)
  if (own) return isToggle(own) ? toggles[own] : undefined
  const key = ENGINE_CHECKS[command]
  return key && engine ? engine[key] : undefined
}

export type KeyLike = { key: string; code: string; metaKey: boolean; ctrlKey: boolean; altKey: boolean; shiftKey: boolean }

/**
 * The View menu's own key `event` presses (Cmd or Ctrl held), or null: Cmd+Y Outline, Cmd+Alt+Y
 * Pixel Preview, Cmd+U Smart Guides, Cmd+H edges, Cmd+Shift+B the bounding box, Cmd+R rulers,
 * Cmd+; guides, Cmd+' the grid, Cmd+Shift+' Snap to Grid, Cmd+Alt+' Snap to Point. Keys by their
 * place on the keyboard (`code`), so a layout's shifted characters do not matter.
 */
export function viewKey(event: KeyLike): ViewToggle | null {
  if (!(event.metaKey || event.ctrlKey)) return null
  const { altKey: alt, shiftKey: shift } = event
  switch (event.code) {
    case 'KeyY':
      return shift ? null : alt ? 'pixelPreview' : 'outline'
    case 'KeyU':
      return alt || shift ? null : 'smartGuides'
    case 'KeyH':
      return alt || shift ? null : 'edges'
    case 'KeyB':
      return shift && !alt ? 'boundingBox' : null
    case 'KeyR':
      return alt || shift ? null : 'rulers'
    case 'Semicolon':
      return alt || shift ? null : 'guides'
    case 'Quote':
      return alt && shift ? null : alt ? 'snapToPoint' : shift ? 'snapToGrid' : 'grid'
    default:
      return null
  }
}

/** The engine's `inspect().view` checked, or null. */
export function engineViewOf(value: unknown): EngineViewState | null {
  if (!value || typeof value !== 'object') return null
  const v = value as Record<string, unknown>
  return {
    guidesLocked: v.guidesLocked === true,
    slicesHidden: v.slicesHidden === true,
    slicesLocked: v.slicesLocked === true,
    transparencyGrid: v.transparencyGrid === true,
    proofColors: v.proofColors === true,
    overprintPreview: v.overprintPreview === true,
  }
}

/** How the grid and the guides are drawn (Preferences › Guides & Grid, the document's grid). */
export type Looks = {
  grid: { spacing: number; subdivisions: number; color: string; dots: boolean; pixelGrid: boolean }
  guide: { color: string; dots: boolean }
}

export const DEFAULT_LOOKS: Looks = { grid: { spacing: 72, subdivisions: 8, color: '#c8c8c8', dots: false, pixelGrid: true }, guide: { color: '#4affff', dots: false } }

const colour = (value: unknown, fallback: string) => (typeof value === 'string' && /^#[0-9a-f]{6}$/i.test(value) ? value : fallback)

/** The looks `inspect().view` reports, with the defaults where it says nothing. */
export function looksOf(value: unknown): Looks {
  const v = (value && typeof value === 'object' ? value : {}) as { gridLook?: Record<string, unknown>; guideLook?: Record<string, unknown> }
  const grid = v.gridLook ?? {}
  const guide = v.guideLook ?? {}
  const spacing = typeof grid.spacing === 'number' && grid.spacing > 0 ? grid.spacing : DEFAULT_LOOKS.grid.spacing
  const subdivisions = typeof grid.subdivisions === 'number' && grid.subdivisions >= 1 ? Math.floor(grid.subdivisions) : DEFAULT_LOOKS.grid.subdivisions
  return {
    grid: { spacing, subdivisions, color: colour(grid.color, DEFAULT_LOOKS.grid.color), dots: grid.style === 'dots', pixelGrid: grid.pixelGrid !== false },
    guide: { color: colour(guide.color, DEFAULT_LOOKS.guide.color), dots: guide.style === 'dots' },
  }
}

/** The zoom the pixel grid shows from in Pixel Preview (600%). */
export const PIXEL_GRID_ZOOM = 6

/** The canvas's zoom range (the SDK's: 1% to 6400%). */
export const ZOOM_MIN = 0.01
export const ZOOM_MAX = 64

/** A zoom typed in the status bar (`150`, `150%`, `33.3 %`), as a factor in range, or null. */
export function typedZoom(text: string): number | null {
  const match = /^\s*(\d+(?:[.,]\d+)?)\s*%?\s*$/.exec(text)
  if (!match) return null
  const percent = Number(match[1].replace(',', '.'))
  if (!Number.isFinite(percent) || percent <= 0) return null
  return Math.min(ZOOM_MAX, Math.max(ZOOM_MIN, percent / 100))
}

/**
 * Ruler ticks for `zoom` (screen pixels per point) in a unit of `perUnit` points: the step between
 * labelled ticks (1, 2 or 5 times a power of ten, in units) so labels stand at least `minimum`
 * screen pixels apart, and how many minor ticks divide it.
 */
export function rulerStep(zoom: number, perUnit: number, minimum = 56): { step: number; minor: number } {
  const pixelsPerUnit = zoom * perUnit
  if (!(pixelsPerUnit > 0) || !Number.isFinite(pixelsPerUnit)) return { step: 1, minor: 1 }
  const wanted = minimum / pixelsPerUnit
  const power = 10 ** Math.floor(Math.log10(wanted))
  for (const factor of [1, 2, 5, 10]) {
    const step = factor * power
    if (step >= wanted) return { step, minor: factor === 2 ? 4 : factor === 5 ? 5 : 10 }
  }
  return { step: 10 * power, minor: 10 }
}

/** A ruler label: whole numbers plain, else up to two decimals without trailing zeros. */
export function rulerLabel(value: number): string {
  const rounded = Math.round(value * 100) / 100
  return String(Object.is(rounded, -0) ? 0 : rounded)
}

/**
 * The canvas's scrollbars: the content they scroll over (the drawing's extent, `width × height`
 * document pixels, with the view) and where the view is in it; null for an axis whose content is
 * all in view (no scrollbar).
 */
export function scrollRange(view: { x: number; y: number; zoom: number; width: number; height: number }, width: number, height: number) {
  const axis = (at: number, page: number, size: number) => {
    const start = Math.min(0, at)
    const end = Math.max(size, at + page)
    const max = end - start - page
    return max > 0.5 ? { start, value: at - start, max, page } : null
  }
  return { x: axis(view.x, view.width / view.zoom, width), y: axis(view.y, view.height / view.zoom, height) }
}
