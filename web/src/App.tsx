import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from 'react'
import { ArrowLeftRightIcon as ArrowLeftRight, BanIcon as Ban, RotateCcwIcon as RotateCcw } from '@kition/tessera/icons/react'
import {
  EditorFrame,
  HAND_TOOL,
  IconButton,
  KitDocumentCanvas,
  KitEditorProvider,
  commandForShortcut,
  commandMenuTabs,
  createKitStrings,
  shortcutMatches,
  toast,
  useEngineTools,
  useKitEditor,
  useKitEditorState,
  useKitSession,
  type FrameTab,
  type HostConnection,
  type KitCanvasHandle,
  type MenuRow,
  type Phrases,
  type RasterRect,
  type RasterView,
} from '@kition/sdk/editor'
import manifest from '../../kit.json'
import { engineManifest } from './engine'
import { CATALOGS } from './strings'
import { MENU_ORDER, HIDDEN_TOOLS, VECTOR_PRIMARY, describe, nodePath, type VectorInfo } from './vector'
import { ExportDialog } from './ExportDialog'
import { HomeTab } from './HomeTab'
import { SidePanel } from './SidePanel'
import { DialogsProvider, VectorDialogs, type Dialogs } from './dialogs/DialogHost'
import { APPLY_LAST, EFFECT_ROW, LAST_DIALOG, effectRows } from './dialogs/effects'
import { withoutEllipsis } from './dialogs/routes'
import type { EffectEntry } from './dialogs/params'
import { HelpDialog, type HelpTopic } from './menus/HelpDialog'
import { isHiddenEverywhere, isHiddenRow } from './menus/hidden'
import { errorMessage } from './io/errors'
import { installFocusGuard } from './canvas/focusGuard'
import { canvasKey, singleKeyCommands } from './canvas/keys'
import { toolDialog } from './canvas/toolDialogs'
import { contextRows, type ContextRow } from './canvas/contextRows'
import { CanvasMenu, IsolationBar } from './canvas/CanvasMenu'
import { VECTOR_ZOOM_TOOL, wheelFactor } from './canvas/zoomTool'
import { artboardMarks, drawReadout } from './canvas/decorations'
import { VectorToolRail } from './canvas/VectorToolRail'

/** The view tools are the host's (the Zoom tool Kition Vector's own, with its marquee); the engine's other tools run in the engine. */
const HOST_TOOLS = { hand: HAND_TOOL, zoom: VECTOR_ZOOM_TOOL }
const HIDDEN = new Set(HIDDEN_TOOLS)
const round = (value: number) => Math.round(value * 100) / 100
/** Menus kept in a read-only document. */
const VIEW_MENUS = ['View', 'Window', 'Help']
/** File › Document Setup…'s shortcut, as the reference's. */
const DOCUMENT_SETUP_KEY = 'Cmd+Alt+P'
const HELP_SHORTCUTS = 'kit:help.shortcuts'
const HELP_ABOUT = 'kit:help.about'

/** The editor's workspace: where the keyboard belongs when no field has it. */
const workspace = () => document.querySelector<HTMLElement>('.kui-app')

/** The artboards' union in document pixels (the drawing's points less the extent's origin). */
function artboardsArea(info: VectorInfo | null): RasterRect | null {
  if (!info?.artboards.length) return null
  const [ox, oy] = info.origin
  const x0 = Math.min(...info.artboards.map((board) => board.x)) - ox
  const y0 = Math.min(...info.artboards.map((board) => board.y)) - oy
  const x1 = Math.max(...info.artboards.map((board) => board.x + board.width)) - ox
  const y1 = Math.max(...info.artboards.map((board) => board.y + board.height)) - oy
  return { x0, y0, x1, y1 }
}

/** The artboard the selection's centre is on (the current one), else the first. */
function currentArtboard(info: VectorInfo | null) {
  const bounds = info?.selectionBounds
  if (!info || !bounds) return 0
  const [cx, cy] = [bounds.x + bounds.width / 2, bounds.y + bounds.height / 2]
  const index = info.artboards.findIndex((board) => cx >= board.x && cx <= board.x + board.width && cy >= board.y && cy <= board.y + board.height)
  return Math.max(0, index)
}

/** A colour of the engine's (`{model: 'rgb', r, g, b}`, 0 to 1) as `#rrggbb`, or null. */
function hexOf(color: unknown): string | null {
  const c = color as { r?: number; g?: number; b?: number; gray?: number } | null
  if (!c) return null
  const channel = (value: number) => Math.round(Math.min(1, Math.max(0, value)) * 255).toString(16).padStart(2, '0')
  if (typeof c.r === 'number' && typeof c.g === 'number' && typeof c.b === 'number') return `#${channel(c.r)}${channel(c.g)}${channel(c.b)}`
  if (typeof c.gray === 'number') return `#${channel(1 - c.gray).repeat(3)}`
  return null
}

export function App({ host, phrase }: { host: HostConnection; phrase: Phrases }) {
  const t = useMemo(() => createKitStrings(CATALOGS, host.init.locale), [host])
  const manifestNow = useMemo(engineManifest, [])
  const session = useKitSession(host, manifestNow, describe)
  // The engine's errors in the editor's words (plan 2026-11-25, stage 1: `io/errors.ts`).
  const editorOptions = useMemo(() => ({ report: (title: string, error: unknown) => void toast('error', title || t('error.title'), errorMessage(t, error)) }), [t])
  const editor = useKitEditorState(session, host, t, 'vector', phrase, undefined, editorOptions)
  const { doc, change, state } = session
  // A tool's dialog request (a shape tool's click, an Alt-click with Rotate, a gradient stop's
  // double-click) opens its dialog (plan 2026-11-25, stage 2: `canvas/toolDialogs.ts`).
  const onToolDialog = useRef<(name: string, params: unknown) => void>(() => undefined)
  const toolOptions = useMemo(() => ({ initial: 'selection', hostTools: HOST_TOOLS, hidden: HIDDEN, onDialog: (name: string, params: unknown) => onToolDialog.current(name, params), drawOverlay: drawReadout }), [])
  const tools = useEngineTools(session.tool, doc != null, change, toolOptions)
  const canvas = useRef<KitCanvasHandle>(null)
  const viewMemory = useRef<RasterView | null>(null)
  const [zoom, setZoom] = useState(1)
  const bounds = state?.selectionBounds
  const { commands } = editor
  const [exporting, setExporting] = useState(false)
  const [help, setHelp] = useState<HelpTopic | null>(null)

  // The effect catalogue, read once the drawing is open: Effect's menu is made from it.
  const [effects, setEffects] = useState<EffectEntry[]>([])
  const [lastEffect, setLastEffect] = useState<EffectEntry | null>(null)
  const queryNow = useRef(editor.query)
  queryNow.current = editor.query
  useEffect(() => {
    if (!doc) return
    let live = true
    void queryNow.current<{ catalog?: EffectEntry[] }>('effect.list', {}).then((answer) => {
      if (live && answer?.catalog) setEffects(answer.catalog)
    })
    return () => {
      live = false
    }
  }, [doc])

  // The dialog host (`VectorDialogs`, inside the editor's provider) answers through `dialogApi`;
  // the menus, built out here, reach it through `dialogs`. A "…" row opens its dialog instead of running.
  const dialogApi = useRef<Dialogs | null>(null)
  const runNow = useRef(editor.run)
  runNow.current = editor.run
  const dialogs = useMemo<Dialogs>(
    () => ({
      invoke: (command) => (dialogApi.current ? dialogApi.current.invoke(command) : void runNow.current(command)),
      open: (command, values) => dialogApi.current?.open(command, values) ?? false,
      asks: (command) => dialogApi.current?.asks(command) ?? false,
      ask: (dialog) => dialogApi.current?.ask(dialog) ?? false,
      effectRow: (row) => dialogApi.current?.effectRow(row),
      newDocument: () => dialogApi.current?.newDocument(),
      documentSetup: () => dialogApi.current?.documentSetup(),
    }),
    [],
  )
  onToolDialog.current = (name, params) => {
    const info = state
    const open = (stopColor: string | null) => {
      const dialog = toolDialog(name, params, {
        stopColor: () => stopColor,
        widthPoint: (id, index) => {
          const node = info ? nodePath(info.layers, id)?.node : null
          const point = node?.widthPoints?.[index]
          const half = (node?.strokeWidth ?? 1) / 2
          return point && point.length >= 3 ? { t: point[0], left: point[1] * half, right: point[2] * half } : null
        },
      })
      if (!dialog) return
      if (dialog.kind === 'open') dialogs.open(dialog.command, dialog.values)
      else dialogs.ask({ ...dialog, title: t(`toolDialog.${name}`), memory: `tool:${name}` })
    }
    if (name !== 'gradientStop') return open(null)
    // The stop's colour, to start from: the gradient behind the active proxy.
    const index = typeof (params as { index?: unknown })?.index === 'number' ? (params as { index: number }).index : 0
    void editor
      .query<{ fill?: { gradient?: { stops?: Array<{ color?: unknown }> } }; stroke?: { gradient?: { stops?: Array<{ color?: unknown }> } }; fillActive?: boolean }>('paint.proxies', {})
      .then((proxies) => {
        const paint = proxies?.fillActive === false ? proxies?.stroke : proxies?.fill
        open(hexOf(paint?.gradient?.stops?.[index]?.color))
      })
      .catch(() => open(null))
  }

  // The keyboard is the editor's from the start, and never falls to the page's body: after a
  // panel button, Enter in a field or a dialog it goes back to the workspace (`canvas/focusGuard.ts`).
  useEffect(() => installFocusGuard(workspace), [])
  const focused = useRef(false)
  useEffect(() => {
    if (!state || focused.current) return
    focused.current = true
    const active = document.activeElement
    if (active && active !== document.body && active !== document.documentElement) return
    window.focus()
    workspace()?.focus({ preventScroll: true })
  }, [state])
  // Text typed on the canvas ended (Escape, Cmd+Enter, a click elsewhere): the keyboard leaves the
  // text field for the workspace, so V and the other tool keys pick tools again.
  const typing = tools.wantsText
  useEffect(() => {
    if (typing) return
    const active = document.activeElement
    if (active instanceof HTMLTextAreaElement && active.classList.contains('kui-text-sink')) workspace()?.focus({ preventScroll: true })
  }, [typing])

  // Art off the artboards widens the extent and may move its origin: the view moves with it, so
  // the drawing stays where it is on the screen.
  const origin = state?.origin
  const lastOrigin = useRef<readonly [number, number] | null>(null)
  useLayoutEffect(() => {
    if (!origin) return
    const before = lastOrigin.current
    lastOrigin.current = origin
    const view = canvas.current?.view()
    if (!before || !view || (before[0] === origin[0] && before[1] === origin[1])) return
    canvas.current?.panTo(view.x + (before[0] - origin[0]), view.y + (before[1] - origin[1]))
  }, [origin?.[0], origin?.[1]])

  const boards = artboardsArea(state)
  const initialArea = useMemo(() => boards, [boards?.x0, boards?.y0, boards?.x1, boards?.y1])
  const marks = useMemo(
    () => (state ? artboardMarks({ boards: state.artboards, current: currentArtboard(state), origin: state.origin }) : null),
    [state?.artboards, state?.origin, state?.selectionBounds],
  )

  // Cmd (Ctrl) with the mouse wheel zooms at the pointer by at most 1.2 times a notch (the canvas's
  // own would take 2.7 times); a trackpad's pinch, in small steps, zooms as smoothly as before.
  const region = useRef<HTMLDivElement>(null)
  useEffect(() => {
    const element = region.current
    if (!element) return undefined
    const wheel = (event: WheelEvent) => {
      if (!(event.ctrlKey || event.metaKey) || !(event.target instanceof HTMLCanvasElement)) return
      const view = canvas.current?.view()
      if (!view) return
      event.preventDefault()
      event.stopPropagation()
      const box = event.target.getBoundingClientRect()
      const [sx, sy] = [event.clientX - box.left, event.clientY - box.top]
      const [px, py] = [view.x + sx / view.zoom, view.y + sy / view.zoom]
      canvas.current?.zoomTo(view.zoom * wheelFactor(event.deltaY, event.deltaMode))
      const zoomed = canvas.current?.view()
      if (zoomed) canvas.current?.panTo(px - sx / zoomed.zoom, py - sy / zoomed.zoom)
    }
    element.addEventListener('wheel', wheel, { capture: true, passive: false })
    return () => element.removeEventListener('wheel', wheel, { capture: true })
  }, [])

  // The canvas's right-click menu, by what is selected (`canvas/contextRows.ts`).
  const [menuAt, setMenuAt] = useState<{ x: number; y: number } | null>(null)
  const stateNow = useRef(state)
  stateNow.current = state
  useEffect(() => {
    const menu = (event: MouseEvent) => {
      const target = event.target
      if (!(target instanceof Element) || target.getAttribute('data-testid') !== 'kit-canvas-surface') return
      event.preventDefault()
      event.stopPropagation()
      setMenuAt({ x: event.clientX, y: event.clientY })
    }
    window.addEventListener('contextmenu', menu, true)
    return () => window.removeEventListener('contextmenu', menu, true)
  }, [])
  const menuRows = useMemo<ContextRow[]>(() => {
    if (!menuAt || !state) return []
    const kinds = state.selection.map((id) => nodePath(state.layers, id)?.node.kind ?? 'Path')
    return contextRows({ kinds, only: state.selection.length === 1 ? state.selection[0] : null, isolated: state.isolation !== null })
  }, [menuAt, state])

  /** Gives `key` to the active tool, which takes it now (`toolKeys`), and shows its answer. */
  const toTool = (key: string, mods: { shift: boolean; alt: boolean }) => {
    const zoom = canvas.current?.view()?.zoom ?? 1
    void session.tool('toolKey', [key, JSON.stringify({ shift: mods.shift, alt: mods.alt, command: false, control: false, space: false }), zoom], true).then(() => tools.refresh())
  }
  const singleKeys = useMemo(() => singleKeyCommands(commands.filter((command) => !isHiddenEverywhere(command.id))), [commands])
  const isolationPath = state?.isolation != null ? (nodePath(state.layers, state.isolation)?.names ?? []) : null
  const exitIsolation = () => void editor.run('object.exitIsolation', {}).then(() => workspace()?.focus({ preventScroll: true }))

  const kitCommand = useCallback(
    (command: string) => {
      if (command.startsWith(EFFECT_ROW)) return dialogs.effectRow(command)
      if (command === HELP_SHORTCUTS) return setHelp('shortcuts')
      if (command === HELP_ABOUT) return setHelp('about')
    },
    [dialogs],
  )

  const lastName = lastEffect ? withoutEllipsis(phrase(lastEffect.label)) : null
  const ownShortcuts = useMemo(
    () => [
      { label: `${phrase('File')} › ${withoutEllipsis(t('dialog.documentSetupMenu'))}`, shortcut: DOCUMENT_SETUP_KEY },
      { label: `${phrase('Effect')} › ${t('effect.applyLastPlain')}`, shortcut: 'Cmd+Shift+E' },
      { label: `${phrase('Effect')} › ${t('effect.lastDialogPlain')}`, shortcut: 'Cmd+Alt+Shift+E' },
    ],
    [phrase, t],
  )
  // Home first (its own ribbon; a drop-down panel in Menus), then the engine's menus (without the
  // rows `menus/hidden.ts` leaves out, each "…" row opening its dialog), File starting with
  // Kition's own New…, Document Setup… and Export…, Effect made from the effect catalogue, and Help.
  const tabs = useMemo<FrameTab[]>(() => {
    const engineTabs = commandMenuTabs(commands, {
      order: MENU_ORDER,
      phrase,
      view: VIEW_MENUS,
      primary: VECTOR_PRIMARY,
      hidden: isHiddenRow,
      onCommand: dialogs.invoke,
      onKitCommand: kitCommand,
      before: {
        File: [
          { label: t('dialog.newDocumentMenu'), onSelect: dialogs.newDocument, testId: 'vector-menu-new', primary: true },
          { label: t('dialog.documentSetupMenu'), onSelect: dialogs.documentSetup, testId: 'vector-menu-document-setup' },
          { label: t('exportMenu'), onSelect: () => setExporting(true), testId: 'vector-menu-export', primary: true },
        ],
      },
    })
    const effectMenu = effectRows(effects, lastName, {
      applyLast: (name) => (name ? t('effect.applyLast', { name }) : t('effect.applyLastPlain')),
      lastDialog: (name) => (name ? t('effect.lastDialog', { name }) : t('effect.lastDialogPlain')),
    })
    const withEffects = engineTabs.map((tab) => (tab.id === 'Effect' && tab.menu ? { ...tab, menu: { ...tab.menu, rows: [...effectMenu.slice(0, 2), ...tab.menu.rows, ...effectMenu.slice(2)] as MenuRow[] } } : tab))
    const helpRows: MenuRow[] = [
      { path: ['Help'], label: t('help.shortcutsMenu'), command: HELP_SHORTCUTS },
      { path: ['Help'], label: t('help.aboutMenu'), command: HELP_ABOUT },
    ]
    return [
      { id: 'home', label: t('homeTab'), ribbon: <HomeTab /> },
      ...withEffects,
      { id: 'Help', label: phrase('Help'), view: true, menu: { rows: helpRows, onKitCommand: kitCommand, primary: [HELP_SHORTCUTS] } },
    ]
  }, [commands, phrase, t, dialogs, kitCommand, effects, lastName])

  return (
    <KitEditorProvider editor={editor} tools={tools}>
      <DialogsProvider value={dialogs}>
        <EditorFrame
          tabs={tabs}
          defaultTab="home"
          canvas={canvas}
          zoom={zoom}
          hiddenCommands={isHiddenEverywhere}
          onKeyDown={(event) => {
            const plain = !event.metaKey && !event.ctrlKey && !event.altKey && !event.shiftKey
            // Escape a busy tool did not take deselects (which also ends a pen path, as the path being
            // drawn is the one selected), then leaves isolation mode; taken (default prevented) only
            // then, so in Focus an Escape with nothing to cancel still leaves Focus (KitPointer 4.4).
            if (event.key === 'Escape') {
              if (!plain || !state) return false
              if (state.selection.length) void editor.run('edit.deselectAll', {})
              else if (state.isolation !== null) exitIsolation()
              else return false
              return true
            }
            // The canvas's own keys (`canvas/keys.ts`): what the tool takes now, the arrows, the
            // single-key commands, actual size and fitting the artboards, ending typed text.
            const own = state
              ? canvasKey(event, {
                  toolKeys: state.toolKeys,
                  editingText: tools.wantsText,
                  selection: state.selection.length > 0,
                  singleKeys,
                  toolLetter: (key, shift) => tools.toolForKey(key, shift) !== null,
                })
              : null
            if (own) {
              switch (own.kind) {
                case 'tool':
                  toTool(own.key, { shift: event.shiftKey, alt: event.altKey })
                  break
                case 'endText':
                  toTool('Escape', { shift: false, alt: false })
                  break
                case 'nudge':
                  void editor.run('object.nudge', { dx: own.dx, dy: own.dy, big: own.big, copy: own.copy })
                  break
                case 'command':
                  if (!host.readOnly) void editor.run(own.id, {})
                  break
                case 'pickTool':
                  tools.select(own.id)
                  break
                case 'zoom': {
                  if (own.to === 'actual') canvas.current?.zoomTo(1)
                  else if (own.to === 'all' || !boards) canvas.current?.fit()
                  else canvas.current?.show(boards)
                  break
                }
              }
              return true
            }
            if (plain) return false
            // Kition's own rows' shortcuts: Document Setup…, Apply Last Effect, Last Effect….
            if (shortcutMatches(DOCUMENT_SETUP_KEY, event)) {
              dialogs.documentSetup()
              return true
            }
            if (shortcutMatches('Cmd+Shift+E', event)) {
              dialogs.effectRow(APPLY_LAST)
              return true
            }
            if (shortcutMatches('Cmd+Alt+Shift+E', event)) {
              dialogs.effectRow(LAST_DIALOG)
              return true
            }
            // A command's shortcut opens its dialog as its menu row does (Move…'s Cmd+Shift+M).
            const command = commandForShortcut(
              commands.filter((item) => !isHiddenEverywhere(item.id)),
              event,
            )
            if (command && dialogs.asks(command.id)) {
              dialogs.open(command.id)
              return true
            }
            return false
          }}
          status={
            <span data-testid="vector-status">
              {tools.status ? phrase(tools.status) : state ? t(state.artboards.length === 1 ? 'summaryOne' : 'summary', { objects: state.objects, artboards: state.artboards.length }) : ''}
            </span>
          }
          statusEnd={
            bounds ? (
              <span className="tabular-nums" data-testid="vector-selection-size">
                {t('size', { width: round(bounds.width), height: round(bounds.height) })}
              </span>
            ) : null
          }
          rail={
            <div className="vector-rail">
              <VectorToolRail hidden={HIDDEN_TOOLS} />
              {state ? <Proxies info={state} readOnly={host.readOnly === true} /> : null}
            </div>
          }
          panels={state ? <SidePanel info={state} /> : null}
        >
          <div className="kui-row" ref={region}>
            <KitDocumentCanvas canvas={canvas} viewMemory={viewMemory} framed={false} initialArea={initialArea} decorateUnder={marks} onView={(view) => setZoom(view.zoom)}>
              {isolationPath ? <IsolationBar path={isolationPath} onExit={exitIsolation} /> : null}
            </KitDocumentCanvas>
          </div>
          <CanvasMenu
            at={menuAt}
            rows={menuRows}
            onChoose={(row) => {
              setMenuAt(null)
              if (row.params) void editor.run(row.command, row.params)
              else dialogs.invoke(row.command)
            }}
            onClose={() => setMenuAt(null)}
          />
          <VectorDialogs api={dialogApi} info={state} effects={effects} onEffectApplied={setLastEffect} />
          {exporting && state ? <ExportDialog info={state} onClose={() => setExporting(false)} /> : null}
          {help ? <HelpDialog topic={help} version={manifest.version} own={ownShortcuts} onClose={() => setHelp(null)} /> : null}
        </EditorFrame>
      </DialogsProvider>
    </KitEditorProvider>
  )
}

const isNone = (color: string | null) => color == null || color === 'None' || color === 'none'

/**
 * The Fill and Stroke proxies at the foot of the rail (plan 2026-11-25, stage 2): a click brings
 * one to the front (what the swatches and the colour fields paint), a double-click picks its
 * colour (the system's picker until stage 3's own); Swap, Default and None below them.
 */
function Proxies({ info, readOnly }: { info: VectorInfo; readOnly: boolean }) {
  const { run, t } = useKitEditor()
  const picker = useRef<HTMLInputElement>(null)
  const [picking, setPicking] = useState<'fill' | 'stroke'>('fill')
  const { fill, stroke, fillActive } = info.paint
  const pick = (which: 'fill' | 'stroke') => {
    if (readOnly) return
    setPicking(which)
    const input = picker.current
    if (!input) return
    const current = which === 'fill' ? fill : stroke
    input.value = current && /^#[0-9a-f]{6}$/i.test(current) ? current : '#000000'
    input.click()
  }
  return (
    <div className="vector-proxies">
      <button
        type="button"
        className="vector-proxy vector-proxy-fill"
        style={{ background: isNone(fill) ? undefined : (fill ?? undefined) }}
        data-none={isNone(fill) || undefined}
        title={t('fill')}
        aria-label={t('fill')}
        aria-pressed={fillActive}
        data-active={fillActive}
        onClick={() => void run('paint.toggleActive', { fill: true })}
        onDoubleClick={() => pick('fill')}
        data-testid="vector-proxy-fill"
      />
      <button
        type="button"
        className="vector-proxy vector-proxy-stroke"
        style={{ borderColor: isNone(stroke) ? undefined : (stroke ?? undefined) }}
        data-none={isNone(stroke) || undefined}
        title={t('stroke')}
        aria-label={t('stroke')}
        aria-pressed={!fillActive}
        data-active={!fillActive}
        onClick={() => void run('paint.toggleActive', { fill: false })}
        onDoubleClick={() => pick('stroke')}
        data-testid="vector-proxy-stroke"
      />
      <input
        ref={picker}
        type="color"
        className="vector-proxy-picker"
        tabIndex={-1}
        aria-hidden="true"
        onChange={(event) => void run(picking === 'fill' ? 'paint.setFill' : 'paint.setStroke', { color: event.target.value })}
      />
      <span className="vector-proxy-actions">
        <IconButton icon={ArrowLeftRight} label={t('swapColors')} disabled={readOnly} onClick={() => void run('paint.swap', {})} data-testid="vector-proxy-swap" />
        <IconButton icon={RotateCcw} label={t('defaultColors')} disabled={readOnly} onClick={() => void run('paint.default', {})} data-testid="vector-proxy-default" />
        <IconButton icon={Ban} label={t('noneColor')} disabled={readOnly} onClick={() => void run('paint.none', {})} data-testid="vector-proxy-none" />
      </span>
    </div>
  )
}
