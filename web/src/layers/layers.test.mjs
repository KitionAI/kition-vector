// The layers, artboards, dock and history rules (plan 2026-11-25, stage 4, feature
// vector.editor.layers), and in the built web engine: the inspect reports the current layer and the
// highlighted rows, a row moved is one step, and the menu rows that used to fail for the state
// they were clicked in are greyed out instead. The engine part is skipped until
// `pnpm --dir web engine` has built it. Run with `pnpm --dir web test:unit` (node --test).
import { existsSync, readFileSync } from 'node:fs'
import test from 'node:test'
import assert from 'node:assert/strict'
import { ancestorsOf, BIG_LAYER, canDrop, dropPlace, objectsAmong, rowRange, rowWindow, selectionMarks, visibleRows } from './tree.ts'
import { DEFAULT_SHOWN, DOCK_GROUPS, DOCK_TABS, TAB_LABELS, WINDOW_TABS, shownOf, windowCommand, windowRows, windowShown } from '../panels/dockModel.ts'
import { historyMove, historyRows } from '../panels/history.ts'
import { historyHeld, holdHistory } from '../panels/historyHold.ts'
import { reorderTarget, stepArtboard } from '../artboards/order.ts'
import { CATALOGS } from '../strings.ts'
import { isHiddenRow } from '../menus/hidden.ts'
import { paramFields } from '../dialogs/params.ts'
import { dialogFor } from '../dialogs/routes.ts'

const node = (id, kind, children = [], extra = {}) => ({ id, name: `#${id}`, kind, visible: true, locked: false, color: kind === 'Layer' ? '#4f80ff' : null, children, ...extra })
const drawing = () => [node(1, 'Layer', [node(2, 'Group', [node(3, 'Path'), node(4, 'Ellipse')]), node(5, 'Path')]), node(6, 'Layer', [node(7, 'Layer', [node(8, 'Path')], { visible: false })])]

test('the rows: layers open, groups closed, the person\'s choice wins, a hidden layer hides what it holds', () => {
  const rows = visibleRows(drawing(), new Map())
  assert.deepEqual(rows.map((row) => row.node.id), [1, 2, 5, 6, 7, 8])
  assert.equal(rows.find((row) => row.node.id === 2).expanded, false)
  assert.equal(rows.find((row) => row.node.id === 8).hiddenAbove, true)
  assert.equal(rows.find((row) => row.node.id === 8).depth, 2)
  assert.equal(rows.find((row) => row.node.id === 5).color, '#4f80ff', 'an object takes its layer\'s colour')
  const opened = visibleRows(drawing(), new Map([[2, true], [6, false]]))
  assert.deepEqual(opened.map((row) => row.node.id), [1, 2, 3, 4, 5, 6])
  assert.equal(opened.find((row) => row.node.id === 3).parent, 2)
})

test('a layer of hundreds of objects opens closed; the list draws only the rows in view', () => {
  const big = [node(1, 'Layer', Array.from({ length: BIG_LAYER + 1 }, (_, index) => node(100 + index, 'Path')))]
  assert.equal(visibleRows(big, new Map()).length, 1)
  assert.equal(visibleRows(big, new Map([[1, true]])).length, BIG_LAYER + 2)
  const window = rowWindow(1000, 2600, 260, 26)
  assert.ok(window.start >= 90 && window.end <= 120 && window.end > window.start, JSON.stringify(window))
  assert.deepEqual(rowWindow(3, 0, 400, 26), { start: 0, end: 3 })
})

test('where a dragged row lands: above, inside a layer or group, below; never into itself', () => {
  assert.equal(dropPlace({ kind: 'Layer' }, 0.1), 'above')
  assert.equal(dropPlace({ kind: 'Layer' }, 0.5), 'inside')
  assert.equal(dropPlace({ kind: 'Group' }, 0.9), 'below')
  assert.equal(dropPlace({ kind: 'Path' }, 0.4), 'above')
  assert.equal(dropPlace({ kind: 'Path' }, 0.6), 'below')
  const layers = drawing()
  assert.equal(canDrop(layers, [3], 5, 'above'), true)
  assert.equal(canDrop(layers, [2], 3, 'above'), false, 'a group into itself')
  assert.equal(canDrop(layers, [2], 2, 'inside'), false)
  assert.equal(canDrop(layers, [5], 3, 'inside'), false, 'a path takes nothing')
  assert.equal(canDrop(layers, [5], 6, 'inside'), true)
  assert.equal(canDrop([node(1, 'Layer', [], { locked: true }), node(2, 'Layer', [node(3, 'Path')])], [3], 1, 'inside'), false, 'a locked layer takes nothing')
  assert.deepEqual(ancestorsOf(layers, 4), [1, 2])
})

test('selection marks, Shift ranges and the objects a click selects', () => {
  const layers = drawing()
  const marks = selectionMarks(layers, [3])
  assert.equal(marks.get(3), 'self')
  assert.equal(marks.get(2), 'inside')
  assert.equal(marks.get(1), 'inside')
  assert.equal(marks.has(6), false)
  const rows = visibleRows(layers, new Map())
  assert.deepEqual(rowRange(rows, 5, 1), [1, 2, 5])
  assert.deepEqual(rowRange(rows, null, 5), [5])
  assert.deepEqual(objectsAmong(layers, [1, 2, 5, 5]), [2, 5])
})

test('the dock: every tab in one group with an upstream label; Window rows show and hide them', () => {
  const tabs = DOCK_GROUPS.flatMap((group) => DOCK_TABS[group])
  assert.equal(new Set(tabs).size, tabs.length, 'a tab is in one group')
  for (const tab of tabs) assert.ok(TAB_LABELS[tab], tab)
  for (const tab of WINDOW_TABS) assert.ok(tabs.includes(tab), tab)
  assert.ok(!windowRows().some((row) => row.label === 'Pathfinder'), 'Window › Pathfinder stays the submenu of its commands')
  assert.ok(windowRows().every((row) => row.path.length === 1 && row.path[0] === 'Window'), 'panel rows are Window\'s own, never a submenu')
  let shown = DEFAULT_SHOWN
  assert.equal(windowShown(shown, 'kit:window.history'), false)
  let next = windowCommand(shown, 'kit:window.history')
  assert.deepEqual([next.shown.history, next.reveal], [true, 'history'])
  shown = next.shown
  assert.equal(windowShown(shown, 'kit:window.history'), true)
  next = windowCommand(shown, 'kit:window.history')
  assert.deepEqual([next.shown.history, next.reveal], [false, null])
  // Artboards is behind Layers: its row brings it forward rather than hide the group.
  const dock = (group) => (group === 'layers-group' ? { tab: 'layers', collapsed: false } : null)
  assert.equal(windowShown(DEFAULT_SHOWN, 'kit:window.artboards', dock), false)
  next = windowCommand(DEFAULT_SHOWN, 'kit:window.artboards', dock)
  assert.deepEqual([next.shown.layers, next.reveal], [true, 'artboards'])
  // A collapsed group's row expands it.
  next = windowCommand(DEFAULT_SHOWN, 'kit:window.layers', () => ({ tab: 'layers', collapsed: true }))
  assert.deepEqual([next.shown.layers, next.reveal], [true, 'layers'])
  assert.equal(windowCommand(DEFAULT_SHOWN, 'layer.new').reveal, null)
  assert.deepEqual(shownOf({ history: true }), { ...DEFAULT_SHOWN, history: true })
  assert.equal(shownOf('nonsense'), undefined)
  assert.equal(shownOf({ layers: 3 }), undefined)
})

test('history: rows, moves to a row, and the hold of gestures and previews', () => {
  const rows = historyRows(['Rectangle', 'Move'], ['Fill'])
  assert.deepEqual(rows.map((row) => [row.position, row.name, row.undone]), [[0, null, false], [1, 'Rectangle', false], [2, 'Move', false], [3, 'Fill', true]])
  assert.deepEqual(historyMove(2, 1, 0), { command: 'edit.undo', times: 2 })
  assert.deepEqual(historyMove(2, 1, 3), { command: 'edit.redo', times: 1 })
  assert.equal(historyMove(2, 1, 2), null)
  assert.deepEqual(historyMove(2, 1, 9), { command: 'edit.redo', times: 1 }, 'never past the last redo')
  assert.equal(historyHeld(), false)
  const release = holdHistory()
  const other = holdHistory()
  assert.equal(historyHeld(), true)
  release()
  release()
  assert.equal(historyHeld(), true, 'one release each')
  other()
  assert.equal(historyHeld(), false)
})

test('artboards: a row dragged lands at its new number; the status bar wraps round', () => {
  assert.equal(reorderTarget(0, 2, 0.8), 2)
  assert.equal(reorderTarget(2, 0, 0.2), 0)
  assert.equal(reorderTarget(1, 1, 0.9), 1)
  assert.equal(stepArtboard(0, -1, 3), 2)
  assert.equal(stepArtboard(2, 1, 3), 0)
  assert.equal(stepArtboard(0, 1, 0), 0)
})

test('the panels\' own words exist in English and Chinese', () => {
  for (const key of ['layers.rename', 'layers.target', 'layers.selectArt', 'layers.count', 'artboards.next', 'history.wait', 'param.layer.setProps.dimImages']) {
    assert.ok(CATALOGS['en-US'][key], key)
    assert.ok(CATALOGS['zh-CN'][key], key)
  }
})

// ---------- In the built engine ----------

const engineDir = new URL('../../public/engine/', import.meta.url)
const sdk = existsSync(new URL('kition_vector_kit_bg.wasm', engineDir))
  ? await (async () => {
      const module = await import(new URL('kition_vector_kit.js', engineDir).href)
      module.initSync({ module: readFileSync(new URL('kition_vector_kit_bg.wasm', engineDir)) })
      return module
    })()
  : null
const skip = sdk ? false : 'the web engine is not built'
const sample = readFileSync(new URL('../../public/sample/Sample.kvector', import.meta.url))
const open = () => sdk.VectorDocument.open('Sample.kvector', sample)
const run = (doc, command, params = {}) => JSON.parse(doc.execute(command, JSON.stringify(params)) ?? 'null')
const inspect = (doc) => JSON.parse(doc.inspect())

test('regressions/issue-layer-rows: the inspect reports the highlighted rows', { skip }, () => {
  const doc = open()
  const [top, layer1] = inspect(doc).layers
  const ellipse = layer1.children[0].id
  run(doc, 'layer.setCurrent', { id: ellipse })
  let state = inspect(doc)
  assert.deepEqual(state.layerRows, [ellipse])
  run(doc, 'layer.highlight', { ids: [top.id], mode: 'add' })
  state = inspect(doc)
  assert.deepEqual(state.layerRows, [ellipse, top.id])
  doc.free()
})

test('a row moved in the Layers panel is one undo step, and Undo puts it back', { skip }, () => {
  const doc = open()
  const [, layer1] = inspect(doc).layers
  const [ellipse, rect] = layer1.children.map((child) => child.id)
  const before = inspect(doc).history.length
  run(doc, 'layer.move', { ids: [rect], target: ellipse, place: 'above' })
  let state = inspect(doc)
  assert.deepEqual(state.layers[1].children.map((child) => child.id), [rect, ellipse])
  assert.equal(state.history.length, before + 1)
  run(doc, 'edit.undo')
  state = inspect(doc)
  assert.deepEqual(state.layers[1].children.map((child) => child.id), [ellipse, rect])
  doc.free()
})

/** The rows a person sees in the menus that run at once (no dialog), as `menuRows.test.mjs` takes them. */
function visibleDirectRows(commands) {
  return commands
    .filter((command) => command.menu?.length)
    .filter((command) => !isHiddenRow({ command: command.id, path: command.menu.slice(0, 2) }))
    .filter((command) => dialogFor({ id: command.id, title: command.title, menu: command.menu, asks: paramFields(command).length > 0 }) === null)
}

/**
 * Commands whose run can refuse what the state is even when the menu offers them: the outcome
 * decides (Pathfinder's empty result, Join's closed paths, Outline Stroke's unstroked paths, a
 * compound path of text), not the kind of what is selected, so the menu cannot know beforehand.
 */
const OUTCOME_DECIDES = new Set([
  'object.compoundPath.make',
  'path.join',
  'select.same.appearanceAttribute',
  'object.path.outlineStroke',
  'object.path.addAnchorPoints',
  'livePaint.make',
  'object.pathfinder.unite',
  'object.pathfinder.minusFront',
  'object.pathfinder.intersect',
  'object.pathfinder.exclude',
  'object.pathfinder.divide',
  'object.pathfinder.trim',
  'object.pathfinder.merge',
  'object.pathfinder.crop',
  'object.pathfinder.minusBack',
])

test('a visible menu row that would fail for the state it is clicked in is greyed out instead', { skip }, () => {
  const commands = JSON.parse(open().commands())
  const states = {
    nothing: (doc) => run(doc, 'edit.deselectAll'),
    everything: (doc) => run(doc, 'edit.selectAll'),
    ellipse: (doc) => run(doc, 'select.set', { ids: [3] }),
    text: (doc) => {
      run(doc, 'edit.deselectAll')
      run(doc, 'select.object.textObjects')
    },
  }
  const failing = []
  let greyed = 0
  for (const command of visibleDirectRows(commands)) {
    if (OUTCOME_DECIDES.has(command.id)) continue
    for (const [name, choose] of Object.entries(states)) {
      const doc = open()
      run(doc, 'text.add', { x: 40, y: 40, text: 'Kition' })
      choose(doc)
      try {
        if (JSON.parse(doc.disabledCommands()).includes(command.id)) {
          greyed += 1
          continue
        }
        doc.execute(command.id, '{}')
      } catch (error) {
        failing.push(`${command.id} (${name}): ${String(error).slice(0, 120)}`)
      } finally {
        doc.free()
      }
    }
  }
  assert.deepEqual(failing, [], 'rows offered in a state they fail in')
  assert.ok(greyed > 100, `${greyed} rows greyed`)
})

test('the rows that used to fail are greyed with nothing selected, and offered once their objects are', { skip }, () => {
  const doc = open()
  run(doc, 'edit.deselectAll')
  const off = JSON.parse(doc.disabledCommands())
  for (const id of ['object.blend.make', 'text.convertToOutlines', 'path.reverse', 'object.clippingMask.editContents', 'livePaint.release', 'imageTrace.release', 'shaper.release', 'view.guides.make', 'object.textWrap.release', 'select.same.fontSize', 'asset.add']) assert.ok(off.includes(id), `${id} is greyed with nothing selected`)
  run(doc, 'edit.selectAll')
  const on = JSON.parse(doc.disabledCommands())
  for (const id of ['object.blend.make', 'path.reverse', 'asset.add', 'view.guides.make']) assert.ok(!on.includes(id), `${id} is offered with everything selected`)
  // An agent naming its objects is never refused for want of a selection.
  run(doc, 'edit.deselectAll')
  const [, layer1] = inspect(doc).layers
  run(doc, 'object.blend.make', { ids: layer1.children.map((child) => child.id) })
  assert.ok(inspect(doc).layers[1].children.some((child) => child.kind === 'Blend'))
  doc.free()
})
