// Escape cancels first in Focus: it ends a pen path or deselects and stays in Focus; only an
// Escape with nothing to cancel leaves Focus. node tests/focus.mjs [chromium|webkit]
import assert from 'node:assert/strict'
import { chromium, webkit } from 'playwright'
import { createServer } from 'vite'

const which = process.argv[2] ?? 'chromium'
const server = await createServer({ configFile: new URL('../vite.config.ts', import.meta.url).pathname, server: { port: 0, host: '127.0.0.1' }, logLevel: 'error' })
await server.listen()
const browser = await (which === 'webkit' ? webkit : chromium).launch()
const page = await browser.newPage({ viewport: { width: 1440, height: 900 } })
const kit = page.frameLocator('#kit')
const app = kit.locator('.kui-app')
const origin = `http://127.0.0.1:${server.httpServer.address().port}`
const errors = []
page.on('pageerror', (error) => errors.push(error.message))
const details = async () => (await page.evaluate(async () => window.devHost.connection().ai.inspect())).details
const selected = async () => (await details()).selection.length
const paths = async () => JSON.stringify((await details()).layers).split('"kind":"Path"').length - 1
const mode = () => app.getAttribute('data-mode')
const until = async (name, check) => {
  for (let attempt = 0; attempt < 100; attempt += 1) {
    if (await check().catch(() => false)) {
      console.log(`PASS ${which} ${name}`)
      return
    }
    await page.waitForTimeout(100)
  }
  assert.fail(`${which} ${name}`)
}

try {
  await page.goto(`${origin}/dev.html?modes=classic,line,menus,focus&locale=en-US`)
  await page.waitForFunction(() => document.getElementById('status')?.textContent?.includes(' open in '), null, { timeout: 60_000 })
  await page.waitForTimeout(800)
  await app.focus()
  await page.keyboard.press('ControlOrMeta+Backslash')
  await until('Cmd+\\ enters Focus', async () => (await mode()) === 'focus')
  const region = await kit.getByTestId('vector-canvas-region').boundingBox()
  const before = await paths()

  // A pen path being drawn (it is the selected path) ends with Escape, which stays in Focus.
  await page.keyboard.press('p')
  await until('P picks the pen', async () => (await kit.locator('.kui-focus-tool').textContent())?.includes('Pen'))
  await page.mouse.click(region.x + region.width * 0.2, region.y + region.height * 0.85)
  await page.mouse.click(region.x + region.width * 0.3, region.y + region.height * 0.9)
  await until('the pen path has started', async () => (await selected()) === 1 && (await paths()) === before + 1)
  await page.waitForTimeout(300)
  await page.keyboard.press('Escape')
  await until('Escape ends the pen path', async () => (await selected()) === 0)
  assert.equal(await mode(), 'focus', 'an Escape that ended a path does not leave Focus')
  console.log(`PASS ${which} an Escape that ended a path stays in Focus`)
  // The next click starts a new path rather than going on with the ended one.
  await page.mouse.click(region.x + region.width * 0.4, region.y + region.height * 0.85)
  await page.mouse.click(region.x + region.width * 0.5, region.y + region.height * 0.9)
  await until('the next clicks start a new path', async () => (await paths()) === before + 2)
  await page.keyboard.press('Escape')
  // The editor's state follows the engine's answer to the click a moment later.
  await page.waitForTimeout(300)
  await until('Escape ends that path too', async () => (await selected()) === 0)

  // A selection is something to cancel: the first Escape deselects, the second leaves Focus.
  await page.keyboard.press('v')
  await page.keyboard.press('ControlOrMeta+a')
  await until('Cmd+A selects everything', async () => (await selected()) > 1)
  await page.waitForTimeout(300)
  await page.keyboard.press('Escape')
  await until('Escape deselects', async () => (await selected()) === 0)
  assert.equal(await mode(), 'focus', 'an Escape that deselected does not leave Focus')
  console.log(`PASS ${which} an Escape that deselected stays in Focus`)
  await page.keyboard.press('Escape')
  await until('Escape with nothing to cancel leaves Focus', async () => (await mode()) === 'classic')
  assert.deepEqual(errors, [])
  console.log(`PASS ${which} no page errors`)
} finally {
  await browser.close()
  await server.close()
}
