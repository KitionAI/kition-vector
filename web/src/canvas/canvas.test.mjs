// The canvas's rules without a browser (plan 2026-11-25, stage 2, feature vector.editor.canvas):
// the keys the canvas takes before the shortcuts, the X/Y/W/H fields' parameters, the dialogs tools
// ask for (each labelled in Chinese, and, against the built engine, each running with its starting
// values), the right-click menu's rows, and the wheel's zoom step. Run with `pnpm --dir web test:unit`.
import { existsSync, readFileSync } from 'node:fs'
import test from 'node:test'
import assert from 'node:assert/strict'
import { canvasKey, singleKeyCommands } from './keys.ts'
import { boundsParams } from './bounds.ts'
import { TOOL_DIALOG_NAMES, toolDialog } from './toolDialogs.ts'
import { contextRows } from './contextRows.ts'
import { wheelFactor, WHEEL_STEP } from './zoomTool.ts'
import { CATALOGS } from '../strings.ts'
import { initialValues, paramsOf } from '../dialogs/params.ts'

const key = (key, extra = {}) => ({ key, code: extra.code ?? (key.length === 1 && /[a-z]/i.test(key) ? `Key${key.toUpperCase()}` : key), metaKey: false, ctrlKey: false, altKey: false, shiftKey: false, ...extra })
const context = (extra = {}) => ({ toolKeys: [], editingText: false, selection: true, singleKeys: singleKeyCommands([{ id: 'paint.toggleActive', shortcut: 'X' }, { id: 'paint.default', shortcut: 'D' }, { id: 'paint.none', shortcut: '/' }, { id: 'paint.lastColor', shortcut: ',' }, { id: 'paint.lastGradient', shortcut: '.' }, { id: 'paint.swap', shortcut: 'Shift+X' }, { id: 'edit.undo', shortcut: 'Cmd+Z' }]), toolLetter: (k) => ['v', 'a', 'p', 'm'].includes(k.toLowerCase()), ...extra })

test('a key the active tool takes goes to it before the shortcuts and the arrows', () => {
  assert.deepEqual(canvasKey(key('Delete'), context({ toolKeys: ['Delete', 'ArrowLeft'] })), { kind: 'tool', key: 'Delete' })
  assert.deepEqual(canvasKey(key('ArrowLeft'), context({ toolKeys: ['Delete', 'ArrowLeft'] })), { kind: 'tool', key: 'ArrowLeft' })
  assert.deepEqual(canvasKey(key('Enter'), context({ toolKeys: ['Enter', 'Backspace'] })), { kind: 'tool', key: 'Enter' })
  assert.equal(canvasKey(key('Delete'), context()), null, 'unclaimed, Delete stays the Clear shortcut')
})

test('the arrows nudge the selection, Shift ten times as far, Alt a copy; nothing selected, they do nothing here', () => {
  assert.deepEqual(canvasKey(key('ArrowRight'), context()), { kind: 'nudge', dx: 1, dy: 0, big: false, copy: false })
  assert.deepEqual(canvasKey(key('ArrowUp', { shiftKey: true }), context()), { kind: 'nudge', dx: 0, dy: -1, big: true, copy: false })
  assert.deepEqual(canvasKey(key('ArrowLeft', { altKey: true }), context()), { kind: 'nudge', dx: -1, dy: 0, big: false, copy: true })
  assert.equal(canvasKey(key('ArrowDown'), context({ selection: false })), null)
  assert.equal(canvasKey(key('ArrowDown'), context({ editingText: true })), null, 'typing: the caret moves')
})

test('X, D, /, comma and period run their commands; tool letters and Shift+X stay with the frame', () => {
  for (const [k, id] of [['x', 'paint.toggleActive'], ['d', 'paint.default'], ['/', 'paint.none'], [',', 'paint.lastColor'], ['.', 'paint.lastGradient']]) {
    assert.deepEqual(canvasKey(key(k), context()), { kind: 'command', id }, k)
  }
  assert.equal(canvasKey(key('v'), context()), null)
  assert.equal(canvasKey(key('x', { shiftKey: true }), context()), null, 'Shift+X is a shortcut the frame binds')
  assert.equal(canvasKey(key('x'), context({ editingText: true })), null, 'typed into the text')
  assert.deepEqual(canvasKey(key('+', { shiftKey: true }), context()), { kind: 'pickTool', id: 'addAnchor' })
})

test('Cmd+1 is actual size, Cmd+0 fits the artboards, Cmd+Alt+0 everything; Cmd+Enter ends typed text', () => {
  assert.deepEqual(canvasKey(key('1', { code: 'Digit1', metaKey: true }), context()), { kind: 'zoom', to: 'actual' })
  assert.deepEqual(canvasKey(key('0', { code: 'Digit0', ctrlKey: true }), context()), { kind: 'zoom', to: 'artboards' })
  assert.deepEqual(canvasKey(key('º', { code: 'Digit0', metaKey: true, altKey: true }), context()), { kind: 'zoom', to: 'all' })
  assert.deepEqual(canvasKey(key('Enter', { metaKey: true }), context({ editingText: true })), { kind: 'endText' })
  assert.equal(canvasKey(key('Enter', { metaKey: true }), context()), null)
  assert.equal(canvasKey(key('z', { metaKey: true }), context()), null, 'Cmd+Z stays the shortcut')
})

test('X, Y, W and H run about the top-left the fields show; W or H not above 0 runs nothing (regression issue-bounds-shift)', () => {
  assert.deepEqual(boundsParams('x', 100), { x: 100, reference: 0 })
  assert.deepEqual(boundsParams('width', 90), { width: 90, reference: 0 })
  assert.equal(boundsParams('height', 0), null)
  assert.equal(boundsParams('width', -5), null)
  assert.equal(boundsParams('y', Number.NaN), null)
  assert.deepEqual(boundsParams('y', -20), { y: -20, reference: 0 }, 'a place above the artboard is fine')
})

test('every dialog a tool asks for opens, its fields labelled in Chinese and titled', () => {
  const zh = CATALOGS['zh-CN']
  const en = CATALOGS['en-US']
  const ctx = { widthPoint: () => ({ t: 0.5, left: 2, right: 3 }), stopColor: () => '#ff0000' }
  for (const name of TOOL_DIALOG_NAMES) {
    const params = { x: 10, y: 20, index: 1, id: 5, origin: [3, 4], corners: [0], name: 'A', width: 100, height: 50 }
    const dialog = toolDialog(name, params, ctx)
    assert.ok(dialog, name)
    if (dialog.kind === 'open') continue
    assert.ok(zh[`toolDialog.${name}`] && en[`toolDialog.${name}`], `${name}'s title`)
    for (const field of dialog.fields) {
      const family = dialog.command.split('.')[0]
      const label = zh[`param.${dialog.command}.${field.key}`] ?? zh[`param.${family}.${field.key}`]
      assert.ok(label && /[一-鿿]|^[XY]$/.test(label), `${dialog.command}.${field.key} has a Chinese label`)
      if (field.kind === 'choice') for (const option of field.options) assert.ok(zh[`option.${option}`], `option ${option}`)
    }
  }
  assert.equal(toolDialog('rectangle', {}), null, 'a shape request without its click opens nothing')
  assert.equal(toolDialog('graph', { x: 1, y: 2 }), null, 'the graph tools are not on the rail')
})

test('the transform tools hand their dialogs only the point they chose', () => {
  assert.deepEqual(toolDialog('rotate', { angle: 0, origin: [120, 130] }), { kind: 'open', command: 'object.rotate', values: { origin: [120, 130] } })
  assert.deepEqual(toolDialog('scale', { sx: 100, sy: 100, origin: [1, 2] }), { kind: 'open', command: 'object.scale', values: { origin: [1, 2] } })
  assert.deepEqual(toolDialog('blendOptions', {}), { kind: 'open', command: 'object.blend.options', values: {} })
})

test('a shape dialog draws where the tool was clicked, the Line Segment by length and angle', () => {
  const rectangle = toolDialog('rectangle', { x: 60, y: 500 })
  assert.deepEqual(rectangle.build({ width: 50, height: 30 }), { x: 60, y: 500, width: 50, height: 30 })
  const star = toolDialog('star', { x: 5, y: 6 })
  assert.deepEqual(star.build({ radius1: 50, radius2: 25, points: 5 }), { cx: 5, cy: 6, radius1: 50, radius2: 25, points: 5 })
  const line = toolDialog('lineSegment', { x: 10, y: 10 })
  assert.deepEqual(line.build({ length: 100, angle: 90 }), { x1: 10, y1: 10, x2: 10, y2: -90 }, 'up the page')
  const stop = toolDialog('gradientStop', { index: 2 }, { stopColor: () => '#336699' })
  assert.deepEqual(stop.start, { color: '#336699' })
  assert.deepEqual(stop.build({ color: '#000000' }), { color: '#000000', stop: 2 })
})

test('the right-click menu follows what is selected', () => {
  const ids = (rows) => rows.filter((row) => row.kind === 'item').map((row) => row.command)
  const blank = ids(contextRows({ kinds: [], only: null, isolated: false }))
  assert.ok(blank.includes('edit.paste') && blank.includes('edit.selectAll') && !blank.includes('edit.cut'))
  const one = ids(contextRows({ kinds: ['Path'], only: 4, isolated: false }))
  assert.ok(one.includes('edit.cut') && one.includes('object.rotate') && one.includes('arrange.bringToFront') && !one.includes('arrange.group'))
  const several = ids(contextRows({ kinds: ['Path', 'Ellipse'], only: null, isolated: false }))
  assert.ok(several.includes('arrange.group') && several.includes('object.clippingMask.make') && several.includes('object.compoundPath.make'))
  const group = contextRows({ kinds: ['Group'], only: 9, isolated: false })
  assert.deepEqual(group.find((row) => row.command === 'object.isolate')?.params, { id: 9 })
  assert.ok(ids(group).includes('arrange.ungroup'))
  assert.ok(ids(contextRows({ kinds: ['Point Type'], only: 3, isolated: false })).includes('text.convertToOutlines'))
  assert.equal(ids(contextRows({ kinds: [], only: null, isolated: true }))[0], 'object.exitIsolation')
  const rows = contextRows({ kinds: ['Path'], only: 4, isolated: false })
  assert.notEqual(rows[0].kind, 'separator')
  assert.notEqual(rows.at(-1).kind, 'separator')
  assert.ok(!rows.some((row, index) => row.kind === 'separator' && rows[index + 1]?.kind === 'separator'))
})

test('a mouse wheel notch with Cmd zooms by at most 1.2 times; a trackpad pinch as before', () => {
  assert.equal(wheelFactor(-100), WHEEL_STEP)
  assert.equal(wheelFactor(100), 1 / WHEEL_STEP)
  assert.equal(wheelFactor(-3, 1), WHEEL_STEP, 'lines')
  assert.ok(Math.abs(wheelFactor(-4) - Math.exp(0.04)) < 1e-12, 'a pinch step is unchanged')
})

// Against the built engine: each dialog a tool asks for runs with its starting values.
const engineDir = new URL('../../public/engine/', import.meta.url)
const sdk = existsSync(new URL('kition_vector_kit_bg.wasm', engineDir))
  ? await (async () => {
      const module = await import(new URL('kition_vector_kit.js', engineDir).href)
      module.initSync({ module: readFileSync(new URL('kition_vector_kit_bg.wasm', engineDir)) })
      return module
    })()
  : null
const sample = readFileSync(new URL('../../public/sample/Sample.kvector', import.meta.url))

test('each dialog a tool asks for runs in the engine with its starting values', { skip: sdk ? false : 'the web engine is not built' }, () => {
  const ellipse = 3
  for (const name of TOOL_DIALOG_NAMES) {
    const doc = sdk.VectorDocument.open('Sample.kvector', sample)
    const run = (command, params) => JSON.parse(doc.execute(command, JSON.stringify(params)))
    // What each request finds when the tool sends it: a selected ellipse, a gradient, a width point.
    run('select.set', { ids: [ellipse] })
    run('stroke.set', { weight: 6 })
    run('paint.setStroke', { color: '#000000' })
    run('stroke.widthPoint.set', { id: ellipse, t: 0.3, left: 2, right: 3 })
    run('paint.setFill', { gradient: { kind: 'linear' } })
    const params = { x: 30, y: 40, index: 0, id: name === 'corners' ? 2 : ellipse, origin: [400, 140], corners: [0], name: 'Board', width: 800, height: 600 }
    const dialog = toolDialog(name, params, { widthPoint: () => ({ t: 0.3, left: 2, right: 3 }), stopColor: () => '#ff0000' })
    if (dialog.kind === 'open') {
      assert.ok(JSON.parse(doc.commands()).some((command) => command.id === dialog.command), `${name}: ${dialog.command} is a command`)
      continue
    }
    const values = paramsOf(dialog.fields, initialValues(dialog.fields, dialog.start))
    const before = JSON.parse(doc.inspect()).history.length
    assert.doesNotThrow(() => run(dialog.command, dialog.build(values)), `${name}: ${dialog.command} ${JSON.stringify(dialog.build(values))}`)
    assert.equal(JSON.parse(doc.inspect()).history.length, before + 1, `${name} is one step`)
  }
})
