/**
 * What the Transform section's X, Y, W and H fields run (plan 2026-11-25, stage 2): the field's
 * value about the selection's top-left, which is what the fields show (`selectionBounds`). Sent
 * about its centre, the engine's default reference point, a new X put the centre there and the
 * object moved by half its size. A width or height that is not a positive number runs nothing:
 * the field goes back to the value it showed. Stage 3's reference-point grid measures the fields
 * from another of the nine points (`reference`; X and Y are then that point's place). Pure, so
 * `node --test` checks it.
 */
export type BoundsKey = 'x' | 'y' | 'width' | 'height'

/** The reference point the fields measure from: the top-left of the 9-point grid. */
export const TOP_LEFT = 0

/** `object.setBounds`'s parameters for `key` set to `value` about `reference` (0 to 8), or null when the value is refused. */
export function boundsParams(key: BoundsKey, value: number, reference = TOP_LEFT): Record<string, number> | null {
  if (!Number.isFinite(value)) return null
  if ((key === 'width' || key === 'height') && value <= 0) return null
  return { [key]: value, reference }
}

/** The place of reference point `reference` (0 top left, 4 centre, 8 bottom right) of `bounds`. */
export function referencePoint(bounds: { x: number; y: number; width: number; height: number }, reference: number): { x: number; y: number } {
  const column = reference % 3
  const row = Math.floor(reference / 3)
  return { x: bounds.x + (bounds.width * column) / 2, y: bounds.y + (bounds.height * row) / 2 }
}
