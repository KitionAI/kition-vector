import { useMemo, useRef, useState } from 'react'
import { ArrowLeftRightIcon as ArrowLeftRight, RotateCcwIcon as RotateCcw } from '@kition/tessera-icons/react'
import {
  EditorFrame,
  EngineToolRail,
  HAND_TOOL,
  IconButton,
  KitDocumentCanvas,
  KitEditorProvider,
  ZOOM_TOOL,
  commandMenuTabs,
  createKitStrings,
  useEngineTools,
  useKitEditorState,
  useKitSession,
  type FrameTab,
  type HostConnection,
  type KitCanvasHandle,
  type Phrases,
  type RasterView,
} from '@kition/kit-editor'
import { engineManifest } from './engine'
import { CATALOGS } from './strings'
import { FILE_WRITERS, HIDDEN_TOOLS, HOST_FILE_COMMANDS, MENU_ORDER, VECTOR_PRIMARY, describe } from './vector'
import { ExportDialog } from './ExportDialog'
import { HomeTab } from './HomeTab'
import { SidePanel } from './SidePanel'

/** The view tools are the host's; the engine's other tools run in the engine. */
const HOST_TOOLS = { hand: HAND_TOOL, zoom: ZOOM_TOOL }
const HIDDEN = new Set(HIDDEN_TOOLS)
const round = (value: number) => Math.round(value * 100) / 100
/** Menus kept in a read-only document. */
const VIEW_MENUS = ['View', 'Window', 'Help']

export function App({ host, phrase }: { host: HostConnection; phrase: Phrases }) {
  const t = useMemo(() => createKitStrings(CATALOGS, host.init.locale), [host])
  const manifest = useMemo(engineManifest, [])
  const session = useKitSession(host, manifest, describe)
  const editor = useKitEditorState(session, host, t, 'vector', phrase)
  const { doc, change, state } = session
  const tools = useEngineTools(session.tool, doc != null, change, { initial: 'selection', hostTools: HOST_TOOLS, hidden: HIDDEN })
  const canvas = useRef<KitCanvasHandle>(null)
  const viewMemory = useRef<RasterView | null>(null)
  const [zoom, setZoom] = useState(1)
  const bounds = state?.selectionBounds
  const { commands } = editor
  const [exporting, setExporting] = useState(false)
  // Home first (its own ribbon; a drop-down panel in Menus), then the engine's menus; File starts
  // with Export, which writes through the host.
  const tabs = useMemo<FrameTab[]>(
    () => [
      { id: 'home', label: t('homeTab'), ribbon: <HomeTab /> },
      ...commandMenuTabs(commands, {
        order: MENU_ORDER,
        exclude: [...HOST_FILE_COMMANDS, ...FILE_WRITERS],
        phrase,
        view: VIEW_MENUS,
        primary: VECTOR_PRIMARY,
        before: { File: [{ label: t('exportMenu'), onSelect: () => setExporting(true), testId: 'vector-menu-export', primary: true }] },
      }),
    ],
    [commands, phrase, t],
  )

  return (
    <KitEditorProvider editor={editor} tools={tools}>
      <EditorFrame
        tabs={tabs}
        defaultTab="home"
        canvas={canvas}
        zoom={zoom}
        onKeyDown={(event) => {
          // Escape a busy tool did not take deselects (which also ends a pen path, as the path being
          // drawn is the one selected); taken (default prevented) only then, so in Focus an Escape
          // with nothing to cancel still leaves Focus (KitPointer 4.4).
          if (event.key !== 'Escape' || event.metaKey || event.ctrlKey || event.altKey || event.shiftKey || !state?.selection.length) return false
          void editor.run('select.none', {})
          return true
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
        {exporting && state ? <ExportDialog info={state} onClose={() => setExporting(false)} /> : null}
      </EditorFrame>
    </KitEditorProvider>
  )
}
