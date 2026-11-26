// Imports carry `.ts` so `node --test` loads this module as it is.
import type { ParamField } from '../dialogs/params.ts'

/**
 * The dialogs a tool asks for (plan 2026-11-25, stage 2): the engine's tools send a request by name
 * (a shape tool clicked on the canvas, a transform tool Alt-clicked, a gradient stop or an artboard
 * double-clicked…); the editor opens the dialog that runs the matching command. Pure, so the
 * mapping is tested without a browser (`toolDialogs.test.mjs`).
 *
 * Two kinds: `open` hands a command with a dialog of its own (stage 1's transform dialogs, a "…"
 * command's generic dialog) the values the tool gave; `ask` is a dialog made here from `fields`,
 * whose values `build` turns into the command's parameters with what the tool gave (`passed`).
 */
export type ToolDialog =
  | { kind: 'open'; command: string; values: Record<string, unknown> }
  | {
      kind: 'ask'
      command: string
      /** The dialog's title, an upstream phrase. */
      title: string
      fields: ParamField[]
      /** Starting values over the fields' own (the tool's options, the object's current values). */
      start: Record<string, unknown>
      /** The parameters made from the fields' values. */
      build: (values: Record<string, unknown>) => Record<string, unknown>
      /** Whether the canvas previews the command while the dialog is open. */
      previewing: boolean
    }

/** What a request reads from the drawing to start from: the width points and paint of an object. */
export type ToolDialogContext = {
  /** The width point `index` of object `id`: where along the path it sits (0..1) and its two sides, in points. */
  widthPoint?: (id: number, index: number) => { t: number; left: number; right: number } | null
  /** The colour of stop `index` of the gradient being edited, as `#rrggbb`. */
  stopColor?: (index: number) => string | null
}

const number = (key: string, initial: number | null, min = 0, max = 100_000, decimals = 2): ParamField => ({ key, kind: 'number', min, max, decimals, initial })
const count = (key: string, initial: number, min = 1, max = 1000): ParamField => ({ key, kind: 'number', min, max, decimals: 0, initial })
const check = (key: string, initial: boolean): ParamField => ({ key, kind: 'check', initial })

const finite = (value: unknown): value is number => typeof value === 'number' && Number.isFinite(value)
/** The click a request carries, or null. */
function clickOf(params: unknown): { x: number; y: number } | null {
  const record = (params ?? {}) as Record<string, unknown>
  return finite(record.x) && finite(record.y) ? { x: record.x, y: record.y } : null
}
const numberOf = (value: unknown, fallback: number) => (finite(value) ? value : fallback)

/** The transform tools' requests: their dialog opens about the point the tool chose, its other values its own. */
const TRANSFORMS: Readonly<Record<string, string>> = { rotate: 'object.rotate', scale: 'object.scale', reflect: 'object.reflect', shear: 'object.shear' }

/** The dialogs the engine's tools ask for, by request name: what `toolDialog` answers. */
export const TOOL_DIALOG_NAMES: readonly string[] = [
  'rectangle',
  'roundedRectangle',
  'ellipse',
  'polygon',
  'star',
  'lineSegment',
  'arc',
  'spiral',
  'rectangularGrid',
  'polarGrid',
  'flare',
  'rotate',
  'scale',
  'reflect',
  'shear',
  'gradientStop',
  'artboardOptions',
  'blendOptions',
  'widthPoint',
  'corners',
  'sliceOptions',
]

/**
 * The dialog the tool's request `name` (with its `params`) opens, or null for a request this
 * editor has no dialog for (the graph and perspective tools, which are not on its rail).
 */
export function toolDialog(name: string, params: unknown, context: ToolDialogContext = {}): ToolDialog | null {
  const record = (params ?? {}) as Record<string, unknown>
  const click = clickOf(params)
  const at = (extra: Record<string, unknown>) => (values: Record<string, unknown>) => ({ ...extra, ...values })
  const shape = (command: string, title: string, fields: ParamField[], build: Build): ToolDialog | null =>
    click ? { kind: 'ask', command, title, fields, start: {}, build, previewing: false } : null
  const transform = TRANSFORMS[name]
  if (transform) return { kind: 'open', command: transform, values: Array.isArray(record.origin) ? { origin: record.origin } : {} }
  switch (name) {
    case 'rectangle':
      return shape('shape.rectangle', 'Rectangle', [number('width', 100), number('height', 100)], at({ x: click?.x, y: click?.y }))
    case 'roundedRectangle':
      return shape('shape.rectangle', 'Rounded Rectangle', [number('width', 100), number('height', 100), number('radius', 12)], at({ x: click?.x, y: click?.y }))
    case 'ellipse':
      return shape('shape.ellipse', 'Ellipse', [number('width', 100), number('height', 100)], at({ x: click?.x, y: click?.y }))
    case 'polygon':
      return shape('shape.polygon', 'Polygon', [number('radius', 50), count('sides', 6, 3)], at({ cx: click?.x, cy: click?.y }))
    case 'star':
      return shape('shape.star', 'Star', [number('radius1', 50), number('radius2', 25), count('points', 5, 2)], at({ cx: click?.x, cy: click?.y }))
    case 'spiral':
      return shape('shape.spiral', 'Spiral', [number('radius', 50), number('decay', 80, 5, 150, 0), count('segments', 10, 2), check('clockwise', true)], at({ cx: click?.x, cy: click?.y }))
    case 'rectangularGrid':
      return shape('shape.rectangularGrid', 'Rectangular Grid', [number('width', 100), number('height', 100), count('rows', 5, 0), count('columns', 5, 0)], at({ x: click?.x, y: click?.y }))
    case 'polarGrid':
      return shape('shape.polarGrid', 'Polar Grid', [number('width', 100), number('height', 100), count('concentric', 5, 0), count('radial', 5, 0)], at({ x: click?.x, y: click?.y }))
    case 'lineSegment':
      // Length and angle (counter-clockwise from the right, as the reference measures it; y grows down).
      return shape('shape.line', 'Line Segment Tool Options', [number('length', 100), number('angle', 0, -360, 360)], (values) => {
        const length = numberOf(values.length, 100)
        const radians = (numberOf(values.angle, 0) * Math.PI) / 180
        const x = click?.x ?? 0
        const y = click?.y ?? 0
        return { x1: x, y1: y, x2: round(x + length * Math.cos(radians)), y2: round(y - length * Math.sin(radians)) }
      })
    case 'arc':
      return shape('shape.arc', 'Arc Segment Tool Options', [number('lengthX', 100), number('lengthY', 100), check('closed', false)], (values) => {
        const x = click?.x ?? 0
        const y = click?.y ?? 0
        return { x1: x, y1: y, x2: x + numberOf(values.lengthX, 100), y2: y + numberOf(values.lengthY, 100), closed: values.closed === true }
      })
    case 'flare':
      return shape(
        'shape.flare',
        'Flare Tool Options',
        [number('diameter', 100), number('opacity', 50, 0, 100, 0), number('brightness', 30, 0, 100, 0), count('rays', 15, 0, 1000), number('longest', 300, 0, 1000, 0), count('rings', 10, 0, 50), number('largest', 50, 0, 250, 0), number('direction', 45, -360, 360, 0), number('pathLength', 300)],
        at({ cx: click?.x, cy: click?.y }),
      )
    case 'gradientStop': {
      // The stop's colour, recoloured where it is ("Sample Color" with `stop`), previewed as it changes.
      const index = numberOf(record.index, 0)
      return { kind: 'ask', command: 'paint.sampleColor', title: 'Gradient Stop', fields: [{ key: 'color', kind: 'color', initial: '#000000' }], start: { color: context.stopColor?.(index) ?? '#000000' }, build: (values) => ({ color: values.color, stop: index }), previewing: true }
    }
    case 'artboardOptions': {
      const index = numberOf(record.index, 0)
      const start = Object.fromEntries(['name', 'x', 'y', 'width', 'height'].filter((key) => key in record).map((key) => [key, record[key]]))
      return {
        kind: 'ask',
        command: 'artboard.setProps',
        title: 'Artboard Options',
        fields: [{ key: 'name', kind: 'text', initial: '' }, number('x', 0, -100_000), number('y', 0, -100_000), number('width', 100, 1), number('height', 100, 1)],
        start,
        build: (values) => ({ index, ...values }),
        previewing: true,
      }
    }
    case 'widthPoint': {
      const id = numberOf(record.id, -1)
      const index = numberOf(record.index, 0)
      const point = context.widthPoint?.(id, index) ?? null
      if (!point) return null
      return {
        kind: 'ask',
        command: 'stroke.widthPoint.set',
        title: 'Width Point Edit',
        fields: [number('left', point.left), number('right', point.right)],
        start: { left: point.left, right: point.right },
        build: (values) => ({ id, index, t: point.t, left: numberOf(values.left, point.left), right: numberOf(values.right, point.right) }),
        previewing: true,
      }
    }
    case 'corners': {
      const passed = { id: record.id, ...(Array.isArray(record.corners) ? { corners: record.corners } : {}) }
      return {
        kind: 'ask',
        command: 'object.setLiveShape',
        title: 'Corners',
        fields: [{ key: 'kind', kind: 'choice', options: ['round', 'invertedRound', 'chamfer'], initial: 'round' }, number('radius', 10)],
        start: {},
        build: (values) => ({ ...passed, ...values }),
        previewing: true,
      }
    }
    case 'blendOptions':
      return { kind: 'open', command: 'object.blend.options', values: {} }
    case 'sliceOptions':
      return { kind: 'open', command: 'object.slice.options', values: {} }
    default:
      return null
  }
}

const round = (value: number) => Math.round(value * 1e6) / 1e6

type Build = (values: Record<string, unknown>) => Record<string, unknown>
