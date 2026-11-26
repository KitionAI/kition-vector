/**
 * The canvas's right-click menu (plan 2026-11-25, stage 2): the rows a right click offers, by what
 * is selected when it opens (nothing, one object, type, a group, several objects) and whether a
 * group is isolated. Pure, so the rows are tested without a browser (`contextRows.test.mjs`);
 * `CanvasMenu.tsx` draws them, each label an upstream phrase, a command the engine refuses greyed.
 * A "…" row opens its dialog, as its menu row does.
 */

export type ContextRow = { kind: 'separator' } | { kind: 'item'; label: string; command: string; params?: Record<string, unknown> }

export type ContextSelection = {
  /** The selected objects' kinds as the inspect names them (`Path`, `Group`, `Text`, `Compound Path`, `Clip Group`…). */
  kinds: readonly string[]
  /** The id of the one selected object, for Isolate Selected Group. */
  only: number | null
  /** A group is isolated now. */
  isolated: boolean
}

const SEP: ContextRow = { kind: 'separator' }
const item = (label: string, command: string, params?: Record<string, unknown>): ContextRow => (params ? { kind: 'item', label, command, params } : { kind: 'item', label, command })

const isGroup = (kind: string) => kind === 'Group' || kind === 'Clip Group' || kind === 'Layer'
const isText = (kind: string) => /type|text/i.test(kind)

/** The rows of the menu for `selection`, separators only between groups of rows. */
export function contextRows(selection: ContextSelection): ContextRow[] {
  const groups: ContextRow[][] = []
  if (selection.isolated) groups.push([item('Exit Isolation Mode', 'object.exitIsolation')])
  groups.push([item('Undo', 'edit.undo'), item('Redo', 'edit.redo')])
  const { kinds } = selection
  if (!kinds.length) {
    groups.push([item('Paste', 'edit.paste'), item('Paste in Place', 'edit.pasteInPlace')])
    groups.push([item('Select All', 'edit.selectAll')])
    return join(groups)
  }
  groups.push([item('Cut', 'edit.cut'), item('Copy', 'edit.copy'), item('Paste', 'edit.paste'), item('Paste in Front', 'edit.pasteInFront')])
  const structure: ContextRow[] = []
  if (kinds.length === 1 && isGroup(kinds[0]) && selection.only !== null) structure.push(item('Isolate Selected Group', 'object.isolate', { id: selection.only }))
  if (kinds.length > 1) structure.push(item('Group', 'arrange.group'))
  if (kinds.some(isGroup)) structure.push(item('Ungroup', 'arrange.ungroup'))
  if (kinds.length > 1) structure.push(item('Make Clipping Mask', 'object.clippingMask.make'))
  if (kinds.includes('Clip Group')) structure.push(item('Release Clipping Mask', 'object.clippingMask.release'))
  if (kinds.length > 1 && !kinds.some(isText)) structure.push(item('Make Compound Path', 'object.compoundPath.make'))
  if (kinds.includes('Compound Path')) structure.push(item('Release Compound Path', 'object.compoundPath.release'))
  if (kinds.length === 1 && isText(kinds[0])) structure.push(item('Create Outlines', 'text.convertToOutlines'))
  if (structure.length) groups.push(structure)
  groups.push([item('Move…', 'object.move'), item('Rotate…', 'object.rotate'), item('Reflect…', 'object.reflect'), item('Scale…', 'object.scale'), item('Shear…', 'object.shear')])
  groups.push([item('Bring to Front', 'arrange.bringToFront'), item('Bring Forward', 'arrange.bringForward'), item('Send Backward', 'arrange.sendBackward'), item('Send to Back', 'arrange.sendToBack')])
  groups.push([item('Deselect', 'edit.deselectAll')])
  return join(groups)
}

function join(groups: ContextRow[][]): ContextRow[] {
  return groups.filter((group) => group.length).flatMap((group, index) => (index ? [SEP, ...group] : group))
}
