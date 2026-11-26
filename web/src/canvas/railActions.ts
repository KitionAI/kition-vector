// Pure, so `node --test` checks it (canvas/canvas.test.mjs).

/**
 * The tools whose dialog opens on a double-click of their rail button or on Enter while they are
 * chosen, as the reference's do (re-walk 2026-11-26): the Selection tools' Move…, and Rotate…,
 * Scale…, Reflect…, Shear… on the selection, about its centre.
 */
export const TOOL_DIALOGS: Readonly<Record<string, string>> = {
  selection: 'object.move',
  directSelection: 'object.move',
  groupSelection: 'object.move',
  rotate: 'object.rotate',
  scale: 'object.scale',
  reflect: 'object.reflect',
  shear: 'object.shear',
}

/** What a double-click on a rail button does: a tool's dialog, or a view (Hand fits the artboard, Zoom shows 100%). */
export type RailDoubleClick = { kind: 'dialog'; command: string } | { kind: 'view'; to: 'fitArtboard' | 'actualSize' }

/** What a double-click on rail button `testId` (`vector-tool-<id>`) does, or null. */
export function railDoubleClick(testId: string | null | undefined): RailDoubleClick | null {
  const id = testId?.startsWith('vector-tool-') ? testId.slice('vector-tool-'.length) : null
  if (!id) return null
  if (id === 'hand') return { kind: 'view', to: 'fitArtboard' }
  if (id === 'zoom') return { kind: 'view', to: 'actualSize' }
  const command = TOOL_DIALOGS[id]
  return command ? { kind: 'dialog', command } : null
}
