/**
 * What the Transform section's X, Y, W and H fields run (plan 2026-11-25, stage 2): the field's
 * value about the selection's top-left, which is what the fields show (`selectionBounds`). Sent
 * about its centre, the engine's default reference point, a new X put the centre there and the
 * object moved by half its size. A width or height that is not a positive number runs nothing:
 * the field goes back to the value it showed. Pure, so `bounds.test.mjs` checks it.
 */
export type BoundsKey = 'x' | 'y' | 'width' | 'height'

/** The reference point the fields measure from: the top-left of the 9-point grid. */
export const TOP_LEFT = 0

/** `object.setBounds`'s parameters for `key` set to `value`, or null when the value is refused. */
export function boundsParams(key: BoundsKey, value: number): Record<string, number> | null {
  if (!Number.isFinite(value)) return null
  if ((key === 'width' || key === 'height') && value <= 0) return null
  return { [key]: value, reference: TOP_LEFT }
}
