import { useMemo, type ReactNode, type RefObject } from 'react'
import { DockArea, useKitEditor, type DockAreaHandle, type DockGroup } from '@kition/sdk/editor'
import { ArtboardsPanel } from '../artboards/ArtboardsPanel'
import { GradientSection } from '../gradient/GradientSection'
import { LayersPanel } from '../layers/LayersPanel'
import type { VectorInfo } from '../vector'
import { AlignSection } from './AlignSection'
import { AppearanceSection } from './AppearanceSection'
import { DOCK_GROUPS, DOCK_TABS, GROUP_SIZES, TAB_LABELS, dockGroupId, type Shown } from './dockModel'
import { HistoryPanel } from './HistoryPanel'
import { PathfinderSection, QuickActions } from './PathfinderSection'
import { StrokeSection } from './StrokeSection'
import { SwatchesSection } from './SwatchesSection'
import { TransformSection } from './TransformSection'

/** The dock's width in CSS pixels. */
export const DOCK_WIDTH = 264

/**
 * Properties, by what is selected (plan 2026-11-25, stage 3): Transform, Appearance, Stroke,
 * Gradient, quick actions, Align and Pathfinder, each a section of its own on `info` (their test
 * ids are the browser tests' way in).
 */
function PropertiesPanel({ info }: { info: VectorInfo }) {
  return (
    <div className="vector-panel-scroll" data-testid="vector-properties-panel">
      <TransformSection info={info} />
      <AppearanceSection info={info} />
      <StrokeSection info={info} />
      <GradientSection info={info} />
      <QuickActions info={info} />
      <AlignSection info={info} />
      <PathfinderSection info={info} />
    </div>
  )
}

/** Each tab's content. Stage 5's Character and Paragraph panels add a line each here. */
const CONTENT: Readonly<Record<string, (info: VectorInfo) => ReactNode>> = {
  properties: (info) => <PropertiesPanel info={info} />,
  align: (info) => (
    <div className="vector-panel-scroll">
      <AlignSection info={info} />
    </div>
  ),
  pathfinder: (info) => (
    <div className="vector-panel-scroll">
      <PathfinderSection info={info} />
    </div>
  ),
  layers: (info) => <LayersPanel info={info} />,
  artboards: (info) => <ArtboardsPanel info={info} />,
  swatches: (info) => (
    <div className="vector-panel-scroll">
      <SwatchesSection info={info} />
    </div>
  ),
  history: (info) => <HistoryPanel info={info} />,
}

/**
 * The panel dock beside the canvas (plan 2026-11-25, stage 4): the groups the Window menu shows,
 * each a strip of tabs (one panel drawn at a time) that collapses to its strip and is resized by its
 * bottom edge; the dock remembers each group's tab, height and collapse (kit-editor's `DockArea`),
 * and the editor which groups show. `dock` receives its handle (the Window menu brings tabs
 * forward through it).
 */
export function VectorDock({ info, shown, dock }: { info: VectorInfo; shown: Shown; dock: RefObject<DockAreaHandle | null> }) {
  const { phrase } = useKitEditor()
  const groups: DockGroup[] = DOCK_GROUPS.map((group) => ({
    id: dockGroupId(group),
    ...GROUP_SIZES[group],
    tabs: DOCK_TABS[group].map((tab) => ({ id: tab, label: phrase(TAB_LABELS[tab] ?? tab), render: () => CONTENT[tab]?.(info) ?? null })),
  }))
  const visible = useMemo(() => new Set(DOCK_GROUPS.filter((group) => shown[group]).map(dockGroupId)), [shown])
  if (visible.size === 0) return null
  return (
    <aside className="kui-panel vector-side vector-dock" data-side="right" data-testid="vector-side" style={{ width: DOCK_WIDTH }}>
      <DockArea kit="kit.vector" groups={groups} shown={visible} handle={dock} testId="vector-dock" />
    </aside>
  )
}
