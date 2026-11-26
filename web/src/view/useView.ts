import { useCallback, useEffect, useMemo, useRef, useState, type RefObject } from 'react'
import { useKitStore, type KitCanvasHandle, type KitCanvasView } from '@kition/sdk/editor'
import type { VectorInfo } from '../vector'
import type { GuideLine } from './decorations'
import { DEFAULT_TOGGLES, RENDERED, engineViewOf, keptOf, keptPart, looksOf, type Toggles, type ViewToggle } from './model'

/** A document's tool call (the session's `tool`). */
type ToolCall = (name: 'toolKey', args: unknown[], settled?: boolean) => Promise<unknown>

/** The `View:` request a key carries to the engine (`bindings/wasm/src/viewing.rs`). */
const REQUEST = 'View:'
const NO_MODS = JSON.stringify({ shift: false, alt: false, command: false, control: false, space: false })

type Options = {
  /** The open document (null before it opened). */
  doc: unknown
  info: VectorInfo | null
  /** The engine's `inspect().view`. */
  inspectedView: unknown
  tool: ToolCall
  refreshTools: () => void
  canvas: RefObject<KitCanvasHandle | null>
  query: <T>(command: string, params: Record<string, unknown>) => Promise<T | null>
  /** Moves with every change of the drawing (the guides are read again). */
  revision: number
}

/** A guide row of `guide.list`. */
type GuideRow = { index?: number; vertical?: boolean; pos?: number; selected?: boolean; artboard?: number; shown?: boolean }

/**
 * The View menu's state (plan 2026-11-25, stage 6): the toggles (kept in the kit's store but for
 * Outline and Pixel Preview), given to the engine with `View:` requests whenever they change and
 * when a drawing opens; the canvas window, given as it moves (a guide dropped on its ruler is
 * deleted); the drawing's ruler guides, read with `guide.list` after every change; a guide dragged
 * out of a ruler.
 */
export function useView({ doc, info, inspectedView, tool, refreshTools, canvas, query, revision }: Options) {
  const [kept, setKept] = useKitStore('kit.vector', 'viewToggles', { initial: keptPart(DEFAULT_TOGGLES), validate: keptOf })
  const [rendered, setRendered] = useState({ outline: DEFAULT_TOGGLES.outline, pixelPreview: DEFAULT_TOGGLES.pixelPreview })
  const toggles: Toggles = useMemo(() => ({ ...DEFAULT_TOGGLES, ...kept, ...rendered }), [kept, rendered])
  const togglesNow = useRef(toggles)
  togglesNow.current = toggles

  const toolNow = useRef(tool)
  toolNow.current = tool
  const send = useCallback((body: unknown, zoom?: number) => {
    const at = zoom ?? canvas.current?.view()?.zoom ?? 1
    return toolNow.current('toolKey', [`${REQUEST}${JSON.stringify(body)}`, NO_MODS, at], true).catch(() => null)
  }, [canvas])

  // A drawing opened: the engine takes the toggles as they are.
  useEffect(() => {
    if (!doc) return
    void send({ set: togglesNow.current }).then(() => {
      refreshTools()
      if (togglesNow.current.outline || togglesNow.current.pixelPreview) void canvas.current?.retryRender()
    })
  }, [doc])

  const setToggle = useCallback(
    (name: ViewToggle, on: boolean) => {
      if (RENDERED.has(name)) setRendered((now) => ({ ...now, [name]: on }))
      else setKept({ ...keptPart(togglesNow.current), [name]: on })
      togglesNow.current = { ...togglesNow.current, [name]: on }
      void send({ set: { [name]: on } }).then(() => {
        refreshTools()
        if (RENDERED.has(name)) void canvas.current?.retryRender()
      })
    },
    [send, setKept, refreshTools, canvas],
  )
  const toggle = useCallback((name: ViewToggle) => setToggle(name, !togglesNow.current[name]), [setToggle])

  // The canvas window, as it moves: one request at a time, the latest one next.
  const screen = useRef<{ pending: KitCanvasView | null; busy: boolean; sent: string }>({ pending: null, busy: false, sent: '' })
  const onView = useCallback(
    (view: KitCanvasView) => {
      const state = screen.current
      const key = `${view.x},${view.y},${view.width},${view.height},${view.zoom}`
      if (key === state.sent) return
      state.sent = key
      state.pending = view
      if (state.busy || !doc) return
      const next = async () => {
        state.busy = true
        while (state.pending) {
          const v = state.pending
          state.pending = null
          await send({ screen: { x: v.x, y: v.y, width: v.width, height: v.height, zoom: v.zoom } }, v.zoom)
        }
        state.busy = false
      }
      void next()
    },
    [doc, send],
  )

  // The ruler guides, read after every change of the drawing.
  const [guideRows, setGuideRows] = useState<GuideRow[] | null>(null)
  const queryNow = useRef(query)
  queryNow.current = query
  const readGuides = useCallback(async () => {
    const rows = await queryNow.current<GuideRow[]>('guide.list', {}).catch(() => null)
    setGuideRows(Array.isArray(rows) ? rows : [])
  }, [])
  useEffect(() => {
    if (doc) void readGuides()
  }, [doc, revision, readGuides])

  const artboards = info?.artboards
  const guides = useMemo<GuideLine[] | null>(() => {
    if (!guideRows || !toggles.guides) return null
    return guideRows
      .filter((row) => typeof row.pos === 'number' && typeof row.index === 'number')
      .map((row) => {
        const board = typeof row.artboard === 'number' ? artboards?.[row.artboard] : undefined
        const span: readonly [number, number] | null = board ? (row.vertical ? [board.y, board.y + board.height] : [board.x, board.x + board.width]) : null
        return { index: row.index as number, vertical: row.vertical === true, pos: row.pos as number, selected: row.selected === true, shown: row.shown !== false, span }
      })
  }, [guideRows, toggles.guides, artboards])

  // A guide out of a ruler: each pointer event in turn (the latest move kept while one is on its way).
  const ruler = useRef<{ busy: boolean; pending: Record<string, unknown> | null }>({ busy: false, pending: null })
  const rulerDrag = useCallback(
    (event: { vertical: boolean; kind: 'drag' | 'up'; x: number; y: number; onCanvas: boolean; shift: boolean }) => {
      const state = ruler.current
      state.pending = event
      if (state.busy) return
      const next = async () => {
        state.busy = true
        while (state.pending) {
          const body = state.pending
          state.pending = null
          await send({ ruler: body })
          await readGuides()
          refreshTools()
        }
        state.busy = false
      }
      void next()
    },
    [send, readGuides, refreshTools],
  )

  const engine = useMemo(() => engineViewOf(inspectedView), [inspectedView])
  const looks = useMemo(() => looksOf(inspectedView), [inspectedView])
  return { toggles, toggle, setToggle, onView, guides, rulerDrag, engine, looks }
}

export type ViewState = ReturnType<typeof useView>
