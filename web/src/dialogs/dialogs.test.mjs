// The dialog host's pure rules (plan 2026-11-25, stage 1, feature vector.editor.dialogs): routing,
// the generic dialog's fields, the Effect menu's routes, the document units, and the engine's errors
// in the editor's words. Run with `pnpm --dir web test:unit` (node --test).
import test from 'node:test'
import assert from 'node:assert/strict'
import { CATALOGS } from '../strings.ts'
import { ERROR_KINDS, errorMessage, explainError } from '../io/errors.ts'
import { hiddenReason, isHiddenEverywhere } from '../menus/hidden.ts'
import { booleanFromNotes, choicesFromNote, defaultFromDescription, effectFields, fieldFor, paramsOf, rangeFromText } from './params.ts'
import { dialogFor, isEllipsis, previews, withoutEllipsis } from './routes.ts'
import { APPLY_LAST, effectOfRow, effectRoute } from './effects.ts'
import { fromPoints, toPoints, unitAbbreviation } from './units.ts'

test('a "…" row with values to ask opens the generic dialog; the transforms open their own', () => {
  assert.equal(dialogFor({ id: 'object.path.offsetPath', title: 'Offset Path…', menu: ['Object', 'Path'], asks: true }), 'params')
  assert.equal(dialogFor({ id: 'object.path.offsetPath', title: 'Offset Path', menu: ['Object', 'Path'], asks: true }), null)
  assert.equal(dialogFor({ id: 'object.move', title: 'Move…', menu: ['Object', 'Transform'], asks: true }), 'move')
  assert.equal(dialogFor({ id: 'object.scale', title: 'Scale…', menu: ['Object', 'Transform'], asks: true }), 'scale')
  assert.equal(dialogFor({ id: 'object.path.cleanUp', title: 'Clean Up…', menu: ['Object', 'Path'], asks: false }), null)
  // A row that needs a value without "…" asks too; a hidden row never opens a dialog.
  assert.equal(dialogFor({ id: 'file.documentColorMode', title: 'Document Color Mode', menu: ['File'], asks: true }), 'params')
  assert.equal(dialogFor({ id: 'layer.new', title: 'New Layer…', menu: ['Window', 'Layers'], asks: true }), null)
  assert.ok(previews('object.move') && previews('effect.apply') && !previews('object.rasterize') && !previews('file.setInfo'))
  assert.ok(isEllipsis('Rasterize…') && isEllipsis('Rasterize...') && !isEllipsis('Rasterize'))
  assert.equal(withoutEllipsis('Offset Path…'), 'Offset Path')
})

test('the hidden table keeps the panel rows off the menus but their shortcuts on', () => {
  assert.ok(hiddenReason({ command: 'layer.new', path: ['Window', 'Layers'] }))
  assert.equal(hiddenReason({ command: 'object.pathfinder.unite', path: ['Window', 'Pathfinder'] }), null)
  assert.equal(hiddenReason({ command: 'object.move', path: ['Object', 'Transform'] }), null)
  assert.ok(hiddenReason({ command: 'file.place', path: ['File'] }))
  assert.ok(!isHiddenEverywhere('layer.new'), "New Layer's Cmd+L still runs it")
  assert.ok(isHiddenEverywhere('file.place') && isHiddenEverywhere('file.saveAs'))
})

test('the generic dialog reads ranges, defaults and choices from the schema and the notes', () => {
  assert.deepEqual(rangeFromText('-100 to 100; %'), { min: -100, max: 100 })
  assert.deepEqual(rangeFromText('points; 0–1000, room around the art'), { min: 0, max: 1000 })
  assert.deepEqual(rangeFromText('{size: 0..100 (5)'), { min: 0, max: 100 })
  assert.equal(defaultFromDescription('points; 12'), 12)
  assert.equal(defaultFromDescription('1 to 1000; 255'), 255)
  assert.equal(defaultFromDescription('degrees; counter-clockwise'), null)
  assert.equal(booleanFromNotes('{style?: arc (arc), horizontal?: bool (true) | orientation?: horizontal|vertical}', 'horizontal'), true)
  assert.equal(booleanFromNotes('{clip?: true (clip the dots), keepOriginal?: false}', 'keepOriginal'), false)
  assert.deepEqual(choicesFromNote(' "smooth"|"corner", seed: int'), ['smooth', 'corner'])
  assert.deepEqual(choicesFromNote(' transparent|white|black|"#rrggbb"'), ['transparent', 'white', 'black'])
  const command = { id: 'object.path.offsetPath', params: '{offset: pt (negative insets), joins?: "miter"|"round"|"bevel", miterLimit?: 4}' }
  // Offset Path's distance has no default in the schema: the dialog starts at 10 pt, so OK runs.
  assert.deepEqual(fieldFor(command, 'offset', { type: 'number', description: 'points; negative insets' }), { key: 'offset', kind: 'number', min: -1_000_000, max: 1_000_000, decimals: 2, initial: 10 })
  assert.equal(fieldFor(command, 'ids', { type: 'array' }), null)
  assert.equal(fieldFor(command, 'columns', { type: 'number', description: 'read by the command beside its listed parameters' }), null)
  // Templates are no options; a choice of one is no choice.
  assert.equal(fieldFor(command, 'target', { type: 'string', enum: ['workingCmyk'] }), null)
  assert.deepEqual(fieldFor({ id: 'view.proofSetup' }, 'target', { type: 'string', enum: ['workingCmyk', 'cmyk:<profile>', 'srgb'] }).options, ['workingCmyk', 'srgb'])
  // A range without a known start is a field to type, left to the engine while empty.
  const ppi = fieldFor({ id: 'object.flattenTransparency' }, 'lineArtPpi', { type: 'integer', description: '1 to 2400' })
  assert.equal(ppi.kind, 'number')
  assert.equal(ppi.initial, null)
  assert.deepEqual(paramsOf([ppi], { lineArtPpi: null }), {})
})

// The catalogue's defaults come in key order, as the engine's JSON map keeps them.
const SHADOW = { id: 'stylize.dropShadow', label: 'Drop Shadow…', menu: ['Effect', 'Stylize'], params: '{mode: blend mode ("multiply"), opacity: % (75), x: pt (7), y: pt (7), blur: pt (5), color: "#rrggbb" ("#000000")}', defaults: { blur: 5, color: '#000000', mode: 'multiply', opacity: 75, x: 7, y: 7 } }

test("an effect's dialog has a control per default value, and its row routes by its label", () => {
  const fields = effectFields(SHADOW)
  assert.deepEqual(
    fields.map((field) => [field.key, field.kind]),
    [['mode', 'choice'], ['opacity', 'slider'], ['x', 'number'], ['y', 'number'], ['blur', 'number'], ['color', 'color']],
    'in the order the notes name them, as the dialog reads',
  )
  assert.deepEqual(effectRoute(SHADOW), { kind: 'dialog' })
  assert.deepEqual(effectRoute({ ...SHADOW, id: 'path.outlineStroke', label: 'Outline Stroke', defaults: {} }), { kind: 'apply' })
  assert.equal(effectRoute({ ...SHADOW, id: 'distort.freeDistort', label: 'Free Distort…', defaults: { corners: [[0, 0]] } }).kind, 'hidden')
  assert.equal(effectRoute({ ...SHADOW, id: 'adjust.curves', label: 'Curves…' }).kind, 'hidden')
  assert.equal(effectOfRow('kit:effect.stylize.dropShadow'), 'stylize.dropShadow')
  assert.equal(effectOfRow(APPLY_LAST), null)
})

test('lengths show in the document units and go back as points', () => {
  assert.equal(fromPoints(72, 'Inches'), 1)
  assert.equal(fromPoints(72, 'Millimeters'), 25.4)
  assert.equal(toPoints(10, 'Millimeters').toFixed(4), '28.3465')
  assert.equal(unitAbbreviation('Centimeters'), 'cm')
  assert.equal(unitAbbreviation('Points'), 'pt')
})

test("the engine's errors read as the editor's sentences, in both languages", () => {
  for (const kind of ERROR_KINDS) {
    assert.ok(CATALOGS['en-US'][`error.${kind}`], `en-US error.${kind}`)
    assert.ok(CATALOGS['zh-CN'][`error.${kind}`], `zh-CN error.${kind}`)
  }
  assert.equal(explainError(new Error('invalid parameters for `object.scale`: missing number `sx`')).key, 'error.badValue')
  assert.equal(explainError('invalid parameters for `text.areaOptions`: no text objects selected').key, 'error.noText')
  assert.equal(explainError('Error: command `arrange.group` is not available right now: nothing selected').key, 'error.needsSelection')
  assert.equal(explainError('command `object.blend.make` is not available right now: select at least two objects').key, 'error.needsTwo')
  assert.equal(explainError('Intersect produced an empty result').key, 'error.emptyResult')
  assert.equal(explainError('path is already closed').key, 'error.openPaths')
  assert.equal(explainError('invalid parameters for `shaper.release`: select a Shaper group').key, 'error.wrongKind')
  const t = (key) => CATALOGS['zh-CN'][key] ?? key
  assert.equal(errorMessage(t, 'no previous transform'), '这里没有可执行的内容。')
  assert.equal(errorMessage(t, 'join needs open paths'), CATALOGS['zh-CN']['error.openPaths'])
  // An error the editor does not know keeps the engine's text under the general sentence, for a report.
  assert.equal(errorMessage(t, 'the moon is in the way'), `${CATALOGS['zh-CN']['error.generic']}\nthe moon is in the way`)
})
