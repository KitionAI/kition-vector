import { GradientSection } from './gradient/GradientSection'
import { LayersSection } from './LayersSection'
import { AlignSection } from './panels/AlignSection'
import { AppearanceSection } from './panels/AppearanceSection'
import { PathfinderSection, QuickActions } from './panels/PathfinderSection'
import { StrokeSection } from './panels/StrokeSection'
import { SwatchesSection } from './panels/SwatchesSection'
import { TransformSection } from './panels/TransformSection'
import type { VectorInfo } from './vector'

/**
 * Kition Vector's panels beside the artboard (plan 2026-11-25, stage 3): Properties by what is
 * selected (Transform, Appearance, Stroke, Gradient, quick actions), Align, Pathfinder, Layers and
 * Swatches. Each section is a component of its own on `info`, so a panel dock can hold any of
 * them as it is.
 */
export function SidePanel({ info }: { info: VectorInfo }) {
  return (
    <aside className="kui-panel vector-side" data-side="right" data-testid="vector-side">
      <div className="kui-panel-body">
        <TransformSection info={info} />
        <AppearanceSection info={info} />
        <StrokeSection info={info} />
        <GradientSection info={info} />
        <QuickActions info={info} />
        <AlignSection info={info} />
        <PathfinderSection info={info} />
        <LayersSection info={info} />
        <SwatchesSection info={info} />
      </div>
    </aside>
  )
}
