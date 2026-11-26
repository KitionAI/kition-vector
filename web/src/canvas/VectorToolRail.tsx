import { ToolRail, toolIcon, useKitEditor, useKitTools } from '@kition/sdk/editor'
import { useDialogs } from '../dialogs/DialogHost'
import { railDoubleClick } from './railActions'

/**
 * The engine's tools on the rail (plan 2026-11-25, stage 2), as kit-editor's `EngineToolRail`
 * draws them, but in two columns when one does not fit the window's height (Classic at 1440 × 900,
 * Menus at 1280 × 720 cut the last tools off): every tool stays on screen.
 */
export function VectorToolRail({ hidden, onView }: { hidden: readonly string[]; onView: (to: 'fitArtboard' | 'actualSize') => void }) {
  const tools = useKitTools()
  const { phrase, host } = useKitEditor()
  const dialogs = useDialogs()
  // A read-only document runs no engine tool (the hand stays): there is nothing to choose.
  if (!tools || host.readOnly) return null
  const items = tools.tools.filter((tool) => !hidden.includes(tool.id)).map((tool) => ({ id: tool.id, label: phrase(tool.label), icon: toolIcon(tool.icon, tool.id), shortcut: tool.shortcut, group: tool.group }))
  return (
    <div
      className="vector-rail-tools"
      onDoubleClick={(event) => {
        const button = event.target instanceof Element ? event.target.closest('[data-testid^="vector-tool-"]') : null
        const action = railDoubleClick(button?.getAttribute('data-testid'))
        if (action?.kind === 'dialog') dialogs.open(action.command)
        else if (action?.kind === 'view') onView(action.to)
      }}
    >
      <ToolRail tools={items} active={tools.active} onSelect={tools.select} columns="auto" />
    </div>
  )
}
