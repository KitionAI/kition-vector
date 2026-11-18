import type { KitEngineInfo } from '@kition/kit-editor'

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
}

type InspectedNode = { id?: number; name?: string; kind?: string; visible?: boolean; locked?: boolean; color?: string; fill?: string | null; stroke?: string | null; strokeWidth?: number; children?: InspectedNode[] }

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
  }
}

export const MENU_ORDER = ['File', 'Edit', 'Object', 'Type', 'Select', 'Effect', 'View', 'Window', 'Help']

/** File commands the host's own Save, Open, and Close already cover. */
export const HOST_FILE_COMMANDS = ['file.new', 'file.close', 'file.closeAll', 'file.saveAs', 'file.saveCopy', 'file.revert']

/**
 * File commands that write files the page cannot keep: in the browser they would answer the bytes
 * and drop them. File › Export (the editor's own dialog, through the host) stands for them; agents
 * still run them.
 */
export const FILE_WRITERS = ['document.exportForScreens', 'document.exportSelection', 'document.exportForOffice', 'file.saveAsTemplate']

/** The engine commands each menu tab shows in Single line's row, by menu. */
export const VECTOR_PRIMARY: Record<string, readonly string[]> = {
  File: ['file.place'],
  Edit: ['edit.undo', 'edit.redo', 'edit.cut', 'edit.copy', 'edit.paste', 'edit.pasteInFront', 'edit.clear', 'edit.findReplace'],
  Object: ['object.group', 'object.ungroup', 'object.arrange.bringForward', 'object.arrange.sendBackward', 'object.move', 'object.rotate', 'object.scale', 'object.reflect', 'object.path.outlineStroke', 'object.compoundPath.make', 'object.clippingMask.make', 'object.lock'],
  Type: ['type.createOutlines', 'type.changeCase', 'text.areaOptions'],
  Select: ['select.all', 'select.none', 'select.inverse', 'select.reselect', 'select.nextAbove', 'select.nextBelow'],
  Effect: ['document.rasterEffectsSettings'],
  View: ['view.guides.make', 'view.guides.lock', 'view.transparencyGrid', 'view.proofColors'],
  Window: ['object.pathfinder.unite', 'object.pathfinder.minusFront', 'object.pathfinder.intersect', 'object.pathfinder.exclude', 'layer.new', 'artboard.new'],
}

/**
 * Tools left off the rail: they need panels or dialogs this editor does not have yet (graphs,
 * symbols, perspective grids), or rotate a view the host keeps square (Rotate View, Print Tiling).
 */
export const HIDDEN_TOOLS = [
  'columnGraph', 'stackedColumnGraph', 'barGraph', 'stackedBarGraph', 'lineGraph', 'areaGraph', 'scatterGraph', 'pieGraph', 'radarGraph',
  'symbolSprayer', 'symbolShifter', 'symbolScruncher', 'symbolSizer', 'symbolSpinner', 'symbolStainer', 'symbolScreener', 'symbolStyler',
  'perspectiveGrid', 'perspectiveSelection', 'rotateView', 'printTiling',
]

export type Swatch = { name: string; kind: string; color?: unknown; hex?: string }
