import type { AskedDialog } from '../dialogs/DialogHost'
import type { ParamField } from '../dialogs/params'
import type { VectorNode } from '../vector'

/** `#rrggbb` of a layer colour the inspect gives (`#rrggbb` already, or anything else as black). */
const hexOf = (color: string | null) => (color && /^#[0-9a-f]{6}$/i.test(color) ? color.toLowerCase() : '#4f80ff')

/**
 * Layer Options… for a layer (name, colour, template, show, preview, lock, print, dim images to),
 * or the options a group or object has (name, show, lock), as one `layer.setProps` step on OK
 * (plan 2026-11-25, stage 4). Opened from the Layers panel's menu and by double-clicking a layer's
 * colour.
 */
export function layerOptionsDialog(node: VectorNode, title: string, memory: string): AskedDialog {
  const layer = node.kind === 'Layer'
  const fields: ParamField[] = layer
    ? [
        { key: 'name', kind: 'text', initial: node.name },
        { key: 'color', kind: 'color', initial: hexOf(node.color) },
        { key: 'template', kind: 'check', initial: node.template },
        { key: 'visible', kind: 'check', initial: node.visible },
        { key: 'preview', kind: 'check', initial: node.preview },
        { key: 'locked', kind: 'check', initial: node.locked },
        { key: 'printable', kind: 'check', initial: node.printable },
        { key: 'dimImages', kind: 'number', min: 0, max: 100, decimals: 0, initial: node.dimImages },
      ]
    : [
        { key: 'name', kind: 'text', initial: node.name },
        { key: 'visible', kind: 'check', initial: node.visible },
        { key: 'locked', kind: 'check', initial: node.locked },
      ]
  const start = layer
    ? { name: node.name, color: hexOf(node.color), template: node.template, visible: node.visible, preview: node.preview, locked: node.locked, printable: node.printable, dimImages: node.dimImages }
    : { name: node.name, visible: node.visible, locked: node.locked }
  return {
    command: 'layer.setProps',
    title,
    memory: `${memory}:${layer ? 'layer' : 'object'}:${node.id}`,
    fields,
    start,
    build: (values) => {
      const params: Record<string, unknown> = { id: node.id }
      for (const [key, value] of Object.entries(values)) {
        if (value === null || value === undefined) continue
        if (key === 'name' && (typeof value !== 'string' || !value.trim())) continue
        params[key] = key === 'name' ? (value as string).trim() : value
      }
      // A layer whose Dim Images box is left empty stops dimming.
      if (layer && values.dimImages == null && node.dimImages !== null) params.dimImages = false
      return params
    },
    previewing: false,
  }
}
