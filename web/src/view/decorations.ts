import type { RasterDecoration } from '@kition/sdk/editor'
import { PIXEL_GRID_ZOOM, type Looks } from './model'

/** A ruler guide as `guide.list` answers it, with the span an artboard guide runs across (points). */
export type GuideLine = { index: number; vertical: boolean; pos: number; selected: boolean; shown: boolean; span: readonly [number, number] | null }

/** What the View menu draws over the drawing: the grid, the pixel grid and the guides. */
export type ViewMarks = {
  /** The drawing's point at document pixel (0, 0). */
  origin: readonly [number, number]
  looks: Looks
  grid: boolean
  /** Pixel Preview (its grid shows from 600%). */
  pixelPreview: boolean
  guides: readonly GuideLine[] | null
}

const token = (name: string, fallback: string) => getComputedStyle(document.documentElement).getPropertyValue(name).trim() || fallback

/** The canvas's size in CSS pixels. */
function cssSize(context: CanvasRenderingContext2D) {
  const canvas = context.canvas as HTMLCanvasElement
  return { width: canvas.clientWidth || canvas.width, height: canvas.clientHeight || canvas.height }
}

/**
 * Lines every `step` points from point 0 across the canvas (screen x of each, `toScreen` mapping
 * document pixels, `origin` the point at pixel 0), at most `most` of them; none when closer than
 * `closest` screen pixels.
 */
function lines(from: number, to: number, step: number, origin: number, zoom: number, closest: number, most = 4000): number[] {
  if (!(step > 0) || step * zoom < closest) return []
  const first = Math.ceil((from + origin) / step)
  const last = Math.floor((to + origin) / step)
  if (last - first > most) return []
  const out: number[] = []
  for (let k = first; k <= last; k += 1) out.push(k * step - origin)
  return out
}

/**
 * Under the engine's overlays, over the art (plan 2026-11-25, stage 6): the grid (View › Show
 * Grid, Guides & Grid's colour, style, spacing and subdivisions; subdivisions only where they stand
 * 6 screen pixels apart), the pixel grid in Pixel Preview from 600%, and the ruler guides (View ›
 * Guides › Show Guides; the selected ones heavier, in the theme's accent). Translucent, so the art
 * reads through: the canvas draws the document's pixels first, so a grid can only lie over them.
 */
export function viewMarks(marks: ViewMarks): RasterDecoration {
  return (context, toScreen, view) => {
    const { width, height } = cssSize(context)
    const [ox, oy] = marks.origin
    const zoom = view.zoom
    // The document pixels in view.
    const left = view.x
    const top = view.y
    const right = view.x + width / zoom
    const bottom = view.y + height / zoom
    const sx = (x: number) => Math.round(toScreen(x, 0).x) + 0.5
    const sy = (y: number) => Math.round(toScreen(0, y).y) + 0.5
    context.save()
    if (marks.grid) {
      const { spacing, subdivisions, color, dots } = marks.looks.grid
      const minor = spacing / Math.max(1, subdivisions)
      context.strokeStyle = color
      context.fillStyle = color
      context.lineWidth = 1
      const draw = (step: number, alpha: number) => {
        const xs = lines(left, right, step, ox, zoom, 6)
        const ys = lines(top, bottom, step, oy, zoom, 6)
        if (!xs.length || !ys.length) return
        context.globalAlpha = alpha
        if (dots) {
          for (const x of xs) for (const y of ys) context.fillRect(sx(x) - 1, sy(y) - 1, 2, 2)
          return
        }
        context.beginPath()
        for (const x of xs) {
          context.moveTo(sx(x), 0)
          context.lineTo(sx(x), height)
        }
        for (const y of ys) {
          context.moveTo(0, sy(y))
          context.lineTo(width, sy(y))
        }
        context.stroke()
      }
      if (subdivisions > 1) draw(minor, 0.35)
      draw(spacing, 0.75)
    }
    if (marks.pixelPreview && marks.looks.grid.pixelGrid && zoom >= PIXEL_GRID_ZOOM) {
      context.globalAlpha = 0.38
      context.strokeStyle = '#404040'
      context.lineWidth = 1
      context.beginPath()
      for (const x of lines(left, right, 1, 0, zoom, 4)) {
        context.moveTo(sx(x), 0)
        context.lineTo(sx(x), height)
      }
      for (const y of lines(top, bottom, 1, 0, zoom, 4)) {
        context.moveTo(0, sy(y))
        context.lineTo(width, sy(y))
      }
      context.stroke()
    }
    if (marks.guides?.length) {
      const accent = token('--kit-primary', '#3b82f6')
      context.globalAlpha = 1
      for (const guide of marks.guides) {
        if (!guide.shown) continue
        context.strokeStyle = guide.selected ? accent : marks.looks.guide.color
        context.lineWidth = guide.selected ? 2 : 1
        context.setLineDash(marks.looks.guide.dots ? [2, 3] : [])
        context.beginPath()
        if (guide.vertical) {
          const x = sx(guide.pos - ox)
          const [a, b] = guide.span ? [toScreen(0, guide.span[0] - oy).y, toScreen(0, guide.span[1] - oy).y] : [0, height]
          context.moveTo(x, a)
          context.lineTo(x, b)
        } else {
          const y = sy(guide.pos - oy)
          const [a, b] = guide.span ? [toScreen(guide.span[0] - ox, 0).x, toScreen(guide.span[1] - ox, 0).x] : [0, width]
          context.moveTo(a, y)
          context.lineTo(b, y)
        }
        context.stroke()
      }
    }
    context.restore()
  }
}

/** Draws `under` and then `over` (decorations combined, the earlier underneath). */
export function both(under: RasterDecoration | null, over: RasterDecoration | null): RasterDecoration | null {
  if (!under) return over
  if (!over) return under
  return (context, toScreen, view) => {
    under(context, toScreen, view)
    over(context, toScreen, view)
  }
}
