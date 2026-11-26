import type { KitEngineInfo } from '@kition/sdk/editor'

/** One row of the Layers panel: a layer or an object in it. */
export type VectorNode = {
  id: number
  name: string
  kind: string
  visible: boolean
  locked: boolean
  color: string | null
  fill: string | null
  stroke: string | null
  strokeWidth: number | null
  /** The stroke's width points, `[t, left, right]` (the sides as factors of half the weight), or null for a uniform stroke. */
  widthPoints: number[][] | null
  children: VectorNode[]
}

export type Bounds = { x: number; y: number; width: number; height: number }

/** What the editor reads from the engine's `inspect()`. */
export type VectorInfo = {
  layers: VectorNode[]
  selection: number[]
  selectionBounds: Bounds | null
  artboards: Array<Bounds & { name: string }>
  /** Top-level objects in the layers (a group counts once). */
  objects: number
  paint: { fill: string | null; stroke: string | null; strokeWidth: number; fillActive: boolean }
  units: string
  /** The drawing's point at document pixel (0, 0): the extent's top-left (the pasteboard grows it). */
  origin: readonly [number, number]
  /** The isolated group or layer, or null. */
  isolation: number | null
  /** The keys the active tool takes now, ahead of their shortcuts (Delete on a selected gradient stop). */
  toolKeys: readonly string[]
}

type InspectedNode = { id?: number; name?: string; kind?: string; visible?: boolean; locked?: boolean; color?: string; fill?: string | null; stroke?: string | null; strokeWidth?: number; strokeOptions?: { widthPoints?: number[][] | null }; children?: InspectedNode[] }

function nodeOf(node: InspectedNode): VectorNode {
  return {
    id: node.id ?? -1,
    name: node.name ?? '',
    kind: node.kind ?? '',
    visible: node.visible !== false,
    locked: node.locked === true,
    color: typeof node.color === 'string' ? node.color : null,
    fill: typeof node.fill === 'string' ? node.fill : null,
    stroke: typeof node.stroke === 'string' ? node.stroke : null,
    strokeWidth: typeof node.strokeWidth === 'number' ? node.strokeWidth : null,
    widthPoints: Array.isArray(node.strokeOptions?.widthPoints) ? node.strokeOptions.widthPoints : null,
    children: (node.children ?? []).map(nodeOf),
  }
}

const count = (nodes: readonly VectorNode[]): number => nodes.reduce((total, node) => total + (node.kind === 'Layer' ? count(node.children) : 1), 0)

export function describe(info: KitEngineInfo): VectorInfo {
  const inspected = info.inspected as {
    layers?: InspectedNode[]
    selection?: number[]
    selectionBounds?: Bounds | null
    artboards?: Array<Bounds & { name: string }>
    paint?: Partial<VectorInfo['paint']>
    units?: string
    origin?: [number, number]
    isolation?: number | null
    toolKeys?: string[]
  }
  const layers = (inspected.layers ?? []).map(nodeOf)
  const selection = inspected.selection ?? []
  // The paint shown: the first selected object's own, else what new objects get.
  const find = (nodes: readonly VectorNode[]): VectorNode | null => {
    for (const node of nodes) {
      if (selection.includes(node.id) && node.kind !== 'Layer') return node
      const inside = find(node.children)
      if (inside) return inside
    }
    return null
  }
  const selected = find(layers)
  const paint = { fill: inspected.paint?.fill ?? null, stroke: inspected.paint?.stroke ?? null, strokeWidth: inspected.paint?.strokeWidth ?? 1, fillActive: inspected.paint?.fillActive !== false }
  return {
    layers,
    selection,
    selectionBounds: inspected.selectionBounds ?? null,
    artboards: inspected.artboards ?? [],
    objects: count(layers),
    paint: selected ? { ...paint, fill: selected.fill, stroke: selected.stroke, strokeWidth: selected.strokeWidth ?? paint.strokeWidth } : paint,
    units: inspected.units ?? 'Points',
    origin: Array.isArray(inspected.origin) && inspected.origin.length === 2 ? [Number(inspected.origin[0]) || 0, Number(inspected.origin[1]) || 0] : [0, 0],
    isolation: typeof inspected.isolation === 'number' ? inspected.isolation : null,
    toolKeys: Array.isArray(inspected.toolKeys) ? inspected.toolKeys.filter((key): key is string => typeof key === 'string') : [],
  }
}

/** The node `id` and the names from its layer down to it, or null. */
export function nodePath(layers: readonly VectorNode[], id: number): { node: VectorNode; names: string[] } | null {
  for (const node of layers) {
    if (node.id === id) return { node, names: [node.name] }
    const inside = nodePath(node.children, id)
    if (inside) return { node: inside.node, names: [node.name, ...inside.names] }
  }
  return null
}

export const MENU_ORDER = ['File', 'Edit', 'Object', 'Type', 'Select', 'Effect', 'View', 'Window', 'Help']

/** The engine commands each menu tab shows in Single line's row, by menu. */
export const VECTOR_PRIMARY: Record<string, readonly string[]> = {
  // File's own rows (New…, Export…) are Kition's, marked primary where App adds them.
  File: [],
  Edit: ['edit.undo', 'edit.redo', 'edit.cut', 'edit.copy', 'edit.paste', 'edit.pasteInFront', 'edit.delete', 'edit.findReplace'],
  Object: ['arrange.group', 'arrange.ungroup', 'arrange.bringForward', 'arrange.sendBackward', 'object.move', 'object.rotate', 'object.scale', 'object.reflect', 'object.path.outlineStroke', 'object.compoundPath.make', 'object.clippingMask.make', 'object.lock'],
  Type: ['text.convertToOutlines', 'text.changeCase', 'text.areaOptions'],
  Select: ['edit.selectAll', 'edit.deselectAll', 'select.inverse', 'select.reselect', 'select.nextAbove', 'select.nextBelow'],
  Effect: ['kit:effect.stylize.dropShadow', 'kit:effect.stylize.roundCorners', 'kit:effect.blur.gaussian'],
  View: ['view.guides.make', 'view.guides.lock', 'view.transparencyGrid', 'view.proofColors'],
  Window: ['object.pathfinder.unite', 'object.pathfinder.minusFront', 'object.pathfinder.intersect', 'object.pathfinder.exclude'],
}

/**
 * Tools left off the rail: they need panels or dialogs this editor does not have yet (graphs,
 * symbols, perspective grids), or rotate a view the host keeps square (Rotate View, Print Tiling).
 */
export const HIDDEN_TOOLS = [
  'columnGraph', 'stackedColumnGraph', 'barGraph', 'stackedBarGraph', 'lineGraph', 'areaGraph', 'scatterGraph', 'pieGraph', 'radarGraph',
  'symbolSprayer', 'symbolShifter', 'symbolScruncher', 'symbolSizer', 'symbolSpinner', 'symbolStainer', 'symbolScreener', 'symbolStyler',
  'perspectiveGrid', 'perspectiveSelection', 'rotateView', 'printTiling',
  // Touch Type has no engine tool yet: on the rail it would do nothing (plan 2026-11-25, stage 2).
  'touchType',
]

export type Swatch = { name: string; kind: string; color?: unknown; hex?: string }
