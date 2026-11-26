/**
 * The Gradient panel's model (plan 2026-11-25, stage 3): a gradient's type, angle and colour stops
 * (each with its colour, opacity and the midpoint to the next stop, where the two colours mix half
 * and half), edited as a draft and sent whole to `paint.editGradient`, which keeps the gradient's
 * placement. A stop keeps the engine's colour as it came (a CMYK or spot colour stays one) until
 * the person gives it another. Pure, so `node --test` checks it.
 */
import { colorHex, stopsCss, type EngineColor, type EnginePaint, type EngineStop } from '../color/paint.ts'

export type GradientKind = 'linear' | 'radial' | 'freeform'
export type Stop = { offset: number; color: EngineColor | string; hex: string; opacity: number; midpoint: number }
export type GradientDraft = { kind: GradientKind; angle: number; stops: Stop[] }

/** The engine keeps a midpoint between 13 % and 87 % of its segment. */
export const MIDPOINT_MIN = 0.13
export const MIDPOINT_MAX = 0.87

const clamp = (value: number, low: number, high: number) => Math.min(high, Math.max(low, Number.isFinite(value) ? value : low))
const round = (value: number, places = 4) => Math.round(value * 10 ** places) / 10 ** places

/** The draft of a gradient paint, or null when the paint is not a gradient. Stops come sorted by offset. */
export function fromPaint(paint: EnginePaint | null | undefined): GradientDraft | null {
  if (paint?.type !== 'gradient') return null
  const kind = (paint.gradient?.kind ?? 'linear').toLowerCase()
  const stops = (paint.gradient?.stops ?? [])
    .map((stop: EngineStop) => ({
      offset: clamp(stop.offset ?? 0, 0, 1),
      color: stop.color ?? '#000000',
      hex: colorHex(stop.color) ?? '#000000',
      opacity: clamp(stop.opacity ?? 1, 0, 1),
      midpoint: clamp(stop.midpoint ?? 0.5, MIDPOINT_MIN, MIDPOINT_MAX),
    }))
    .sort((a, b) => a.offset - b.offset)
  return { kind: kind === 'radial' || kind === 'freeform' ? kind : 'linear', angle: Number.isFinite(paint.angle) ? (paint.angle as number) : 0, stops }
}

/** `paint.editGradient`'s stops for a draft (sorted, rounded). */
export function engineStops(draft: GradientDraft) {
  return [...draft.stops].sort((a, b) => a.offset - b.offset).map((stop) => ({ offset: round(stop.offset), color: stop.color, opacity: round(stop.opacity, 3), midpoint: round(stop.midpoint, 3) }))
}

/** The colour (`#rrggbb`) and opacity a draft has at `t` (0 to 1), midpoints included. */
export function sampleAt(draft: GradientDraft, t: number): { hex: string; opacity: number } {
  const stops = [...draft.stops].sort((a, b) => a.offset - b.offset)
  const first = stops[0]
  if (!first) return { hex: '#000000', opacity: 1 }
  if (t <= first.offset) return { hex: first.hex, opacity: first.opacity }
  for (let index = 1; index < stops.length; index += 1) {
    const a = stops[index - 1]
    const b = stops[index]
    if (t > b.offset) continue
    const span = b.offset - a.offset
    const local = span <= 0 ? 1 : (t - a.offset) / span
    // The midpoint is where the mix is half and half: a piecewise-linear bend of the segment.
    const mid = a.midpoint
    const mix = local <= mid ? (local / mid) * 0.5 : 0.5 + ((local - mid) / (1 - mid)) * 0.5
    const channel = (hex: string, at: number) => parseInt(hex.slice(1 + at * 2, 3 + at * 2), 16)
    const hex = `#${[0, 1, 2].map((at) => Math.round(channel(a.hex, at) + (channel(b.hex, at) - channel(a.hex, at)) * mix).toString(16).padStart(2, '0')).join('')}`
    return { hex, opacity: a.opacity + (b.opacity - a.opacity) * mix }
  }
  const end = stops[stops.length - 1]
  return { hex: end.hex, opacity: end.opacity }
}

/** The draft with a new stop at `at`, of the colour and opacity the gradient has there; answers its index in the sorted stops. */
export function addStop(draft: GradientDraft, at: number): { draft: GradientDraft; index: number } {
  const offset = clamp(at, 0, 1)
  const { hex, opacity } = sampleAt(draft, offset)
  const stops = [...draft.stops, { offset, color: hex, hex, opacity, midpoint: 0.5 }].sort((a, b) => a.offset - b.offset)
  return { draft: { ...draft, stops }, index: stops.findIndex((stop) => stop.offset === offset && stop.hex === hex) }
}

/** The draft with stop `index` moved to `at`; answers where it now sorts. */
export function moveStop(draft: GradientDraft, index: number, at: number): { draft: GradientDraft; index: number } {
  const moved = draft.stops[index]
  if (!moved) return { draft, index }
  const stop = { ...moved, offset: clamp(at, 0, 1) }
  const stops = draft.stops.map((item, i) => (i === index ? stop : item)).sort((a, b) => a.offset - b.offset)
  return { draft: { ...draft, stops }, index: stops.indexOf(stop) }
}

/** The draft without stop `index`, or null when that would leave fewer than two. */
export function removeStop(draft: GradientDraft, index: number): GradientDraft | null {
  if (draft.stops.length <= 2 || !draft.stops[index]) return null
  return { ...draft, stops: draft.stops.filter((_, i) => i !== index) }
}

/** The draft with stop `index`'s colour (an engine colour, with its hex) or opacity (0 to 1) changed. */
export function setStop(draft: GradientDraft, index: number, change: { color?: EngineColor | string; hex?: string; opacity?: number }): GradientDraft {
  return {
    ...draft,
    stops: draft.stops.map((stop, i) =>
      i === index
        ? { ...stop, ...(change.color !== undefined ? { color: change.color, hex: change.hex ?? colorHex(change.color) ?? stop.hex } : {}), ...(change.opacity !== undefined ? { opacity: clamp(change.opacity, 0, 1) } : {}) }
        : stop,
    ),
  }
}

/** The draft with the midpoint after stop `index` (in sorted order) at `fraction` of its segment. */
export function setMidpoint(draft: GradientDraft, index: number, fraction: number): GradientDraft {
  return { ...draft, stops: draft.stops.map((stop, i) => (i === index ? { ...stop, midpoint: clamp(fraction, MIDPOINT_MIN, MIDPOINT_MAX) } : stop)) }
}

/** Where each midpoint sits on the bar (0 to 1), after each stop but the last, in sorted order. */
export function midpointPlaces(draft: GradientDraft): number[] {
  const stops = [...draft.stops].sort((a, b) => a.offset - b.offset)
  return stops.slice(0, -1).map((stop, index) => stop.offset + (stops[index + 1].offset - stop.offset) * stop.midpoint)
}

/** The draft reversed: offsets mirrored, the stops' order and their midpoints with them. */
export function reversed(draft: GradientDraft): GradientDraft {
  const stops = [...draft.stops].sort((a, b) => a.offset - b.offset)
  const mirrored = stops
    .map((stop, index) => ({ ...stop, offset: round(1 - stop.offset), midpoint: index > 0 ? clamp(1 - stops[index - 1].midpoint, MIDPOINT_MIN, MIDPOINT_MAX) : 0.5 }))
    .reverse()
  return { ...draft, stops: mirrored }
}

/** The draft's ramp as CSS, left to right. */
export function rampCss(draft: GradientDraft) {
  return stopsCss(draft.stops.map((stop) => ({ offset: stop.offset, color: stop.hex, opacity: stop.opacity, midpoint: stop.midpoint })))
}
