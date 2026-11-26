// Files in and out without a browser (plan 2026-11-25, stage 7, feature vector.editor.files):
// what is opened and placed, a paste's best flavour and the command that takes it in, a copy's
// flavours for the system clipboard, Export's parameters, the slow-job indicator, the Layers
// panel's names in Chinese; and in the built engine: every Export setting is one the engine takes,
// text as outlines leaves no <text>, PNG at 2x is twice the pixels, pasted SVG, pictures and text
// come in as one step, a drawing read from SVG saves as a .kvector. Run with `pnpm --dir web test:unit`.
import { existsSync, readFileSync } from 'node:fs'
import test from 'node:test'
import assert from 'node:assert/strict'
import { copyName, copyTypes, copiedText, dragHasFiles, dropCenters, extensionOf, fingerprint, flavourOf, formatBytes, fromBase64, importOf, isNative, isPlaceable, isSvgText, nativeName, toBase64, OPEN_ACCEPT, PLACE_ACCEPT } from './files.ts'
import { EXPORT_FORMATS, answerSize, base64Size, defaultExport, exportParams, previewMime, previewParams } from './exportOptions.ts'
import { Busy } from './busy.ts'
import { explainError } from './errors.ts'
import { shownName } from '../layers/tree.ts'
import { CATALOGS, PHRASES } from '../strings.ts'

test('what File › Open… and Place… offer, and which drops are placed', () => {
  for (const extension of ['.kvector', '.svg', '.pdf', '.eps', '.ai']) assert.ok(OPEN_ACCEPT.includes(extension), extension)
  for (const extension of ['.png', '.jpg', '.svg', '.pdf', '.kvector']) assert.ok(PLACE_ACCEPT.includes(extension), extension)
  assert.ok(isPlaceable({ name: 'shot', type: 'image/png' }))
  assert.ok(isPlaceable({ name: 'Logo.SVG' }))
  assert.ok(!isPlaceable({ name: 'notes.docx', type: 'application/msword' }))
  assert.ok(dragHasFiles(['text/plain', 'Files']) && !dragHasFiles(['text/plain']) && !dragHasFiles(null))
  assert.deepEqual(dropCenters({ x: 10.4, y: 20 }, 3), [[10, 20], [34, 44], [58, 68]])
  assert.equal(extensionOf('a/b.Kvector'), 'kvector')
})

test('a drawing read from another format saves as a .kvector beside it, never over it', () => {
  assert.ok(isNative('brand/Logo.kvector') && !isNative('Poster.svg'))
  assert.equal(nativeName('brand/Poster.svg'), 'brand/Poster.kvector')
  assert.equal(nativeName('Scan.final.pdf'), 'Scan.final.kvector')
  assert.equal(copyName('brand/Logo.kvector'), 'brand/Logo copy.kvector')
  assert.equal(explainError(new Error('saving as .svg is an export; save the drawing as .kvector')).key, 'error.saveAsExport')
})

test('a paste takes the best flavour: SVG, then PDF, then a picture, then text', () => {
  const svg = '<?xml version="1.0"?>\n<svg xmlns="http://www.w3.org/2000/svg"><rect width="4" height="4"/></svg>'
  assert.ok(isSvgText(svg) && isSvgText('  <svg viewBox="0 0 1 1"></svg>') && !isSvgText('a <svg> in a sentence'))
  const center = [100, 50]
  assert.equal(importOf({ svg, text: svg }, center).command, 'clipboard.importSvg')
  assert.deepEqual(importOf({ svg }, center).params, { svg, center })
  assert.equal(importOf({ pdf: new Uint8Array([37, 80]), text: 'x' }, center).command, 'clipboard.importPdf')
  const picture = importOf({ image: { bytes: new Uint8Array([1, 2, 3]), type: 'image/png' }, text: 'shot' }, center)
  assert.deepEqual([picture.command, picture.params.mime, picture.params.dataBase64], ['clipboard.importImage', 'image/png', 'AQID'])
  assert.deepEqual(importOf({ text: 'Hello\r\nworld' }, center), { command: 'clipboard.importText', params: { text: 'Hello\nworld', center } })
  assert.equal(importOf({ text: '   ' }, center), null)
  assert.equal(importOf({}, center), null)
})

test("a copy goes onto the system clipboard as SVG, text and PNG, and is known again", async () => {
  const copied = { svg: '<svg/>', text: null, png: new Uint8Array([137, 80]) }
  assert.deepEqual(copyTypes(() => true), ['image/svg+xml', 'text/plain', 'image/png'])
  assert.deepEqual(copyTypes((type) => type !== 'image/svg+xml'), ['text/plain', 'image/png'])
  assert.equal(await flavourOf('text/plain', copied).text(), '<svg/>', 'vector apps paste SVG markup as text')
  assert.equal(await flavourOf('text/plain', { ...copied, text: 'Kition' }).text(), 'Kition', "a type-only copy's words")
  assert.equal(flavourOf('image/png', { ...copied, png: null }), null)
  assert.equal(copiedText({ ...copied, text: 'Kition' }), 'Kition')
  assert.equal(fingerprint('<svg/>'), fingerprint('<svg/>'))
  assert.notEqual(fingerprint('<svg/>'), fingerprint('<svg />'))
  assert.deepEqual([...fromBase64(toBase64(new Uint8Array([0, 255, 7])))], [0, 255, 7])
  assert.equal(formatBytes(812), '812 B')
  assert.equal(formatBytes(14_540), '14.2 KB')
})

test("Export's parameters per format and scope", () => {
  const svg = exportParams({ ...defaultExport(), outlineText: true, embedFonts: true, decimals: 9 })
  assert.deepEqual(svg, { format: 'svg', useArtboards: true, range: 'all', svg: { outlineText: true, decimals: 7 } }, 'outlined text needs no fonts')
  assert.deepEqual(exportParams({ ...defaultExport(), embedFonts: true, minify: true }).svg, { outlineText: false, decimals: 3, embedFonts: true, minify: true })
  assert.deepEqual(exportParams({ ...defaultExport(), format: 'pdf' }), { format: 'pdf' }, 'a PDF has a page per artboard')
  assert.deepEqual(exportParams({ ...defaultExport(), format: 'png', scope: 'artboard:1', scale: 2 }), { format: 'png', artboard: 1, scale: 2, antiAlias: 'art', background: 'transparent' })
  assert.deepEqual(exportParams({ ...defaultExport(), format: 'jpg', scope: 'selection', quality: 101, transparent: true }), { format: 'jpg', selectedOnly: true, useArtboards: false, scale: 1, antiAlias: 'art', background: 'white', quality: 100 }, 'JPEG is always on its background')
  assert.equal(exportParams({ ...defaultExport(), format: 'webp', transparent: false, background: 'black' }).background, 'black')
  assert.deepEqual(previewParams({ ...defaultExport(), format: 'pdf', scope: 'artboard:0' }, 640), { format: 'png', artboard: 0, scale: 0.5, background: 'transparent' })
  assert.equal(previewMime('pdf'), null)
  assert.equal(previewMime('svg'), 'image/svg+xml')
  assert.equal(base64Size('QUJD'), 3)
  assert.equal(base64Size('QUI='), 2)
  assert.deepEqual(answerSize({ data: 'QUJD', files: [{ data: 'QUJD' }, { data: 'QUI=' }] }), { files: 2, bytes: 5 })
})

test('a slow job shows after half a second, and Cancel drops its result', () => {
  let now = 0
  const tasks = []
  const clock = { now: () => now, schedule: (task) => tasks.push(task), unschedule: () => undefined }
  const busy = new Busy(clock)
  const quick = busy.begin('Exporting')
  assert.equal(busy.end(quick), false)
  const slow = busy.begin('Placing')
  now = 600
  for (const task of tasks.splice(0)) task()
  assert.equal(busy.current?.label, 'Placing')
  assert.ok(busy.cancel())
  assert.equal(busy.current, null)
  assert.equal(busy.end(slow), true, 'its result is dropped')
})

test("the Layers panel's made-up names are shown in the person's language", () => {
  const zh = (english) => ({ Path: '路径', Layer: '图层', Group: '编组' })[english] ?? english
  assert.equal(shownName('<Path>', 'Path', zh), '<路径>')
  assert.equal(shownName('Layer 3', 'Layer', zh), '图层 3')
  assert.equal(shownName('<Path>', 'Group', zh), '<Path>', 'a name someone typed stays')
  assert.equal(shownName('Logo', 'Path', zh), 'Logo')
  assert.equal(PHRASES['zh-CN'].Shaper, '塑形', 'Object › Shaper has its Chinese')
  for (const key of ['file.openMenu', 'file.placeMenu', 'exportOutlineText', 'drop.place', 'clipboard.copyLocal', 'busy.export', 'error.saveAsExport']) assert.ok(CATALOGS['zh-CN'][key] && CATALOGS['en-US'][key], key)
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
const sample = readFileSync(new URL('../../public/sample/Sample.kvector', import.meta.url))
const drawing = () => {
  const doc = sdk.VectorDocument.open('Sample.kvector', sample)
  const execute = (command, params = {}) => JSON.parse(doc.execute(command, JSON.stringify(params)) || 'null')
  const history = () => JSON.parse(doc.inspect()).history.length
  return { doc, execute, history }
}
const pngSize = (bytes) => [((bytes[16] << 24) | (bytes[17] << 16) | (bytes[18] << 8) | bytes[19]) >>> 0, ((bytes[20] << 24) | (bytes[21] << 16) | (bytes[22] << 8) | bytes[23]) >>> 0]

test('every Export setting is one the engine takes', { skip }, () => {
  const { execute } = drawing()
  execute('text.add', { x: 40, y: 40, text: 'Kition' })
  execute('edit.selectAll')
  for (const format of EXPORT_FORMATS) {
    for (const scope of ['all', 'artboard:0', 'selection']) {
      const settings = { ...defaultExport(), format, scope, embedFonts: true, minify: true, responsive: true, transparent: false }
      const answer = execute('file.exportBytes', exportParams(settings))
      assert.ok(answer.data && base64Size(answer.data) > 0, `${format} ${scope}`)
      if (format === 'pdf') assert.ok(execute('file.exportBytes', previewParams(settings, 800)).data, `${format} ${scope} preview`)
    }
  }
})

test('SVG with text as outlines has no <text>; PNG at 2x has twice the pixels', { skip }, () => {
  const { execute } = drawing()
  execute('text.add', { x: 40, y: 40, text: 'Kition' })
  const svg = (outlineText) => new TextDecoder().decode(fromBase64(execute('file.exportBytes', exportParams({ ...defaultExport(), outlineText })).data))
  assert.ok(svg(false).includes('<text'))
  assert.ok(!svg(true).includes('<text') && svg(true).includes('<path'))
  const size = (scale) => pngSize(fromBase64(execute('file.exportBytes', exportParams({ ...defaultExport(), format: 'png', scope: 'artboard:0', scale })).data))
  const [one, two] = [size(1), size(2)]
  assert.deepEqual(two, [one[0] * 2, one[1] * 2])
})

test('SVG markup, a picture and text from the clipboard come in as one step each', { skip }, () => {
  const { execute, history } = drawing()
  const take = (pasted) => {
    const taken = importOf(pasted, [200, 150])
    const before = history()
    execute(taken.command, taken.params)
    assert.equal(history(), before, 'loading the clipboard is no step')
    const answer = execute('edit.pasteInPlace')
    assert.equal(history(), before + 1)
    return answer.ids
  }
  assert.equal(take({ svg: '<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><rect width="10" height="10" fill="red"/></svg>' }).length, 1)
  execute('edit.deselectAll')
  const square = fromBase64('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAIAAACQd1PeAAAADElEQVR4nGP4z8AAAAMBAQDJ/pLvAAAAAElFTkSuQmCC')
  assert.equal(take({ image: { bytes: square, type: 'image/png' } }).length, 1)
  assert.equal(take({ text: '你好' }).length, 1)
})

test('a drawing read from SVG is saved as a .kvector (its own Save refuses to write over the SVG)', { skip }, () => {
  const { execute } = drawing()
  const svg = execute('file.exportBytes', { format: 'svg' }).data
  const read = sdk.VectorDocument.open('Poster.svg', fromBase64(svg))
  assert.throws(() => read.save(), /saving as \.svg is an export/)
  const native = JSON.parse(read.execute('file.serialize', JSON.stringify({ format: 'kvector' })))
  const again = sdk.VectorDocument.open(nativeName('Poster.svg'), fromBase64(native.dataBase64))
  assert.equal(JSON.parse(again.inspect()).artboards.length, 1)
})
