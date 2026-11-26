import { useEffect, useLayoutEffect, useRef, type PointerEvent as ReactPointerEvent, type RefObject } from 'react'
import type { KitCanvasHandle } from '@kition/sdk/editor'
import { pointsPer } from '../dialogs/units'
import { rulerLabel, rulerStep } from './model'

/** The rulers' thickness in CSS pixels (app.css's `--vector-ruler`). */
export const RULER = 18

const token = (name: string, fallback: string) => getComputedStyle(document.documentElement).getPropertyValue(name).trim() || fallback

export type RulerDrag = { vertical: boolean; kind: 'drag' | 'up'; x: number; y: number; onCanvas: boolean; shift: boolean }

type Props = {
  /** `x`: the ruler along the top (horizontal guides come out of it); `y`: down the left. */
  axis: 'x' | 'y'
  canvas: RefObject<KitCanvasHandle | null>
  /** The canvas area (its region): the pointer over it is marked on the ruler. */
  area: RefObject<HTMLElement | null>
  /** Moves with every change of the view. */
  tick: number
  /** The drawing's point at document pixel (0, 0). */
  origin: readonly [number, number]
  /** The document's unit (`inspect().units`). */
  unit: string
  label: string
  /** A guide dragged out of the ruler: each move and the release, in document pixels. */
  onGuide: (drag: RulerDrag) => void
}

/**
 * A ruler of the canvas (plan 2026-11-25, stage 6): ticks and numbers in the document's unit
 * from the drawing's point (0, 0), following the zoom and the scroll; the pointer's place marked;
 * dragged from, a guide comes out of it (a horizontal guide from the top ruler, a vertical one from
 * the left), shown as it goes and made where it is released over the canvas.
 */
export function Ruler({ axis, canvas, area, tick, origin, unit, label, onGuide }: Props) {
  const element = useRef<HTMLCanvasElement>(null)
  const pointer = useRef<number | null>(null)
  const horizontal = axis === 'x'

  const draw = () => {
    const ruler = element.current
    const view = canvas.current?.view()
    if (!ruler) return
    const dpr = window.devicePixelRatio || 1
    const length = horizontal ? ruler.clientWidth : ruler.clientHeight
    const thickness = horizontal ? ruler.clientHeight : ruler.clientWidth
    const [w, h] = horizontal ? [length, thickness] : [thickness, length]
    if (ruler.width !== Math.round(w * dpr) || ruler.height !== Math.round(h * dpr)) {
      ruler.width = Math.round(w * dpr)
      ruler.height = Math.round(h * dpr)
    }
    const context = ruler.getContext('2d')
    if (!context) return
    context.setTransform(dpr, 0, 0, dpr, 0, 0)
    context.clearRect(0, 0, w, h)
    context.fillStyle = token('--kit-surface-muted', '#f1f1f4')
    context.fillRect(0, 0, w, h)
    const ink = token('--kit-text-muted', '#6b6b78')
    context.strokeStyle = ink
    context.fillStyle = ink
    context.globalAlpha = 0.5
    context.fillRect(horizontal ? 0 : w - 1, horizontal ? h - 1 : 0, horizontal ? w : 1, horizontal ? 1 : h)
    context.globalAlpha = 1
    if (!view) return
    const per = pointsPer(unit)
    const start = (horizontal ? view.x : view.y) + (horizontal ? origin[0] : origin[1])
    const end = start + length / view.zoom
    const screen = (points: number) => (points - start) * view.zoom
    const { step, minor } = rulerStep(view.zoom, per)
    context.font = `9px ${token('--kit-font', 'system-ui, sans-serif')}`
    context.lineWidth = 1
    context.beginPath()
    const first = Math.floor(start / per / step)
    const last = Math.ceil(end / per / step)
    for (let k = first; k <= last && k - first < 2000; k += 1) {
      for (let m = 0; m < minor; m += 1) {
        const value = (k + m / minor) * step
        const at = Math.round(screen(value * per)) + 0.5
        if (at < 0 || at > length) continue
        const size = m === 0 ? thickness : m === minor / 2 ? thickness * 0.45 : thickness * 0.25
        if (horizontal) {
          context.moveTo(at, h)
          context.lineTo(at, h - size)
        } else {
          context.moveTo(w, at)
          context.lineTo(w - size, at)
        }
      }
      const at = Math.round(screen(k * step * per))
      const text = rulerLabel(k * step)
      if (horizontal) {
        context.textBaseline = 'top'
        context.fillText(text, at + 3, 1)
      } else {
        context.save()
        context.translate(1, at + 3)
        context.rotate(Math.PI / 2)
        context.textBaseline = 'bottom'
        context.fillText(text, 0, 0)
        context.restore()
      }
    }
    context.stroke()
    if (pointer.current !== null) {
      const at = Math.round(pointer.current) + 0.5
      context.strokeStyle = token('--kit-primary', '#3b82f6')
      context.beginPath()
      if (horizontal) {
        context.moveTo(at, 0)
        context.lineTo(at, h)
      } else {
        context.moveTo(0, at)
        context.lineTo(w, at)
      }
      context.stroke()
    }
  }
  const drawNow = useRef(draw)
  drawNow.current = draw

  useLayoutEffect(() => drawNow.current(), [tick, origin[0], origin[1], unit])
  useEffect(() => {
    const ruler = element.current
    if (!ruler) return undefined
    const observer = new ResizeObserver(() => drawNow.current())
    observer.observe(ruler)
    return () => observer.disconnect()
  }, [])
  // The pointer over the canvas, marked on the ruler.
  useEffect(() => {
    const region = area.current
    if (!region) return undefined
    const move = (event: PointerEvent) => {
      const box = element.current?.getBoundingClientRect()
      if (!box) return
      pointer.current = horizontal ? event.clientX - box.left : event.clientY - box.top
      drawNow.current()
    }
    const leave = () => {
      pointer.current = null
      drawNow.current()
    }
    region.addEventListener('pointermove', move)
    region.addEventListener('pointerleave', leave)
    return () => {
      region.removeEventListener('pointermove', move)
      region.removeEventListener('pointerleave', leave)
    }
  }, [area, horizontal])

  // Dragging a guide out: where the pointer is in document pixels, and whether over the canvas.
  const dragging = useRef(false)
  const guideAt = (event: ReactPointerEvent, kind: 'drag' | 'up') => {
    const surface = area.current?.querySelector('.kui-canvas-region') ?? area.current
    const view = canvas.current?.view()
    if (!surface || !view) return
    const box = surface.getBoundingClientRect()
    const [sx, sy] = [event.clientX - box.left, event.clientY - box.top]
    const onCanvas = sx >= 0 && sy >= 0 && sx <= box.width && sy <= box.height
    onGuide({ vertical: !horizontal, kind, x: view.x + sx / view.zoom, y: view.y + sy / view.zoom, onCanvas, shift: event.shiftKey })
  }
  return (
    <canvas
      ref={element}
      className={`vector-ruler vector-ruler-${axis}`}
      role="presentation"
      aria-label={label}
      data-testid={`vector-ruler-${axis}`}
      onPointerDown={(event) => {
        if (event.button !== 0) return
        event.preventDefault()
        event.currentTarget.setPointerCapture(event.pointerId)
        dragging.current = true
      }}
      onPointerMove={(event) => {
        if (dragging.current) guideAt(event, 'drag')
      }}
      onPointerUp={(event) => {
        if (!dragging.current) return
        dragging.current = false
        guideAt(event, 'up')
        if (event.currentTarget.hasPointerCapture(event.pointerId)) event.currentTarget.releasePointerCapture(event.pointerId)
      }}
      onLostPointerCapture={() => {
        dragging.current = false
      }}
    />
  )
}
