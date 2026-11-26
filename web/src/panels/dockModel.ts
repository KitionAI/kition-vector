import type { MenuRow } from '@kition/sdk/editor'

/**
 * The panel dock's groups and the Window menu's panel rows (plan 2026-11-25, stage 4). Pure, so
 * `node --test` checks it. A group is named after its first tab; a tab is one panel. To add a
 * panel: its tab in `DOCK_TABS` (and a group in `DOCK_GROUPS` with its sizes in `GROUP_SIZES` if
 * it is a new group), its upstream label in `TAB_LABELS`, its content in `panels/dock.tsx`, and a
 * Window row in `WINDOW_TABS` if it has one.
 */

/** The dock's groups, top to bottom. Character and Paragraph (stage 5) are a group of their own. */
export const DOCK_GROUPS = ['properties', 'character', 'layers', 'swatches', 'history'] as const
export type DockGroupId = (typeof DOCK_GROUPS)[number]

/**
 * Each group's tabs, its first the one it opens on. Properties holds the sections for the
 * selection (Transform, Appearance, Stroke, Gradient, quick actions, Align, Pathfinder), so Align
 * and Pathfinder share its group: only one of a group's tabs is drawn at a time, and no section
 * shows twice.
 */
export const DOCK_TABS: Readonly<Record<DockGroupId, readonly string[]>> = {
  properties: ['properties', 'align', 'pathfinder'],
  character: ['character', 'paragraph'],
  layers: ['layers', 'artboards'],
  swatches: ['swatches'],
  history: ['history'],
}

/** Each tab's label: upstream's English, through the editor's `phrase`. */
export const TAB_LABELS: Readonly<Record<string, string>> = {
  properties: 'Properties',
  align: 'Align',
  pathfinder: 'Pathfinder',
  character: 'Character',
  paragraph: 'Paragraph',
  layers: 'Layers',
  artboards: 'Artboards',
  swatches: 'Swatches',
  history: 'History',
}

/**
 * Each group's height while nobody resized it (tab strip included), the height it gives way to
 * in a short column, its least, and (for the filler, the last expanded group) the height it asks
 * for first. Layers fills the column under Properties and Swatches.
 */
export const GROUP_SIZES: Readonly<Record<DockGroupId, { defaultHeight: number; compactHeight: number; minHeight?: number; preferredFill?: number }>> = {
  properties: { defaultHeight: 380, compactHeight: 200, minHeight: 120 },
  character: { defaultHeight: 330, compactHeight: 180, minHeight: 110 },
  layers: { defaultHeight: 280, compactHeight: 160, minHeight: 120, preferredFill: 240 },
  swatches: { defaultHeight: 150, compactHeight: 96, minHeight: 72 },
  history: { defaultHeight: 180, compactHeight: 110, minHeight: 80, preferredFill: 160 },
}

/** The groups shown in a new workspace: Character and Paragraph wait for Window (or Properties' Character button), History for Window › History. */
export const DEFAULT_SHOWN: Readonly<Record<DockGroupId, boolean>> = { properties: true, character: false, layers: true, swatches: true, history: false }

/**
 * The tabs the Window menu lists, in its order. Pathfinder has no row of its own: Window ›
 * Pathfinder is the submenu of its ten commands (stage 1's menu test keeps it), and its tab is
 * next to Properties.
 */
export const WINDOW_TABS = ['align', 'artboards', 'character', 'history', 'layers', 'paragraph', 'properties', 'swatches'] as const

const WINDOW_ROW = 'kit:window.'

/** The Window menu's panel rows (`kit:window.<tab>`), labelled with upstream's names. */
export function windowRows(): MenuRow[] {
  return WINDOW_TABS.map((tab) => ({ path: ['Window'], label: TAB_LABELS[tab] ?? tab, shortcut: null, command: `${WINDOW_ROW}${tab}` }))
}

/** The group a tab is in, or null. */
export function groupOf(tab: string): DockGroupId | null {
  return DOCK_GROUPS.find((group) => DOCK_TABS[group].includes(tab)) ?? null
}

/** The tab a Window row stands for, or null for any other command. */
export function windowTab(command: string): string | null {
  if (!command.startsWith(WINDOW_ROW)) return null
  const tab = command.slice(WINDOW_ROW.length)
  return groupOf(tab) ? tab : null
}

/** The tab a group shows and whether it is collapsed (the dock's `activeTab`); null when the dock cannot say. */
export type DockTabs = (group: string) => { tab: string; collapsed: boolean } | null

/** Which groups are shown. */
export type Shown = Readonly<Record<DockGroupId, boolean>>

/** A group's id in the dock (its stored layout, its test ids). */
export const dockGroupId = (group: DockGroupId) => `${group}-group`

const tabOf = (group: DockGroupId, dock?: DockTabs) => dock?.(dockGroupId(group))?.tab ?? DOCK_TABS[group][0]

/** Whether a Window row's panel shows (its check mark): its group shown with its tab in front; undefined for any other command. */
export function windowShown(shown: Shown, command: string, dock?: DockTabs): boolean | undefined {
  const tab = windowTab(command)
  const group = tab ? groupOf(tab) : null
  if (!tab || !group) return undefined
  return shown[group] && tabOf(group, dock) === tab
}

/**
 * What a Window row does, as the reference's Window menu: a panel showing in front, expanded,
 * hides its group; otherwise the group shows with that tab in front (expanded). The shown groups
 * after it, and the tab to bring forward or null.
 */
export function windowCommand(shown: Shown, command: string, dock?: DockTabs): { shown: Shown; reveal: string | null } {
  const tab = windowTab(command)
  const group = tab ? groupOf(tab) : null
  if (!tab || !group) return { shown, reveal: null }
  const showing = shown[group] && tabOf(group, dock) === tab && !dock?.(dockGroupId(group))?.collapsed
  return { shown: { ...shown, [group]: !showing }, reveal: showing ? null : tab }
}

/** A stored set of shown groups as the dock reads it, or undefined when it is not one. */
export function shownOf(raw: unknown): Shown | undefined {
  if (!raw || typeof raw !== 'object') return undefined
  const record = raw as Record<string, unknown>
  if (!DOCK_GROUPS.every((group) => typeof record[group] === 'boolean' || record[group] === undefined)) return undefined
  return Object.fromEntries(DOCK_GROUPS.map((group) => [group, typeof record[group] === 'boolean' ? record[group] : DEFAULT_SHOWN[group]])) as Shown
}
