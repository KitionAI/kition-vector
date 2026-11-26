/**
 * The panels' pure parts (plan 2026-11-25, stage 3): the Stroke panel's arrowheads and dash
 * fields, the blending modes, what Align aligns to, and how a swatch is drawn. `node --test`
 * checks them without a browser.
 */
// Imports carry `.ts` so `node --test` loads this module as it is.
import { colorHex, stopsCss, type EngineColor, type EngineStop } from '../color/paint.ts'

/** The blending modes, by the engine's names. */
export const BLEND_MODES = ['Normal', 'Darken', 'Multiply', 'Color Burn', 'Lighten', 'Screen', 'Color Dodge', 'Overlay', 'Soft Light', 'Hard Light', 'Difference', 'Exclusion', 'Hue', 'Saturation', 'Color', 'Luminosity'] as const

/** The arrowheads `stroke.set` knows, in the Stroke panel's order. */
export const ARROWHEADS = [
  'Arrow', 'ArrowOpen', 'Barbed', 'Concave', 'DoubleArrow', 'HalfArrowLeft', 'HalfArrowRight', 'Chevron', 'Feather', 'Swallowtail', 'Triangle', 'TriangleOpen', 'TriangleReverse', 'TriangleBar',
  'Kite', 'Leaf', 'Drop', 'Circle', 'CircleOpen', 'HalfCircle', 'Oval', 'Target', 'Square', 'SquareOpen', 'Tag', 'TagOpen', 'Diamond', 'DiamondOpen', 'Hexagon', 'HexagonOpen', 'Star',
  'Cross', 'Plus', 'Bar', 'DoubleBar', 'DotOnBar', 'Slash', 'DoubleSlash', 'Bracket', 'Fork',
] as const

/** The dash pattern's six fields (dash, gap, dash, gap, dash, gap) from a pattern; empty past its end. */
export function dashFields(dash: readonly number[] | null): (number | null)[] {
  return Array.from({ length: 6 }, (_, index) => (dash && index < dash.length ? dash[index] : null))
}

/**
 * The dash pattern with field `index` set to `value` (null: emptied). Fields after the first empty
 * one are dropped, and a dash without its gap takes a gap as long as itself, as the reference does.
 */
export function withDash(dash: readonly number[] | null, index: number, value: number | null): number[] {
  const fields = dashFields(dash)
  fields[index] = value
  const out: number[] = []
  for (let at = 0; at < 6; at += 2) {
    const length = fields[at]
    if (length === null) break
    out.push(length, fields[at + 1] ?? length)
  }
  return out.length ? out : [12, 12]
}

export type AlignTo = 'selection' | 'key' | 'artboard'

/**
 * What Align sends for "align to" `to` with `selected` objects: one object alone aligns to its
 * artboard (as the reference does: aligned to itself it would not move); "selection" leaves it to
 * the engine, which aligns to the key object when the selection has one, as the reference's panel
 * switches to Key Object when one is clicked. Pure.
 */
export function alignTo(to: AlignTo, selected: number): { to?: AlignTo } {
  if (selected === 1) return { to: 'artboard' }
  return to === 'selection' ? {} : { to }
}

/** A swatch as `swatch.list` answers it. */
export type Swatch = { name: string; kind: 'none' | 'color' | 'gradient' | 'pattern' | string; group: string | null; global?: boolean; spot?: boolean; color?: EngineColor; hex?: string; gradient?: string; stops?: EngineStop[]; pattern?: string; tintOf?: string }
export type SwatchList = { swatches?: Swatch[]; groups?: { name: string; swatches: string[] }[] }

/** What a swatch's chip shows: its colour (greys and spot colours too), its gradient's ramp, a pattern's mark, None's slash. */
export function swatchView(swatch: Swatch) {
  switch (swatch.kind) {
    case 'none':
      return { kind: 'none' as const }
    case 'gradient':
      return { kind: 'gradient' as const, css: stopsCss(swatch.stops ?? [], swatch.gradient === 'radial'), radial: swatch.gradient === 'radial' }
    case 'pattern':
      return { kind: 'pattern' as const, name: swatch.pattern ?? swatch.name }
    default: {
      const hex = (swatch.hex && /^#[0-9a-f]{6}$/i.test(swatch.hex) ? swatch.hex.toLowerCase() : null) ?? colorHex(swatch.color)
      return hex ? { kind: 'color' as const, hex } : { kind: 'none' as const }
    }
  }
}


/**
 * What a click on `swatch` sends to `paint.setFill` (or `paint.setStroke`): None as `none`, any
 * other swatch by its name (`swatch`), which links a global colour and fits a gradient or pattern
 * to each object. A gradient swatch's colour was sent before, and the engine refused it.
 */
export function swatchParams(swatch: Pick<Swatch, 'name' | 'kind'>): Record<string, unknown> {
  return swatch.kind === 'none' ? { none: true } : { swatch: swatch.name }
}
