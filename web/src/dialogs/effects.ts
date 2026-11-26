// Imports carry `.ts` so `node --test` loads this module as it is.
import { effectAsksAll, effectFields, type EffectEntry } from './params.ts'
import { isEllipsis } from './routes.ts'

/**
 * The Effect menu (plan 2026-11-25, stage 1), made from the engine's effect catalogue
 * (`effect.list`'s `catalog`: each effect's id, label, Effect-menu path, notes and defaults). A row
 * is a `kit:effect.<id>` row the editor runs: an effect whose label ends in "…" opens the generic
 * dialog on its values, previewing `effect.apply`; one without (Outline Stroke, the Pathfinder
 * effects) applies at once. Pure, so `node --test` checks it.
 */

/** The kit row prefix of an effect. */
export const EFFECT_ROW = 'kit:effect.'
/** Effect › Apply Last Effect and Effect › Last Effect…, as the reference's first two rows. */
export const APPLY_LAST = 'kit:effect.applyLast'
export const LAST_DIALOG = 'kit:effect.lastDialog'

/** Effects the menu leaves out, with why: a value they need has no control in the generic dialog yet. */
const NOT_BUILT: Readonly<Record<string, string>> = {
  'adjust.curves': 'Curves needs a curve editor, not built yet; Levels and Brightness/Contrast adjust tones meanwhile.',
}

/** How an effect's row runs: a dialog, at once, or not at all (with why). */
export type EffectRoute = { kind: 'dialog' } | { kind: 'apply' } | { kind: 'hidden'; reason: string }

export function effectRoute(effect: EffectEntry): EffectRoute {
  const own = NOT_BUILT[effect.id]
  if (own) return { kind: 'hidden', reason: own }
  if (!isEllipsis(effect.label)) return { kind: 'apply' }
  if (!effectAsksAll(effect)) return { kind: 'hidden', reason: 'A value of this effect needs a control of its own (a list of points), not built yet.' }
  return effectFields(effect).length ? { kind: 'dialog' } : { kind: 'apply' }
}

/** The effect a `kit:effect.<id>` row names, or null for another row. */
export function effectOfRow(command: string): string | null {
  return command.startsWith(EFFECT_ROW) && command !== APPLY_LAST && command !== LAST_DIALOG ? command.slice(EFFECT_ROW.length) : null
}

/** A menu row (as the SDK's `MenuRow`). */
export type EffectRow = { path: readonly string[]; label: string; command: string; shortcut?: string | null }

/**
 * The Effect menu's rows: Apply Last Effect and Last Effect… (labelled with the last effect when
 * there is one), then each shown effect under its submenu (an effect at the menu's top level stays
 * there), in the catalogue's order.
 */
export function effectRows(catalog: readonly EffectEntry[], lastLabel: string | null, words: { applyLast: (label: string | null) => string; lastDialog: (label: string | null) => string }): EffectRow[] {
  const rows: EffectRow[] = [
    { path: ['Effect'], label: words.applyLast(lastLabel), command: APPLY_LAST, shortcut: 'Cmd+Shift+E' },
    { path: ['Effect'], label: words.lastDialog(lastLabel), command: LAST_DIALOG, shortcut: 'Cmd+Alt+Shift+E' },
  ]
  for (const effect of catalog) {
    if (effectRoute(effect).kind === 'hidden') continue
    const path = effect.menu.length > 1 ? effect.menu.slice(0, 2) : ['Effect']
    rows.push({ path, label: effect.label, command: `${EFFECT_ROW}${effect.id}` })
  }
  return rows
}
