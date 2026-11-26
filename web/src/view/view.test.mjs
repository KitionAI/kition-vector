// The View menu's rules (plan 2026-11-25, stage 6, feature vector.editor.view): its rows, check
// marks and keys, the typed zoom, the rulers' steps, the scrollbars' range; and, on the built web
// engine, the `View:` requests: toggles that are no edit, Outline and Pixel Preview and the
// transparency grid in the pixels, Smart Guides off, a guide out of a ruler in one undo step.
import { existsSync, readFileSync } from 'node:fs'
import test from 'node:test'
import assert from 'node:assert/strict'
import { DEFAULT_TOGGLES, KEPT, VIEW_ROW, engineViewOf, keptOf, looksOf, rulerLabel, rulerStep, scrollRange, typedZoom, viewChecked, viewKey, viewRowOf, viewRows } from './model.ts'
import { CATALOGS, PHRASES } from '../strings.ts'
import { isHiddenRow } from '../menus/hidden.ts'

const key = (code, mods = {}) => ({ key: '', code, metaKey: true, ctrlKey: false, altKey: false, shiftKey: false, ...mods })

test('the View menu: Kition rows among the engine rows, in the reference order', () => {
  const engine = [
    { path: ['View'], label: 'Show Transparency Grid', shortcut: 'Cmd+Shift+D', command: 'view.transparencyGrid' },
    { path: ['View', 'Guides'], label: 'Lock Guides', shortcut: 'Cmd+Alt+;', command: 'view.guides.lock' },
    { path: ['View'], label: 'Something Else', shortcut: null, command: 'view.other' },
  ]
  const rows = viewRows(engine)
  const commands = rows.map((row) => row.command)
  assert.equal(commands[0], `${VIEW_ROW}outline`)
  assert.ok(commands.indexOf(`${VIEW_ROW}boundingBox`) < commands.indexOf('view.transparencyGrid'))
  assert.ok(commands.indexOf(`${VIEW_ROW}guides`) < commands.indexOf('view.guides.lock'))
  assert.equal(commands.at(-1), 'view.other', 'an engine row it does not place keeps its place after them')
  assert.deepEqual(rows.find((row) => row.command === `${VIEW_ROW}guides`).path, ['View', 'Guides'])
  assert.deepEqual(rows.find((row) => row.command === `${VIEW_ROW}rulers`).path, ['View', 'Rulers'])
  assert.equal(rows.find((row) => row.command === `${VIEW_ROW}outline`).shortcut, 'Cmd+Y')
  // Every own row is a toggle or a zoom; every label is translated.
  const zh = { ...PHRASES['zh-CN'] }
  const upstream = readFileSync(new URL('../i18n/upstream/zh-hans.tsv', import.meta.url), 'utf8')
  for (const row of rows.filter((row) => row.command.startsWith(VIEW_ROW))) {
    assert.ok(viewRowOf(row.command), row.command)
    const translated = zh[row.label] ?? upstream.split('\n').find((line) => line.split('\t')[0] === '' && line.split('\t')[1] === row.label)
    assert.ok(translated, `${row.label} has a Chinese label`)
  }
})

test('check marks: the toggles, and the engine view commands\' states', () => {
  const engine = engineViewOf({ guidesLocked: true, transparencyGrid: false, proofColors: true })
  assert.equal(viewChecked(`${VIEW_ROW}smartGuides`, DEFAULT_TOGGLES, engine), true)
  assert.equal(viewChecked(`${VIEW_ROW}outline`, DEFAULT_TOGGLES, engine), false)
  assert.equal(viewChecked(`${VIEW_ROW}zoomIn`, DEFAULT_TOGGLES, engine), undefined, 'a zoom row is no toggle')
  assert.equal(viewChecked('view.guides.lock', DEFAULT_TOGGLES, engine), true)
  assert.equal(viewChecked('view.transparencyGrid', DEFAULT_TOGGLES, engine), false)
  assert.equal(viewChecked('view.proofColors', DEFAULT_TOGGLES, engine), true)
  assert.equal(viewChecked('view.guides.make', DEFAULT_TOGGLES, engine), undefined)
})

test('the View menu\'s keys', () => {
  assert.equal(viewKey(key('KeyY')), 'outline')
  assert.equal(viewKey(key('KeyY', { altKey: true })), 'pixelPreview')
  assert.equal(viewKey(key('KeyY', { altKey: true, shiftKey: true })), null, 'Overprint Preview is the engine\'s')
  assert.equal(viewKey(key('KeyU')), 'smartGuides')
  assert.equal(viewKey(key('KeyH')), 'edges')
  assert.equal(viewKey(key('KeyB', { shiftKey: true })), 'boundingBox')
  assert.equal(viewKey(key('KeyB')), null)
  assert.equal(viewKey(key('KeyR')), 'rulers')
  assert.equal(viewKey(key('Semicolon')), 'guides')
  assert.equal(viewKey(key('Semicolon', { altKey: true })), null, 'Lock Guides is the engine\'s')
  assert.equal(viewKey(key('Quote')), 'grid')
  assert.equal(viewKey(key('Quote', { shiftKey: true })), 'snapToGrid')
  assert.equal(viewKey(key('Quote', { altKey: true })), 'snapToPoint')
  assert.equal(viewKey({ ...key('KeyY'), metaKey: false }), null, 'Cmd or Ctrl only')
  assert.equal(viewKey({ ...key('KeyY'), metaKey: false, ctrlKey: true }), 'outline')
})

test('the kept toggles: not Outline or Pixel Preview, and only booleans', () => {
  assert.ok(!KEPT.includes('outline') && !KEPT.includes('pixelPreview') && KEPT.includes('rulers'))
  assert.deepEqual(keptOf({ rulers: true, grid: 'yes', outline: true }), { rulers: true })
  assert.equal(keptOf(null), null)
})

test('a typed zoom, the rulers\' steps, the scrollbars\' range', () => {
  assert.equal(typedZoom('150'), 1.5)
  assert.ok(Math.abs(typedZoom(' 33.3 % ') - 0.333) < 1e-12)
  assert.equal(typedZoom('12,5%'), 0.125)
  assert.equal(typedZoom('100000'), 64, 'at most 6400%')
  assert.equal(typedZoom('abc'), null)
  assert.equal(typedZoom('0'), null)
  // At 100% in points labels stand every 100 points (56 pixels wanted), ten ticks between.
  assert.deepEqual(rulerStep(1, 1), { step: 100, minor: 10 })
  assert.deepEqual(rulerStep(4, 1), { step: 20, minor: 4 })
  // Millimetres at 100%: 2.83 pixels a millimetre.
  assert.deepEqual(rulerStep(1, 72 / 25.4), { step: 20, minor: 4 })
  assert.equal(rulerLabel(-0.000001), '0')
  assert.equal(rulerLabel(12.5), '12.5')
  // The extent all in view: no scrollbars. Zoomed in: both.
  assert.deepEqual(scrollRange({ x: -50, y: -50, zoom: 1, width: 800, height: 600 }, 612, 500), { x: null, y: null })
  const zoomed = scrollRange({ x: 100, y: 0, zoom: 4, width: 800, height: 600 }, 612, 792)
  assert.deepEqual(zoomed.x, { start: 0, value: 100, max: 412, page: 200 })
  assert.equal(zoomed.y.max, 792 - 150)
})

test('the looks of the grid and the guides, with defaults', () => {
  const looks = looksOf({ gridLook: { spacing: 36, subdivisions: 4, color: '#112233', style: 'dots', pixelGrid: false }, guideLook: { color: 'red' } })
  assert.deepEqual(looks.grid, { spacing: 36, subdivisions: 4, color: '#112233', dots: true, pixelGrid: false })
  assert.equal(looks.guide.color, '#4affff', 'a colour that is not #rrggbb falls back')
})

test('the view strings are in both catalogs', () => {
  for (const name of ['view.zoomField', 'view.rulerTop', 'view.rulerLeft', 'view.scrollHorizontal', 'view.scrollVertical', 'view.fit']) {
    assert.ok(CATALOGS['en-US'][name] && CATALOGS['zh-CN'][name], name)
  }
  assert.ok(isHiddenRow({ path: ['View'], label: 'Show Print Tiling', command: 'view.printTiling' }), 'Show Print Tiling has nothing to show')
})

// On the built web engine.
const engineDir = new URL('../../public/engine/', import.meta.url)
const sdk = existsSync(new URL('kition_vector_kit_bg.wasm', engineDir))
  ? await (async () => {
      const module = await import(new URL('kition_vector_kit.js', engineDir).href)
      module.initSync({ module: readFileSync(new URL('kition_vector_kit_bg.wasm', engineDir)) })
      return module
    })()
  : null
const skip = sdk ? false : 'the web engine is not built'
const NO_MODS = JSON.stringify({ shift: false, alt: false, command: false, control: false, space: false })

const drawing = (width = 400, height = 300) => {
  const doc = sdk.VectorDocument.create('view.kvector', width, height)
  const execute = (command, params = {}) => JSON.parse(doc.execute(command, JSON.stringify(params)) || 'null')
  const inspect = () => JSON.parse(doc.inspect())
  const request = (body) => JSON.parse(doc.toolKey(`View:${JSON.stringify(body)}`, NO_MODS, 1))
  const origin = () => inspect().origin
  const pixel = (x, y) => {
    const [ox, oy] = origin()
    return [...doc.render(x - ox, y - oy, 1, 1, 1)]
  }
  return { doc, execute, inspect, request, origin, pixel }
}

test('view toggles are no edit, and inspect reports them', { skip }, () => {
  const { doc, execute, inspect, request } = drawing()
  execute('shape.rectangle', { x: 20, y: 20, width: 100, height: 80 })
  assert.equal(inspect().view.smartGuides, true)
  assert.equal(inspect().view.outline, false)
  const before = { revision: doc.revision, dirty: doc.dirty, history: inspect().history.length }
  request({ set: { outline: true, smartGuides: false, grid: true, rulers: true } })
  const view = inspect().view
  assert.deepEqual([view.outline, view.smartGuides, view.grid, view.rulers], [true, false, true, true])
  assert.deepEqual({ revision: doc.revision, dirty: doc.dirty, history: inspect().history.length }, before)
  assert.throws(() => doc.toolKey('View:{"set":{"wireframe":true}}', NO_MODS, 1))
  execute('view.guides.lock', { locked: true })
  execute('view.transparencyGrid', { on: true })
  assert.equal(inspect().view.guidesLocked, true)
  assert.equal(inspect().view.transparencyGrid, true)
})

test('Outline, the transparency grid and Pixel Preview change the pixels', { skip }, () => {
  const { doc, execute, request, pixel, origin } = drawing()
  const id = execute('shape.rectangle', { x: 20, y: 20, width: 100, height: 80 }).id
  execute('paint.setFill', { ids: [id], color: '#ff0000' })
  execute('edit.deselectAll')
  assert.deepEqual(pixel(70, 60), [255, 0, 0, 255])
  request({ set: { outline: true } })
  assert.deepEqual(pixel(70, 60), [255, 255, 255, 255])
  const edge = pixel(20, 60)
  assert.ok(edge[0] < 160 && edge[1] < 160, `the path as a dark line: ${edge}`)
  request({ set: { outline: false } })
  assert.deepEqual(pixel(70, 60), [255, 0, 0, 255])
  execute('view.transparencyGrid', { on: true })
  const [ox, oy] = origin()
  const tile = doc.render(200 - ox, 150 - oy, 1, 32, 32)
  const colours = new Set()
  for (let i = 0; i < tile.length; i += 4) colours.add(`${tile[i]},${tile[i + 1]},${tile[i + 2]},${tile[i + 3]}`)
  assert.equal(colours.size, 2, [...colours].join(' '))
  assert.deepEqual(pixel(70, 60), [255, 0, 0, 255])
  execute('view.transparencyGrid', { on: false })
  // The colours of points 19 (half covered by the stroke) and 20, drawn at 1:1.
  const [p19, p20] = [pixel(19, 50), pixel(20, 50)]
  assert.notDeepEqual(p19, p20)
  request({ set: { pixelPreview: true } })
  const block = doc.render(19.5 - ox, 50 - oy, 8, 32, 8)
  for (let i = 0; i < 32 * 8; i += 1) {
    const [column, row] = [i % 32, Math.floor(i / 32)]
    const start = column < 4 ? 0 : 4 + Math.floor((column - 4) / 8) * 8
    assert.deepEqual([...block.slice(i * 4, i * 4 + 4)], [...block.slice((row * 32 + start) * 4, (row * 32 + start) * 4 + 4)], `pixel ${column},${row}`)
  }
  assert.deepEqual([...block.slice(0, 4)], p19, 'point 19 as it is at 1:1')
  assert.deepEqual([...block.slice(16, 20)], p20, 'point 20 as it is at 1:1')
})

test('Smart Guides off leave a dragged object where the pointer puts it', { skip }, () => {
  const dragged = (smart) => {
    const { doc, execute, inspect, request, origin } = drawing(600, 400)
    const a = execute('shape.rectangle', { x: 20, y: 20, width: 100, height: 80 }).id
    execute('shape.rectangle', { x: 220, y: 220, width: 100, height: 80 })
    execute('select.set', { ids: [a] })
    doc.selectTool('selection', 1)
    request({ set: { smartGuides: smart, snapToPoint: false } })
    const [ox, oy] = origin()
    const at = (action, x, y) => doc.pointer(JSON.stringify({ action, x: x - ox, y: y - oy, mods: JSON.parse(NO_MODS), pressure: 1, zoom: 1 }))
    at('down', 70, 60)
    for (let step = 1; step <= 10; step += 1) at('drag', 70 + 9.75 * step, 60)
    at('up', 167.5, 60)
    return inspect().layers[0].children.find((node) => node.id === a).bounds.x
  }
  assert.ok(Math.abs(dragged(true) - 120) < 1e-6, `on: ${dragged(true)}`)
  assert.ok(Math.abs(dragged(false) - 117.5) < 1e-6, `off: ${dragged(false)}`)
})

test('Show Bounding Box and Show Edges hide the selection\'s overlays', { skip }, () => {
  const { doc, execute, request } = drawing()
  const id = execute('shape.rectangle', { x: 20, y: 20, width: 100, height: 80 }).id
  execute('select.set', { ids: [id] })
  const all = JSON.parse(doc.selectTool('selection', 1)).overlays.length
  const noBox = request({ set: { boundingBox: false } }).overlays.length
  const none = request({ set: { edges: false } }).overlays.length
  assert.ok(all > noBox && noBox > none, `${all} > ${noBox} > ${none}`)
  assert.equal(none, 0)
})

test('a guide out of a ruler is made where it is released over the canvas, one undo step', { skip }, () => {
  const { execute, inspect, request, origin } = drawing()
  const [ox, oy] = origin()
  const steps = inspect().history.length
  const ruler = (kind, x, onCanvas) => request({ ruler: { vertical: true, kind, x: x - ox, y: 100 - oy, onCanvas } })
  ruler('drag', -5, false)
  assert.deepEqual(execute('guide.list'), [])
  ruler('drag', 80, true)
  ruler('drag', 150.3, true)
  assert.equal(execute('guide.list').length, 1, 'shown as it goes')
  ruler('up', 150.3, true)
  const guides = execute('guide.list')
  assert.equal(guides.length, 1)
  assert.equal(guides[0].vertical, true)
  assert.ok(Math.abs(guides[0].pos - 150) <= 0.5, JSON.stringify(guides))
  assert.equal(inspect().history.length, steps + 1)
  ruler('drag', 60, true)
  ruler('up', -5, false)
  assert.equal(execute('guide.list').length, 1, 'released over the ruler: none')
})
