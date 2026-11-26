import type { KitTool } from '@kition/sdk/editor'

/** How far (CSS pixels) a press must move before it is a drag. */
const DRAG = 4
/** One click of the Zoom tool, and one notch of a mouse wheel with Cmd held, zoom by this. */
export const ZOOM_CLICK = 2
/** The largest step one wheel event zooms by (a mouse's notch), so a mouse can zoom finely. */
export const WHEEL_STEP = 1.2

/**
 * The Zoom tool (plan 2026-11-25, stage 2): a click zooms in at the pointer, Alt-click out, and a
 * drag zooms to the area dragged over (its rectangle drawn while dragging, in the theme's accent).
 */
export const VECTOR_ZOOM_TOOL: KitTool = {
  id: 'Zoom',
  cursor: 'zoom-in',
  begin: (start) => {
    const moved = (pointer: { clientX: number; clientY: number }) => Math.hypot(pointer.clientX - start.clientX, pointer.clientY - start.clientY) > DRAG
    return {
      move: (pointer, { overlay }) => {
        if (!moved(pointer)) return
        overlay.draw((context, toScreen) => {
          const a = toScreen(start.x, start.y)
          const b = toScreen(pointer.x, pointer.y)
          const accent = getComputedStyle(document.documentElement).getPropertyValue('--kit-primary').trim() || '#3b82f6'
          context.strokeStyle = accent
          context.lineWidth = 1
          context.setLineDash([4, 3])
          context.strokeRect(Math.min(a.x, b.x) + 0.5, Math.min(a.y, b.y) + 0.5, Math.abs(b.x - a.x), Math.abs(b.y - a.y))
        })
      },
      end: (pointer, { viewport, overlay }) => {
        overlay.clear()
        if (moved(pointer)) {
          viewport.fitArea({ x0: Math.min(start.x, pointer.x), y0: Math.min(start.y, pointer.y), x1: Math.max(start.x, pointer.x), y1: Math.max(start.y, pointer.y) })
          return
        }
        viewport.zoomTo(viewport.view.zoom * (pointer.alt ? 1 / ZOOM_CLICK : ZOOM_CLICK), pointer.clientX, pointer.clientY)
      },
      cancel: ({ overlay }) => overlay.clear(),
    }
  },
}

/** The zoom factor of one wheel event with Cmd or Ctrl held (`deltaY` in pixels): at most `WHEEL_STEP` either way. */
export function wheelFactor(deltaY: number, deltaMode = 0) {
  const pixels = deltaMode === 1 ? deltaY * 16 : deltaMode === 2 ? deltaY * 400 : deltaY
  const factor = Math.exp(-pixels * 0.01)
  return Math.min(WHEEL_STEP, Math.max(1 / WHEEL_STEP, factor))
}
