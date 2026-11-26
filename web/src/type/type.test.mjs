// Type's pure parts (plan 2026-11-25, stage 5, feature vector.editor.type): what the engine reports
// of the Type tool, what the Character and Paragraph panels send, the font catalog and recent list,
// the caret's blink, the keys that go to the text; and, against the built web engine when it is
// there, the requests and commands they send. Run with `pnpm --dir web test:unit`.
import { existsSync, readFileSync } from 'node:fs'
import test from 'node:test'
import assert from 'node:assert/strict'
import { ALIGNMENTS, NO_TYPING, byteSlice, characterStep, hasRange, labelPaint, paragraphStep, typeRequest, typingOf } from './typing.ts'
import { fontList, parseManifest, recentOf, withRecent } from './catalog.ts'
import { caretHidden, BLINK_MS } from './caret.ts'
import { canvasKey } from '../canvas/keys.ts'
import { CATALOGS, PHRASES } from '../strings.ts'

const editing = (start, end) => typingOf({ editing: 7, start, end, composing: false, caret: [1, 2, 1, 14], newType: {}, character: { size: 12, leading: 'auto', kerning: 'auto', fill: '#ff0000' }, paragraph: { justify: 'auto' } })

test('typing as the engine reports it, and nothing for anything else', () => {
  const t = editing(2, 5)
  assert.equal(t.editing, 7)
  assert.deepEqual(t.caret, [1, 2, 1, 14])
  assert.equal(t.character.leading, 'auto')
  assert.equal(t.character.size, 12)
  assert.equal(t.character.font, null, 'a missing (mixed) attribute is null')
  assert.equal(hasRange(t), true)
  assert.equal(hasRange(editing(3, 3)), false)
  assert.deepEqual(typingOf(null), NO_TYPING)
  assert.deepEqual(typingOf({ caret: [1, 'x', 3, 4] }).caret, null)
})

test('character attributes go to the selected characters, else the whole type, else new type', () => {
  // Characters selected in the type being edited: those only.
  assert.deepEqual(characterStep(editing(2, 5), true, { size: 36 }), { command: 'text.setRangeStyle', params: { id: 7, start: 2, end: 5, size: 36 } })
  // A caret: all of its text.
  assert.deepEqual(characterStep(editing(3, 3), true, { fill: '#ff0000' }), { command: 'text.setRangeStyle', params: { id: 7, fill: '#ff0000' } })
  // Type objects selected: every one (text.setStyle's attributes, else text.setFormat's).
  assert.deepEqual(characterStep(NO_TYPING, true, { font: 'Lora' }), { command: 'text.setStyle', params: { font: 'Lora' } })
  assert.deepEqual(characterStep(NO_TYPING, true, { underline: true }), { command: 'text.setFormat', params: { underline: true } })
  // Nothing: what new type is set in (family, style and size only).
  assert.deepEqual(characterStep(NO_TYPING, false, { size: 30, tracking: 100 }), { request: { newType: { size: 30 } } })
  assert.equal(characterStep(NO_TYPING, false, { tracking: 100 }), null)
  assert.equal(typeRequest({ newType: { size: 30 } }), 'Type:{"newType":{"size":30}}')
})

test('paragraph attributes go to the paragraphs the selection or the caret touches', () => {
  assert.deepEqual(paragraphStep(editing(3, 3), { justify: 'center' }), { command: 'text.setStyle', params: { id: 7, start: 3, end: 3, justify: 'center' } })
  assert.deepEqual(paragraphStep(NO_TYPING, { spaceBefore: 12 }), { command: 'text.setFormat', params: { spaceBefore: 12 } })
  assert.deepEqual([...ALIGNMENTS], ['left', 'center', 'right', 'justifyLeft', 'justifyCenter', 'justifyRight', 'justifyAll'])
})

test('paint labels read as paints; byte offsets as the engine counts them', () => {
  assert.deepEqual(labelPaint('None'), { type: 'none' })
  assert.deepEqual(labelPaint('#e03131'), { type: 'solid', color: '#e03131' })
  assert.deepEqual(labelPaint('Brand (#123456)'), { type: 'solid', color: '#123456' })
  assert.equal(labelPaint('Linear gradient').type, 'gradient')
  assert.equal(labelPaint(null), null)
  assert.equal(byteSlice('A中文B', 1, 7), '中文')
  assert.equal(byteSlice('Hello world', 6, 11), 'world')
})

test('the font menu: served families, recently used first, a missing family kept', () => {
  const catalog = parseManifest('# x\nLora | Regular | fonts/lora/Lora-Variable.ttf | Latn,Cyrl | OFL-1.1\nBad | Regular | ../escape.ttf | Latn\nMa Shan Zheng | Regular | fonts/ma-shan-zheng/MaShanZheng-Regular.ttf | Hans')
  assert.deepEqual([...catalog.keys()], ['Lora', 'Ma Shan Zheng'])
  const list = fontList(['Inter', 'Source Sans 3'], catalog, new Set(['Ma Shan Zheng']), 'Gone')
  assert.deepEqual(list.map((row) => `${row.family}:${row.state}`), ['Gone:missing', 'Inter:loaded', 'Lora:served', 'Ma Shan Zheng:loading', 'Source Sans 3:loaded'])
  assert.deepEqual(withRecent(['Inter', 'Lora', 'A', 'B', 'C'], 'lora'), ['lora', 'Inter', 'A', 'B', 'C'])
  assert.equal(recentOf('nope'), undefined)
  assert.deepEqual(recentOf(['a']), ['a'])
})

test('the caret shows, then hides, each half period', () => {
  assert.equal(caretHidden(0, 0), false)
  assert.equal(caretHidden(BLINK_MS + 1, 0), true)
  assert.equal(caretHidden(2 * BLINK_MS + 1, 0), false)
})

test('Cmd with the caret keys goes to the text being typed, not to the editor', () => {
  const key = (k, extra = {}) => ({ key: k, code: '', metaKey: true, ctrlKey: false, altKey: false, shiftKey: false, ...extra })
  const context = (editingText) => ({ toolKeys: [], editingText, selection: true, singleKeys: new Map(), toolLetter: () => false })
  assert.deepEqual(canvasKey(key('ArrowLeft'), context(true)), { kind: 'textKey', key: 'ArrowLeft' })
  assert.deepEqual(canvasKey(key('Backspace', { shiftKey: true }), context(true)), { kind: 'textKey', key: 'Backspace' })
  assert.equal(canvasKey(key('ArrowLeft'), context(false)), null)
  assert.equal(canvasKey(key('a'), context(true)), null, 'Cmd+A stays Select All (which selects the text)')
})

test('every type string has Chinese', () => {
  for (const key of Object.keys(CATALOGS['en-US']).filter((k) => k.startsWith('type.') || k.startsWith('para.'))) assert.ok(CATALOGS['zh-CN'][key], key)
  assert.equal(PHRASES['zh-CN'].Character, '字符')
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
const NO_MODS = JSON.stringify({ shift: false, alt: false, command: false, control: false, space: false })

const typed = () => {
  const doc = sdk.VectorDocument.create('type.kvector', 600, 400)
  const execute = (command, params = {}) => JSON.parse(doc.execute(command, JSON.stringify(params)) || 'null')
  const inspect = () => JSON.parse(doc.inspect())
  const id = execute('text.add', { x: 50, y: 100, text: 'Hello world', size: 24, color: '#000000' }).id
  doc.selectTool('type', 1)
  const [ox, oy] = inspect().origin
  for (const action of ['down', 'up']) doc.pointer(JSON.stringify({ action, x: 52 - ox, y: 95 - oy, mods: JSON.parse(NO_MODS), pressure: 1, zoom: 1 }))
  return { doc, execute, inspect, id }
}

test('the Character panel\'s steps on selected characters style them only (issue-range-colour)', { skip }, () => {
  const { doc, execute, inspect, id } = typed()
  doc.toolKey('Home', NO_MODS, 1)
  for (let i = 0; i < 6; i += 1) doc.toolKey('ArrowRight', NO_MODS, 1)
  for (let i = 0; i < 5; i += 1) doc.toolKey('ArrowRight', JSON.stringify({ shift: true }), 1)
  const typing = typingOf(inspect().typing)
  assert.deepEqual([typing.start, typing.end], [6, 11])
  const step = characterStep(typing, true, { size: 36 })
  execute(step.command, step.params)
  execute('paint.setFill', { color: '#e03131' })
  const runs = execute('text.getRange', { id }).runs
  assert.deepEqual(runs.map((run) => [run.text, run.style.size]), [['Hello ', 24], ['world', 36]])
  assert.notDeepEqual(runs[0].style.fill, runs[1].style.fill)
  assert.equal(typingOf(inspect().typing).character.fill, '#e03131')
})

test('an input method\'s marked text and new type\'s size are Type requests', { skip }, () => {
  const { doc, inspect, id } = typed()
  doc.toolKey(typeRequest({ preedit: { text: 'zhong' } }), NO_MODS, 1)
  const text = () => inspect().layers[0].children.find((node) => node.id === id).text
  assert.match(text(), /zhong/)
  assert.equal(inspect().typing.composing, true)
  doc.toolText('中', 1)
  assert.match(text(), /中/)
  assert.doesNotMatch(text(), /zhong/)
  doc.toolKey(typeRequest({ newType: { size: 30, font: 'Source Serif 4' } }), NO_MODS, 1)
  assert.deepEqual(inspect().typing.newType, { font: 'Source Serif 4', style: null, size: 30 })
})

test('a served family given to the engine joins its families', { skip }, () => {
  const lora = new URL('../../../../kition-fonts/fonts/lora/Lora-Variable.ttf', import.meta.url)
  if (!existsSync(lora)) return
  const { doc, execute } = typed()
  const answer = JSON.parse(doc.addFace(readFileSync(lora)))
  assert.ok(answer.faces > 0)
  assert.ok(execute('text.fontList').families.includes('Lora'))
})
