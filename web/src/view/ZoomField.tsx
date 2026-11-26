import { useRef, useState, type RefObject } from 'react'
import { Scrollbar } from '@kition/tessera/ui'
import type { KitCanvasHandle, KitCanvasView } from '@kition/sdk/editor'
import { ZOOM_MAX, ZOOM_MIN, scrollRange, typedZoom } from './model'

type Labels = { out: string; in: string; fit: string; field: string }

/**
 * The status bar's zoom (plan 2026-11-25, stage 6), in place of the frame's: zoom out, a slider,
 * zoom in, the percentage, and fit. A click on the percentage makes it a field: a typed value
 * (`150`, `150%`) zooms there on Enter or when the field is left, Escape leaves it as it was.
 */
export function ZoomField({ zoom, canvas, onFit, labels }: { zoom: number; canvas: RefObject<KitCanvasHandle | null>; onFit: () => void; labels: Labels }) {
  const [editing, setEditing] = useState<string | null>(null)
  const done = useRef(false)
  const min = 0.05
  const max = ZOOM_MAX
  const toSlider = (value: number) => Math.log(value / min) / Math.log(max / min)
  const fromSlider = (value: number) => min * (max / min) ** value
  const zoomTo = (value: number) => canvas.current?.zoomTo(Math.min(ZOOM_MAX, Math.max(ZOOM_MIN, value)))
  const percent = Math.round(zoom * 1000) / 10
  const commit = (text: string) => {
    if (done.current) return
    done.current = true
    setEditing(null)
    const value = typedZoom(text)
    if (value !== null) zoomTo(value)
    // The keyboard goes back to the workspace, as after any field.
    ;(document.querySelector('.kui-app') as HTMLElement | null)?.focus({ preventScroll: true })
  }
  return (
    <div className="kui-zoom vector-zoom" data-testid="vector-zoom">
      <button type="button" className="kui-icon-button" aria-label={labels.out} title={labels.out} onClick={() => zoomTo(zoom / 1.25)}>
        −
      </button>
      <input type="range" min={0} max={1} step={0.001} value={toSlider(Math.max(min, zoom))} onChange={(event) => zoomTo(fromSlider(Number(event.target.value)))} aria-label={labels.fit} />
      <button type="button" className="kui-icon-button" aria-label={labels.in} title={labels.in} onClick={() => zoomTo(zoom * 1.25)}>
        +
      </button>
      {editing === null ? (
        <button
          type="button"
          className="kui-zoom-value vector-zoom-value"
          title={labels.field}
          aria-label={labels.field}
          data-testid="vector-zoom-value"
          onClick={() => {
            done.current = false
            setEditing(String(percent))
          }}
        >
          {Math.round(zoom * 100)}%
        </button>
      ) : (
        <input
          className="kui-input vector-zoom-input"
          aria-label={labels.field}
          data-testid="vector-zoom-input"
          value={editing}
          autoFocus
          onFocus={(event) => event.currentTarget.select()}
          onChange={(event) => setEditing(event.target.value)}
          onKeyDown={(event) => {
            event.stopPropagation()
            if (event.key === 'Enter') commit(event.currentTarget.value)
            else if (event.key === 'Escape') {
              done.current = true
              setEditing(null)
              ;(document.querySelector('.kui-app') as HTMLElement | null)?.focus({ preventScroll: true })
            }
          }}
          onBlur={(event) => commit(event.currentTarget.value)}
        />
      )}
      <button type="button" className="kui-button" data-variant="ghost" onClick={onFit}>
        {labels.fit}
      </button>
    </div>
  )
}

/**
 * The canvas's scrollbars (plan 2026-11-25, stage 6), over its right and bottom edges while the
 * drawing's extent (`width × height` document pixels) is not all in view: the content is the extent
 * and the view together, the thumb the view.
 */
export function CanvasScrollbars({ view, width, height, canvas, labels }: { view: KitCanvasView | null; width: number; height: number; canvas: RefObject<KitCanvasHandle | null>; labels: { horizontal: string; vertical: string } }) {
  if (!view || !(view.zoom > 0)) return null
  const range = scrollRange(view, width, height)
  return (
    <>
      {range.y ? (
        <div className="vector-scroll vector-scroll-y" data-testid="vector-scroll-y">
          <Scrollbar label={labels.vertical} orientation="vertical" value={range.y.value} max={range.y.max} page={range.y.page} onChange={(value) => canvas.current?.panTo(view.x, range.y!.start + value)} />
        </div>
      ) : null}
      {range.x ? (
        <div className="vector-scroll vector-scroll-x" data-testid="vector-scroll-x" data-corner={range.y ? true : undefined}>
          <Scrollbar label={labels.horizontal} orientation="horizontal" value={range.x.value} max={range.x.max} page={range.x.page} onChange={(value) => canvas.current?.panTo(range.x!.start + value, view.y)} />
        </div>
      ) : null}
    </>
  )
}
