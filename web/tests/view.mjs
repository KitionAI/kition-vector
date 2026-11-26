// The view by hand in a real browser, through the development host: node tests/view.mjs [chromium|webkit]
// Starts the Vite dev server itself, opens dev.html, and prints PASS or FAIL per step (plan
// 2026-11-25, stage 6, feature vector.editor.view): Outline (Cmd+Y) changes the pixels and back,
// checked in the View menu; the transparency grid shows; rulers (Cmd+R) and a guide dragged out of
// the top ruler, shown as it goes and made on release; a moved rectangle snapping onto the guide,
// and not with Smart Guides off (Cmd+U); the grid (Cmd+') drawn and gone; a typed zoom
// percentage; Alt+wheel zoom; scrollbars while zoomed in; none of the toggles an undo step.
// SHOTS=<folder> saves screenshots.
import { mkdirSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { chromium, webkit } from 'playwright'
import { createServer } from 'vite'

const which = process.argv[2] ?? 'chromium'
const locale = process.env.LOCALE ?? 'en-US'
const shots = process.env.SHOTS ? fileURLToPath(new URL(process.env.SHOTS, `file://${process.cwd()}/`)) : null
if (shots) mkdirSync(shots, { recursive: true })
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
  await page.goto(`${origin}/dev.html?modes=menus,line,classic,focus&locale=${locale}`)
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
  const shot = async (name) => {
    if (shots) await page.screenshot({ path: `${shots}/${name}.png` })
  }
  const details = async () => (await page.evaluate(async () => window.devHost.connection().ai.inspect())).details
  const run = async (command, params = {}) => (await page.evaluate(([c, p]) => window.devHost.connection().ai.run([{ command: c, params: p }], false), [command, params])).completed?.[0]?.result
  const batch = (steps) => page.evaluate((s) => window.devHost.connection().ai.run(s, false), steps)
  const nodes = async () => (await details()).layers.flatMap((layer) => layer.children)
  const steps = async () => {
    const d = await details()
    return `${d.history.length}/${d.redo.length}`
  }
  const primary = (await page.evaluate(() => /Mac|iPhone|iPad/i.test(navigator.platform))) ? 'Meta' : 'Control'
  const zoomOf = async () => Number(((await kit.locator('.kui-zoom-value').first().textContent()) ?? '').replace(/[^0-9.]/g, ''))

  // A clean artboard: everything deleted (one agent batch, which keeps its own selection).
  await batch([{ command: 'edit.selectAll', params: {} }, { command: 'edit.delete', params: {} }])
  await until(async () => (await nodes()).length === 0)
  const board = (await details()).artboards[0]
  const surface = kit.getByTestId('kit-canvas-surface')
  // Where a drawing's point is on the screen, from the white artboard's pixels.
  const calibrate = async () => {
    const box = await surface.boundingBox()
    const found = await surface.evaluate((canvas) => {
      const { width: w, height: h } = canvas
      const data = canvas.getContext('2d').getImageData(0, 0, w, h).data
      const columns = new Uint32Array(w)
      const rows = new Uint32Array(h)
      for (let y = 0; y < h; y += 1) {
        for (let x = 0; x < w; x += 1) {
          const i = (y * w + x) * 4
          if (data[i] === 255 && data[i + 1] === 255 && data[i + 2] === 255) {
            columns[x] += 1
            rows[y] += 1
          }
        }
      }
      const span = (counts) => {
        const most = Math.max(...counts)
        const first = counts.findIndex((count) => count > most / 2)
        let last = counts.length - 1
        while (last > first && counts[last] <= most / 2) last -= 1
        return [first, last]
      }
      const [x0, x1] = span(columns)
      const [y0] = span(rows)
      return { x0, y0, x1, scale: canvas.width / canvas.clientWidth }
    })
    const zoom = (found.x1 - found.x0 + 1) / found.scale / board.width
    const left = box.x + found.x0 / found.scale - board.x * zoom
    const top = box.y + found.y0 / found.scale - board.y * zoom
    return { at: (x, y) => ({ x: left + x * zoom, y: top + y * zoom }), zoom }
  }
  await page.waitForTimeout(500)
  let screen = await calibrate()
  /** The canvas's pixels along a short horizontal strip at screen point `at` (RGB triples). */
  const stripAt = async (at, width = 7) =>
    surface.evaluate(
      (canvas, [point, w]) => {
        const box = canvas.getBoundingClientRect()
        const scale = canvas.width / canvas.clientWidth
        const data = canvas.getContext('2d').getImageData(Math.round((point.x - box.left) * scale) - Math.floor(w / 2), Math.round((point.y - box.top) * scale), w, 1).data
        const out = []
        for (let i = 0; i < data.length; i += 4) out.push([data[i], data[i + 1], data[i + 2]])
        return out
      },
      [await frameToCanvas(at), width],
    )
  // Page coordinates to the frame's: the kit runs in an iframe.
  const frameBox = await page.locator('#kit').boundingBox()
  async function frameToCanvas(at) {
    return { x: at.x - frameBox.x, y: at.y - frameBox.y }
  }
  const pixel = async (at) => {
    const p = await frameToCanvas(at)
    return surface.evaluate((canvas, point) => {
      const box = canvas.getBoundingClientRect()
      const scale = canvas.width / canvas.clientWidth
      return [...canvas.getContext('2d').getImageData(Math.round((point.x - box.left) * scale), Math.round((point.y - box.top) * scale), 1, 1).data]
    }, p)
  }
  await kit.getByTestId('vector-canvas-region').click({ position: { x: 5, y: 5 } }).catch(() => undefined)

  // A red rectangle on the artboard, nothing selected.
  const rect = (await run('shape.rectangle', { x: board.x + 100, y: board.y + 100, width: 200, height: 150 })).id
  await run('paint.setFill', { ids: [rect], color: '#ff0000' })
  await run('edit.deselectAll', {})
  const inside = screen.at(board.x + 200, board.y + 175)
  const red = (rgba) => rgba[0] > 200 && rgba[1] < 60 && rgba[2] < 60
  const white = (rgba) => rgba[0] > 245 && rgba[1] > 245 && rgba[2] > 245
  await until(async () => red(await pixel(inside)))
  const before = await steps()

  // ---------- Outline: Cmd+Y, checked in View, and back ----------
  await page.keyboard.press(`${primary}+y`)
  const outlined = await until(async () => white(await pixel(inside)))
  await shot('view-outline')
  await kit.getByTestId('vector-menu-View').click()
  const outlineRow = kit.getByTestId('vector-control-view.outline')
  await outlineRow.waitFor({ state: 'visible', timeout: 5000 })
  const checkedOutline = await outlineRow.getAttribute('aria-checked')
  await shot('view-menu')
  await page.keyboard.press('Escape')
  step('Cmd+Y shows the outlines: the fill goes, View › Outline is checked', outlined && checkedOutline === 'true', JSON.stringify({ pixel: await pixel(inside), checkedOutline }))
  await page.keyboard.press(`${primary}+y`)
  step('Cmd+Y again: the fill comes back', await until(async () => red(await pixel(inside))), JSON.stringify(await pixel(inside)))

  // ---------- The transparency grid: the artboard's white becomes two greys ----------
  const empty = screen.at(board.x + 420, board.y + 520)
  await page.keyboard.press(`${primary}+Shift+d`)
  const grid = await until(async () => {
    const strip = await stripAt(empty, 41)
    return new Set(strip.map((c) => c.join(','))).size >= 2 && strip.some((c) => c[0] < 245)
  })
  await shot('view-transparency-grid')
  await kit.getByTestId('vector-menu-View').click()
  const gridRow = kit.getByTestId('vector-cmd-view.transparencyGrid')
  await gridRow.waitFor({ state: 'visible', timeout: 5000 })
  const gridChecked = await gridRow.getAttribute('aria-checked')
  await page.keyboard.press('Escape')
  await page.keyboard.press(`${primary}+Shift+d`)
  const gridGone = await until(async () => (await stripAt(empty, 41)).every((c) => c[0] > 245))
  step('View › Show Transparency Grid shows its cells, checked, and goes again', grid && gridChecked === 'true' && gridGone, JSON.stringify({ grid, gridChecked, gridGone }))

  // ---------- Rulers and a guide out of the top ruler ----------
  await page.keyboard.press(`${primary}+r`)
  const ruler = kit.getByTestId('vector-ruler-x')
  const rulers = await until(async () => (await ruler.count()) === 1 && (await kit.getByTestId('vector-ruler-y').count()) === 1)
  await page.waitForTimeout(300)
  screen = await calibrate()
  step('Cmd+R shows the rulers', rulers)
  // Off the artboard's centre line (which Smart Guides pull to anyway).
  const guideY = board.y + (Math.abs(330 - board.height / 2) < 12 ? 280 : 330)
  const rulerBox = await ruler.boundingBox()
  const from = { x: screen.at(board.x + 450, 0).x, y: rulerBox.y + rulerBox.height / 2 }
  const to = screen.at(board.x + 450, guideY)
  await page.mouse.move(from.x, from.y)
  await page.mouse.down()
  for (let i = 1; i <= 8; i += 1) await page.mouse.move(from.x, from.y + ((to.y - from.y) * i) / 8)
  await page.waitForTimeout(300)
  const live = await until(async () => ((await run('guide.list', {})) ?? []).length === 1, 30)
  await shot('view-guide-drag')
  await page.mouse.up()
  await until(async () => (await steps()) !== before, 30)
  const guides = (await run('guide.list', {})) ?? []
  step('a guide dragged out of the top ruler shows as it goes and is made where it is released', live && guides.length === 1 && guides[0].vertical === false && Math.abs(guides[0].pos - guideY) <= 1.5 / screen.zoom + 1, JSON.stringify(guides))
  const withGuide = await steps()

  // ---------- A moved rectangle snaps onto the guide; not with Smart Guides off ----------
  await page.keyboard.press('v')
  const drag = async (dy) => {
    const start = screen.at(board.x + 200, board.y + 175)
    await page.mouse.move(start.x, start.y)
    await page.mouse.down()
    for (let i = 1; i <= 10; i += 1) await page.mouse.move(start.x, start.y + (dy * screen.zoom * i) / 10)
    await page.waitForTimeout(150)
    await page.mouse.up()
    await page.waitForTimeout(300)
  }
  const top = async () => (await nodes()).find((node) => node.id === rect)?.bounds?.y
  // The bottom edge from 250 to 2.5 points short of the guide.
  await drag(guides[0].pos - board.y - 250 - 2.5)
  await until(async () => Math.abs((await top()) - (board.y + 100)) > 0.5, 30)
  const snapped = await top()
  step('a moved rectangle\'s edge snaps onto the guide', Math.abs(snapped + 150 - guides[0].pos) < 0.05, JSON.stringify({ snapped, guide: guides[0].pos }))
  await page.keyboard.press(`${primary}+z`)
  await until(async () => Math.abs((await top()) - (board.y + 100)) < 0.01, 30)
  await page.keyboard.press(`${primary}+u`)
  await page.waitForTimeout(300)
  await drag(guides[0].pos - board.y - 250 - 2.5)
  await until(async () => Math.abs((await top()) - (board.y + 100)) > 0.5, 30)
  const free = await top()
  step('with Smart Guides off (Cmd+U) it stays where the pointer puts it', Math.abs(free + 150 - guides[0].pos) > 1 && Math.abs(free + 150 - (guides[0].pos - 2.5)) < 1.5, JSON.stringify({ free, guide: guides[0].pos }))
  await page.keyboard.press(`${primary}+z`)
  await until(async () => Math.abs((await top()) - (board.y + 100)) < 0.01, 30)
  await kit.getByTestId('vector-menu-View').click()
  const smartRow = kit.getByTestId('vector-control-view.smartGuides')
  await smartRow.waitFor({ state: 'visible', timeout: 5000 })
  const smartOff = await smartRow.getAttribute('aria-checked')
  await smartRow.click()
  await page.waitForTimeout(200)
  await kit.getByTestId('vector-menu-View').click()
  await smartRow.waitFor({ state: 'visible', timeout: 5000 })
  const smartOn = await smartRow.getAttribute('aria-checked')
  await page.keyboard.press('Escape')
  step('View › Smart Guides shows its check and turns it back on', smartOff === 'false' && smartOn === 'true', JSON.stringify({ smartOff, smartOn }))

  // ---------- A guide is selected with the Selection tool and deleted with Delete ----------
  await run('edit.deselectAll', {})
  const onGuide = screen.at(board.x + 450, guides[0].pos)
  await page.mouse.click(onGuide.x, onGuide.y)
  const picked = await until(async () => ((await run('guide.list', {})) ?? [])[0]?.selected === true, 30)
  await page.keyboard.press('Delete')
  const deleted = await until(async () => ((await run('guide.list', {})) ?? []).length === 0, 30)
  await page.keyboard.press(`${primary}+z`)
  const back = await until(async () => ((await run('guide.list', {})) ?? []).length === 1, 30)
  step('a guide clicked with the Selection tool is selected, Delete deletes it, Undo brings it back', picked && deleted && back, JSON.stringify({ picked, deleted, back }))

  // ---------- The grid: Cmd+' draws it, again takes it away ----------
  await run('edit.deselectAll', {})
  const gridAt = screen.at(Math.ceil((board.x + 400) / 72) * 72, board.y + 520)
  const plain = await stripAt(gridAt, 9)
  await page.keyboard.press(`${primary}+'`)
  const drawn = await until(async () => (await stripAt(gridAt, 9)).some((c) => c[0] < 240))
  await shot('view-grid')
  await page.keyboard.press(`${primary}+'`)
  const undrawn = await until(async () => (await stripAt(gridAt, 9)).every((c) => c[0] > 245))
  step('Cmd+\' draws the grid and takes it away', plain.every((c) => c[0] > 245) && drawn && undrawn, JSON.stringify({ drawn, undrawn }))

  // ---------- None of the toggles is an undo step ----------
  const beforeToggles = await steps()
  for (const key of ['y', 'h', 'Shift+b', "'", 'u', ';']) {
    await page.keyboard.press(`${primary}+${key}`)
    await page.waitForTimeout(80)
    await page.keyboard.press(`${primary}+${key}`)
    await page.waitForTimeout(80)
  }
  await page.waitForTimeout(300)
  step('the view toggles add no undo step and change no redo', (await steps()) === beforeToggles && withGuide === '1/0', JSON.stringify({ withGuide, beforeToggles, now: await steps() }))

  // For the help page only: Pixel Preview zoomed in (a Zoom-tool marquee round the rectangle's
  // corner) with the pixel grid.
  if (shots) {
    await page.keyboard.press(`${primary}+Alt+y`)
    await page.keyboard.press('z')
    const a = screen.at(board.x + 88, board.y + 92)
    const b = screen.at(board.x + 112, board.y + 108)
    await page.mouse.move(a.x, a.y)
    await page.mouse.down()
    await page.mouse.move((a.x + b.x) / 2, (a.y + b.y) / 2)
    await page.mouse.move(b.x, b.y)
    await page.mouse.up()
    await page.waitForTimeout(800)
    await shot('view-pixel-preview')
    await page.keyboard.press(`${primary}+Alt+y`)
    await page.keyboard.press('v')
    await page.keyboard.press(`${primary}+0`)
    await page.waitForTimeout(300)
  }

  // ---------- Zoom: typed, Alt+wheel, scrollbars ----------
  await kit.getByTestId('vector-zoom-value').click()
  const field = kit.getByTestId('vector-zoom-input')
  await field.waitFor({ state: 'visible', timeout: 5000 })
  await field.fill('250')
  await field.press('Enter')
  const typed = await until(async () => (await zoomOf()) === 250, 30)
  step('a zoom typed in the status bar lands on it', typed, String(await zoomOf()))
  const scrollbars = await until(async () => (await kit.getByTestId('vector-scroll-x').count()) === 1 && (await kit.getByTestId('vector-scroll-y').count()) === 1, 30)
  await shot('view-zoomed')
  const thumbBefore = await kit.locator('[data-testid="vector-scroll-x"] [role="scrollbar"]').getAttribute('aria-valuenow')
  await kit.locator('[data-testid="vector-scroll-x"] [role="scrollbar"]').press('End')
  const thumbAfter = await kit.locator('[data-testid="vector-scroll-x"] [role="scrollbar"]').getAttribute('aria-valuenow')
  step('zoomed in, scrollbars show and scroll the view', scrollbars && Number(thumbAfter) > Number(thumbBefore), JSON.stringify({ scrollbars, thumbBefore, thumbAfter }))
  const canvasBox = await surface.boundingBox()
  await page.mouse.move(canvasBox.x + canvasBox.width / 2, canvasBox.y + canvasBox.height / 2)
  const zoomBefore = await zoomOf()
  await page.keyboard.down('Alt')
  await page.mouse.wheel(0, 100)
  await page.keyboard.up('Alt')
  const zoomedOut = await until(async () => (await zoomOf()) < zoomBefore, 30)
  step('Alt with the wheel zooms', zoomedOut, JSON.stringify({ zoomBefore, after: await zoomOf() }))
  await page.keyboard.press(`${primary}+0`)
  const fitted = await until(async () => (await kit.getByTestId('vector-scroll-x').count()) === 0, 30)
  step('fitted, the scrollbars go', fitted)
  await page.keyboard.press(`${primary}+r`)
  step('Cmd+R hides the rulers again', await until(async () => (await kit.getByTestId('vector-ruler-x').count()) === 0, 30))

  step('no page errors', errors.length === 0, errors.join(' | '))
  await context.close()
} catch (error) {
  failures += 1
  console.log(`FAIL ${which} ${error.stack ?? error}`)
} finally {
  await browser.close()
  await server.close()
}
process.exit(failures ? 1 : 0)
