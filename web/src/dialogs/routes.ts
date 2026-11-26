// Imports carry `.ts` so `node --test` loads this module as it is.
import { isHiddenCommand } from '../menus/hidden.ts'

/**
 * Which dialog a command opens (plan 2026-11-25, stage 1): a menu row whose label ends in "…"
 * asks before it runs. Every way a person runs a command (a menu row, a ribbon button, a shortcut,
 * a panel, a tool's request) asks this first. Pure, so the rules are tested without a browser.
 */

/** The purpose-built dialogs, and `params`, the generic one drawn from a command's parameters. */
export type DialogKind = 'move' | 'rotate' | 'scale' | 'reflect' | 'shear' | 'transformEach' | 'colorBalance' | 'params'

/** Commands with a dialog of their own. */
export const PURPOSE_BUILT: Readonly<Record<string, DialogKind>> = {
  'object.move': 'move',
  'object.rotate': 'rotate',
  'object.scale': 'scale',
  'object.reflect': 'reflect',
  'object.shear': 'shear',
  'object.transformEach': 'transformEach',
  // Its channels depend on the colour mode chosen, which the schema cannot say.
  'edit.colors.adjustBalance': 'colorBalance',
}

/**
 * Rows without "…" whose command cannot run without a value a person chooses: the engine refuses
 * them bare, so they ask too (Document Color Mode, Change Case, Proof Setup).
 */
export const ASKS_ALWAYS: ReadonlySet<string> = new Set(['file.documentColorMode', 'text.changeCase', 'view.proofSetup'])

/** A command as the routing reads it: its id, its title ("Offset Path…"), its menu, and whether it has a parameter a person can set. */
export type RoutedCommand = { id: string; title: string; menu?: readonly string[] | null; asks: boolean }

/**
 * The dialog `command` opens instead of running, or null when it runs at once: a purpose-built
 * dialog, else the generic one for a "…" command (or one that always asks) with something to ask.
 * A "…" command with nothing to ask runs, as it has no values to choose.
 */
export function dialogFor(command: RoutedCommand): DialogKind | null {
  if (isHiddenCommand(command.id, command.menu)) return null
  const own = PURPOSE_BUILT[command.id]
  if (own) return own
  return (isEllipsis(command.title) || ASKS_ALWAYS.has(command.id)) && command.asks ? 'params' : null
}

/** Whether a label ends in "…" (or three dots), the mark of a row that asks first. */
export function isEllipsis(label: string) {
  return /(…|\.\.\.)\s*$/.test(label)
}

/** A label without its trailing "…", for a dialog's title. */
export function withoutEllipsis(label: string) {
  return label.replace(/\s*(…|\.\.\.)\s*$/, '')
}

/**
 * Whether a dialog previews its command on the canvas while it is open: the transforms, the path
 * commands that reshape, the colour edits, envelopes, blends and meshes, and every effect, as the
 * reference's dialogs with a Preview box do. Settings, naming, rasterizing and file rows do not.
 */
export function previews(id: string) {
  if (id === 'effect.apply' || id.startsWith('object.envelope.')) return true
  return [
    'object.move',
    'object.rotate',
    'object.scale',
    'object.reflect',
    'object.shear',
    'object.transformEach',
    'object.path.offsetPath',
    'object.path.simplify',
    'object.path.splitIntoGrid',
    'path.average',
    'object.blend.options',
    'object.mesh.add',
    'object.addObjectMosaic',
    'object.vectorHalftone',
    'object.repeat.options',
    'object.textWrap.options',
    'edit.colors.saturate',
    'edit.colors.adjustBalance',
    'text.areaOptions',
    'text.pathOptions',
  ].includes(id)
}
