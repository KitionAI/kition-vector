import { useEffect, useRef, type RefObject } from 'react'
import type { KitCanvasHandle } from '@kition/sdk/editor'
import { nodePath, type VectorInfo } from '../vector'
import { restartBlink, setBlinking } from './caret'
import { byteSlice, hasRange, typeRequest } from './typing'

/** The Type tools, whose clicks count to a triple-click. */
const TYPE_TOOLS = new Set(['type', 'areaType', 'typeOnPath', 'verticalType', 'verticalAreaType', 'verticalTypeOnPath'])

/** The longest pause (ms) and the farthest move (CSS pixels) between the clicks of a double- or triple-click. */
const CLICK_MS = 500
const CLICK_REACH = 4

/** The hidden text field kit-editor types into the engine through (`TextSink`). */
const sink = () => document.querySelector<HTMLTextAreaElement>('textarea.kui-text-sink')

type ToolCall = (name: 'toolKey', args: unknown[], settled?: boolean) => Promise<unknown>

/**
 * Text typed on the canvas, beyond what kit-editor's text field does (plan 2026-11-25, stage 5):
 *
 * - An input method's composition shows at the caret as it is typed, underlined (the engine's
 *   marked text, through the Type tool's `preedit` request), and its candidate window opens at the
 *   caret: the hidden text field moves there. Its commit lands once (kit-editor sends it as typed
 *   text, which replaces the marked text).
 * - Cmd+C, Cmd+X and Cmd+V while editing copy, cut and paste the selected characters through the
 *   system clipboard (the text field's own copy and paste), not objects.
 * - A click after a pause (half a second) or away from the last starts a new click count, so
 *   only a quick third click selects the paragraph.
 * - The caret blinks while nothing is selected (`caret.ts`).
 */
export function useTypeEditing({
  info,
  wantsText,
  activeTool,
  tool,
  refresh,
  canvas,
  view,
}: {
  info: VectorInfo | null
  wantsText: boolean
  activeTool: string
  tool: ToolCall
  refresh: () => void
  canvas: RefObject<KitCanvasHandle | null>
  /** Moves with every change of the view (scroll, zoom, resize). */
  view: unknown
}) {
  const typing = info?.typing ?? null
  const now = useRef({ info, wantsText, activeTool, tool, refresh, canvas })
  now.current = { info, wantsText, activeTool, tool, refresh, canvas }

  const zoom = () => now.current.canvas.current?.view()?.zoom ?? 1
  const request = (body: Record<string, unknown>) => {
    const { tool: call, refresh: redraw } = now.current
    return call('toolKey', [typeRequest(body), '{}', zoom()], true).then(redraw, () => undefined)
  }

  // The caret blinks while text is edited with no characters selected; it shows again at once
  // when it moves or text is typed.
  const collapsed = wantsText && typing !== null && typing.editing !== null && !hasRange(typing) && !typing.composing
  useEffect(() => setBlinking(collapsed), [collapsed])
  useEffect(() => restartBlink(), [typing?.start, typing?.end, typing?.editing, info?.layers])

  // The hidden text field sits at the caret, so an input method's candidate window opens there.
  const caret = typing?.caret ?? null
  useEffect(() => {
    const field = sink()
    if (!field) return
    const surface = document.querySelector<HTMLElement>('[data-testid="kit-canvas-surface"]')
    const shown = canvas.current?.view()
    if (!wantsText || !caret || !surface || !shown) {
      field.style.removeProperty('left')
      field.style.removeProperty('top')
      field.style.removeProperty('height')
      return
    }
    const box = surface.getBoundingClientRect()
    const parent = (field.offsetParent as HTMLElement | null)?.getBoundingClientRect() ?? { left: 0, top: 0 }
    const [ax, ay, bx, by] = caret
    const x = box.left + (Math.min(ax, bx) - shown.x) * shown.zoom
    const top = box.top + (Math.min(ay, by) - shown.y) * shown.zoom
    const bottom = box.top + (Math.max(ay, by) - shown.y) * shown.zoom
    field.style.left = `${Math.round(x - parent.left)}px`
    field.style.top = `${Math.round(top - parent.top)}px`
    field.style.height = `${Math.max(1, Math.round(bottom - top))}px`
  }, [wantsText, caret?.[0], caret?.[1], caret?.[2], caret?.[3], view])

  useEffect(() => {
    const isSink = (target: EventTarget | null) => target instanceof HTMLTextAreaElement && target.classList.contains('kui-text-sink')
    // An input method's marked text, shown in place as it changes.
    const update = (event: CompositionEvent) => {
      if (!isSink(event.target) || !now.current.wantsText) return
      void request({ preedit: { text: event.data ?? '' } })
    }
    // A composition given up leaves no marked text (a commit comes as typed text instead).
    const end = (event: CompositionEvent) => {
      if (!isSink(event.target) || !now.current.wantsText) return
      if (!event.data) void request({ preedit: { text: '' } })
    }
    // Copy, cut and paste of the selected characters while editing.
    const key = (event: KeyboardEvent) => {
      if (!isSink(event.target) || !now.current.wantsText || event.isComposing) return
      if (!(event.metaKey || event.ctrlKey) || event.altKey) return
      const letter = event.key.toLowerCase()
      if (letter !== 'c' && letter !== 'x' && letter !== 'v') return
      // The field's own copy, cut or paste runs; the editor's object commands do not.
      event.stopPropagation()
      if (letter === 'v') return
      const field = event.target as HTMLTextAreaElement
      const { info: shown } = now.current
      const editing = shown?.typing.editing ?? null
      const node = shown && editing !== null ? nodePath(shown.layers, editing)?.node : null
      const text = node && shown ? byteSlice(node.text ?? '', shown.typing.start, shown.typing.end) : ''
      if (!text) {
        event.preventDefault()
        return
      }
      field.value = text
      field.select()
      // The styled copy: pasting the same characters keeps their style.
      void request({ copy: true })
      window.setTimeout(() => {
        field.value = ''
        if (letter === 'x') {
          const { tool: call, refresh: redraw } = now.current
          void call('toolKey', ['Delete', '{}', zoom()], true).then(redraw, () => undefined)
        }
      }, 0)
    }
    // A press long after the last one, or away from it, starts counting clicks again: only a
    // quick third click selects the paragraph.
    let last = { at: -Infinity, x: 0, y: 0 }
    const press = (event: PointerEvent) => {
      if (event.button !== 0 || !(event.target instanceof Element) || event.target.getAttribute('data-testid') !== 'kit-canvas-surface') return
      const at = performance.now()
      const again = at - last.at <= CLICK_MS && Math.hypot(event.clientX - last.x, event.clientY - last.y) <= CLICK_REACH
      last = { at, x: event.clientX, y: event.clientY }
      if (!again && now.current.wantsText && TYPE_TOOLS.has(now.current.activeTool)) void request({ resetClicks: true })
    }
    document.addEventListener('compositionupdate', update, true)
    document.addEventListener('compositionend', end, true)
    window.addEventListener('keydown', key, true)
    window.addEventListener('pointerdown', press, true)
    return () => {
      document.removeEventListener('compositionupdate', update, true)
      document.removeEventListener('compositionend', end, true)
      window.removeEventListener('keydown', key, true)
      window.removeEventListener('pointerdown', press, true)
    }
  }, [])

  return { collapsed, request }
}
