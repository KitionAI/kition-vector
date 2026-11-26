import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { ArrowLeftRightIcon as ArrowLeftRight, RotateCcwIcon as RotateCcw } from '@kition/tessera/icons/react'
import {
  EditorFrame,
  EngineToolRail,
  HAND_TOOL,
  IconButton,
  KitDocumentCanvas,
  KitEditorProvider,
  ZOOM_TOOL,
  commandForShortcut,
  commandMenuTabs,
  createKitStrings,
  shortcutMatches,
  toast,
  useEngineTools,
  useKitEditorState,
  useKitSession,
  type FrameTab,
  type HostConnection,
  type KitCanvasHandle,
  type MenuRow,
  type Phrases,
  type RasterView,
} from '@kition/sdk/editor'
import manifest from '../../kit.json'
import { engineManifest } from './engine'
import { CATALOGS } from './strings'
import { MENU_ORDER, HIDDEN_TOOLS, VECTOR_PRIMARY, describe } from './vector'
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

/** The view tools are the host's; the engine's other tools run in the engine. */
const HOST_TOOLS = { hand: HAND_TOOL, zoom: ZOOM_TOOL }
const HIDDEN = new Set(HIDDEN_TOOLS)
const round = (value: number) => Math.round(value * 100) / 100
/** Menus kept in a read-only document. */
const VIEW_MENUS = ['View', 'Window', 'Help']
/** File › Document Setup…'s shortcut, as the reference's. */
const DOCUMENT_SETUP_KEY = 'Cmd+Alt+P'
const HELP_SHORTCUTS = 'kit:help.shortcuts'
const HELP_ABOUT = 'kit:help.about'

export function App({ host, phrase }: { host: HostConnection; phrase: Phrases }) {
  const t = useMemo(() => createKitStrings(CATALOGS, host.init.locale), [host])
  const manifestNow = useMemo(engineManifest, [])
  const session = useKitSession(host, manifestNow, describe)
  // The engine's errors in the editor's words (plan 2026-11-25, stage 1: `io/errors.ts`).
  const editorOptions = useMemo(() => ({ report: (title: string, error: unknown) => void toast('error', title || t('error.title'), errorMessage(t, error)) }), [t])
  const editor = useKitEditorState(session, host, t, 'vector', phrase, undefined, editorOptions)
  const { doc, change, state } = session
  const tools = useEngineTools(session.tool, doc != null, change, { initial: 'selection', hostTools: HOST_TOOLS, hidden: HIDDEN })
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
      effectRow: (row) => dialogApi.current?.effectRow(row),
      newDocument: () => dialogApi.current?.newDocument(),
      documentSetup: () => dialogApi.current?.documentSetup(),
    }),
    [],
  )
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
            // drawn is the one selected); taken (default prevented) only then, so in Focus an Escape
            // with nothing to cancel still leaves Focus (KitPointer 4.4).
            if (event.key === 'Escape') {
              if (!plain || !state?.selection.length) return false
              void editor.run('edit.deselectAll', {})
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
              <EngineToolRail hidden={HIDDEN_TOOLS} />
              {state ? (
                <div className="vector-proxies">
                  <span className="vector-proxy vector-proxy-fill" style={{ background: state.paint.fill ?? 'transparent' }} title={t('fill')} data-active={state.paint.fillActive} />
                  <span className="vector-proxy vector-proxy-stroke" style={{ borderColor: state.paint.stroke ?? 'transparent' }} title={t('stroke')} data-active={!state.paint.fillActive} />
                  <span className="vector-proxy-actions">
                    <IconButton icon={ArrowLeftRight} label={t('swapColors')} disabled={host.readOnly === true} onClick={() => void editor.run('paint.swap', {})} />
                    <IconButton icon={RotateCcw} label={t('defaultColors')} disabled={host.readOnly === true} onClick={() => void editor.run('paint.default', {})} />
                  </span>
                </div>
              ) : null}
            </div>
          }
          panels={state ? <SidePanel info={state} /> : null}
        >
          <div className="kui-row">
            <KitDocumentCanvas canvas={canvas} viewMemory={viewMemory} onView={(view) => setZoom(view.zoom)} />
          </div>
          <VectorDialogs api={dialogApi} info={state} effects={effects} onEffectApplied={setLastEffect} />
          {exporting && state ? <ExportDialog info={state} onClose={() => setExporting(false)} /> : null}
          {help ? <HelpDialog topic={help} version={manifest.version} own={ownShortcuts} onClose={() => setHelp(null)} /> : null}
        </EditorFrame>
      </DialogsProvider>
    </KitEditorProvider>
  )
}
