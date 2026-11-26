/** The artboards' order and the status bar's stepping (plan 2026-11-25, stage 4). Pure, so `node --test` checks it. */

/** Where artboard `from` lands when dropped on row `to` at `fraction` of its height: its new index. */
export function reorderTarget(from: number, to: number, fraction: number) {
  const before = fraction < 0.5 ? to : to + 1
  return before > from ? before - 1 : before
}

/** The next artboard from `active` by `step` (wrapping round), or `active` when there is one. */
export function stepArtboard(active: number, step: 1 | -1, count: number) {
  if (count <= 0) return 0
  return (((active + step) % count) + count) % count
}
