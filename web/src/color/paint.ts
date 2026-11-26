/**
 * The engine's paints as the editor shows and sends them (plan 2026-11-25, stage 3). `paint.proxies`
 * answers a paint as `{type: 'none'}`, `{type: 'solid', color}`, `{type: 'gradient', gradient:
 * {kind, stops}, angle}` or `{type: 'pattern', pattern}`, a colour as `{model: 'rgb', r, g, b}`
 * (0 to 1), `{model: 'cmyk', c, m, y, k}`, `{model: 'gray', k}` or `{model: 'lab', l, a, b}`. The
 * panels draw each one as it is (None as a slash, a gradient as its ramp), never as black.
 * Pure, so `node --test` checks it.
 */
import { cmykToRgb, rgbToHex, type Cmyk, type Rgb } from './colorMath.ts'

export type EngineColor = { model?: string; r?: number; g?: number; b?: number; c?: number; m?: number; y?: number; k?: number; l?: number; a?: number }
export type EngineStop = { offset?: number; color?: EngineColor | string; opacity?: number; midpoint?: number }
export type EnginePaint =
  | { type: 'none' }
  | { type: 'solid'; color: EngineColor; swatch?: string }
  | { type: 'gradient'; gradient: { kind?: string; stops?: EngineStop[] }; angle?: number; swatch?: string }
  | { type: 'pattern'; pattern?: string }

/** What `paint.proxies` answers. */
export type Proxies = { fill?: EnginePaint; stroke?: EnginePaint; fillActive?: boolean; fillMixed?: boolean; strokeMixed?: boolean }

/** A colour the picker chose: None, or a colour (with the CMYK numbers when they were typed). */
export type Picked = { kind: 'none' } | { kind: 'color'; hex: string; cmyk?: Cmyk }

const unit = (value: unknown) => (typeof value === 'number' && Number.isFinite(value) ? Math.min(1, Math.max(0, value)) : 0)

/** sRGB from CIE L*a*b* (D50), for showing Lab colours (spot inks): linear light through XYZ and the Bradford matrix. */
function labRgb(l: number, a: number, b: number): Rgb {
  const fy = (l + 16) / 116
  const fx = fy + a / 500
  const fz = fy - b / 200
  const inverse = (t: number) => (t ** 3 > 216 / 24389 ? t ** 3 : (116 * t - 16) / (24389 / 27))
  const xyz = [inverse(fx) * 0.96422, inverse(fy), inverse(fz) * 0.82521]
  const matrix = [
    [3.1338561, -1.6168667, -0.4906146],
    [-0.9787684, 1.9161415, 0.033454],
    [0.0719453, -0.2289914, 1.4052427],
  ]
  const gamma = (value: number) => {
    const v = Math.min(1, Math.max(0, value))
    return (v <= 0.0031308 ? v * 12.92 : 1.055 * v ** (1 / 2.4) - 0.055) * 255
  }
  return matrix.map((row) => gamma(row[0] * xyz[0] + row[1] * xyz[1] + row[2] * xyz[2])) as unknown as Rgb
}

/** `#rrggbb` of an engine colour (a hex string as given), or null when it is not one. */
export function colorHex(color: EngineColor | string | null | undefined): string | null {
  if (typeof color === 'string') return /^#[0-9a-f]{6}$/i.test(color) ? color.toLowerCase() : null
  if (!color || typeof color !== 'object') return null
  switch (color.model) {
    case 'cmyk':
      return rgbToHex(cmykToRgb({ c: unit(color.c) * 100, m: unit(color.m) * 100, y: unit(color.y) * 100, k: unit(color.k) * 100 }))
    case 'gray':
      return rgbToHex([1, 1, 1].map(() => (1 - unit(color.k)) * 255) as unknown as Rgb)
    case 'lab':
      return rgbToHex(labRgb(color.l ?? 0, color.a ?? 0, color.b ?? 0))
    default:
      if (typeof color.r === 'number' && typeof color.g === 'number' && typeof color.b === 'number') return rgbToHex([unit(color.r) * 255, unit(color.g) * 255, unit(color.b) * 255])
      return null
  }
}

/** The CMYK numbers (percent) of an engine CMYK colour, else null. */
export function colorCmyk(color: EngineColor | string | null | undefined): Cmyk | null {
  if (!color || typeof color !== 'object' || color.model !== 'cmyk') return null
  return { c: unit(color.c) * 100, m: unit(color.m) * 100, y: unit(color.y) * 100, k: unit(color.k) * 100 }
}

/** The colour parameter a picked colour sends (`paint.setFill`'s `color`): CMYK as typed, else hex. */
export function colorParam(picked: Extract<Picked, { kind: 'color' }>): string | Record<string, number> {
  const cmyk = picked.cmyk
  return cmyk ? { c: cmyk.c / 100, m: cmyk.m / 100, y: cmyk.y / 100, k: cmyk.k / 100 } : picked.hex
}

/** `paint.setFill` / `paint.setStroke` parameters for a picked colour. */
export function pickedParams(picked: Picked): Record<string, unknown> {
  return picked.kind === 'none' ? { none: true } : { color: colorParam(picked) }
}

/** What a paint chip shows. */
export type PaintView =
  | { kind: 'none' }
  | { kind: 'mixed' }
  | { kind: 'color'; hex: string }
  | { kind: 'gradient'; css: string; radial: boolean }
  | { kind: 'pattern'; name: string }

const rgba = (hex: string, opacity: number) => {
  const value = parseInt(hex.slice(1), 16)
  return `rgba(${(value >> 16) & 255}, ${(value >> 8) & 255}, ${value & 255}, ${Math.round(unit(opacity) * 1000) / 1000})`
}

/**
 * CSS for gradient stops (`linear-gradient(to right, …)` or a radial one): each stop's colour at
 * its opacity, and between two stops the colour hint where their midpoint puts the half-way mix.
 */
export function stopsCss(stops: readonly EngineStop[], radial = false): string {
  const sorted = [...stops].sort((a, b) => (a.offset ?? 0) - (b.offset ?? 0))
  const parts: string[] = []
  sorted.forEach((stop, index) => {
    const at = unit(stop.offset) * 100
    parts.push(`${rgba(colorHex(stop.color) ?? '#000000', stop.opacity ?? 1)} ${Math.round(at * 100) / 100}%`)
    const next = sorted[index + 1]
    const mid = stop.midpoint ?? 0.5
    if (next && Math.abs(mid - 0.5) > 1e-3) parts.push(`${Math.round((at + (unit(next.offset) * 100 - at) * mid) * 100) / 100}%`)
  })
  if (parts.length === 1) parts.push(parts[0])
  return radial ? `radial-gradient(circle, ${parts.join(', ')})` : `linear-gradient(to right, ${parts.join(', ')})`
}

/** What a chip shows for `paint` (`mixed` when the selection's paints differ). */
export function paintView(paint: EnginePaint | null | undefined, mixed = false): PaintView {
  if (mixed) return { kind: 'mixed' }
  if (!paint || paint.type === 'none') return { kind: 'none' }
  if (paint.type === 'solid') {
    const hex = colorHex(paint.color)
    return hex ? { kind: 'color', hex } : { kind: 'none' }
  }
  if (paint.type === 'gradient') {
    const radial = (paint.gradient?.kind ?? '').toLowerCase() === 'radial'
    return { kind: 'gradient', css: stopsCss(paint.gradient?.stops ?? [], radial), radial }
  }
  return { kind: 'pattern', name: paint.pattern ?? '' }
}

/** The colour a picker starts from for `paint`: its colour, a gradient's first stop's, else none. */
export function pickedOf(paint: EnginePaint | null | undefined): Picked {
  if (paint?.type === 'solid') {
    const hex = colorHex(paint.color)
    const cmyk = colorCmyk(paint.color)
    return hex ? { kind: 'color', hex, ...(cmyk ? { cmyk } : {}) } : { kind: 'none' }
  }
  if (paint?.type === 'gradient') return { kind: 'color', hex: colorHex(paint.gradient?.stops?.[0]?.color) ?? '#000000' }
  return { kind: 'none' }
}

/** The hex a colour field shows for `paint`, or '' (None, a gradient, a pattern, mixed: the field says which instead). */
export function paintHex(paint: EnginePaint | null | undefined, mixed = false): string {
  const view = paintView(paint, mixed)
  return view.kind === 'color' ? view.hex : ''
}
