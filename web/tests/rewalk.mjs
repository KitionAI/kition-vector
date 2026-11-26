// What the second walkthrough (plan 2026-11-25, stage 7, 2026-11-26) found and fixed, in a real
// browser through the development host: node tests/rewalk.mjs [chromium|webkit]
// Starts the Vite dev server itself, opens dev.html, and prints PASS or FAIL per step: a
// double-click on the Rotate, Scale, Reflect or Shear tool of the rail opens its dialog on the
// selection, as the reference's does, and the Selection tool's (or Enter with it) opens Move…; a
// double-click on the Zoom tool shows 100%, on the Hand tool fits the artboard; two quick presses of the Up arrow in the H field step the
// height twice, one history step each, and Cmd+Z takes the last one back.
import { fileURLToPath } from 'node:url'
import { chromium, webkit } from 'playwright'
import { createServer } from 'vite'

const which = process.argv[2] ?? 'chromium'
const server = await createServer({ configFile: fileURLToPath(new URL('../vite.config.ts', import.meta.url)), server: { port: 0, host: '127.0.0.1' }, logLevel: 'error' })
await server.listen()
const origin = `http://127.0.0.1:${server.httpServer.address().port}`
const browser = await (which === 'webkit' ? webkit : chromium).launch()
const errors = []
let failures = 0
const step = (name, ok, detail = '') => {
  if (!ok) failures += 1
  console.log(`${ok ? 'PASS' : 'FAIL'} ${which} ${name}${detail ? ` - ${detail}` : ''}`)
}

try {
  const context = await browser.newContext({ viewport: { width: 1440, height: 1000 } })
  const page = await context.newPage()
  page.on('pageerror', (error) => errors.push(error.message))
  await page.goto(`${origin}/dev.html?locale=en-US`)
  await page.waitForFunction(() => document.getElementById('status')?.textContent?.includes(' open in '), null, { timeout: 60000 })
  const kit = page.frameLocator('#kit')
  await page.waitForTimeout(1200)
  const until = async (check, tries = 60) => {
    for (let i = 0; i < tries; i += 1) {
      if (await check().catch(() => false)) return true
      await page.waitForTimeout(100)
    }
    return false
  }
  const details = async () => (await page.evaluate(async () => window.devHost.connection().ai.inspect())).details
  const primary = (await page.evaluate(() => /Mac|iPhone|iPad/i.test(navigator.platform))) ? 'Meta' : 'Control'
  const dialog = kit.getByTestId('vector-dialog')

  // The sample's ellipse, selected as a person does: its row in the Layers panel.
  const ellipse = (await details()).layers.flatMap((layer) => layer.children).find((node) => node.kind === 'Ellipse')
  await kit.getByTestId(`vector-layer-name-${ellipse.id}`).click()
  await until(async () => (await details()).selection.length === 1)

  // ---------- a double-click on a transform tool of the rail opens its dialog ----------
  for (const [tool, title, slot] of [['rotate', 'Rotate'], ['scale', 'Scale'], ['reflect', 'Reflect', 'rotate'], ['shear', 'Shear', 'scale']]) {
    // Reflect and Shear share their rail slots with Rotate and Scale: chosen from the slot's flyout first.
    if (slot) {
      await kit.getByTestId(`vector-tool-${slot}`).first().click({ button: 'right' })
      await kit.locator(`[role="menu"] [data-testid="vector-tool-${tool}"]`).first().click()
      await page.waitForTimeout(200)
    }
    await kit.locator(`.vector-rail-tools [data-testid="vector-tool-${tool}"]`).first().dblclick()
    const opened = await until(async () => (await dialog.count()) === 1, 40)
    const label = opened ? ((await dialog.getAttribute('aria-label').catch(() => null)) ?? (await dialog.textContent())) : ''
    step(`a double-click on the ${title} tool opens the ${title} dialog`, opened && label.includes(title), label.slice(0, 40))
    if (opened) await kit.getByTestId('vector-dialog-cancel').click()
    await until(async () => (await dialog.count()) === 0, 40)
  }
  await page.keyboard.press('v')

  // ---------- the Selection tool: a double-click on it, and Enter, open Move… ----------
  await kit.locator('.vector-rail-tools [data-testid="vector-tool-selection"]').first().dblclick()
  const move = await until(async () => (await dialog.count()) === 1, 40)
  const moveLabel = move ? ((await dialog.getAttribute('aria-label').catch(() => null)) ?? (await dialog.textContent())) : ''
  step('a double-click on the Selection tool opens Move…', move && moveLabel.includes('Move'), moveLabel.slice(0, 40))
  if (move) await kit.getByTestId('vector-dialog-cancel').click()
  await until(async () => (await dialog.count()) === 0, 40)
  await kit.getByTestId('kit-canvas-surface').focus().catch(() => undefined)
  await page.keyboard.press('Enter')
  const entered = await until(async () => (await dialog.count()) === 1, 40)
  const enterLabel = entered ? ((await dialog.getAttribute('aria-label').catch(() => null)) ?? (await dialog.textContent())) : ''
  step('Enter with the Selection tool and a selection opens Move…', entered && enterLabel.includes('Move') && (await details()).selection.length === 1, enterLabel.slice(0, 40))
  if (entered) await kit.getByTestId('vector-dialog-cancel').click()
  await until(async () => (await dialog.count()) === 0, 40)

  // ---------- a double-click on the Zoom tool shows 100%, on the Hand tool fits the artboard ----------
  const zoom = async () => Number(((await kit.locator('.kui-zoom-value').first().textContent()) ?? '').replace(/[^0-9.]/g, ''))
  const fitted = await zoom()
  await kit.locator('.vector-rail-tools [data-testid="vector-tool-zoom"]').first().dblclick()
  const actual = await until(async () => (await zoom()) === 100, 40)
  step('a double-click on the Zoom tool shows 100%', actual, `${fitted} -> ${await zoom()}`)
  await kit.locator('.vector-rail-tools [data-testid="vector-tool-hand"]').first().dblclick()
  step('a double-click on the Hand tool fits the artboard again', await until(async () => Math.abs((await zoom()) - fitted) < 1, 40), `${await zoom()}`)
  await page.keyboard.press('v')

  // ---------- two quick Up presses in H: two steps up, one undo takes one back ----------
  const height = async () => (await details()).selectionBounds?.height
  const steps = async () => (await details()).history.length
  const field = kit.getByTestId('vector-bounds-H')
  await field.fill('75')
  await field.press('Enter')
  await until(async () => Math.abs((await height()) - 75) < 0.01)
  const before = await steps()
  await field.click()
  await page.keyboard.press('ArrowUp')
  await page.keyboard.press('ArrowUp')
  const stepped = await until(async () => Math.abs((await height()) - 77) < 0.01, 40)
  step('two quick Up presses in H step the height twice (75 to 77), one history step each', stepped && (await steps()) === before + 2, JSON.stringify({ height: await height(), steps: (await steps()) - before }))
  await page.keyboard.press('Enter')
  await page.keyboard.press(`${primary}+z`)
  step('Cmd+Z after Enter takes the last step back (76)', await until(async () => Math.abs((await height()) - 76) < 0.01, 40), String(await height()))

} catch (error) {
  failures += 1
  console.log(`FAIL ${which} ${error instanceof Error ? error.message.split('\n').slice(0, 4).join(' | ') : String(error)}`)
} finally {
  if (errors.length) console.log(`page errors:\n${errors.slice(0, 8).join('\n')}`)
  await browser.close()
  await server.close()
}
process.exit(failures ? 1 : 0)
