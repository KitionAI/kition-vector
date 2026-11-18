// Export's file handling: node --test src/exporting.test.mjs (Node strips the module's types).
import assert from 'node:assert/strict'
import { test } from 'node:test'
import { decodeBase64, exportFiles, exportFolder, exportStem, writeExports } from './exporting.ts'

const PNG = 'iVBORw0KGgo='
const doc = { name: 'Logo.kvector', path: 'brand/Logo.kvector' }

test('names the file after the drawing, next to it', () => {
  assert.equal(exportStem('Logo.kvector'), 'Logo')
  assert.equal(exportStem('a:b?.kvector'), 'a_b_')
  assert.equal(exportStem('.kvector'), 'Drawing')
  assert.equal(exportFolder('brand/Logo.kvector'), 'brand/')
  assert.equal(exportFolder('Logo.kvector'), '')
  const [svg] = exportFiles({ data: btoa('<svg/>') }, doc, 'svg')
  assert.equal(svg.path, 'brand/Logo.svg')
  assert.equal(new TextDecoder().decode(svg.bytes), '<svg/>')
})

test('one file per artboard takes the artboard names, else numbers', () => {
  const answer = { data: PNG, files: [{ name: 'x-1.png', data: PNG }, { name: 'x-2.png', data: PNG }] }
  assert.deepEqual(exportFiles(answer, doc, 'png', ['Icon', 'Banner/Wide']).map((file) => file.path), ['brand/Logo - Icon.png', 'brand/Logo - Banner_Wide.png'])
  assert.deepEqual(exportFiles(answer, doc, 'png', ['Same', 'Same']).map((file) => file.path), ['brand/Logo - 1.png', 'brand/Logo - 2.png'])
  assert.deepEqual(exportFiles(answer, doc, 'png').map((file) => file.path), ['brand/Logo - 1.png', 'brand/Logo - 2.png'])
  assert.deepEqual([...decodeBase64(PNG).slice(0, 4)], [0x89, 0x50, 0x4e, 0x47])
})

test('answers the paths the host wrote, and stops at its refusal', async () => {
  const files = exportFiles({ data: PNG }, doc, 'png')
  assert.deepEqual(await writeExports(files, async (path) => path.replace('.png', ' 2.png')), ['brand/Logo 2.png'], 'the host may choose an unused name')
  await assert.rejects(
    writeExports(files, async () => {
      throw new Error('the disk is full')
    }),
    /the disk is full/,
  )
})
