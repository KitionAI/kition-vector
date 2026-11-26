import { BringToFrontIcon as BringToFront, CombineIcon as Combine, GroupIcon as Group, ScissorsIcon as Scissors, SendToBackIcon as SendToBack, SquareSplitHorizontalIcon as SquareSplitHorizontal, SquaresExcludeIcon as SquaresExclude, SquaresIntersectIcon as SquaresIntersect, SquaresSubtractIcon as SquaresSubtract, SquaresUniteIcon as SquaresUnite, UngroupIcon as Ungroup, UnlinkIcon as Unlink, VectorSquareIcon as VectorSquare } from '@kition/tessera/icons/react'
import { IconButton, PanelSection, useKitEditor, useReadOnly, type IconComponent } from '@kition/sdk/editor'
import { useDialogs } from '../dialogs/DialogHost'
import type { VectorInfo } from '../vector'
import { PathfinderCrop, PathfinderMerge, PathfinderMinusBack, PathfinderOutline } from './icons'

/** Pathfinder's ten: the four shape modes, then the six pathfinders, as the reference's panel. */
export const SHAPE_MODES = [
  { icon: SquaresUnite, command: 'object.pathfinder.unite' },
  { icon: SquaresSubtract, command: 'object.pathfinder.minusFront' },
  { icon: SquaresIntersect, command: 'object.pathfinder.intersect' },
  { icon: SquaresExclude, command: 'object.pathfinder.exclude' },
] as const
export const PATHFINDERS = [
  { icon: SquareSplitHorizontal, command: 'object.pathfinder.divide' },
  { icon: Combine, command: 'object.pathfinder.trim' },
  { icon: PathfinderMerge, command: 'object.pathfinder.merge' },
  { icon: PathfinderCrop, command: 'object.pathfinder.crop' },
  { icon: PathfinderOutline, command: 'object.pathfinder.outline' },
  { icon: PathfinderMinusBack, command: 'object.pathfinder.minusBack' },
] as const

/** Window › Pathfinder's commands: all ten are in the panel, so the menu lists none of them. */
export const PATHFINDER_COMMANDS: readonly string[] = [...SHAPE_MODES, ...PATHFINDERS].map((item) => item.command)

/**
 * Properties › Pathfinder (plan 2026-11-25, stage 3): Shape Modes (Unite, Minus Front, Intersect,
 * Exclude) and Pathfinders (Divide, Trim, Merge, Crop, Outline, Minus Back), on two or more
 * selected objects.
 */
export function PathfinderSection({ info }: { info: VectorInfo }) {
  const { run, enabled, phrase: p, byId, t } = useKitEditor()
  const readOnly = useReadOnly()
  const off = readOnly || info.selection.length < 2
  const row = (items: readonly { icon: IconComponent; command: string }[]) => (
    <div className="vector-buttons">
      {items
        .filter((item) => byId.has(item.command))
        .map((item) => (
          <IconButton key={item.command} icon={item.icon} label={p(byId.get(item.command)?.title ?? item.command)} disabled={off || !enabled(item.command)} onClick={() => void run(item.command, {})} data-testid={`vector-${item.command.replace('object.pathfinder.', 'pathfinder-')}`} />
        ))}
    </div>
  )
  return (
    <PanelSection title={p('Pathfinder')} testId="vector-pathfinder">
      <div className="vector-align">
        <span className="vector-align-caption">{t('pathfinder.shapeModes')}</span>
        {row(SHAPE_MODES)}
        <span className="vector-align-caption">{t('pathfinder.pathfinders')}</span>
        {row(PATHFINDERS)}
      </div>
    </PanelSection>
  )
}

const QUICK = [
  { icon: Group, command: 'arrange.group' },
  { icon: Ungroup, command: 'arrange.ungroup' },
  { icon: Scissors, command: 'object.clippingMask.make' },
  { icon: Unlink, command: 'object.clippingMask.release' },
  { icon: VectorSquare, command: 'object.expand' },
  { icon: BringToFront, command: 'arrange.bringToFront' },
  { icon: SendToBack, command: 'arrange.sendToBack' },
] as const

/** Properties › Quick Actions: group, ungroup, clipping masks, Expand… and stacking, for the selection. */
export function QuickActions({ info }: { info: VectorInfo }) {
  const { enabled, phrase: p, byId, t } = useKitEditor()
  const readOnly = useReadOnly()
  const { invoke } = useDialogs()
  if (!info.selection.length) return null
  return (
    <PanelSection title={t('quickActions')} testId="vector-quick-actions">
      <div className="vector-buttons">
        {QUICK.filter((item) => byId.has(item.command)).map((item) => (
          <IconButton key={item.command} icon={item.icon} label={p(byId.get(item.command)?.title ?? item.command)} disabled={readOnly || !enabled(item.command)} onClick={() => invoke(item.command)} data-testid={`vector-quick-${item.command}`} />
        ))}
      </div>
    </PanelSection>
  )
}
