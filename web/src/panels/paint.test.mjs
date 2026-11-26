// Colour, paint and the panels' pure parts (plan 2026-11-25, stage 3, feature vector.editor.paint),
// and what they send, against the built web engine when it is there. Run with `pnpm --dir web test:unit`.
import { existsSync, readFileSync } from 'node:fs'
import test from 'node:test'
import assert from 'node:assert/strict'
import { cmykToRgb, fieldsOfCmyk, rgbToCmyk, withField, fieldsOf } from '../color/colorMath.ts'
import { colorHex, paintView, pickedOf, pickedParams, stopsCss } from '../color/paint.ts'
import { addStop, engineStops, fromPaint, midpointPlaces, moveStop, removeStop, reversed, sampleAt, setStop } from '../gradient/gradient.ts'
import { evaluate, formatNumber } from './arithmetic.ts'
import { createLiveEdit } from './live.ts'
import { alignTo, dashFields, swatchParams, swatchView, withDash, BLEND_MODES } from './model.ts'
import { boundsParams, referencePoint } from '../canvas/bounds.ts'
import { CATALOGS } from '../strings.ts'
import { ARROWHEADS } from './model.ts'

test('CMYK follows RGB and keeps the numbers typed', () => {
  assert.deepEqual(rgbToCmyk([255, 0, 0]), { c: 0, m: 100, y: 100, k: 0 })
  assert.deepEqual(rgbToCmyk([0, 0, 0]), { c: 0, m: 0, y: 0, k: 100 })
  assert.deepEqual(cmykToRgb({ c: 0, m: 0, y: 0, k: 0 }), [255, 255, 255])
  const typed = withField(fieldsOf([255, 255, 255]), 'c', 40)
  assert.equal(typed.typedCmyk, true)
  assert.equal(typed.cmyk.c, 40)
  assert.equal(typed.hex, '#99ffff')
  assert.equal(fieldsOfCmyk({ c: 10, m: 20, y: 30, k: 5 }).cmyk.k, 5)
  // A field of RGB after CMYK goes back to colour from RGB.
  assert.equal(withField(typed, 'r', 0).typedCmyk, false)
})

test('engine colours show as hex in every model', () => {
  assert.equal(colorHex({ model: 'rgb', r: 1, g: 0, b: 0.5 }), '#ff0080')
  assert.equal(colorHex({ model: 'gray', k: 0.4 }), '#999999')
  assert.equal(colorHex({ model: 'cmyk', c: 0, m: 1, y: 1, k: 0 }), '#ff0000')
  assert.match(colorHex({ model: 'lab', l: 50, a: 0, b: 0 }), /^#7[67]7[67]7[67]$/)
  assert.equal(colorHex('#ABCDEF'), '#abcdef')
  assert.equal(colorHex('None'), null)
})

test('a paint shows as what it is: None, a gradient, a pattern, mixed, never black (issue-paint-black)', () => {
  assert.deepEqual(paintView({ type: 'none' }), { kind: 'none' })
  assert.deepEqual(paintView(null), { kind: 'none' })
  assert.deepEqual(paintView({ type: 'solid', color: { model: 'rgb', r: 0, g: 0, b: 0 } }, true), { kind: 'mixed' })
  assert.deepEqual(paintView({ type: 'pattern', pattern: 'Dots' }), { kind: 'pattern', name: 'Dots' })
  const gradient = paintView({ type: 'gradient', gradient: { kind: 'Radial', stops: [{ offset: 0, color: { model: 'rgb', r: 1, g: 0, b: 0 } }, { offset: 1, color: { model: 'rgb', r: 0, g: 0, b: 1 } }] } })
  assert.equal(gradient.kind, 'gradient')
  assert.equal(gradient.radial, true)
  assert.match(gradient.css, /^radial-gradient\(circle, rgba\(255, 0, 0, 1\) 0%, rgba\(0, 0, 255, 1\) 100%\)$/)
  // A midpoint off the middle is a colour hint between the two stops.
  assert.match(stopsCss([{ offset: 0, color: '#000000', midpoint: 0.25 }, { offset: 1, color: '#ffffff' }]), /0%, 25%, rgba\(255, 255, 255, 1\) 100%/)
})

test('a picked colour is sent as hex, as its CMYK numbers, or as None', () => {
  assert.deepEqual(pickedParams({ kind: 'none' }), { none: true })
  assert.deepEqual(pickedParams({ kind: 'color', hex: '#336699' }), { color: '#336699' })
  assert.deepEqual(pickedParams({ kind: 'color', hex: '#336699', cmyk: { c: 50, m: 20, y: 0, k: 40 } }), { color: { c: 0.5, m: 0.2, y: 0, k: 0.4 } })
  assert.deepEqual(pickedOf({ type: 'solid', color: { model: 'cmyk', c: 0.5, m: 0, y: 0, k: 0 } }), { kind: 'color', hex: '#80ffff', cmyk: { c: 50, m: 0, y: 0, k: 0 } })
  assert.deepEqual(pickedOf({ type: 'none' }), { kind: 'none' })
})

const sunset = { type: 'gradient', angle: 30, gradient: { kind: 'Linear', stops: [{ offset: 1, color: { model: 'rgb', r: 0, g: 0, b: 1 }, opacity: 1, midpoint: 0.5 }, { offset: 0, color: { model: 'cmyk', c: 0, m: 1, y: 1, k: 0 }, opacity: 0.5, midpoint: 0.3 }] } }

test('the gradient draft: sorted stops, added, moved, removed, recoloured and reversed', () => {
  const draft = fromPaint(sunset)
  assert.equal(draft.kind, 'linear')
  assert.equal(draft.angle, 30)
  assert.deepEqual(draft.stops.map((stop) => stop.offset), [0, 1])
  // A stop keeps the engine's colour as it came (CMYK stays CMYK).
  assert.deepEqual(engineStops(draft)[0].color, { model: 'cmyk', c: 0, m: 1, y: 1, k: 0 })
  const half = sampleAt(draft, 0.3)
  assert.equal(half.hex, '#800080')
  assert.equal(half.opacity, 0.75)
  const { draft: three, index } = addStop(draft, 0.3)
  assert.equal(three.stops.length, 3)
  assert.equal(index, 1)
  assert.equal(three.stops[1].hex, '#800080')
  const moved = moveStop(three, 1, 0.9)
  assert.equal(moved.index, 1)
  assert.equal(moved.draft.stops[1].offset, 0.9)
  const passed = moveStop(three, 1, 1)
  assert.deepEqual(passed.draft.stops.map((stop) => stop.offset), [0, 1, 1])
  assert.equal(removeStop(draft, 0), null, 'two stops stay two')
  assert.equal(removeStop(three, 1).stops.length, 2)
  const blue = setStop(draft, 0, { color: '#0000ff' })
  assert.equal(blue.stops[0].hex, '#0000ff')
  assert.equal(blue.stops[0].color, '#0000ff')
  const back = reversed(draft)
  assert.deepEqual(back.stops.map((stop) => stop.offset), [0, 1])
  assert.equal(back.stops[0].hex, '#0000ff')
  assert.equal(back.stops[0].midpoint, 0.7, 'the midpoint after the first stop is mirrored')
  assert.deepEqual(midpointPlaces(draft), [0.3])
  assert.equal(fromPaint({ type: 'solid', color: { model: 'rgb', r: 1, g: 1, b: 1 } }), null)
})

test('number fields read arithmetic and lengths with units (issue-field-arithmetic)', () => {
  assert.equal(evaluate('50+25'), 75)
  assert.equal(evaluate('100/2'), 50)
  assert.equal(evaluate(' (10 + 4) * 2 '), 28)
  assert.equal(evaluate('-5'), -5)
  assert.equal(evaluate('2,5'), 2.5)
  assert.equal(evaluate('1in', 1, true), 72)
  assert.equal(Math.round(evaluate('10 mm', 1, true) * 1000) / 1000, 28.346)
  assert.equal(evaluate('72pt', 72, true), 1, 'points into a field in inches')
  assert.equal(evaluate('3px + 1', 1, true), 4)
  assert.equal(evaluate('1in', 1, false), null, 'no units where the field is not a length')
  assert.equal(evaluate('abc'), null)
  assert.equal(evaluate('5/0'), null)
  assert.equal(evaluate(''), null)
  assert.equal(formatNumber(260, 2), '260')
  assert.equal(formatNumber(2.3456, 2), '2.35')
})

/** An engine of numbered steps: run adds one, undo takes the last away. */
function fakeEngine() {
  const steps = []
  let revision = 0
  return {
    steps,
    engine: {
      run: async (command, params) => {
        await new Promise((resolve) => setTimeout(resolve, 2))
        steps.push([command, params])
        revision += 1
      },
      undo: async () => {
        steps.pop()
        revision += 1
      },
      revision: () => revision,
    },
    foreign: () => {
      steps.push(['someone else', {}])
      revision += 1
    },
  }
}

test('a live gesture leaves one step: the last value, or the final one', async () => {
  const { steps, engine } = fakeEngine()
  const live = createLiveEdit(engine)
  for (let value = 1; value <= 20; value += 1) live.set('transparency.set', { opacity: value })
  await live.commit()
  assert.deepEqual(steps, [['transparency.set', { opacity: 20 }]])
  const again = createLiveEdit(engine)
  again.set('transparency.set', { opacity: 30 })
  await again.commit('transparency.set', { opacity: 40 })
  assert.deepEqual(steps.at(-1), ['transparency.set', { opacity: 40 }])
  assert.equal(steps.length, 2)
})

test('a cancelled gesture leaves the drawing as it was, and never takes away someone else\'s step', async () => {
  const { steps, engine, foreign } = fakeEngine()
  const live = createLiveEdit(engine)
  live.set('paint.setFill', { color: '#ff0000' })
  live.set('paint.setFill', { color: '#00ff00' })
  await live.cancel()
  assert.deepEqual(steps, [])
  const other = createLiveEdit(engine)
  other.set('paint.setFill', { color: '#0000ff' })
  await new Promise((resolve) => setTimeout(resolve, 10))
  foreign()
  await other.cancel()
  assert.equal(steps.length, 2, 'the live step stays when another edit came after it')
})

test('the panels\' pure parts', () => {
  assert.deepEqual(dashFields([4, 2]), [4, 2, null, null, null, null])
  assert.deepEqual(withDash([12, 12], 0, 4), [4, 12])
  assert.deepEqual(withDash([12, 12], 2, 6), [12, 12, 6, 6])
  assert.deepEqual(withDash([4, 2, 6, 3], 2, null), [4, 2])
  assert.deepEqual(alignTo('selection', 1), { to: 'artboard' })
  assert.deepEqual(alignTo('selection', 3), {})
  assert.deepEqual(alignTo('key', 2), { to: 'key' })
  assert.deepEqual(swatchView({ name: 'K=40', kind: 'color', group: 'Grays', color: { model: 'gray', k: 0.4 }, hex: '#999999' }), { kind: 'color', hex: '#999999' })
  assert.deepEqual(swatchView({ name: 'K=40', kind: 'color', group: 'Grays', color: { model: 'gray', k: 0.4 } }), { kind: 'color', hex: '#999999' })
  assert.equal(swatchView({ name: 'Sunset', kind: 'gradient', group: null, gradient: 'linear', stops: [{ offset: 0, color: '#ff0000' }, { offset: 1, color: '#0000ff' }] }).kind, 'gradient')
  assert.deepEqual(swatchView({ name: '[None]', kind: 'none', group: null }), { kind: 'none' })
  assert.deepEqual(boundsParams('x', 10, 4), { x: 10, reference: 4 })
  assert.deepEqual(referencePoint({ x: 10, y: 20, width: 100, height: 50 }, 8), { x: 110, y: 70 })
  assert.deepEqual(referencePoint({ x: 10, y: 20, width: 100, height: 50 }, 4), { x: 60, y: 45 })
})

test('every arrowhead, stroke option and blending mode the panels show has a Chinese label', () => {
  const zh = CATALOGS['zh-CN']
  for (const name of ARROWHEADS) assert.ok(zh[`arrow.${name}`], name)
  for (const key of Object.keys(CATALOGS['en-US']).filter((key) => /^(color|paint|stroke|gradient|swatch|align|transform|appearance|control|pathfinder)\./.test(key))) assert.ok(zh[key], key)
  assert.equal(BLEND_MODES.length, 16)
})

// What the panels send, run by the engine.
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
const execute = (doc, command, params = {}) => JSON.parse(doc.execute(command, JSON.stringify(params)) || 'null')

test('the swatches list their gradients\' stops, and every swatch draws as itself (issue-swatch-slashes)', { skip }, () => {
  const doc = open()
  const { swatches } = execute(doc, 'swatch.list')
  for (const swatch of swatches) {
    const view = swatchView(swatch)
    if (swatch.kind === 'none') assert.equal(view.kind, 'none')
    else assert.notEqual(view.kind, 'none', `${swatch.name} draws as a slash`)
  }
  const sunsetSwatch = swatches.find((swatch) => swatch.name === 'Sunset')
  assert.ok(sunsetSwatch.stops.length >= 2)
  assert.match(swatchView(sunsetSwatch).css, /^linear-gradient/)
})

test('a click on any swatch paints with it, a gradient swatch too (issue-gradient-swatch)', { skip }, () => {
  const doc = open()
  execute(doc, 'select.set', { ids: [3] })
  const { swatches } = execute(doc, 'swatch.list')
  for (const swatch of swatches.filter((item) => item.name !== '[Registration]')) {
    execute(doc, 'paint.setFill', swatchParams(swatch))
    const fill = execute(doc, 'paint.proxies').fill
    assert.equal(fill.type, { none: 'none', color: 'solid', gradient: 'gradient', pattern: 'pattern' }[swatch.kind], swatch.name)
  }
  assert.deepEqual(swatchParams({ name: '[None]', kind: 'none' }), { none: true })
})

test('the Gradient panel\'s edits keep the gradient: stops, a stop\'s colour, reverse, a gradient swatch', { skip }, () => {
  const doc = open()
  execute(doc, 'select.set', { ids: [3] })
  execute(doc, 'paint.setFill', { swatch: 'Sunset' })
  let fill = execute(doc, 'paint.proxies').fill
  assert.equal(fill.type, 'gradient')
  const { draft } = addStop(fromPaint(fill), 0.5)
  execute(doc, 'paint.editGradient', { stroke: false, stops: engineStops(setStop(draft, 1, { color: '#2266ff' })) })
  fill = execute(doc, 'paint.proxies').fill
  assert.equal(fill.type, 'gradient')
  assert.equal(fill.gradient.stops.length, 3)
  assert.equal(colorHex(fill.gradient.stops[1].color), '#2266ff')
  execute(doc, 'paint.editGradient', { stroke: false, stops: engineStops(reversed(fromPaint(fill))) })
  assert.equal(colorHex(execute(doc, 'paint.proxies').fill.gradient.stops[2].color), colorHex(fill.gradient.stops[0].color))
  // The canvas annotator's stop: the picker's colour recolours that stop and keeps the gradient.
  execute(doc, 'paint.sampleColor', { color: { c: 0, m: 0, y: 1, k: 0 }, stop: 0 })
  fill = execute(doc, 'paint.proxies').fill
  assert.equal(fill.type, 'gradient')
  assert.equal(colorHex(fill.gradient.stops[0].color), '#ffff00')
})

test('the Stroke, Appearance and Align panels\' parameters run', { skip }, () => {
  const doc = open()
  execute(doc, 'select.set', { ids: [3] })
  execute(doc, 'paint.setStroke', { color: '#000000' })
  for (const params of [{ weight: 4 }, { cap: 'round' }, { join: 'bevel' }, { miterLimit: 4 }, { align: 'inside' }, { dash: withDash([12, 12], 0, 4) }, { alignDashes: true }, { startArrow: 'Arrow' }, { endArrow: 'Circle' }, { arrowAlign: 'tip' }, { profile: 'lens' }]) {
    execute(doc, 'stroke.set', params)
  }
  for (const blend of BLEND_MODES) execute(doc, 'transparency.set', { blend })
  execute(doc, 'transparency.set', { opacity: 40 })
  assert.equal(execute(doc, 'transparency.info').opacity, 40)
  execute(doc, 'object.setBounds', boundsParams('x', 100, 4))
  const bounds = JSON.parse(doc.inspect()).selectionBounds
  assert.equal(Math.round(bounds.x + bounds.width / 2), 100)
  execute(doc, 'select.set', { ids: [2, 3, 5] })
  execute(doc, 'object.distribute', { horizontal: 'center' })
  execute(doc, 'object.distributeSpacing', { axis: 'vertical' })
  execute(doc, 'object.align', { horizontal: 'left', ...alignTo('artboard', 3) })
  for (const command of ['unite', 'minusFront', 'intersect', 'exclude', 'divide', 'trim', 'merge', 'crop', 'outline', 'minusBack']) assert.ok(JSON.parse(doc.commands()).some((item) => item.id === `object.pathfinder.${command}`), command)
})
