/**
 * The document's units for the dialogs (plan 2026-11-25, stage 1): the engine keeps lengths in
 * points and names the document's unit (`inspect().units`, as File › Document Setup sets it); a
 * dialog shows and reads lengths in that unit. Pure, so `node --test` checks it.
 */

/** Points per unit, by the engine's unit names. */
const POINTS: Readonly<Record<string, number>> = {
  Points: 1,
  Pixels: 1,
  Picas: 12,
  Inches: 72,
  'Feet & Inches': 72,
  Feet: 864,
  Yards: 2592,
  Millimeters: 72 / 25.4,
  Centimeters: 72 / 2.54,
  Meters: 7200 / 2.54,
}

/** The units the engine knows, in Document Setup's order. */
export const UNITS: readonly string[] = ['Points', 'Picas', 'Inches', 'Millimeters', 'Centimeters', 'Pixels', 'Feet & Inches', 'Meters', 'Yards', 'Feet']

/** Points per one `unit` (a unit the engine does not name counts as points). */
export function pointsPer(unit: string) {
  return POINTS[unit] ?? 1
}

/** `points` in `unit`, rounded to what the unit shows. */
export function fromPoints(points: number, unit: string) {
  const value = points / pointsPer(unit)
  const scale = 10 ** unitDecimals(unit)
  return Math.round(value * scale) / scale
}

/** `value` in `unit` as points. */
export function toPoints(value: number, unit: string) {
  return value * pointsPer(unit)
}

/** Decimals a length keeps in `unit`: a fine unit two, a coarse one three or four. */
export function unitDecimals(unit: string) {
  const per = pointsPer(unit)
  return per >= 500 ? 4 : per >= 12 ? 3 : 2
}

/** The short name of `unit` (`pt`, `mm`), the label beside a length. */
export function unitAbbreviation(unit: string) {
  switch (unit) {
    case 'Pixels':
      return 'px'
    case 'Picas':
      return 'pc'
    case 'Inches':
    case 'Feet & Inches':
      return 'in'
    case 'Feet':
      return 'ft'
    case 'Yards':
      return 'yd'
    case 'Millimeters':
      return 'mm'
    case 'Centimeters':
      return 'cm'
    case 'Meters':
      return 'm'
    default:
      return 'pt'
  }
}
