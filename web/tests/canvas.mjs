// The canvas in a real browser, through the development host: node tests/canvas.mjs [chromium|webkit]
// Starts the Vite dev server itself, opens dev.html, and prints PASS or FAIL per step (plan
// 2026-11-25, stage 2, feature vector.editor.canvas): the keyboard is the editor's from the start
// and comes back after a panel field or button; X, Y, W and H set what they say; the arrows nudge;
// art off the artboard stays drawn where it was; a shape tool's click asks its size; Delete on a
// selected gradient stop removes the stop, not the object; Enter and Backspace while drawing with
// the Pen; leaving typed text; the right-click menu; isolation mode; the single-key commands,
// actual size and the wheel's zoom step; and the rail fits small windows. SHOTS=<folder> saves screenshots.
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
const errors = []
let failures = 0
const step = (name, ok, detail = '') => {
  if (!ok) failures += 1
  console.log(`${ok ? 'PASS' : 'FAIL'} ${which} ${name}${detail ? ` - ${detail}` : ''}`)
}

/** Opens dev.html at `width` × `height` (and `query`), waits for the sample, answers the helpers. */
async function open({ width = 1440, height = 900, query = '' } = {}) {
  const context = await browser.newContext({ viewport: { width, height } })
  const page = await context.newPage()
  page.on('pageerror', (error) => errors.push(error.message))
  await page.goto(`${origin}/dev.html?locale=en-US${query}`)
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
  const run = (command, params = {}) => page.evaluate(([c, p]) => window.devHost.connection().ai.run([{ command: c, params: p }], false), [command, params])
  const nodes = async () => (await details()).layers.flatMap((layer) => layer.children)
  const primary = (await page.evaluate(() => /Mac|iPhone|iPad/i.test(navigator.platform))) ? 'Meta' : 'Control'
  const region = await kit.getByTestId('vector-canvas-region').boundingBox()
  /** The artboard (white on the backdrop) found in the canvas's pixels: screen = left + x · zoom. */
  const calibrate = async () => {
    const box = await kit.getByTestId('kit-canvas-surface').boundingBox()
    const found = await kit.getByTestId('kit-canvas-surface').evaluate((canvas) => {
      // The artboard is the white area: the columns and rows that are white over half its
      // extent (a selection's white handles are only a few pixels each).
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
      const [y0, y1] = span(rows)
      return { x0, y0, x1, y1, scale: canvas.width / canvas.clientWidth }
    })
    const board = (await details()).artboards[0]
    const zoom = (found.x1 - found.x0 + 1) / found.scale / board.width
    const left = box.x + found.x0 / found.scale - board.x * zoom
    const top = box.y + found.y0 / found.scale - board.y * zoom
    return { zoom, left, top, toScreen: (x, y) => ({ x: left + x * zoom, y: top + y * zoom }) }
  }
  /** The canvas's colour at a screen point, as [r, g, b]. */
  const colorAt = async (point) => {
    const box = await kit.getByTestId('kit-canvas-surface').boundingBox()
    return kit.getByTestId('kit-canvas-surface').evaluate((canvas, [rx, ry]) => {
      const scale = canvas.width / canvas.clientWidth
      const data = canvas.getContext('2d').getImageData(Math.round(rx * scale), Math.round(ry * scale), 1, 1).data
      return [data[0], data[1], data[2]]
    }, [point.x - box.x, point.y - box.y])
  }
  const shot = async (name) => {
    if (shots) await page.screenshot({ path: `${shots}/${name}.png` })
  }
  return { page, kit, until, details, run, nodes, primary, region, calibrate, colorAt, shot, close: () => context.close() }
}

try {
  const w = await open()
  const { page, kit, until, details, run, nodes, primary } = w
  const tool = (id) => kit.getByTestId(`vector-tool-${id}`)
  const pressed = async (id) => (await tool(id).getAttribute('aria-pressed').catch(() => null)) === 'true'

  // The keyboard is the editor's as soon as the drawing opens: M picks the Rectangle tool unclicked.
  await page.keyboard.press('m')
  step('the editor takes the keyboard on open: M picks the Rectangle tool', await until(() => pressed('rectangle'), 30))
  await page.keyboard.press('v')
  await until(() => pressed('selection'), 30)

  let view = await w.calibrate()
  const ellipse = (await nodes()).find((node) => node.kind === 'Ellipse')
  const centre = (bounds) => view.toScreen(bounds.x + bounds.width / 2, bounds.y + bounds.height / 2)
  await page.mouse.click(centre(ellipse.bounds).x, centre(ellipse.bounds).y)
  await until(async () => (await details()).selection.length === 1)
  const before = (await details()).selectionBounds

  // X 100 moves the ellipse's left edge to 100, nothing else (it moved by half its size before).
  await kit.getByTestId('vector-bounds-X').fill('100')
  await kit.getByTestId('vector-bounds-X').press('Enter')
  await until(async () => Math.round((await details()).selectionBounds.x) === 100)
  const moved = (await details()).selectionBounds
  step('X 100 puts the left edge at 100 and moves nothing else', Math.round(moved.x) === 100 && Math.round(moved.y) === Math.round(before.y) && Math.round(moved.width) === Math.round(before.width), JSON.stringify(moved))
  await kit.getByTestId('vector-bounds-W').fill('90')
  await kit.getByTestId('vector-bounds-W').press('Enter')
  await until(async () => Math.round((await details()).selectionBounds.width) === 90)
  const narrowed = (await details()).selectionBounds
  step('W 90 keeps the left edge where it is', Math.round(narrowed.width) === 90 && Math.round(narrowed.x) === 100, JSON.stringify(narrowed))
  await kit.getByTestId('vector-bounds-H').fill('0')
  await kit.getByTestId('vector-bounds-H').press('Enter')
  await page.waitForTimeout(500)
  const flat = (await details()).selectionBounds
  step('H 0 is refused and the field shows the height again', Math.round(flat.height) === Math.round(before.height) && Number(await kit.getByTestId('vector-bounds-H').inputValue()) === Math.round(before.height * 100) / 100, JSON.stringify(flat))
  // Enter gave the keyboard back to the canvas: Cmd+Z undoes the width at once.
  await page.keyboard.press(`${primary}+z`)
  step('after Enter in a field the shortcuts work without a click (Cmd+Z)', await until(async () => Math.round((await details()).selectionBounds.width) === Math.round(before.width)))
  await page.keyboard.press(`${primary}+z`)
  await until(async () => Math.round((await details()).selectionBounds.x) === Math.round(before.x))

  // The arrows nudge: 1 pt, ten with Shift.
  const at = (await details()).selectionBounds
  await page.keyboard.press('ArrowRight')
  await until(async () => Math.round((await details()).selectionBounds.x - at.x) === 1)
  await page.keyboard.press('Shift+ArrowDown')
  await until(async () => Math.round((await details()).selectionBounds.y - at.y) === 10)
  const nudged = (await details()).selectionBounds
  step('ArrowRight nudges 1 pt and Shift+ArrowDown 10 pt', Math.round(nudged.x - at.x) === 1 && Math.round(nudged.y - at.y) === 10, JSON.stringify({ at, nudged }))

  // A button keeps the keyboard the editor's: Swap Fill and Stroke, then Cmd+Z at once.
  const steps = (await details()).history.length
  await kit.getByTestId('vector-proxy-swap').click()
  await until(async () => (await details()).history.length === steps + 1)
  await page.keyboard.press(`${primary}+z`)
  step('after a button Cmd+Z undoes at once', await until(async () => (await details()).history.length === steps))

  // Art moved off the artboard stays drawn where it went; the drawing does not move on the screen.
  await page.keyboard.press(`${primary}+-`)
  await page.keyboard.press(`${primary}+-`)
  await page.waitForTimeout(800)
  view = await w.calibrate()
  const id = (await details()).selection[0]
  const sel = (await details()).selectionBounds
  await run('object.move', { dx: -40 - sel.x - sel.width, dy: 0 })
  await until(async () => (await details()).selectionBounds.x < -40 - sel.width + 1)
  await page.waitForTimeout(800)
  const after = await w.calibrate()
  const off = (await details()).selectionBounds
  // A quarter in from its centre, clear of the selection's centre mark.
  const spot = after.toScreen(off.x + off.width / 4, off.y + off.height / 2)
  const color = await w.colorAt(spot)
  const fill = (await nodes()).find((node) => node.id === id)?.fill ?? ''
  const hex = `#${color.map((c) => c.toString(16).padStart(2, '0')).join('')}`
  step('art off the artboard is drawn on the pasteboard', Math.abs(parseInt(hex.slice(1, 3), 16) - parseInt(fill.slice(1, 3), 16)) < 12 && Math.abs(parseInt(hex.slice(3, 5), 16) - parseInt(fill.slice(3, 5), 16)) < 12, `${hex} vs ${fill}`)
  step('the artboard stays where it was on the screen when the extent grows', Math.abs(after.left - view.left) < 2 && Math.abs(after.top - view.top) < 2, JSON.stringify({ before: [view.left, view.top], after: [after.left, after.top] }))
  step('its own coordinates are what the command made them', Math.round(off.x) === Math.round(-40 - sel.width), JSON.stringify(off))
  await w.shot('pasteboard')
  // Back on the artboard (the agent's move is the agent's to undo, not Cmd+Z's).
  await run('object.move', { dx: sel.x - off.x, dy: 0 })
  await until(async () => (await details()).selectionBounds.x > 0)
  await page.keyboard.press(`${primary}+0`)
  await page.waitForTimeout(600)
  view = await w.calibrate()

  // A click with the Rectangle tool asks for its size, and OK draws it there.
  await page.keyboard.press('Escape')
  await page.keyboard.press('m')
  await until(() => pressed('rectangle'), 30)
  const count = (await nodes()).length
  const spotA = view.toScreen(60, 500)
  await page.mouse.click(spotA.x, spotA.y)
  const asked = await until(async () => (await kit.getByTestId('vector-dialog').count()) === 1, 40)
  step('a click with the Rectangle tool opens its size dialog', asked)
  await w.shot('rectangle-dialog')
  if (asked) {
    await kit.getByTestId('vector-dialog-param-width').locator('input').fill('50').catch(() => kit.getByTestId('vector-dialog-param-width').fill('50'))
    await kit.getByTestId('vector-dialog-param-height').locator('input').fill('30').catch(() => kit.getByTestId('vector-dialog-param-height').fill('30'))
    await page.waitForTimeout(300)
    await kit.getByTestId('vector-dialog-ok').click()
  }
  await until(async () => (await nodes()).length === count + 1)
  const drawn = (await details()).selectionBounds
  step('OK draws the rectangle at the click, the size typed', (await nodes()).length === count + 1 && Math.round(drawn?.width) === 50 && Math.round(drawn?.height) === 30 && Math.abs(drawn.x - 60) < 3 && Math.abs(drawn.y - 500) < 3, JSON.stringify(drawn))

  // Alt-click with the Rotate tool opens Rotate… about the point clicked.
  await page.keyboard.press('r')
  await until(() => pressed('rotate'), 30)
  const pivot = view.toScreen(drawn.x, drawn.y)
  await page.keyboard.down('Alt')
  await page.mouse.click(pivot.x, pivot.y)
  await page.keyboard.up('Alt')
  const rotate = await until(async () => (await kit.getByTestId('vector-dialog-angle').count()) === 1, 40)
  step('Alt-click with the Rotate tool opens Rotate…', rotate)
  if (rotate) await kit.getByTestId('vector-dialog-cancel').click()
  await until(async () => (await kit.getByTestId('vector-dialog').count()) === 0)

  // Delete on a gradient stop picked on the canvas removes the stop, and the object stays.
  await page.keyboard.press('Escape')
  await page.keyboard.press('v')
  await until(() => pressed('selection'), 30)
  const target = (await nodes()).find((node) => node.kind === 'Ellipse')
  const middleOf = centre(target.bounds)
  await page.mouse.click(middleOf.x, middleOf.y)
  await until(async () => (await details()).selection.length === 1)
  await run('paint.setFill', { gradient: { kind: 'linear', stops: [{ offset: 0, color: '#ffffff' }, { offset: 0.5, color: '#ff0000' }, { offset: 1, color: '#000000' }] } })
  await page.keyboard.press('g')
  await until(() => pressed('gradient'), 30)
  await page.waitForTimeout(400)
  // The annotator's bar runs across the ellipse's middle; its stops' chips sit just under it.
  const box = (await details()).selectionBounds
  const chip = view.toScreen(box.x + box.width / 2, box.y + box.height / 2 + 10 / view.zoom)
  await page.mouse.click(chip.x, chip.y)
  const claimed = await until(async () => (await details()).toolKeys?.includes('Delete'), 30)
  const objects = (await nodes()).length
  await page.keyboard.press('Delete')
  await page.waitForTimeout(800)
  const stops = (await run('paint.proxies', {})).completed?.[0]?.result?.fill?.gradient?.stops?.length
  step('Delete on a gradient stop picked on the canvas removes the stop, not the object', claimed && (await nodes()).length === objects && stops === 2, JSON.stringify({ claimed, objects, now: (await nodes()).length, stops }))
  await w.shot('gradient-stop')

  // The Pen: Backspace takes back the last anchor, Enter ends the path.
  await page.keyboard.press('Escape')
  await page.keyboard.press('p')
  await until(() => pressed('pen'), 30)
  const paths = async () => (await nodes()).filter((node) => node.kind === 'Path').length
  const penPaths = await paths()
  for (const [x, y] of [[100, 420], [180, 460], [260, 420]]) {
    const point = view.toScreen(x, y)
    await page.mouse.click(point.x, point.y)
    await page.waitForTimeout(150)
  }
  const anchorsOf = async () => {
    const selected = (await details()).selection[0]
    return (await nodes()).find((node) => node.id === selected)?.anchors
  }
  await until(async () => (await anchorsOf()) === 3)
  await page.waitForTimeout(300)
  await page.keyboard.press('Backspace')
  step('Backspace while drawing takes back the last anchor', await until(async () => (await anchorsOf()) === 2), String(await anchorsOf()))
  await page.keyboard.press('Enter')
  await page.waitForTimeout(400)
  const next = view.toScreen(400, 540)
  await page.mouse.click(next.x, next.y)
  await page.waitForTimeout(400)
  step('Enter ends the path: the next click starts another', await until(async () => (await paths()) === penPaths + 2), String((await paths()) - penPaths))
  await w.shot('pen-cursor')
  await page.keyboard.press('Escape')

  // Typed text: Escape ends it, and V then picks the Selection tool; Cmd+Enter ends it too.
  await page.keyboard.press('t')
  await until(() => pressed('type'), 30)
  const textAt = view.toScreen(500, 560)
  await page.mouse.click(textAt.x, textAt.y)
  await page.waitForTimeout(500)
  await page.keyboard.type('Hi')
  await page.waitForTimeout(300)
  await page.keyboard.press(`${primary}+Enter`)
  await page.waitForTimeout(400)
  await page.keyboard.press('v')
  step('Cmd+Enter ends typed text and V then picks the Selection tool', await until(() => pressed('selection'), 30))

  // The single keys: X brings the stroke to the front, D gives the default colours.
  const front = async () => (await details()).paint?.fillActive
  const was = await front()
  await page.keyboard.press('x')
  step('X brings the other proxy to the front', await until(async () => (await front()) === !was))
  await page.keyboard.press('x')
  await until(async () => (await front()) === was)
  // Clicking a proxy brings it to the front too.
  await w.shot('proxies')
  await kit.getByTestId('vector-proxy-stroke').click({ position: { x: 12, y: 12 } })
  step('a click on the Stroke proxy brings it to the front', await until(async () => (await front()) === false))
  await kit.getByTestId('vector-proxy-fill').click({ position: { x: 3, y: 3 } })
  await until(async () => (await front()) === true)

  // The right-click menu, by what is selected.
  await page.keyboard.press(`${primary}+a`)
  await until(async () => (await details()).selection.length > 1)
  await page.waitForTimeout(300)
  const middle = view.toScreen(400, 300)
  await page.mouse.click(middle.x, middle.y, { button: 'right' })
  const menu = await until(async () => (await kit.getByTestId('vector-canvas-menu').count()) === 1, 30)
  const groupRow = await kit.getByTestId('vector-canvas-menu-arrange-group').count()
  step('a right click opens the canvas menu with Group for several objects', menu && groupRow === 1)
  await w.shot('canvas-menu')
  if (groupRow) await kit.getByTestId('vector-canvas-menu-arrange-group').click()
  const grouped = await until(async () => (await nodes()).some((node) => node.kind === 'Group'))
  step('choosing Group groups them', grouped)

  // Isolation mode: a double-click on the group isolates it (a bar names it), Escape leaves it.
  const group = (await nodes()).find((node) => node.kind === 'Group')
  await page.keyboard.press('Escape')
  await until(async () => (await details()).selection.length === 0)
  const inside = group.children?.[0]?.bounds ?? group.bounds
  const into = view.toScreen(inside.x + inside.width / 2, inside.y + inside.height / 2)
  await page.mouse.dblclick(into.x, into.y)
  const isolated = await until(async () => (await kit.getByTestId('vector-isolation').count()) === 1, 40)
  step('a double-click on a group isolates it and the bar names it', isolated)
  await w.shot('isolation')
  await page.keyboard.press('Escape')
  await page.waitForTimeout(200)
  await page.keyboard.press('Escape')
  step('Escape leaves isolation mode', await until(async () => (await kit.getByTestId('vector-isolation').count()) === 0, 40))

  // Cmd+1 is actual size; a mouse wheel notch with Cmd zooms by at most 1.2 times.
  await page.keyboard.press(`${primary}+1`)
  const percent = async () => Number(((await kit.locator('.kui-zoom-value').first().textContent()) ?? '').replace(/[^0-9.]/g, ''))
  step('Cmd+1 shows actual size', await until(async () => (await percent()) === 100), String(await percent()))
  const canvasBox = await kit.getByTestId('kit-canvas-surface').boundingBox()
  await page.mouse.move(canvasBox.x + canvasBox.width / 2, canvasBox.y + canvasBox.height / 2)
  await page.keyboard.down(primary)
  await page.mouse.wheel(0, -100)
  await page.keyboard.up(primary)
  await page.waitForTimeout(400)
  const zoomed = await percent()
  step('one wheel notch with Cmd zooms in by at most 1.2 times', zoomed > 100 && zoomed <= 121, String(zoomed))
  await w.close()

  // The rail fits: every tool on screen in Menus at 1280 × 720 and Classic at 1440 × 900.
  for (const [width, height, query] of [[1280, 720, ''], [1440, 900, '&modes=classic']]) {
    const small = await open({ width, height, query })
    const last = await small.kit.getByTestId('vector-tool-zoom').boundingBox()
    step(`every tool is on screen at ${width} × ${height}${query ? ' (Classic)' : ''}`, last != null && last.y + last.height <= height, JSON.stringify(last))
    await small.shot(`rail-${width}x${height}`)
    await small.close()
  }
} catch (error) {
  failures += 1
  console.log(`FAIL ${which} ${error instanceof Error ? error.message.split('\n').slice(0, 12).join(' | ') : String(error)}`)
} finally {
  if (errors.length) console.log(`page errors:\n${errors.slice(0, 8).join('\n')}`)
  step('no page errors', errors.length === 0)
  await browser.close()
  await server.close()
}
process.exit(failures ? 1 : 0)
