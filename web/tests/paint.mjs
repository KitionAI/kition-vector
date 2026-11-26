// Colour, paint and the panels in a real browser, through the development host: node tests/paint.mjs [chromium|webkit]
// Starts the Vite dev server itself, opens dev.html, and prints PASS or FAIL per step (plan
// 2026-11-25, stage 3, feature vector.editor.paint): the fill, stroke weight, dashes and opacity
// each leave one history step and the panel reads them back; a label scrubbed and a colour dragged
// in the Color Picker are one step each, Cancel none; None and gradients show as what they are;
// a gradient swatch applies, the Gradient panel adds, drags and recolours stops without losing the
// gradient, and the canvas annotator's stop opens the picker; align to the artboard, distribute,
// arithmetic and Tab in the fields, the reference point, Shift+X, the swatches' previews and New
// Swatch, the eyedropper, Pathfinder's ten and the control bar. SHOTS=<folder> saves screenshots.
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
const warnings = []
let failures = 0
const step = (name, ok, detail = '') => {
  if (!ok) failures += 1
  console.log(`${ok ? 'PASS' : 'FAIL'} ${which} ${name}${detail ? ` - ${detail}` : ''}`)
}

try {
  const context = await browser.newContext({ viewport: { width: 1440, height: 1000 } })
  const page = await context.newPage()
  page.on('pageerror', (error) => errors.push(error.message))
  page.on('console', (message) => {
    if (message.type() === 'warning' || message.type() === 'error') warnings.push(message.text())
  })
  await page.goto(`${origin}/dev.html?locale=${locale}`)
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
  const nodes = async () => (await details()).layers.flatMap((layer) => layer.children)
  const node = async (id) => (await nodes()).find((item) => item.id === id)
  const steps = async () => (await details()).history.length
  const proxies = () => run('paint.proxies', {})
  const tool = (id) => kit.getByTestId(`vector-tool-${id}`)
  const pressed = async (id) => (await tool(id).getAttribute('aria-pressed').catch(() => null)) === 'true'
  const primary = (await page.evaluate(() => /Mac|iPhone|iPad/i.test(navigator.platform))) ? 'Meta' : 'Control'

  // Where the drawing is on the screen: the artboard (white) found in the canvas's pixels.
  const calibrate = async () => {
    const box = await kit.getByTestId('kit-canvas-surface').boundingBox()
    const found = await kit.getByTestId('kit-canvas-surface').evaluate((canvas) => {
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
    const board = (await details()).artboards[0]
    const zoom = (found.x1 - found.x0 + 1) / found.scale / board.width
    const left = box.x + found.x0 / found.scale - board.x * zoom
    const top = box.y + found.y0 / found.scale - board.y * zoom
    return { zoom, toScreen: (x, y) => ({ x: left + x * zoom, y: top + y * zoom }) }
  }
  const view = await calibrate()
  const centre = (bounds) => view.toScreen(bounds.x + bounds.width / 2, bounds.y + bounds.height / 2)
  const all = await nodes()
  const ellipse = all.find((item) => item.kind === 'Ellipse')
  const rect = all.find((item) => item.kind.includes('Rect'))
  const star = all.find((item) => item.kind === 'Path')
  // The rectangle is clicked near its top-right corner, clear of the ellipse moved over it.
  const spot = (item, bounds) => (item.id === rect.id ? view.toScreen(bounds.x + bounds.width * 0.9, bounds.y + bounds.height * 0.12) : centre(bounds))
  const clickOn = async (item, modifiers = []) => {
    const at = spot(item, (await node(item.id)).bounds)
    for (const key of modifiers) await page.keyboard.down(key)
    await page.mouse.click(at.x, at.y)
    for (const key of modifiers) await page.keyboard.up(key)
  }
  const selectOnly = async (item) => {
    await page.keyboard.press('Escape')
    await clickOn(item)
    await until(async () => JSON.stringify((await details()).selection) === JSON.stringify([item.id]))
  }

  // The keyboard is the editor's; V picks the Selection tool, then the ellipse is selected.
  await kit.getByTestId('vector-canvas-region').click({ position: { x: 5, y: 5 } }).catch(() => undefined)
  await page.keyboard.press('v')
  await until(() => pressed('selection'), 30)
  await selectOnly(ellipse)
  step('selects the ellipse on the canvas', JSON.stringify((await details()).selection) === JSON.stringify([ellipse.id]))

  // Fill: a hex code typed is one step, read back by the object and the field.
  let before = await steps()
  await kit.getByTestId('vector-fill').fill('#e03131')
  await until(async () => (await node(ellipse.id)).fill === '#e03131')
  await page.waitForTimeout(300)
  step('a fill typed as hex is one history step and the field reads it back', (await steps()) === before + 1 && (await kit.getByTestId('vector-fill').inputValue()) === '#e03131', `${await steps()} vs ${before}`)

  // A stroke typed as hex, then stroke weight 6: one step each, the field and the object agree.
  await kit.getByTestId('vector-stroke').fill('#1a1a1a')
  await until(async () => (await node(ellipse.id)).stroke === '#1a1a1a')
  step('a stroke typed as hex is set', (await node(ellipse.id)).stroke === '#1a1a1a')
  before = await steps()
  await kit.getByTestId('vector-stroke-width').fill('6')
  await kit.getByTestId('vector-stroke-width').press('Enter')
  await until(async () => (await node(ellipse.id)).strokeWidth === 6)
  await page.waitForTimeout(300)
  step('stroke weight 6 is one step and reads back', (await steps()) === before + 1 && (await kit.getByTestId('vector-stroke-width').inputValue()) === '6', JSON.stringify({ now: await steps(), before, weight: (await node(ellipse.id)).strokeWidth }))

  // Dashes: Dashed line on, then the first dash 4: one step each, the pattern reads back.
  before = await steps()
  await kit.getByTestId('vector-stroke-dashed').click()
  const dashed = await until(async () => (await kit.getByTestId('vector-stroke-dash-0').count()) === 1)
  await kit.getByTestId('vector-stroke-dash-0').fill('4')
  await kit.getByTestId('vector-stroke-dash-0').press('Enter')
  await page.waitForTimeout(600)
  const dashInspect = await page.evaluate(async (id) => {
    const find = (list) => {
      for (const item of list) {
        if (item.id === id) return item
        const inside = find(item.children ?? [])
        if (inside) return inside
      }
      return null
    }
    return find((await window.devHost.connection().ai.inspect()).details.layers)?.strokeOptions?.dash ?? null
  }, ellipse.id)
  step('Dashed line and a dash of 4: one step each, the pattern is [4, 12]', dashed && (await steps()) === before + 2 && JSON.stringify(dashInspect) === '[4,12]', JSON.stringify({ dashInspect, now: await steps(), before }))

  // Opacity 50: one step, and Transparency reads it.
  before = await steps()
  await kit.getByTestId('vector-opacity').fill('50')
  await kit.getByTestId('vector-opacity').press('Enter')
  await until(async () => (await run('transparency.info', {}))?.opacity === 50)
  step('opacity 50 is one step and reads back', (await steps()) === before + 1 && (await run('transparency.info', {}))?.opacity === 50 && (await kit.getByTestId('vector-opacity').inputValue()) === '50')

  // A label dragged sideways scrubs the value live, and leaves one step.
  before = await steps()
  await kit.getByTestId('vector-opacity-label').scrollIntoViewIfNeeded()
  const label = await kit.getByTestId('vector-opacity-label').boundingBox()
  await page.mouse.move(label.x + label.width / 2, label.y + label.height / 2)
  await page.mouse.down()
  for (let i = 1; i <= 8; i += 1) await page.mouse.move(label.x + label.width / 2 + i * 6, label.y + label.height / 2)
  await page.waitForTimeout(400)
  const midway = (await run('transparency.info', {}))?.opacity
  await page.mouse.up()
  await until(async () => (await run('transparency.info', {}))?.opacity === 74)
  await page.waitForTimeout(400)
  step('scrubbing Opacity\'s label shows live and leaves one step', midway > 50 && (await run('transparency.info', {}))?.opacity === 74 && (await steps()) === before + 1, JSON.stringify({ midway, now: (await run('transparency.info', {}))?.opacity, steps: await steps(), before }))

  // The Color Picker from the Fill proxy: a drag in the square shows live; OK leaves one step.
  before = await steps()
  await kit.getByTestId('vector-proxy-fill').dblclick({ position: { x: 4, y: 4 } })
  const opened = await until(async () => (await kit.getByTestId('vector-color-dialog').count()) === 1)
  await shot('picker')
  const square = await kit.getByTestId('vector-color-dialog-square').boundingBox()
  await page.mouse.move(square.x + square.width * 0.2, square.y + square.height * 0.2)
  await page.mouse.down()
  for (let i = 1; i <= 6; i += 1) await page.mouse.move(square.x + square.width * (0.2 + i * 0.1), square.y + square.height * (0.2 + i * 0.05))
  await page.mouse.up()
  const shown = await kit.getByTestId('vector-color-dialog').locator('[data-color]').first().getAttribute('data-color')
  const live = await until(async () => (await node(ellipse.id)).fill === shown)
  await kit.getByTestId('vector-color-dialog-ok').click()
  await until(async () => (await kit.getByTestId('vector-color-dialog').count()) === 0)
  await page.waitForTimeout(500)
  step('the Color Picker shows the dragged colour live, and OK leaves one step', opened && live && (await node(ellipse.id)).fill === shown && (await steps()) === before + 1, JSON.stringify({ shown, fill: (await node(ellipse.id)).fill, steps: await steps(), before }))

  // Typed in the picker (CMYK), then Cancel: the fill goes back and no step is left.
  before = await steps()
  const was = (await node(ellipse.id)).fill
  await kit.getByTestId('vector-proxy-fill').dblclick({ position: { x: 4, y: 4 } })
  await until(async () => (await kit.getByTestId('vector-color-dialog').count()) === 1)
  await kit.getByTestId('vector-color-dialog-c').fill('100')
  await until(async () => (await node(ellipse.id)).fill !== was)
  await kit.getByTestId('vector-color-dialog-cancel').click()
  await until(async () => (await node(ellipse.id)).fill === was)
  await page.waitForTimeout(400)
  step('Cancel in the Color Picker puts the fill back and leaves no step', (await node(ellipse.id)).fill === was && (await steps()) === before, JSON.stringify({ fill: (await node(ellipse.id)).fill, was, steps: await steps(), before }))

  // None from the picker: the fill is None, and the panel says so instead of showing black.
  await kit.getByTestId('vector-proxy-fill').dblclick({ position: { x: 4, y: 4 } })
  await until(async () => (await kit.getByTestId('vector-color-dialog').count()) === 1)
  await kit.getByTestId('vector-color-dialog-none').click()
  await kit.getByTestId('vector-color-dialog-ok').click()
  await until(async () => (await node(ellipse.id)).fill === 'None')
  await page.waitForTimeout(400)
  step('None from the picker: the fill field and chip show None, not a colour', (await node(ellipse.id)).fill === 'None' && (await kit.getByTestId('vector-fill').inputValue()) === '' && (await kit.getByTestId('vector-fill').getAttribute('data-kind')) === 'none' && (await kit.getByTestId('vector-fill-chip').getAttribute('data-kind')) === 'none')

  // A gradient swatch applies (it failed with an error before) and shows as a gradient.
  const swatchKinds = await kit.locator('[data-testid^="vector-swatch-"][data-kind]').evaluateAll((chips) => chips.map((chip) => ({ id: chip.getAttribute('data-testid'), kind: chip.getAttribute('data-kind'), background: chip.style.background })))
  const grays = swatchKinds.filter((chip) => /K=\d0$/.test(chip.id))
  const gradients = swatchKinds.filter((chip) => chip.kind === 'gradient')
  step('swatches show greys as colours and gradients as ramps; only [None] is a slash', grays.length === 9 && grays.every((chip) => chip.kind === 'color' && chip.background) && gradients.length >= 4 && gradients.every((chip) => chip.background.includes('gradient')) && swatchKinds.filter((chip) => chip.kind === 'none').length === 1, JSON.stringify({ grays: grays.length, gradients: gradients.length, none: swatchKinds.filter((chip) => chip.kind === 'none').map((chip) => chip.id) }))
  await shot('swatches')
  before = await steps()
  await kit.getByTestId('vector-swatch-Sunset').click()
  await until(async () => (await proxies())?.fill?.type === 'gradient')
  await page.waitForTimeout(400)
  const sunsetStops = (await proxies())?.fill?.gradient?.stops?.length
  step('a click on the Sunset swatch fills with its gradient, one step', (await proxies())?.fill?.type === 'gradient' && (await steps()) === before + 1 && (await kit.getByTestId('vector-fill-chip').getAttribute('data-kind')) === 'gradient' && (await kit.getByTestId('vector-gradient-stop-0').count()) === 1, JSON.stringify({ sunsetStops, steps: await steps(), before }))

  // Gradient panel: a press under the ramp adds a stop (one step); dragging a stop is one step.
  before = await steps()
  await kit.getByTestId('vector-gradient-stops').scrollIntoViewIfNeeded()
  const row = await kit.getByTestId('vector-gradient-stops').boundingBox()
  await page.mouse.click(row.x + row.width * 0.5, row.y + row.height / 2)
  await until(async () => (await proxies())?.fill?.gradient?.stops?.length === sunsetStops + 1)
  await page.waitForTimeout(300)
  step('a press under the ramp adds a stop, one step', (await proxies())?.fill?.gradient?.stops?.length === sunsetStops + 1 && (await steps()) === before + 1, JSON.stringify({ stops: (await proxies())?.fill?.gradient?.stops?.length, steps: await steps(), before }))
  before = await steps()
  const added = kit.getByTestId('vector-gradient-stop-1')
  await added.scrollIntoViewIfNeeded()
  const addedBox = await added.boundingBox()
  await page.mouse.move(addedBox.x + addedBox.width / 2, addedBox.y + addedBox.height / 2)
  await page.mouse.down()
  for (let i = 1; i <= 6; i += 1) await page.mouse.move(addedBox.x + addedBox.width / 2 - i * 8, addedBox.y + addedBox.height / 2)
  await page.mouse.up()
  await page.waitForTimeout(600)
  const offsets = (await proxies())?.fill?.gradient?.stops?.map((stop) => Math.round(stop.offset * 100))
  step('dragging a stop moves it and leaves one step', offsets?.[1] < 45 && (await steps()) === before + 1, JSON.stringify({ offsets, steps: await steps(), before }))

  // The stop's colour from the picker: the stop changes, the gradient stays a gradient.
  before = await steps()
  await kit.getByTestId('vector-gradient-stop-color').click()
  await until(async () => (await kit.getByTestId('vector-color-dialog').count()) === 1)
  await kit.getByTestId('vector-color-dialog-hex').fill('2266ff')
  await kit.getByTestId('vector-color-dialog-ok').click()
  await until(async () => (await kit.getByTestId('vector-color-dialog').count()) === 0)
  await page.waitForTimeout(500)
  const recoloured = await proxies()
  const blue = recoloured?.fill?.gradient?.stops?.[1]?.color
  step('a stop recoloured in the picker keeps the gradient, one step', recoloured?.fill?.type === 'gradient' && blue && Math.round(blue.b * 255) === 255 && Math.round(blue.r * 255) === 0x22 && (await steps()) === before + 1, JSON.stringify({ type: recoloured?.fill?.type, blue, steps: await steps(), before }))
  await shot('gradient-panel')

  // Reverse and Radial.
  before = await steps()
  const first = (await proxies())?.fill?.gradient?.stops?.[0]?.color
  await kit.getByTestId('vector-gradient-reverse').click()
  await until(async () => JSON.stringify((await proxies())?.fill?.gradient?.stops?.at(-1)?.color) === JSON.stringify(first))
  await kit.getByTestId('vector-gradient-kind-radial').click()
  await until(async () => (await proxies())?.fill?.gradient?.kind === 'Radial')
  step('Reverse and Radial: the stops turn round, the gradient becomes radial, a step each', JSON.stringify((await proxies())?.fill?.gradient?.stops?.at(-1)?.color) === JSON.stringify(first) && (await proxies())?.fill?.gradient?.kind === 'Radial' && (await steps()) === before + 2)
  await kit.getByTestId('vector-gradient-kind-linear').click()
  await until(async () => (await proxies())?.fill?.gradient?.kind === 'Linear')

  // The canvas's gradient annotator: a double-click on a stop opens the Color Picker on it.
  await page.keyboard.press('g')
  await until(() => pressed('gradient'), 30)
  await page.waitForTimeout(400)
  const box = (await details()).selectionBounds
  const stopsNow = (await proxies())?.fill?.gradient?.stops ?? []
  const lastStop = stopsNow.at(-1)
  const chip = view.toScreen(box.x + box.width * (lastStop?.offset ?? 1), box.y + box.height / 2 + 10 / view.zoom)
  await page.mouse.dblclick(chip.x, chip.y)
  const asked = await until(async () => (await kit.getByTestId('vector-color-dialog').count()) === 1, 40)
  before = await steps()
  if (asked) {
    await kit.getByTestId('vector-color-dialog-hex').fill('11aa44')
    await kit.getByTestId('vector-color-dialog-ok').click()
    await until(async () => (await kit.getByTestId('vector-color-dialog').count()) === 0)
    await page.waitForTimeout(500)
  }
  const annotated = await proxies()
  const green = annotated?.fill?.gradient?.stops?.at(-1)?.color
  step('a stop double-clicked on the canvas opens the Color Picker, and recolours that stop as one step', asked && annotated?.fill?.type === 'gradient' && green && Math.round(green.g * 255) === 0xaa && (await steps()) === before + 1, JSON.stringify({ asked, green, steps: await steps(), before }))
  await page.keyboard.press('Escape')
  await page.keyboard.press('v')
  await until(() => pressed('selection'), 30)

  // Arithmetic in a field, and Tab moves on to the next one.
  await selectOnly(ellipse)
  const xField = kit.getByTestId('vector-bounds-X')
  await xField.fill('50+25')
  await xField.press('Tab')
  await until(async () => Math.round((await details()).selectionBounds.x) === 75)
  const focused = await kit.locator(':focus').getAttribute('data-testid').catch(() => null)
  step('X takes 50+25 as 75, and Tab moves on to Y', Math.round((await details()).selectionBounds.x) === 75 && focused === 'vector-bounds-Y', JSON.stringify({ x: (await details()).selectionBounds.x, focused }))
  await page.keyboard.press('Escape')

  // The reference point: the centre makes X the centre's place.
  await kit.getByTestId('vector-reference-4').click()
  const bounds = (await details()).selectionBounds
  await until(async () => Number(await xField.inputValue()) === Math.round((bounds.x + bounds.width / 2) * 100) / 100)
  step('the centre reference point shows X at the centre', Number(await xField.inputValue()) === Math.round((bounds.x + bounds.width / 2) * 100) / 100, await xField.inputValue())
  await kit.getByTestId('vector-reference-0').click()

  // Shift+X swaps the fill and the stroke.
  const swapBefore = await proxies()
  await page.keyboard.press('Shift+X')
  await until(async () => (await proxies())?.stroke?.type === swapBefore?.fill?.type && (await proxies())?.fill?.type === swapBefore?.stroke?.type)
  step('Shift+X swaps the fill and the stroke', (await proxies())?.stroke?.type === 'gradient' && (await proxies())?.fill?.type === swapBefore?.stroke?.type)
  await page.keyboard.press(`${primary}+z`)

  // A swatch with the Stroke proxy in front paints the stroke.
  await kit.getByTestId('vector-proxy-stroke').click({ position: { x: 12, y: 12 } })
  await until(async () => (await details()).paint?.fillActive === false)
  await kit.getByTestId('vector-swatch-Black').click()
  await until(async () => (await node(ellipse.id)).stroke === '#000000')
  step('with the Stroke proxy in front a swatch paints the stroke', (await node(ellipse.id)).stroke === '#000000')
  await kit.getByTestId('vector-proxy-fill').click({ position: { x: 3, y: 3 } })
  await until(async () => (await details()).paint?.fillActive === true)

  // New Swatch from the colour in front: a dialog with its colour, then one more swatch.
  await kit.getByTestId('vector-fill').fill('#336699')
  await until(async () => (await node(ellipse.id)).fill === '#336699')
  const swatchCount = (await run('swatch.list', {}))?.swatches?.length
  await kit.getByTestId('vector-swatch-new').click()
  await until(async () => (await kit.getByTestId('vector-dialog').count()) === 1)
  const startColor = await kit.getByTestId('vector-dialog-param-color').inputValue()
  await kit.getByTestId('vector-dialog-param-name').fill('Brand Blue')
  await kit.getByTestId('vector-dialog-param-global').click()
  await kit.getByTestId('vector-dialog-ok').click()
  await until(async () => (await run('swatch.list', {}))?.swatches?.length === swatchCount + 1)
  const brand = (await run('swatch.list', {}))?.swatches?.find((swatch) => swatch.name === 'Brand Blue')
  await page.waitForTimeout(300)
  step('New Swatch starts from the fill and adds a global swatch, marked as global', startColor === '#336699' && brand?.hex === '#336699' && brand?.global === true && (await kit.locator('.vector-swatch[data-global]').count()) >= 1, JSON.stringify({ startColor, brand }))

  // Align to the artboard: two objects, both to the artboard's left edge.
  await selectOnly(rect)
  await clickOn(ellipse, ['Shift'])
  await until(async () => (await details()).selection.length === 2)
  await kit.getByTestId('vector-align-to').selectOption('artboard')
  before = await steps()
  await kit.getByTestId('vector-align-left').click()
  const board = (await details()).artboards[0]
  await until(async () => Math.round((await node(ellipse.id)).bounds.x) === Math.round(board.x) && Math.round((await node(rect.id)).bounds.x) === Math.round(board.x))
  step('Align to the artboard puts both objects on its left edge, one step', Math.round((await node(ellipse.id)).bounds.x) === Math.round(board.x) && Math.round((await node(rect.id)).bounds.x) === Math.round(board.x) && (await steps()) === before + 1)
  // Again: nothing moves, and the panel says so.
  await kit.getByTestId('vector-align-left').click()
  const said = await until(async () => (await kit.locator('.kui-toast').filter({ hasText: /moved|移动/ }).count()) > 0, 30)
  step('aligning what is already aligned says nothing moved', said, await kit.locator('.kui-toast').allTextContents().then((texts) => texts.join(' | ')))
  await kit.getByTestId('vector-align-to').selectOption('selection')

  // Distribute: three objects, horizontal centres evenly spaced, one step.
  await page.keyboard.press(`${primary}+a`)
  await until(async () => (await details()).selection.length === 3)
  before = await steps()
  await kit.getByTestId('vector-distribute-hcenter').click()
  await until(async () => (await steps()) === before + 1)
  const centres = (await nodes()).map((item) => item.bounds.x + item.bounds.width / 2).sort((a, b) => a - b)
  step('Distribute horizontal centres spaces the centres evenly, one step', Math.abs(centres[1] - centres[0] - (centres[2] - centres[1])) < 0.5 && (await steps()) === before + 1, JSON.stringify(centres))
  // Back to where they were before Align and Distribute.
  await page.keyboard.press(`${primary}+z`)
  await page.keyboard.press(`${primary}+z`)
  await page.waitForTimeout(400)

  // Pathfinder has all ten.
  const pathfinders = await kit.locator('[data-testid^="vector-pathfinder-"]').count()
  step('Pathfinder shows its ten operations', pathfinders === 10, String(pathfinders))
  await shot('panels')

  // The eyedropper: a click copies the clicked object's appearance; Shift copies only the colour.
  await selectOnly(rect)
  await page.keyboard.press('i')
  await until(() => pressed('eyedropper'), 30)
  const starFill = (await node(star.id)).fill
  await clickOn(star)
  await until(async () => (await node(rect.id)).fill === starFill)
  step('the eyedropper copies the clicked object\'s fill to the selection', (await node(rect.id)).fill === starFill, JSON.stringify({ rect: (await node(rect.id)).fill, starFill }))
  await run('paint.setStroke', { color: '#ff00ff' })
  await run('stroke.set', { weight: 3 })
  await until(async () => (await node(rect.id)).strokeWidth === 3)
  const ellipseFill = (await node(ellipse.id)).fill
  await clickOn(ellipse, ['Shift'])
  await until(async () => (await node(rect.id)).fill === ellipseFill)
  const after = await node(rect.id)
  step('Shift with the eyedropper takes only the colour: the stroke stays', after.fill === ellipseFill && after.stroke === '#ff00ff' && after.strokeWidth === 3, JSON.stringify({ fill: after.fill, ellipseFill, stroke: after.stroke, weight: after.strokeWidth }))
  await page.keyboard.press('v')
  await until(() => pressed('selection'), 30)

  // The control bar: the weight set there is one step and the Stroke panel agrees.
  await selectOnly(star)
  await run('paint.setStroke', { color: '#202020' })
  await until(async () => (await node(star.id)).stroke === '#202020')
  before = await steps()
  await kit.getByTestId('vector-control-weight').fill('2.5')
  await kit.getByTestId('vector-control-weight').press('Enter')
  await until(async () => (await node(star.id)).strokeWidth === 2.5)
  await page.waitForTimeout(300)
  step('the control bar sets the stroke weight, one step, and the panel reads it', (await node(star.id)).strokeWidth === 2.5 && (await steps()) === before + 1 && (await kit.getByTestId('vector-stroke-width').inputValue()) === '2.5' && (await kit.getByTestId('vector-control-fill').count()) === 1 && (await kit.getByTestId('vector-control-align-left').count()) === 1)
  await shot('control-bar')

  const colourWarnings = warnings.filter((text) => /does not conform to the required format|#rrggbb/i.test(text))
  step('no colour input warnings in the console ("None", gradients)', colourWarnings.length === 0, colourWarnings.slice(0, 2).join(' | '))
  step('no page errors', errors.length === 0, errors.slice(0, 3).join(' | '))
  await context.close()
} catch (error) {
  failures += 1
  console.log(`FAIL ${which} the test stopped: ${error?.stack ?? error}`)
} finally {
  await browser.close()
  await server.close()
}
console.log(failures ? `${failures} failed` : 'all passed')
process.exit(failures ? 1 : 0)
