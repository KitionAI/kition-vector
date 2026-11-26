import type { OverlayDrawer, RasterDecoration } from '@kition/sdk/editor'
import type { Bounds } from '../vector'

/** The artboards as the canvas draws them: in document points, the current one marked. */
export type BoardMarks = { boards: ReadonlyArray<Bounds & { name: string }>; current: number; origin: readonly [number, number] }

const token = (name: string, fallback: string) => getComputedStyle(document.documentElement).getPropertyValue(name).trim() || fallback

/**
 * Under the engine's overlays (plan 2026-11-25, stage 2): each artboard's edge and its name above
 * its top-left corner, the current artboard's edge heavier, in the theme's colours. The engine
 * draws the artboards' white; the pasteboard around them is the canvas's own backdrop.
 */
export function artboardMarks(marks: BoardMarks): RasterDecoration {
  return (context, toScreen) => {
    const edge = token('--kit-text-muted', '#6b6b78')
    const strong = token('--kit-text', '#1f1f28')
    const font = `11px ${token('--kit-font', 'system-ui, sans-serif')}`
    const [ox, oy] = marks.origin
    marks.boards.forEach((board, index) => {
      const a = toScreen(board.x - ox, board.y - oy)
      const b = toScreen(board.x + board.width - ox, board.y + board.height - oy)
      const current = index === marks.current
      context.save()
      context.strokeStyle = current ? strong : edge
      context.lineWidth = current ? 1.5 : 1
      context.globalAlpha = current ? 0.9 : 0.55
      context.strokeRect(Math.round(a.x) + 0.5, Math.round(a.y) + 0.5, Math.round(b.x - a.x), Math.round(b.y - a.y))
      context.globalAlpha = 1
      context.font = current ? `600 ${font}` : font
      context.fillStyle = current ? strong : edge
      context.textBaseline = 'bottom'
      context.fillText(board.name, Math.round(a.x), Math.round(a.y) - 4, Math.max(40, b.x - a.x))
      context.restore()
    })
  }
}

/**
 * The engine's readouts drawn on two lines where it gives two (`W: 120 pt` over `H: 80 pt`,
 * `dX` over `dY`): kit-editor draws a measurement on one line, which runs them together.
 */
export const drawReadout: OverlayDrawer = (overlay, { context, toScreen, style }) => {
  if (overlay.kind !== 'measure' || !overlay.text.includes('\n')) return false
  const lines = overlay.text.split('\n')
  const m = style.measure
  const font = m.font ?? style.font
  context.font = font
  const size = Number(/(\d+(?:\.\d+)?)px/.exec(font)?.[1] ?? 11)
  const lineHeight = Math.round(size * 1.3)
  const width = Math.max(...lines.map((line) => context.measureText(line).width)) + m.padX * 2
  const height = lines.length * lineHeight + m.padY * 2 + 2
  const p = toScreen(overlay.x, overlay.y)
  const left = p.x + m.dx
  const top = p.y + m.dy
  const fill = m.fill ?? style.ink
  context.fillStyle = fill
  context.beginPath()
  context.roundRect(left, top, width, height, m.radius)
  context.fill()
  context.fillStyle = m.color ?? (fill === style.surface ? style.ink : style.surface)
  context.textBaseline = 'top'
  lines.forEach((line, index) => context.fillText(line, left + m.padX, top + m.padY + 1 + index * lineHeight))
  return true
}
