/**
 * The Color Picker's arithmetic (plan 2026-11-25, stage 3): one colour shown as HSB, RGB, CMYK
 * and hex, each field set from the others. Pure, so `node --test` checks it without a browser.
 *
 * RGB is 8-bit sRGB. HSB has the hue in degrees (0 to 360), saturation and brightness in percent.
 * CMYK is in percent, the plain conversion through RGB (black the darkest channel's distance from
 * white); a colour typed in CMYK keeps its four numbers so the engine receives them as given, not
 * as the RGB they show.
 */

export type Rgb = readonly [number, number, number]
export type Hsb = { h: number; s: number; b: number }
export type Cmyk = { c: number; m: number; y: number; k: number }

const clamp = (value: number, low: number, high: number) => Math.min(high, Math.max(low, Number.isFinite(value) ? value : low))
const byte = (value: number) => Math.round(clamp(value, 0, 255))

/** `#rrggbb` (lower case) from 8-bit RGB. */
export function rgbToHex([r, g, b]: Rgb): string {
  return `#${[r, g, b].map((value) => byte(value).toString(16).padStart(2, '0')).join('')}`
}

/** 8-bit RGB from `#rgb`, `#rrggbb`, with or without `#`, any case; null for anything else. */
export function hexToRgb(text: string): Rgb | null {
  const hex = text.trim().replace(/^#/, '')
  if (/^[0-9a-f]{3}$/i.test(hex)) return [0, 1, 2].map((index) => parseInt(hex[index] + hex[index], 16)) as unknown as Rgb
  if (/^[0-9a-f]{6}$/i.test(hex)) return [0, 2, 4].map((index) => parseInt(hex.slice(index, index + 2), 16)) as unknown as Rgb
  return null
}

/** `#rrggbb` written the one way the editor stores colours, or null when `text` is not a colour. */
export function normalHex(text: string): string | null {
  const rgb = hexToRgb(text)
  return rgb ? rgbToHex(rgb) : null
}

/** HSB (hue in degrees, saturation and brightness in percent) from 8-bit RGB; a grey keeps `hue`. */
export function rgbToHsb([r, g, b]: Rgb, hue = 0): Hsb {
  const [rr, gg, bb] = [r, g, b].map((value) => clamp(value, 0, 255) / 255)
  const max = Math.max(rr, gg, bb)
  const min = Math.min(rr, gg, bb)
  const delta = max - min
  let h = hue
  if (delta > 1e-9) {
    if (max === rr) h = 60 * (((gg - bb) / delta) % 6)
    else if (max === gg) h = 60 * ((bb - rr) / delta + 2)
    else h = 60 * ((rr - gg) / delta + 4)
    if (h < 0) h += 360
  }
  return { h, s: max <= 1e-9 ? 0 : (delta / max) * 100, b: max * 100 }
}

/** 8-bit RGB (rounded) from HSB. */
export function hsbToRgb({ h, s, b }: Hsb): Rgb {
  const hue = (((Number.isFinite(h) ? h : 0) % 360) + 360) % 360
  const sat = clamp(s, 0, 100) / 100
  const val = clamp(b, 0, 100) / 100
  const chroma = val * sat
  const x = chroma * (1 - Math.abs(((hue / 60) % 2) - 1))
  const m = val - chroma
  const [r, g, bl] = hue < 60 ? [chroma, x, 0] : hue < 120 ? [x, chroma, 0] : hue < 180 ? [0, chroma, x] : hue < 240 ? [0, x, chroma] : hue < 300 ? [x, 0, chroma] : [chroma, 0, x]
  return [byte((r + m) * 255), byte((g + m) * 255), byte((bl + m) * 255)]
}

/** CMYK in percent from 8-bit RGB. */
export function rgbToCmyk([r, g, b]: Rgb): Cmyk {
  const [rr, gg, bb] = [r, g, b].map((value) => clamp(value, 0, 255) / 255)
  const k = 1 - Math.max(rr, gg, bb)
  if (k >= 1 - 1e-9) return { c: 0, m: 0, y: 0, k: 100 }
  const ink = (channel: number) => ((1 - channel - k) / (1 - k)) * 100
  return { c: ink(rr), m: ink(gg), y: ink(bb), k: k * 100 }
}

/** 8-bit RGB (rounded) from CMYK in percent. */
export function cmykToRgb({ c, m, y, k }: Cmyk): Rgb {
  const white = 1 - clamp(k, 0, 100) / 100
  const channel = (ink: number) => byte(255 * (1 - clamp(ink, 0, 100) / 100) * white)
  return [channel(c), channel(m), channel(y)]
}

/**
 * Every view of one colour, as the dialog's fields show it; hue kept for greys. `typedCmyk` says
 * the CMYK numbers were typed (they stay as typed while the other fields follow them).
 */
export type ColorFields = { hex: string; rgb: Rgb; hsb: Hsb; cmyk: Cmyk; typedCmyk: boolean }

/** The fields of a colour given as RGB, keeping `hue` where RGB has none (a grey, black). */
export function fieldsOf(rgb: Rgb, hue = 0): ColorFields {
  const clean: Rgb = [byte(rgb[0]), byte(rgb[1]), byte(rgb[2])]
  return { hex: rgbToHex(clean), rgb: clean, hsb: rgbToHsb(clean, hue), cmyk: rgbToCmyk(clean), typedCmyk: false }
}

/** The fields of a colour given as HSB: the HSB itself stays exactly as given (the square and strip do not jump). */
export function fieldsOfHsb(hsb: Hsb): ColorFields {
  const h = clamp(hsb.h, 0, 360)
  const exact = { h: h === 360 ? 0 : h, s: clamp(hsb.s, 0, 100), b: clamp(hsb.b, 0, 100) }
  const rgb = hsbToRgb(exact)
  return { hex: rgbToHex(rgb), rgb, hsb: exact, cmyk: rgbToCmyk(rgb), typedCmyk: false }
}

/** The fields of a colour given as CMYK (percent): the four numbers stay as given. */
export function fieldsOfCmyk(cmyk: Cmyk, hue = 0): ColorFields {
  const exact = { c: clamp(cmyk.c, 0, 100), m: clamp(cmyk.m, 0, 100), y: clamp(cmyk.y, 0, 100), k: clamp(cmyk.k, 0, 100) }
  const rgb = cmykToRgb(exact)
  return { hex: rgbToHex(rgb), rgb, hsb: rgbToHsb(rgb, hue), cmyk: exact, typedCmyk: true }
}

/** Which field a person typed in, and its new number. */
export type ColorField = 'h' | 's' | 'b' | 'r' | 'g' | 'bl' | 'c' | 'm' | 'y' | 'k'

/** The colour after a person typed `value` into `field`, from `current`. */
export function withField(current: ColorFields, field: ColorField, value: number): ColorFields {
  if (!Number.isFinite(value)) return current
  switch (field) {
    case 'h':
      return fieldsOfHsb({ ...current.hsb, h: value })
    case 's':
      return fieldsOfHsb({ ...current.hsb, s: value })
    case 'b':
      return fieldsOfHsb({ ...current.hsb, b: value })
    case 'r':
      return fieldsOf([value, current.rgb[1], current.rgb[2]], current.hsb.h)
    case 'g':
      return fieldsOf([current.rgb[0], value, current.rgb[2]], current.hsb.h)
    case 'bl':
      return fieldsOf([current.rgb[0], current.rgb[1], value], current.hsb.h)
    case 'c':
    case 'm':
    case 'y':
    case 'k':
      return fieldsOfCmyk({ ...current.cmyk, [field]: value }, current.hsb.h)
  }
}

/** Where a colour sits in the saturation and brightness square (0 to 1 across, 0 to 1 down). */
export function squarePoint(hsb: Hsb) {
  return { x: clamp(hsb.s, 0, 100) / 100, y: 1 - clamp(hsb.b, 0, 100) / 100 }
}

/** The saturation and brightness at a point of the square (fractions across and down, clamped). */
export function squareValue(x: number, y: number) {
  return { s: clamp(x, 0, 1) * 100, b: (1 - clamp(y, 0, 1)) * 100 }
}

/** The hue at a point of the strip (a fraction down: red at the top, then magenta, … back to red). */
export function stripHue(y: number) {
  return (1 - clamp(y, 0, 1)) * 360
}
