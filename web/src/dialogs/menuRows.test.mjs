// The menus against the built web engine (plan 2026-11-25, stage 1, feature vector.editor.dialogs):
// every visible "…" row opens a dialog or has nothing to ask, every left-out row says why, no row
// that runs at once fails for want of a value, every dialog's control has a Chinese label, and every
// dialog's starting values run. Skipped until `pnpm --dir web engine` has built the engine.
// Run with `pnpm --dir web test:unit` (node --test).
import { existsSync, readFileSync } from 'node:fs'
import test from 'node:test'
import assert from 'node:assert/strict'
import { hiddenReason, isHiddenRow } from '../menus/hidden.ts'
import { CATALOGS } from '../strings.ts'
import { explainError } from '../io/errors.ts'
import { effectFields, humanize, initialValues, paramFields, paramsOf } from './params.ts'
import { dialogFor, isEllipsis } from './routes.ts'
import { effectRoute, effectRows } from './effects.ts'

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
const commands = sdk ? JSON.parse(open().commands()) : []
const catalog = sdk ? JSON.parse(open().execute('effect.list', '{}')).catalog : []

/** The rows the engine's menus have (as the SDK's `commandMenuTabs` draws them: the menu and its first submenu). */
const menuRows = () => commands.filter((command) => command.menu?.length).map((command) => ({ command, row: { command: command.id, path: command.menu.slice(0, 2) } }))
const route = (command) => dialogFor({ id: command.id, title: command.title, menu: command.menu, asks: paramFields(command).length > 0 })
const visible = () => menuRows().filter(({ row }) => !isHiddenRow(row))

test('the engine keeps the "…" of the titles that ask first', { skip }, () => {
  assert.ok(commands.filter((command) => isEllipsis(command.title)).length > 50)
  assert.equal(commands.find((command) => command.id === 'object.path.offsetPath')?.title, 'Offset Path…')
})

test('every visible "…" row opens a dialog, or has nothing at all to ask', { skip }, () => {
  const bare = visible()
    .filter(({ command }) => isEllipsis(command.title) && route(command) === null)
    .map(({ command }) => command.id)
  assert.deepEqual(bare, [], 'rows that would run with the engine\'s defaults')
})

test('every row the menus leave out says why', { skip }, () => {
  for (const { row } of menuRows()) {
    if (!isHiddenRow(row)) continue
    const reason = hiddenReason(row)
    assert.ok(reason && reason.length > 20, `${row.command}: ${reason}`)
  }
})

/** A drawing with every kind of object the rows act on: the sample's shapes, a text object, an open path. */
function richDocument(selectAll) {
  const doc = open()
  doc.execute('text.add', JSON.stringify({ x: 40, y: 40, text: 'Kition' }))
  doc.execute('shape.line', JSON.stringify({ x1: 10, y1: 300, x2: 200, y2: 320 }))
  doc.execute(selectAll ? 'edit.selectAll' : 'edit.deselectAll', '{}')
  return doc
}

test('no visible row that runs at once fails for want of a value on a plain click', { skip }, () => {
  const wanting = []
  for (const { command } of visible()) {
    if (route(command) !== null) continue
    for (const selectAll of [false, true]) {
      const doc = richDocument(selectAll)
      try {
        if (JSON.parse(doc.disabledCommands()).includes(command.id)) continue
        doc.execute(command.id, '{}')
      } catch (error) {
        if (explainError(error).key === 'error.badValue') wanting.push(`${command.id}: ${String(error).slice(0, 140)}`)
      } finally {
        doc.free()
      }
    }
  }
  assert.deepEqual([...new Set(wanting)], [])
})

/** Upstream's Chinese phrases (plain UI text, the first column empty). */
const upstreamChinese = new Set(
  readFileSync(new URL('../i18n/upstream/zh-hans.tsv', import.meta.url), 'utf8')
    .split('\n')
    .filter((line) => !line.startsWith('#'))
    .map((line) => line.split('\t'))
    .filter(([context, , chinese]) => context === '' && chinese)
    .map(([, english]) => english),
)

/** Every generic dialog: its scope (the command or the effect) and its fields. */
const genericDialogs = () => [
  ...visible()
    .filter(({ command }) => route(command) === 'params')
    .map(({ command }) => ({ scope: command.id, fields: paramFields(command) })),
  ...catalog.filter((effect) => effectRoute(effect).kind === 'dialog').map((effect) => ({ scope: effect.id, fields: effectFields(effect) })),
]

test("every generic dialog's control and choice has a Chinese label (the editor's own or upstream's)", { skip }, () => {
  const zh = CATALOGS['zh-CN']
  const en = CATALOGS['en-US']
  const missing = []
  for (const { scope, fields } of genericDialogs()) {
    const family = scope.split('.')[0]
    for (const field of fields) {
      const own = zh[`param.${scope}.${field.key}`] ?? zh[`param.${family}.${field.key}`] ?? zh[`param.${field.key}`]
      if (!own && !upstreamChinese.has(humanize(field.key))) missing.push(`${scope}: ${field.key}`)
      if (field.kind === 'choice') for (const option of field.options) if (!zh[`option.${option}`] && !upstreamChinese.has(humanize(option))) missing.push(`${scope}: ${field.key}=${option}`)
    }
  }
  assert.deepEqual(missing, [], 'labels without a Chinese translation (add them to strings.ts)')
  // The editor's own strings exist in both languages.
  assert.deepEqual(Object.keys(zh).sort(), Object.keys(en).sort())
})

/** A drawing ready for `command`: a rich drawing with everything selected, or the objects it needs. */
function documentFor(command) {
  const doc = richDocument(true)
  if (command.startsWith('text.') || command === 'edit.findReplace') {
    doc.execute('edit.deselectAll', '{}')
    doc.execute('select.object.textObjects', '{}')
  }
  if (command === 'text.areaOptions') doc.execute('text.convertToAreaType', '{}')
  if (command === 'text.pathOptions') {
    const line = JSON.parse(doc.execute('shape.line', JSON.stringify({ x1: 20, y1: 500, x2: 400, y2: 520 }))).id
    doc.execute('text.addInPath', JSON.stringify({ path: line, mode: 'onPath', text: 'Kition' }))
  }
  if (command === 'object.textWrap.options') doc.execute('object.textWrap.make', '{}')
  return doc
}

test("every generic dialog's starting values run in the engine (the rows that failed bare included)", { skip }, () => {
  const refused = []
  let ran = 0
  for (const { command } of visible()) {
    if (route(command) !== 'params') continue
    const doc = documentFor(command.id)
    try {
      if (JSON.parse(doc.disabledCommands()).includes(command.id)) continue
      const fields = paramFields(command)
      const params = paramsOf(fields, initialValues(fields))
      // Find and Replace needs something to find; the dialog's OK waits for it.
      if (command.id === 'edit.findReplace') params.find = 'Kition'
      doc.execute(command.id, JSON.stringify(params))
      ran += 1
    } catch (error) {
      refused.push(`${command.id}: ${String(error).slice(0, 140)}`)
    } finally {
      doc.free()
    }
  }
  assert.deepEqual(refused, [], 'dialogs whose starting values the engine refuses')
  assert.ok(ran >= 20, `ran ${ran}`)
})

test('every effect the Effect menu shows applies with its dialog\'s starting values, as one step', { skip }, () => {
  const refused = []
  const rows = effectRows(catalog, null, { applyLast: () => 'Apply Last Effect', lastDialog: () => 'Last Effect…' })
  const shown = new Set(rows.map((row) => row.command.replace('kit:effect.', '')))
  for (const name of ['stylize.dropShadow', 'stylize.innerGlow', 'stylize.outerGlow', 'stylize.feather', 'blur.gaussian', 'stylize.roundCorners']) assert.ok(shown.has(name), `${name} is in the Effect menu`)
  for (const effect of catalog) {
    if (!shown.has(effect.id)) continue
    const doc = richDocument(true)
    try {
      // Revolve turns one profile path; the others act on every selected object.
      if (effect.id === 'threeD.revolve') {
        const line = JSON.parse(doc.execute('shape.line', JSON.stringify({ x1: 600, y1: 20, x2: 640, y2: 200 }))).id
        doc.execute('edit.deselectAll', '{}')
        doc.execute('select.set', JSON.stringify({ ids: [line] }))
      }
      const fields = effectFields(effect)
      const before = JSON.parse(doc.inspect()).history.length
      doc.execute('effect.apply', JSON.stringify({ effect: effect.id, params: paramsOf(fields, initialValues(fields)) }))
      assert.equal(JSON.parse(doc.inspect()).history.length, before + 1, effect.id)
    } catch (error) {
      refused.push(`${effect.id}: ${String(error).slice(0, 140)}`)
    } finally {
      doc.free()
    }
  }
  assert.deepEqual(refused, [])
  assert.ok(shown.size > 60, `${shown.size} effects`)
})
