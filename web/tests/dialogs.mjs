// The command dialogs in a real browser, through the development host: node tests/dialogs.mjs [chromium|webkit]
// Starts the Vite dev server itself, opens dev.html in Chinese, and prints PASS or FAIL per step
// (plan 2026-11-25, stage 1, feature vector.editor.dialogs: a "…" menu row asks first, previews,
// and Cancel and OK keep the history exact; Move…, File › New…, the Effect menu, Help, the menus'
// left-out rows, and the engine's errors in the editor's words). SHOTS=<folder> saves screenshots.
import { mkdirSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { chromium, webkit } from 'playwright'
import { createServer } from 'vite'

const which = process.argv[2] ?? 'chromium'
const shots = process.env.SHOTS ? fileURLToPath(new URL(process.env.SHOTS, `file://${process.cwd()}/`)) : null
if (shots) mkdirSync(shots, { recursive: true })
const server = await createServer({ configFile: fileURLToPath(new URL('../vite.config.ts', import.meta.url)), server: { port: 0, host: '127.0.0.1' }, logLevel: 'error' })
await server.listen()
const origin = `http://127.0.0.1:${server.httpServer.address().port}`
const browser = await (which === 'webkit' ? webkit : chromium).launch()
const context = await browser.newContext({ viewport: { width: 1440, height: 900 } })
const page = await context.newPage()
const errors = []
let failures = 0
page.on('pageerror', (error) => errors.push(error.message))
const step = (name, ok, detail = '') => {
  if (!ok) failures += 1
  console.log(`${ok ? 'PASS' : 'FAIL'} ${which} ${name}${detail ? ` - ${detail}` : ''}`)
}
const until = async (check, tries = 60) => {
  for (let i = 0; i < tries; i += 1) {
    if (await check().catch(() => false)) return true
    await page.waitForTimeout(100)
  }
  return false
}
const details = async () => (await page.evaluate(async () => window.devHost.connection().ai.inspect())).details
const run = (command, params = {}) => page.evaluate(([c, p]) => window.devHost.connection().ai.run([{ command: c, params: p }], false), [command, params])
const shot = async (name) => {
  if (shots) await page.screenshot({ path: `${shots}/${name}.png` })
}
const primary = async () => ((await page.evaluate(() => /Mac|iPhone|iPad/i.test(navigator.platform))) ? 'Meta' : 'Control')

try {
  await page.goto(`${origin}/dev.html?locale=zh-CN`)
  await page.waitForFunction(() => document.getElementById('status')?.textContent?.includes(' open in '), null, { timeout: 60000 })
  const kit = page.frameLocator('#kit')
  const dialog = kit.getByTestId('vector-dialog')
  const count = (testId) => kit.getByTestId(testId).count()
  await page.waitForTimeout(1000)

  /** A fingerprint of the drawing on the canvas (every 7th byte, hashed). */
  const picture = () =>
    kit.getByTestId('kit-canvas-surface').evaluate((canvas) => {
      const data = canvas.getContext('2d').getImageData(0, 0, canvas.width, canvas.height).data
      let hash = 2166136261
      for (let i = 0; i < data.length; i += 7) hash = Math.imul(hash ^ data[i], 16777619) >>> 0
      return hash
    })
  /** Waits for the canvas to settle (two equal fingerprints in a row) and answers it. */
  const settled = async () => {
    let last = await picture()
    for (let i = 0; i < 40; i += 1) {
      await page.waitForTimeout(150)
      const next = await picture()
      if (next === last) return next
      last = next
    }
    return last
  }
  const history = async () => JSON.stringify((await details()).history)
  /** Clicks a menu row, once the editor's state (a moment behind the engine's) enables it. */
  const openRow = async (menu, submenu, testId) => {
    for (let attempt = 0; attempt < 20; attempt += 1) {
      await kit.getByTestId(`vector-menu-${menu}`).click()
      if (submenu) await kit.locator(`[data-kit-ui-id="menu:${menu}/${submenu}"]`).first().hover()
      const row = kit.getByTestId(testId)
      await row.waitFor({ state: 'visible', timeout: 5000 })
      if (await row.isEnabled()) return row.click()
      await page.keyboard.press('Escape')
      await page.keyboard.press('Escape')
      await page.waitForTimeout(150)
    }
    throw new Error(`${testId} stays disabled`)
  }
  const closed = () => until(async () => (await dialog.count()) === 0, 40)
  const toast = () => kit.locator('.kui-toast').last().textContent().catch(() => '')

  // The rectangle of the sample, selected alone with a click on the canvas (as a person selects:
  // the editor's menus follow its own state, which an agent's selection does not move).
  const region = await kit.getByTestId('vector-canvas-region').boundingBox()
  const shape = (await details()).layers.flatMap((layer) => layer.children).find((node) => node.kind.includes('Rect'))
  // The artboard is fitted; map its points to the screen (as tests/flow.mjs does).
  const toScreen = (x, y) => ({ x: region.x + 85 + (x / 800) * (region.width - 170), y: region.y + 30 + (y / 600) * ((region.width - 170) * 0.75) })
  const selectShape = async () => {
    const center = toScreen(shape.bounds.x + shape.bounds.width / 2, shape.bounds.y + shape.bounds.height / 2)
    await page.mouse.click(center.x, center.y)
    await until(async () => (await details()).selection.length === 1)
  }
  await selectShape()
  const before = await settled()
  const historyBefore = await history()

  // Object › Path › Offset Path… opens its dialog, its distance labelled in Chinese.
  await openRow('Object', 'Path', 'vector-cmd-object.path.offsetPath')
  const opened = await until(async () => (await dialog.count()) === 1, 40)
  const offset = kit.getByTestId('vector-dialog-param-offset')
  const label = opened ? ((await kit.locator('label[for="vector-dialog-param-offset-input"]').textContent()) ?? '') : ''
  step('Offset Path… opens a dialog with its distance labelled in Chinese', opened && label === '偏移', JSON.stringify({ opened, label }))
  await shot('offset-path-dialog')
  // Preview off shows the drawing as it was: the menu click committed nothing.
  await kit.getByTestId('vector-dialog-preview').uncheck()
  await until(async () => (await history()) === historyBefore, 40)
  const previewOff = await settled()
  step('with Preview off the drawing and the history are as before the click', previewOff === before && (await history()) === historyBefore)
  await kit.getByTestId('vector-dialog-preview').check()
  const previewed = await until(async () => (await settled()) !== before, 40)
  step('Preview on shows the offset path on the canvas', previewed)
  const atDefault = await settled()
  await offset.fill('30')
  const changed = await until(async () => (await settled()) !== atDefault, 40)
  step('a new distance changes the preview', changed)
  await shot('offset-path-preview')
  // Cancel restores the drawing and leaves no extra history step.
  await kit.getByTestId('vector-dialog-cancel').click()
  await closed()
  await until(async () => (await history()) === historyBefore, 40)
  const cancelled = await settled()
  step('Cancel restores the drawing and adds no history step', (await dialog.count()) === 0 && cancelled === before && (await history()) === historyBefore, JSON.stringify({ history: await history(), historyBefore }))

  // OK leaves exactly one step: the offset with the dialog's values.
  await openRow('Object', 'Path', 'vector-cmd-object.path.offsetPath')
  await until(async () => (await dialog.count()) === 1, 40)
  await offset.fill('25')
  await page.waitForTimeout(600)
  await kit.getByTestId('vector-dialog-ok').click()
  await closed()
  await page.waitForTimeout(300)
  const was = JSON.parse(historyBefore)
  const states = (await details()).history
  step('OK adds exactly one history step', states.length === was.length + 1 && JSON.stringify(states.slice(0, -1)) === JSON.stringify(was), JSON.stringify({ last: states.at(-1), was: was.length, now: states.length }))
  // Undone, the next Offset Path… starts from the distance OK'd last.
  const undo = async () => {
    await kit.locator('.kui-app').focus()
    await page.keyboard.press(`${await primary()}+z`)
  }
  await undo()
  await until(async () => (await history()) === historyBefore, 40)
  await openRow('Object', 'Path', 'vector-cmd-object.path.offsetPath')
  await until(async () => (await dialog.count()) === 1, 40)
  step('the dialog opens with the distance it was last OK\'d with', (await offset.inputValue()) === '25', await offset.inputValue())
  await kit.getByTestId('vector-dialog-cancel').click()
  await closed()
  await until(async () => (await history()) === historyBefore, 40)

  // Object › Transform › Move…: 50 to the right, one step; Copy moves a copy.
  await selectShape()
  const x0 = (await details()).selectionBounds.x
  await openRow('Object', 'Transform', 'vector-cmd-object.move')
  await until(async () => (await dialog.count()) === 1, 40)
  await kit.getByTestId('vector-dialog-dx').fill('50')
  await kit.getByTestId('vector-dialog-dy').fill('0')
  await shot('move-dialog')
  await kit.getByTestId('vector-dialog-ok').click()
  await closed()
  await until(async () => Math.round((await details()).selectionBounds.x - x0) === 50, 40)
  const moved = (await details()).selectionBounds.x - x0
  step('Move… moves the selection by the distance typed, as one step', Math.round(moved) === 50 && (await details()).history.length === was.length + 1, `moved ${moved}`)
  const objects = async () => (await details()).layers.flatMap((layer) => layer.children).length
  const objectsBefore = await objects()
  // Its shortcut opens the same dialog; Copy leaves the original and moves a copy.
  await kit.locator('.kui-app').focus()
  await page.keyboard.press(`${await primary()}+Shift+M`)
  const byKey = await until(async () => (await dialog.count()) === 1, 40)
  step('Cmd+Shift+M opens Move…', byKey)
  if (byKey) {
    await kit.getByTestId('vector-dialog-copy').click()
    await closed()
  }
  await until(async () => (await objects()) === objectsBefore + 1, 40)
  // One step for the menu's Move, one for the copy: the shortcut ran nothing by itself.
  step('Copy moves a copy and keeps the original, and the shortcut ran nothing by itself', (await objects()) === objectsBefore + 1 && (await details()).history.length === was.length + 2)

  // Effect › Stylize › Drop Shadow…: a dialog from the effect catalogue, one step on OK.
  const steps = (await details()).history.length
  await openRow('Effect', 'Stylize', 'vector-control-effect.stylize.dropShadow')
  const shadow = await until(async () => (await dialog.count()) === 1, 40)
  step('Effect › Stylize › Drop Shadow… opens its dialog', shadow && (await count('vector-dialog-param-opacity')) === 1)
  await shot('drop-shadow-dialog')
  await page.waitForTimeout(400)
  await kit.getByTestId('vector-dialog-ok').click()
  await closed()
  await page.waitForTimeout(300)
  step('OK applies the drop shadow as one step', (await details()).history.length === steps + 1, JSON.stringify((await details()).history.slice(-2)))

  // Rows the editor cannot run yet are not in the menus; Pathfinder's stay under Window.
  await kit.getByTestId('vector-menu-File').click()
  const place = await count('vector-cmd-file.place')
  const newRow = await count('vector-menu-new')
  await page.keyboard.press('Escape')
  await kit.getByTestId('vector-menu-Window').click()
  const layerRow = await kit.locator('[data-kit-ui-id="menu:Window/Layers"]').count()
  const pathfinder = await kit.locator('[data-kit-ui-id="menu:Window/Pathfinder"]').count()
  await page.keyboard.press('Escape')
  step('File has New… and no Place yet; Window keeps Pathfinder, not the panels\' own rows', place === 0 && newRow === 1 && layerRow === 0 && pathfinder === 1, JSON.stringify({ place, newRow, layerRow, pathfinder }))

  // The engine's errors in the editor's words: Join on the closed rectangle, which has no open ends.
  await selectShape()
  await openRow('Object', 'Path', 'vector-cmd-path.join')
  await until(async () => (await toast()).length > 0, 40)
  const said = await toast()
  step('an engine error reads as a Chinese sentence, not the engine\'s English', /[一-鿿]/.test(said) && !/invalid parameters|not available right now/.test(said), said)

  // Help › Keyboard Shortcuts lists the tools and Move…'s shortcut.
  await openRow('Help', null, 'vector-control-help.shortcuts')
  const listed = await until(async () => (await count('vector-shortcuts')) === 1, 40)
  const text = listed ? ((await kit.getByTestId('vector-shortcuts').textContent()) ?? '') : ''
  step('Help › Keyboard Shortcuts lists tool keys and menu shortcuts', listed && text.includes('Cmd+Shift+M') && text.includes('Cmd+Alt+P'), text.slice(0, 80))
  await shot('help-shortcuts')
  await kit.getByTestId('vector-help-close').click()

  // File › New…: a preset, three artboards, written beside the drawing through the host.
  await openRow('File', null, 'vector-menu-new')
  const newOpen = await until(async () => (await dialog.count()) === 1, 40)
  step('File › New… opens the New Document dialog with presets', newOpen && (await kit.getByTestId('vector-dialog-preset').locator('option').count()) > 1)
  await kit.getByTestId('vector-dialog-name').locator('input').fill('Poster').catch(() => kit.getByTestId('vector-dialog-name').fill('Poster'))
  await kit.getByTestId('vector-dialog-artboards').fill('3')
  await shot('new-document-dialog')
  await kit.getByTestId('vector-dialog-ok').click()
  await closed()
  const written = await until(async () => (await page.evaluate(() => window.devHost.files.get('Poster.kvector')?.bytes?.byteLength ?? 0)) > 0, 100)
  step('Create writes Poster.kvector beside the drawing and says so', written && (await toast()).includes('Poster.kvector'), await toast())
} catch (error) {
  failures += 1
  console.log(`FAIL ${which} ${error instanceof Error ? error.message.split('\n').slice(0, 4).join(' | ') : String(error)}`)
} finally {
  if (errors.length) console.log(`page errors:\n${errors.slice(0, 8).join('\n')}`)
  await browser.close()
  await server.close()
}
process.exit(failures ? 1 : 0)
