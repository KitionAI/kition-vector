/**
 * The Layers panel's rows (plan 2026-11-25, stage 4): the drawing's tree (layer → sublayer → group
 * → object) as the rows the panel draws, which ones are open, where a dragged row lands, and which
 * rows hold selected art. Pure, so `node --test` checks it.
 */

/** A node as the panel reads it (`VectorNode`'s fields it needs). */
export type TreeNode = { id: number; name: string; kind: string; visible: boolean; locked: boolean; color: string | null; children: readonly TreeNode[] }

/** One row the panel draws. */
export type Row = {
  node: TreeNode
  depth: number
  /** The row's layer (the nearest layer up from it, itself for a layer). */
  layer: number
  /** Its layer's colour, which its selection marks take. */
  color: string | null
  /** The row above it in the tree, or null at the top level. */
  parent: number | null
  /** Whether it has rows inside it, and whether they are shown. */
  expandable: boolean
  expanded: boolean
  /** Hidden or locked by a row above it (its own eye and lock are drawn dimmed). */
  hiddenAbove: boolean
  lockedAbove: boolean
}

/** A layer with more rows than this opens closed: a drawing of hundreds of objects stays quick to show. */
export const BIG_LAYER = 200

/** Whether `node` takes rows dropped into it: a layer or a group (the engine refuses the rest). */
export function isContainer(node: Pick<TreeNode, 'kind'>) {
  return node.kind === 'Layer' || /group/i.test(node.kind)
}

/** Whether `node` is open when the person has not opened or closed it: a layer, unless it is big. */
export function openByDefault(node: TreeNode) {
  return node.kind === 'Layer' && node.children.length <= BIG_LAYER
}

/** The rows shown: each node, and the rows inside the open ones (`open` overrides the default per id). */
export function visibleRows(layers: readonly TreeNode[], open: ReadonlyMap<number, boolean>): Row[] {
  const rows: Row[] = []
  const walk = (nodes: readonly TreeNode[], depth: number, parent: number | null, layer: number | null, color: string | null, hidden: boolean, locked: boolean) => {
    for (const node of nodes) {
      const isLayer = node.kind === 'Layer'
      const ownLayer = isLayer ? node.id : (layer ?? node.id)
      const ownColor = isLayer ? node.color : color
      const expandable = node.children.length > 0
      const expanded = expandable && (open.get(node.id) ?? openByDefault(node))
      rows.push({ node, depth, layer: ownLayer, color: ownColor, parent, expandable, expanded, hiddenAbove: hidden, lockedAbove: locked })
      if (expanded) walk(node.children, depth + 1, node.id, ownLayer, ownColor, hidden || !node.visible, locked || node.locked)
    }
  }
  walk(layers, 0, null, null, null, false, false)
  return rows
}

/** The ids of the rows above `id` in the tree, outermost first, or null when it is not there. */
export function ancestorsOf(layers: readonly TreeNode[], id: number): number[] | null {
  for (const node of layers) {
    if (node.id === id) return []
    const inside = ancestorsOf(node.children, id)
    if (inside) return [node.id, ...inside]
  }
  return null
}

/**
 * How each row holds the selection: `self` when it is selected, `inside` when it holds selected
 * art (a layer or group whose objects are), the reference's big and small squares.
 */
export function selectionMarks(layers: readonly TreeNode[], selection: readonly number[]): Map<number, 'self' | 'inside'> {
  const chosen = new Set(selection)
  const marks = new Map<number, 'self' | 'inside'>()
  const walk = (node: TreeNode): boolean => {
    let inside = false
    for (const child of node.children) inside = walk(child) || inside
    if (chosen.has(node.id) && node.kind !== 'Layer') marks.set(node.id, 'self')
    else if (inside) marks.set(node.id, 'inside')
    return inside || chosen.has(node.id)
  }
  for (const layer of layers) walk(layer)
  return marks
}

/** Where a row dropped at `fraction` of its height (0 top, 1 bottom) lands: above it, inside it (a layer or group), or below it. */
export function dropPlace(node: Pick<TreeNode, 'kind'>, fraction: number): 'above' | 'inside' | 'below' {
  if (isContainer(node)) return fraction < 0.25 ? 'above' : fraction > 0.75 ? 'below' : 'inside'
  return fraction < 0.5 ? 'above' : 'below'
}

/**
 * Whether rows `moving` may land at `target`: never on or inside one of themselves (the engine
 * refuses a row into itself), and only beside or into a row that takes them.
 */
export function canDrop(layers: readonly TreeNode[], moving: readonly number[], target: number, place: 'above' | 'inside' | 'below') {
  if (moving.includes(target)) return false
  const above = ancestorsOf(layers, target) ?? []
  if (above.some((id) => moving.includes(id))) return false
  if (place !== 'inside') return true
  const node = findNode(layers, target)
  return node !== null && isContainer(node) && !node.locked
}

/** The node `id`, or null. */
export function findNode(layers: readonly TreeNode[], id: number): TreeNode | null {
  for (const node of layers) {
    if (node.id === id) return node
    const inside = findNode(node.children, id)
    if (inside) return inside
  }
  return null
}

/** The ids of the rows from `from` to `to` in the rows shown (Shift-click's range), in order; `[to]` when `from` is not shown. */
export function rowRange(rows: readonly Row[], from: number | null, to: number): number[] {
  const a = from === null ? -1 : rows.findIndex((row) => row.node.id === from)
  const b = rows.findIndex((row) => row.node.id === to)
  if (b < 0) return []
  if (a < 0) return [to]
  const [start, end] = a <= b ? [a, b] : [b, a]
  return rows.slice(start, end + 1).map((row) => row.node.id)
}

/** The objects among `ids` (not layers), each once: what a click in the panel selects on the canvas. */
export function objectsAmong(layers: readonly TreeNode[], ids: readonly number[]): number[] {
  return [...new Set(ids)].filter((id) => {
    const node = findNode(layers, id)
    return node !== null && node.kind !== 'Layer'
  })
}

/** The window of rows a list `height` tall scrolled `top` down draws, `rowHeight` each, with `spare` rows around it. */
export function rowWindow(count: number, top: number, height: number, rowHeight: number, spare = 8): { start: number; end: number } {
  const start = Math.max(0, Math.floor(top / rowHeight) - spare)
  const end = Math.min(count, Math.ceil((top + Math.max(height, rowHeight)) / rowHeight) + spare)
  return { start, end: Math.max(start, end) }
}
