import { ArrowLeftRightIcon as ArrowLeftRight, BringToFrontIcon as BringToFront, CombineIcon as Combine, ContrastIcon as Contrast, GroupIcon as Group, LayersIcon as Layers, LayersPlusIcon as LayersPlus, MousePointer2Icon as MousePointer2, PaintBucketIcon as PaintBucket, PenLineIcon as PenLine, RotateCcwIcon as RotateCcw, SendToBackIcon as SendToBack, ShapesIcon as Shapes, SquareDashedMousePointerIcon as SquareDashedMousePointer, SquarePlusIcon as SquarePlus, SquareXIcon as SquareX, SquaresExcludeIcon as SquaresExclude, SquaresIntersectIcon as SquaresIntersect, SquaresSubtractIcon as SquaresSubtract, UngroupIcon as Ungroup } from '@kition/tessera/icons/react'
import { ColorGrid, CommandButton, MenuItem, RibbonGroup, RibbonMenu, RibbonStack, ToolButton, toolIcon, useKitEditor, useKitTools } from '@kition/sdk/editor'
import { ALIGN } from './SidePanel'
import { useDialogs } from './dialogs/DialogHost'

/** The shape tools the Shapes menu chooses, in Illustrator's order. */
const SHAPE_TOOLS = ['rectangle', 'roundedRectangle', 'ellipse', 'polygon', 'star', 'lineSegment', 'arc', 'spiral']

/**
 * One of the engine's tools by id (the rail draws it too, so it has its own test id). The rail
 * always shows the tools, so they are not `primary`: Classic's ribbon has them, Single line's row
 * does not repeat them.
 */
function HomeTool({ id, primary, size = 'large' }: { id: string; primary?: boolean; size?: 'large' | 'small' }) {
  const tools = useKitTools()
  const { phrase: p } = useKitEditor()
  const tool = tools?.tools.find((item) => item.id === id)
  if (!tool) return null
  return <ToolButton tool={id} icon={toolIcon(tool.icon, id)} label={p(tool.label)} size={size} testId={`vector-home-tool-${id}`} primary={primary} />
}

/**
 * Vector's Home tab: the tasks done most, in every display mode (a drop-down panel in Menus).
 * Drawing (the shape tools, Pen, Type), arranging and aligning, combining shapes (Pathfinder),
 * the selection's fill and stroke, selecting, and new layers and artboards.
 */
export function HomeTab() {
  const { run, enabled, phrase: p, t } = useKitEditor()
  const tools = useKitTools()
  // Home's buttons run their commands as the menus do: through the dialog host.
  const { invoke } = useDialogs()
  return (
    <>
      <RibbonGroup label={t('home.draw')} icon={Shapes}>
        <RibbonMenu icon={Shapes} label={t('home.shapes')} size="large" disabled={!tools} testId="vector-home-shapes" primary>
          {SHAPE_TOOLS.flatMap((id) => {
            const tool = tools?.tools.find((item) => item.id === id)
            return tool ? [<MenuItem key={id} icon={toolIcon(tool.icon, id)} label={p(tool.label)} checked={tools?.active === id} onSelect={() => tools?.select(id)} testId={`vector-home-shape-${id}`} />] : []
          })}
        </RibbonMenu>
        <HomeTool id="pen" />
        <HomeTool id="type" />
        <RibbonStack>
          <HomeTool id="pencil" size="small" />
          <HomeTool id="paintbrush" size="small" />
        </RibbonStack>
      </RibbonGroup>

      <RibbonGroup label={t('home.arrange')} icon={Group}>
        <RibbonStack>
          <CommandButton onRun={invoke} command="arrange.bringForward" icon={BringToFront} primary />
          <CommandButton onRun={invoke} command="arrange.sendBackward" icon={SendToBack} primary />
          <CommandButton onRun={invoke} command="arrange.bringToFront" icon={BringToFront} />
        </RibbonStack>
        <RibbonStack>
          <CommandButton onRun={invoke} command="arrange.group" icon={Group} primary />
          <CommandButton onRun={invoke} command="arrange.ungroup" icon={Ungroup} primary />
          <CommandButton onRun={invoke} command="arrange.sendToBack" icon={SendToBack} />
        </RibbonStack>
        <RibbonMenu icon={ALIGN[1].icon} label={p('Align')} size="large" disabled={!enabled('object.align')} testId="vector-home-align" primary>
          {ALIGN.map((item) => (
            <MenuItem key={item.label} icon={item.icon} label={p(item.label)} onSelect={() => void run('object.align', item.params)} testId={`vector-home-align-${Object.values(item.params)[0]}`} />
          ))}
        </RibbonMenu>
      </RibbonGroup>

      <RibbonGroup label={p('Pathfinder')} icon={Combine}>
        <CommandButton onRun={invoke} command="object.pathfinder.unite" icon={Combine} primary />
        <CommandButton onRun={invoke} command="object.pathfinder.minusFront" icon={SquaresSubtract} primary />
        <CommandButton onRun={invoke} command="object.pathfinder.intersect" icon={SquaresIntersect} primary />
        <CommandButton onRun={invoke} command="object.pathfinder.exclude" icon={SquaresExclude} primary />
      </RibbonGroup>

      <RibbonGroup label={t('home.paint')} icon={PaintBucket}>
        <RibbonMenu icon={PaintBucket} label={t('fill')} disabled={!enabled('paint.setFill')} testId="vector-home-fill" primary>
          <ColorGrid onPick={(color) => void run('paint.setFill', { color })} testId="vector-home-fill-colors" />
        </RibbonMenu>
        <RibbonMenu icon={PenLine} label={t('stroke')} disabled={!enabled('paint.setStroke')} testId="vector-home-stroke" primary>
          <ColorGrid onPick={(color) => void run('paint.setStroke', { color })} testId="vector-home-stroke-colors" />
        </RibbonMenu>
        <RibbonStack>
          <CommandButton onRun={invoke} command="paint.swap" icon={ArrowLeftRight} />
          <CommandButton onRun={invoke} command="paint.default" icon={RotateCcw} />
        </RibbonStack>
      </RibbonGroup>

      <RibbonGroup label={p('Select')} icon={MousePointer2}>
        <CommandButton onRun={invoke} command="edit.selectAll" icon={SquareDashedMousePointer} size="large" primary />
        <RibbonStack>
          <CommandButton onRun={invoke} command="select.inverse" icon={Contrast} />
          <CommandButton onRun={invoke} command="edit.deselectAll" icon={SquareX} />
        </RibbonStack>
      </RibbonGroup>

      <RibbonGroup label={p('Document')} icon={Layers}>
        <CommandButton onRun={invoke} command="layer.new" icon={LayersPlus} size="large" primary />
        <CommandButton onRun={invoke} command="artboard.new" icon={SquarePlus} size="large" primary />
      </RibbonGroup>
    </>
  )
}
