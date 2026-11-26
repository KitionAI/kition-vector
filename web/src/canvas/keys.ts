/**
 * The canvas's own keys (plan 2026-11-25, stage 2), decided before the commands' shortcuts: what a
 * key the active tool did not take as text or mid-gesture does. Pure, so the rules are tested
 * without a browser (`keys.test.mjs`).
 *
 * - A key the active tool takes now (`toolKeys` in the engine's inspect: Delete on a selected
 *   gradient stop, width point, pin or artboard; Enter and Backspace while the Pen draws) goes to it.
 * - The arrows nudge the selection (its selected anchors, else its objects) by the keyboard
 *   increment; Shift ten times as far, Alt a copy.
 * - The single-key commands the shortcuts the frame binds leave out: X (fill or stroke in front),
 *   D (default colours), / (none), "," and "." (last colour, last gradient).
 * - Cmd+1 shows the drawing at actual size, Cmd+0 fits the artboards, Cmd+Alt+0 fits everything.
 * - Cmd+Enter ends the text being typed (as Escape does); Cmd with an arrow, Home, End, Backspace
 *   or Delete moves or deletes by words, lines and paragraphs in it (kit-editor gives the Type tool
 *   only the keys without Cmd).
 * - "+" picks the Add Anchor Point tool (Shift is part of the key on most layouts).
 */
export type KeyLike = { key: string; code: string; metaKey: boolean; ctrlKey: boolean; altKey: boolean; shiftKey: boolean }

export type CanvasKey =
  | { kind: 'tool'; key: string }
  | { kind: 'nudge'; dx: -1 | 0 | 1; dy: -1 | 0 | 1; big: boolean; copy: boolean }
  | { kind: 'command'; id: string }
  | { kind: 'zoom'; to: 'actual' | 'artboards' | 'all' }
  | { kind: 'endText' }
  | { kind: 'textKey'; key: string }
  | { kind: 'pickTool'; id: string }

export type KeyContext = {
  /** The keys the active tool takes now (the engine's `toolKeys`). */
  toolKeys: readonly string[]
  /** Text is being typed on the canvas. */
  editingText: boolean
  /** Something is selected (objects, anchors, guides) for the arrows to move. */
  selection: boolean
  /** The engine's commands bound to one key without modifiers (`X` → `paint.toggleActive`). */
  singleKeys: ReadonlyMap<string, string>
  /** Whether the key is a tool's own letter (V, P…), which keeps picking its tool. */
  toolLetter: (key: string, shift: boolean) => boolean
}

/** The keys that move or delete by words, lines and paragraphs with Cmd held in the text being typed. */
const TEXT_KEYS: ReadonlySet<string> = new Set(['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown', 'Home', 'End', 'Backspace', 'Delete'])

const ARROWS: Readonly<Record<string, readonly [-1 | 0 | 1, -1 | 0 | 1]>> = { ArrowLeft: [-1, 0], ArrowRight: [1, 0], ArrowUp: [0, -1], ArrowDown: [0, 1] }

/** What `event` does on the canvas, or null when the editor's shortcuts are to have it. */
export function canvasKey(event: KeyLike, context: KeyContext): CanvasKey | null {
  const command = event.metaKey || event.ctrlKey
  if (command) {
    if (context.editingText && event.key === 'Enter' && !event.altKey) return { kind: 'endText' }
    if (context.editingText && TEXT_KEYS.has(event.key)) return { kind: 'textKey', key: event.key }
    if (event.code === 'Digit1' && !event.altKey && !event.shiftKey) return { kind: 'zoom', to: 'actual' }
    if (event.code === 'Digit0' && !event.shiftKey) return { kind: 'zoom', to: event.altKey ? 'all' : 'artboards' }
    return null
  }
  if (context.editingText) return null
  if (context.toolKeys.includes(event.key) && !event.altKey) return { kind: 'tool', key: event.key }
  const arrow = ARROWS[event.key]
  if (arrow) return context.selection ? { kind: 'nudge', dx: arrow[0], dy: arrow[1], big: event.shiftKey, copy: event.altKey } : null
  if (event.key === '+' && !event.altKey) return { kind: 'pickTool', id: 'addAnchor' }
  if (event.key.length === 1 && !event.altKey && !event.shiftKey && !context.toolLetter(event.key, false)) {
    const id = context.singleKeys.get(event.key.toUpperCase()) ?? context.singleKeys.get(event.key)
    if (id) return { kind: 'command', id }
  }
  return null
}

/** The commands bound to a single key without modifiers, by key: what `KeyContext.singleKeys` holds. */
export function singleKeyCommands(commands: readonly { id: string; shortcut?: string | null }[]): Map<string, string> {
  const keys = new Map<string, string>()
  for (const command of commands) {
    const shortcut = command.shortcut
    if (shortcut && shortcut.length === 1 && !keys.has(shortcut)) keys.set(shortcut, command.id)
  }
  return keys
}
