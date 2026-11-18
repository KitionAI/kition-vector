// The editor's flow in a real browser, through the development host: node tests/flow.mjs [chromium|webkit]
// Starts the Vite dev server itself, opens dev.html in Chinese, and prints PASS or FAIL per step.
import { chromium, webkit } from 'playwright'
import { createServer } from 'vite'

const which = process.argv[2] ?? 'chromium'
const server = await createServer({ configFile: new URL('../vite.config.ts', import.meta.url).pathname, server: { port: 0, host: '127.0.0.1' }, logLevel: 'error' })
await server.listen()
const origin = `http://127.0.0.1:${server.httpServer.address().port}`
const browser = await (which === 'webkit' ? webkit : chromium).launch()
const context = await browser.newContext({ viewport: { width: 1440, height: 900 } })
if (which === 'chromium') await context.grantPermissions(['clipboard-read', 'clipboard-write'])
const page = await context.newPage()
const errors = []
let failures = 0
page.on('pageerror', (error) => errors.push(error.message))
const step = (name, ok, detail = '') => {
  if (!ok) failures += 1
  console.log(`${ok ? 'PASS' : 'FAIL'} ${name}${detail ? ` - ${detail}` : ''}`)
}
const until = async (check, tries = 60) => {
  for (let i = 0; i < tries; i += 1) {
    if (await check().catch(() => false)) return true
    await page.waitForTimeout(100)
  }
  return false
}
const inspect = () => page.evaluate(async () => window.devHost.connection().ai.inspect())
const details = async () => (await inspect()).details

try {
  await page.goto(`${origin}/dev.html?locale=zh-CN`)
  await page.waitForFunction(() => document.getElementById('status')?.textContent?.includes(' open in '), null, { timeout: 60000 })
  const kit = page.frameLocator('#kit')
  await page.waitForTimeout(1000)
  step('Chinese menu names', (await kit.getByTestId('vector-menu-Object').textContent()) === '对象')
  step('status counts the objects', (await kit.getByTestId('vector-status').textContent())?.startsWith('3 个对象'))
  const region = await kit.getByTestId('vector-canvas-region').boundingBox()
  const shape = (await details()).layers.flatMap((layer) => layer.children).find((node) => node.kind.includes('Rect'))
  // The artboard is fitted; map its points to the screen.
  const toScreen = (x, y) => ({ x: region.x + 85 + (x / 800) * (region.width - 170), y: region.y + 30 + (y / 600) * ((region.width - 170) * 0.75) })
  const center = toScreen(shape.bounds.x + shape.bounds.width / 2, shape.bounds.y + shape.bounds.height / 2)
  await page.mouse.click(center.x, center.y)
  await until(async () => (await details()).selection.length === 1)
  step('selects a shape on the canvas', (await details()).selection.length === 1)
  await until(async () => (await kit.getByTestId('vector-bounds-W').count()) === 1)
  step('the transform fields show its size', (await kit.getByTestId('vector-bounds-W').inputValue()) === '260')
  await kit.getByTestId('vector-bounds-W').fill('300')
  await kit.getByTestId('vector-bounds-W').press('Enter')
  await until(async () => Math.round((await details()).selectionBounds.width) === 300)
  step('sets the width', Math.round((await details()).selectionBounds.width) === 300)
  await kit.getByTestId('vector-fill').fill('#e03131')
  const fillOf = async () => (await details()).layers.flatMap((layer) => layer.children).find((node) => node.id === shape.id)?.fill
  await until(async () => (await fillOf()) === '#e03131')
  step('sets the fill', (await fillOf()) === '#e03131')
  step('the fill swatch shows the selection', (await kit.getByTestId('vector-fill').inputValue()) === '#e03131')
  await kit.getByTestId('vector-align-left').click()
  await until(async () => (await details()).selectionBounds.x < shape.bounds.x)
  step('aligns the selection', (await details()).selectionBounds.x < shape.bounds.x)

  await kit.getByTestId('vector-tool-rectangle').click({ button: 'right' })
  await kit.getByTestId('vector-tool-ellipse').click()
  await until(async () => (await kit.getByTestId('vector-tool-ellipse').getAttribute('aria-pressed')) === 'true')
  step('chooses a tool from a flyout', (await kit.getByTestId('vector-tool-ellipse').getAttribute('aria-pressed')) === 'true')
  const from = toScreen(500, 420)
  await page.mouse.move(from.x, from.y)
  await page.mouse.down()
  await page.mouse.move(from.x + 60, from.y + 40, { steps: 5 })
  await page.mouse.up()
  const count = async () => (await details()).layers.flatMap((layer) => layer.children).length
  await until(async () => (await count()) === 4)
  step('draws an ellipse with the tool', (await count()) === 4)

  await page.evaluate(() => window.devHost.connection().ai.run([{ command: 'select.all', params: {} }], false))
  await kit.getByTestId('vector-menu-Object').click()
  await kit.getByTestId('vector-cmd-object.group').click()
  await until(async () => (await details()).layers.some((layer) => layer.children.some((node) => node.kind === 'Group')))
  step('Object > Group from the menu bar', (await details()).layers.some((layer) => layer.children.some((node) => node.kind === 'Group')))
  await kit.getByTestId('vector-new-layer').click()
  await until(async () => (await details()).layers.length === 3)
  step('adds a layer', (await details()).layers.length === 3)
  const top = (await details()).layers[0].id
  await kit.getByTestId(`vector-layer-visibility-${top}`).click()
  await until(async () => (await details()).layers[0].visible === false)
  step('hides a layer', (await details()).layers[0].visible === false)

  const run = await page.evaluate(async () => window.devHost.connection().ai.run([{ command: 'shape.rectangle', params: { x: 20, y: 20, width: 40, height: 40 } }], false))
  await page.waitForTimeout(300)
  step('the agent edits the live drawing', run.completed[0].changed && (await page.evaluate(() => window.devHost.changes.at(-1).by)) === 'agent')
  await kit.getByTestId('vector-save').click()
  await until(async () => (await page.evaluate(() => window.devHost.files.get('Sample.kvector').version)) > 1)
  step('saves through the host', (await page.evaluate(() => window.devHost.files.get('Sample.kvector').version)) > 1)
  // File › Export: the engine writes the file into memory, the host writes it next to the drawing.
  const exported = (name) => page.evaluate((file) => window.devHost.files.get(file)?.bytes?.byteLength ?? 0, name)
  const head = (name, count) => page.evaluate(([file, n]) => [...new Uint8Array(window.devHost.files.get(file).bytes).slice(0, n)], [name, count])
  const toast = () => kit.locator('.kui-toast').last().textContent().catch(() => '')
  const openExport = async () => {
    await kit.getByTestId('vector-menu-File').click()
    await kit.getByTestId('vector-menu-export').click()
  }
  await openExport()
  await kit.getByTestId('vector-export-format').selectOption('svg')
  await kit.getByTestId('vector-export-run').click()
  await until(async () => (await exported('Sample.svg')) > 0, 200)
  step('exports SVG through the host', String.fromCharCode(...(await head('Sample.svg', 200))).includes('<svg'))
  step('says it exported once the host wrote the file', (await toast()).includes('Sample.svg'), await toast())
  await openExport()
  await kit.getByTestId('vector-export-format').selectOption('pdf')
  await kit.getByTestId('vector-export-run').click()
  await until(async () => (await exported('Sample.pdf')) > 0, 200)
  step('exports PDF through the host', String.fromCharCode(...(await head('Sample.pdf', 5))) === '%PDF-')
  await openExport()
  await kit.getByTestId('vector-export-format').selectOption('png')
  await kit.getByTestId('vector-export-scope').selectOption('artboard:0')
  await kit.getByTestId('vector-export-scale').selectOption('2')
  await kit.getByTestId('vector-export-run').click()
  await until(async () => (await exported('Sample.png')) > 0, 200)
  step('exports an artboard as PNG through the host', JSON.stringify(await head('Sample.png', 4)) === '[137,80,78,71]')
  // The host refuses the write: its error is shown, and no success.
  await page.evaluate(() => {
    const files = window.devHost.files
    const set = files.set.bind(files)
    files.set = (name, file) => {
      if (name.startsWith('Sample 2.')) throw new Error('the workspace is read-only')
      return set(name, file)
    }
  })
  await openExport()
  await kit.getByTestId('vector-export-format').selectOption('svg')
  await kit.getByTestId('vector-export-run').click()
  await until(async () => (await toast()).includes('read-only'), 100)
  step("shows the host's error when it does not write the file", (await toast()).includes('read-only') && !(await toast()).includes('已导出'), await toast())
  step('the dialog stays open after a refusal', await kit.getByTestId('vector-export-dialog').isVisible())
  await kit.getByTestId('vector-export-cancel').click()
} catch (error) {
  failures += 1
  console.log(`FAIL ${error instanceof Error ? error.message.split('\n')[0] : String(error)}`)
} finally {
  if (errors.length) console.log(`page errors:\n${errors.slice(0, 8).join('\n')}`)
  await browser.close()
  await server.close()
}
process.exit(failures ? 1 : 0)
