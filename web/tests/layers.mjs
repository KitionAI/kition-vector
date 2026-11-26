// Layers, artboards and the panel dock in a real browser, through the development host:
// node tests/layers.mjs [chromium|webkit]
// Starts the Vite dev server itself, opens dev.html, and prints PASS or FAIL per step (plan
// 2026-11-25, stage 4, feature vector.editor.layers): the dock's groups and tabs, the Window menu's
// panel rows, nothing overlapping or cut off at 1440 × 900, 1280 × 720 and 900 wide in every display
// mode; the Layers panel's rows, a drag that restacks (one step, undone by one Undo) and one into
// another layer, renaming with tool letters typed, the eye and the lock (a locked object is not
// picked on the canvas), Shift and Cmd clicks, new sublayer, duplicate and delete, the right-click
// menu, a drawing of hundreds of objects; the Artboards panel's new, rename, options, reorder and
// delete, the status bar's artboard navigation and fitting; History's jumps; and menu rows that
// grey out instead of failing. SHOTS=<folder> saves screenshots.
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

/** Opens the editor at `width` × `height` in display mode `mode`; the page, the kit's frame and helpers. */
async function open(width, height, mode = 'menus') {
  const context = await browser.newContext({ viewport: { width, height } })
  const page = await context.newPage()
  page.on('pageerror', (error) => errors.push(error.message))
  await page.goto(`${origin}/dev.html?modes=${mode},line,classic,menus,focus&locale=${locale}`)
  await page.waitForFunction(() => document.getElementById('status')?.textContent?.includes(' open in '), null, { timeout: 60000 })
  const kit = page.frameLocator('#kit')
  await kit.getByTestId('vector-dock').waitFor({ state: 'visible', timeout: 30000 })
  await page.waitForTimeout(600)
  return { context, page, kit }
}

/**
 * The editor's parts on the screen (the rail, the canvas, the dock, the status bar, the menu bar),
 * which overlap, and which reach outside the window or are cut off inside the dock.
 */
async function layoutReport(page) {
  const frame = await (await page.$('#kit')).contentFrame()
  return frame.evaluate(() => {
    const box = (selector) => {
      const element = document.querySelector(selector)
      if (!element) return null
      const rect = element.getBoundingClientRect()
      return rect.width > 0 && rect.height > 0 ? { x0: rect.left, y0: rect.top, x1: rect.right, y1: rect.bottom } : null
    }
    const parts = {
      rail: box('.vector-rail'),
      canvas: box('[data-testid="kit-canvas-surface"]'),
      dock: box('[data-testid="vector-side"]'),
      status: box('[data-testid="vector-status"]')?.y0 != null ? box('.kui-status') ?? box('[data-testid="vector-status"]') : null,
    }
    const overlaps = []
    const names = Object.keys(parts).filter((name) => parts[name])
    for (let i = 0; i < names.length; i += 1) {
      for (let j = i + 1; j < names.length; j += 1) {
        const a = parts[names[i]]
        const b = parts[names[j]]
        const w = Math.min(a.x1, b.x1) - Math.max(a.x0, b.x0)
        const h = Math.min(a.y1, b.y1) - Math.max(a.y0, b.y0)
        if (w > 1 && h > 1) overlaps.push(`${names[i]}×${names[j]}`)
      }
    }
    const outside = names.filter((name) => parts[name].x0 < -1 || parts[name].y0 < -1 || parts[name].x1 > innerWidth + 1 || parts[name].y1 > innerHeight + 1)
    // Every shown group's tab strip is on screen, and every tab in it inside the strip.
    const clipped = []
    for (const strip of document.querySelectorAll('.kui-dock-strip')) {
      const s = strip.getBoundingClientRect()
      if (s.bottom > innerHeight + 1 || s.top < -1) clipped.push(`strip ${strip.getAttribute('data-kit-ui-id')}`)
      for (const tab of strip.querySelectorAll('.kui-dock-tab')) {
        const t = tab.getBoundingClientRect()
        if (t.right > s.right + 1) clipped.push(`tab ${tab.textContent}`)
      }
    }
    return { parts, overlaps, outside, clipped }
  })
}

try {
  // ---------- The dock in every display mode and size ----------
  for (const mode of ['menus', 'line', 'classic', 'focus']) {
    const { context, page, kit } = await open(1440, 900, mode === 'focus' ? 'menus' : mode)
    if (mode === 'focus') {
      await kit.locator('.kui-app').focus()
      await page.keyboard.press(`${(await page.evaluate(() => /Mac|iPhone|iPad/i.test(navigator.platform))) ? 'Meta' : 'Control'}+Backslash`)
      await page.waitForTimeout(400)
    }
    for (const [width, height] of [
      [1440, 900],
      [1280, 720],
      [900, 800],
    ]) {
      await page.setViewportSize({ width, height })
      await page.waitForTimeout(400)
      const report = await layoutReport(page)
      const needed = mode === 'focus' ? ['canvas'] : ['rail', 'canvas', 'dock']
      const missing = needed.filter((name) => !report.parts[name])
      step(`${mode} ${width}×${height}: nothing overlaps, nothing is cut off`, missing.length === 0 && report.overlaps.length === 0 && report.outside.length === 0 && report.clipped.length === 0, JSON.stringify({ missing, overlaps: report.overlaps, outside: report.outside, clipped: report.clipped }))
      if (shots && width === 1280 && mode !== 'focus') await page.screenshot({ path: `${shots}/dock-${mode}-${width}x${height}.png` })
    }
    await context.close()
  }

  const { page, kit } = await open(1440, 900)
  const shot = async (name) => {
    if (shots) await page.screenshot({ path: `${shots}/${name}.png` })
  }
  const until = async (check, tries = 60) => {
    for (let i = 0; i < tries; i += 1) {
      if (await check().catch(() => false)) return true
      await page.waitForTimeout(100)
    }
    return false
  }
  const details = async () => (await page.evaluate(async () => window.devHost.connection().ai.inspect())).details
  const run = async (command, params = {}) => (await page.evaluate(([c, p]) => window.devHost.connection().ai.run([{ command: c, params: p }], false), [command, params])).completed?.[0]?.result
  const layers = async () => (await details()).layers
  const childrenOf = async (id) => (await layers()).find((layer) => layer.id === id)?.children ?? []
  const steps = async () => (await details()).history.length
  const primary = (await page.evaluate(() => /Mac|iPhone|iPad/i.test(navigator.platform))) ? 'Meta' : 'Control'
  const row = (id) => kit.getByTestId(`vector-layer-${id}`)
  /** The artboards on screen, found in the canvas's pixels (white): their left edge in CSS pixels and the zoom. */
  const whiteSpan = () =>
    kit.getByTestId('kit-canvas-surface').evaluate((canvas) => {
      const { width: w, height: h } = canvas
      const data = canvas.getContext('2d').getImageData(0, 0, w, h).data
      const y = Math.floor(h / 2)
      let first = -1
      let last = -1
      for (let x = 0; x < w; x += 1) {
        const i = (y * w + x) * 4
        if (data[i] === 255 && data[i + 1] === 255 && data[i + 2] === 255) {
          if (first < 0) first = x
          last = x
        }
      }
      const scale = canvas.width / canvas.clientWidth
      return { first: first / scale, last: last / scale }
    })
  /** The middle of `bounds` (the drawing's points) on the screen, from the artboard found in the canvas's pixels. */
  const toScreen = async (bounds) => {
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
    return { x: left + (bounds.x + bounds.width / 2) * zoom, y: top + (bounds.y + bounds.height / 2) * zoom }
  }

  // ---------- The dock ----------
  const groups = await kit.locator('[data-testid^="vector-dock-group-"]').evaluateAll((elements) => elements.map((element) => element.getAttribute('data-testid')))
  step('the dock shows Properties, Layers and Swatches, each a group of tabs', JSON.stringify(groups) === JSON.stringify(['vector-dock-group-properties-group', 'vector-dock-group-layers-group', 'vector-dock-group-swatches-group']), JSON.stringify(groups))
  const sections = await Promise.all(['vector-transform', 'vector-appearance', 'vector-gradient', 'vector-align', 'vector-pathfinder', 'vector-layers-panel', 'vector-swatches'].map((id) => kit.getByTestId(id).count()))
  step('every section the earlier stages test is drawn once', sections.every((count) => count === 1), JSON.stringify(sections))
  await shot('dock')
  // A tab brings its panel forward; collapsing a group leaves its strip.
  await kit.getByTestId('vector-dock-tab-artboards').click()
  step('the Artboards tab shows the Artboards panel in place of Layers', (await kit.getByTestId('vector-artboards-panel').count()) === 1 && (await kit.getByTestId('vector-layers-panel').count()) === 0)
  await kit.getByTestId('vector-dock-tab-layers').click()
  await kit.getByTestId('vector-dock-collapse-swatches-group').click()
  step('a group collapses to its tab strip', (await kit.getByTestId('vector-swatches').count()) === 0 && (await kit.getByTestId('vector-dock-group-swatches-group').getAttribute('data-collapsed')) === 'true')
  await kit.getByTestId('vector-dock-collapse-swatches-group').click()

  // ---------- Window lists the panels ----------
  const windowRow = async (tab) => {
    await kit.getByTestId('vector-menu-Window').click()
    const item = kit.getByTestId(`vector-control-window.${tab}`)
    await item.waitFor({ state: 'visible', timeout: 5000 })
    return item
  }
  let historyRow = await windowRow('history')
  const unchecked = await historyRow.getAttribute('aria-checked')
  await historyRow.click()
  await until(async () => (await kit.getByTestId('vector-history-panel').count()) === 1, 30)
  historyRow = await windowRow('history')
  const checked = await historyRow.getAttribute('aria-checked')
  const pathfinderMenu = await kit.locator('[data-kit-ui-id="menu:Window/Pathfinder"]').count()
  await page.keyboard.press('Escape')
  step('Window › History shows the History panel, checked once shown; Pathfinder keeps its rows', (await kit.getByTestId('vector-history-panel').count()) === 1 && unchecked === 'false' && checked === 'true' && pathfinderMenu === 1, JSON.stringify({ unchecked, checked, pathfinderMenu }))
  const layersRow = await windowRow('layers')
  await layersRow.click()
  await until(async () => (await kit.getByTestId('vector-layers-panel').count()) === 0, 30)
  step('Window › Layers hides the Layers group while it shows in front', (await kit.getByTestId('vector-dock-group-layers-group').count()) === 0)
  await (await windowRow('layers')).click()
  await until(async () => (await kit.getByTestId('vector-layers-panel').count()) === 1, 30)
  step('and shows it again', (await kit.getByTestId('vector-layers-panel').count()) === 1)
  await (await windowRow('history')).click()
  await until(async () => (await kit.getByTestId('vector-history-panel').count()) === 0, 30)

  // ---------- Layers: rows, restacking by drag, one undo ----------
  const [details0, layer1] = await layers()
  const [ellipse, rect] = layer1.children
  step('every row has an eye and a lock, objects too', (await kit.getByTestId(`vector-layer-visibility-${ellipse.id}`).count()) === 1 && (await kit.getByTestId(`vector-layer-lock-${rect.id}`).count()) === 1 && (await kit.getByTestId(`vector-layer-lock-${layer1.id}`).count()) === 1)
  const dragRow = async (from, to, where) => {
    const a = await row(from).boundingBox()
    const b = await row(to).boundingBox()
    const y = where === 'above' ? b.y + 3 : where === 'below' ? b.y + b.height - 3 : b.y + b.height / 2
    await page.mouse.move(a.x + a.width * 0.6, a.y + a.height / 2)
    await page.mouse.down()
    await page.mouse.move(a.x + a.width * 0.6, a.y + a.height / 2 + 6, { steps: 2 })
    await page.mouse.move(b.x + b.width * 0.6, y, { steps: 6 })
    await page.mouse.up()
  }
  let before = await steps()
  await dragRow(rect.id, ellipse.id, 'above')
  await until(async () => (await childrenOf(layer1.id))[0]?.id === rect.id)
  const restacked = (await childrenOf(layer1.id)).map((node) => node.id)
  step('a row dragged above another restacks it, as one history step', restacked[0] === rect.id && (await steps()) === before + 1, JSON.stringify({ restacked, steps: await steps(), before }))
  await shot('layers-dragged')
  await kit.locator('.kui-app').focus()
  await page.keyboard.press(`${primary}+z`)
  await until(async () => (await childrenOf(layer1.id))[0]?.id === ellipse.id)
  step('one Undo puts the stacking back', (await childrenOf(layer1.id))[0]?.id === ellipse.id)

  // Into another layer.
  await dragRow(ellipse.id, details0.id, 'inside')
  await until(async () => (await childrenOf(details0.id)).some((node) => node.id === ellipse.id))
  step('a row dropped on another layer goes into it', (await childrenOf(details0.id)).some((node) => node.id === ellipse.id) && !(await childrenOf(layer1.id)).some((node) => node.id === ellipse.id))
  await page.keyboard.press(`${primary}+z`)
  await until(async () => (await childrenOf(layer1.id)).some((node) => node.id === ellipse.id))

  // ---------- Rename: the letters typed are the name's, not tool keys ----------
  const tool = async () => (await details()).tool
  const toolBefore = await tool()
  await kit.getByTestId(`vector-layer-name-${rect.id}`).dblclick()
  const field = kit.getByTestId(`vector-layer-rename-${rect.id}`)
  await field.waitFor({ state: 'visible', timeout: 5000 })
  await page.keyboard.press(`${primary}+a`)
  await page.keyboard.type('Panel vp')
  await page.keyboard.press('Enter')
  await until(async () => (await childrenOf(layer1.id)).find((node) => node.id === rect.id)?.name === 'Panel vp')
  step('a double-click renames a row; v and p typed do not pick tools', (await childrenOf(layer1.id)).find((node) => node.id === rect.id)?.name === 'Panel vp' && (await tool()) === toolBefore, JSON.stringify({ tool: await tool(), toolBefore }))

  // ---------- Clicks: select, Shift and Cmd ----------
  await kit.getByTestId(`vector-layer-name-${ellipse.id}`).click()
  await until(async () => JSON.stringify((await details()).selection) === JSON.stringify([ellipse.id]))
  step('a click on an object row selects it on the canvas', JSON.stringify((await details()).selection) === JSON.stringify([ellipse.id]))
  await kit.getByTestId(`vector-layer-name-${rect.id}`).click({ modifiers: ['Shift'] })
  await until(async () => (await details()).selection.length === 2)
  step('Shift-click selects the rows between', (await details()).selection.length === 2, JSON.stringify((await details()).selection))
  await kit.getByTestId(`vector-layer-name-${ellipse.id}`).click({ modifiers: [primary] })
  await until(async () => JSON.stringify((await details()).selection) === JSON.stringify([rect.id]))
  step('Cmd-click takes one row out', JSON.stringify((await details()).selection) === JSON.stringify([rect.id]), JSON.stringify((await details()).selection))
  step('the selected row shows the selection square in its layer colour', (await kit.getByTestId(`vector-layer-select-${rect.id}`).getAttribute('data-mark')) === 'self' && (await kit.getByTestId(`vector-layer-select-${layer1.id}`).getAttribute('data-mark')) === 'inside')

  // ---------- Eye and lock ----------
  await kit.getByTestId(`vector-layer-visibility-${ellipse.id}`).click()
  await until(async () => (await childrenOf(layer1.id)).find((node) => node.id === ellipse.id)?.visible === false)
  step('the eye on an object row hides the object', (await childrenOf(layer1.id)).find((node) => node.id === ellipse.id)?.visible === false)
  await kit.getByTestId(`vector-layer-visibility-${ellipse.id}`).click()
  await until(async () => (await childrenOf(layer1.id)).find((node) => node.id === ellipse.id)?.visible === true)
  await run('edit.deselectAll')
  await kit.getByTestId(`vector-layer-lock-${ellipse.id}`).click()
  await until(async () => (await childrenOf(layer1.id)).find((node) => node.id === ellipse.id)?.locked === true)
  // The canvas: a click on the locked ellipse's middle does not pick it.
  await kit.locator('.kui-app').focus()
  await page.keyboard.press('v')
  const at = await toScreen((await childrenOf(layer1.id)).find((node) => node.id === ellipse.id).bounds)
  await page.mouse.click(at.x, at.y)
  await page.waitForTimeout(400)
  const pickedLocked = (await details()).selection.includes(ellipse.id)
  step('the lock on an object row locks it, and a click on it on the canvas does not pick it', (await childrenOf(layer1.id)).find((node) => node.id === ellipse.id)?.locked === true && !pickedLocked, JSON.stringify((await details()).selection))
  await kit.getByTestId(`vector-layer-lock-${ellipse.id}`).click()
  await until(async () => (await childrenOf(layer1.id)).find((node) => node.id === ellipse.id)?.locked === false)

  // ---------- New sublayer, duplicate, delete, the menus ----------
  await kit.getByTestId(`vector-layer-name-${layer1.id}`).click()
  await until(async () => (await details()).layerRows.includes(layer1.id))
  before = (await childrenOf(layer1.id)).length
  await kit.getByTestId('vector-new-sublayer').click()
  await until(async () => (await childrenOf(layer1.id)).length === before + 1)
  const sub = (await childrenOf(layer1.id)).find((node) => node.kind === 'Layer')
  step('New Sublayer adds a layer inside the current one', Boolean(sub), JSON.stringify((await childrenOf(layer1.id)).map((node) => node.kind)))
  await kit.getByTestId(`vector-layer-name-${rect.id}`).click()
  await row(rect.id).click({ button: 'right' })
  const menu = kit.getByTestId('vector-layers-context')
  await menu.waitFor({ state: 'visible', timeout: 5000 })
  step('a right-click opens the panel menu on the row', (await menu.count()) === 1)
  await shot('layers-context-menu')
  const count = async () => (await childrenOf(layer1.id)).length
  before = await count()
  await kit.getByTestId('vector-layers-context-layer.duplicate').click()
  await until(async () => (await count()) === before + 1)
  step('Duplicate copies the row', (await count()) === before + 1)
  await kit.getByTestId('vector-delete-layer').click()
  await until(async () => (await count()) === before)
  step('Delete removes the highlighted row', (await count()) === before)

  // ---------- A drawing of hundreds of objects ----------
  const many = []
  for (let i = 0; i < 400; i += 1) many.push({ command: 'shape.rectangle', params: { x: (i % 20) * 30, y: Math.floor(i / 20) * 25, width: 20, height: 20 } })
  await page.evaluate((list) => window.devHost.connection().ai.run(list, false), many)
  await until(async () => (await count()) > 400, 100)
  const total = (await count()) + 2
  const drawn = await kit.locator('[role="treeitem"]').count()
  const t0 = Date.now()
  await kit.getByTestId('vector-layers-list').evaluate((list) => {
    list.scrollTop = list.scrollHeight / 2
  })
  await page.waitForTimeout(100)
  const middle = await kit.locator('[role="treeitem"]').count()
  const elapsed = Date.now() - t0
  const lastRow = await kit.locator('[role="treeitem"]').last().getAttribute('data-testid')
  step('a layer of 400 objects (open) draws only the rows in view, and scrolls', total > 400 && drawn < 80 && middle > 0 && middle < 80 && lastRow !== null && elapsed < 2000, JSON.stringify({ total, drawn, middle, elapsed }))
  await shot('layers-many')
  await kit.locator('.kui-app').focus()
  await page.keyboard.press(`${primary}+z`)

  // ---------- Artboards ----------
  await kit.getByTestId('vector-dock-tab-artboards').click()
  await kit.getByTestId('vector-artboard-new').click()
  await until(async () => (await details()).artboards.length === 2)
  step('New Artboard adds an artboard to the list', (await details()).artboards.length === 2 && (await kit.getByTestId('vector-artboard-row-1').count()) === 1)
  await kit.getByTestId('vector-artboard-name-1').dblclick()
  await kit.getByTestId('vector-artboard-rename-1').waitFor({ state: 'visible', timeout: 5000 })
  await page.keyboard.press(`${primary}+a`)
  await page.keyboard.type('Back')
  await page.keyboard.press('Enter')
  await until(async () => (await details()).artboards[1]?.name === 'Back')
  step('a double-click renames an artboard', (await details()).artboards[1]?.name === 'Back')
  // Options: the width typed is previewed and set on OK.
  await kit.getByTestId('vector-artboard-options-1').click()
  await kit.getByTestId('vector-dialog').waitFor({ state: 'visible', timeout: 5000 })
  await kit.getByTestId('vector-artboard-width').fill('400')
  await kit.getByTestId('vector-artboard-width').press('Tab')
  await page.waitForTimeout(400)
  await shot('artboard-options')
  await kit.getByTestId('vector-dialog-ok').click()
  await until(async () => (await details()).artboards[1]?.width === 400)
  step('Artboard Options sets the width', (await details()).artboards[1]?.width === 400, JSON.stringify((await details()).artboards[1]))
  // The status bar: next, previous and the list move the view to the artboard.
  const zoomOf = async () => Number(((await kit.locator('.kui-zoom-value').first().textContent()) ?? '').replace(/[^0-9.]/g, ''))
  const pick = kit.getByTestId('vector-artboard-pick')
  await kit.getByTestId('vector-artboard-previous').click()
  await until(async () => (await pick.inputValue()) === '0')
  await page.waitForTimeout(300)
  const s0 = await whiteSpan()
  await kit.getByTestId('vector-artboard-next').click()
  await until(async () => (await pick.inputValue()) === '1')
  await page.waitForTimeout(300)
  const s1 = await whiteSpan()
  await pick.selectOption('0')
  await page.waitForTimeout(300)
  const back = await whiteSpan()
  step('the status bar steps through the artboards and the list goes to one, each fitted in the window', Math.abs(s1.first - s0.first) > 20 && Math.abs(back.first - s0.first) < 3, JSON.stringify({ s0, s1, back }))
  const fitted = await zoomOf()
  await kit.getByTestId('vector-artboard-fit-all').click()
  await page.waitForTimeout(300)
  step('Fit All in Window shows both artboards (zooms out)', (await zoomOf()) < fitted, JSON.stringify({ fitted, all: await zoomOf() }))
  await shot('artboard-nav')
  // Reorder: Back moves up to number 1.
  await kit.getByTestId('vector-artboard-row-1').click()
  await kit.getByTestId('vector-artboard-up').click()
  await until(async () => (await details()).artboards[0]?.name === 'Back')
  step('the up arrow moves an artboard up the order', (await details()).artboards[0]?.name === 'Back')
  await kit.getByTestId('vector-artboard-row-0').click()
  await kit.getByTestId('vector-artboard-delete').click()
  await until(async () => (await details()).artboards.length === 1)
  step('Delete Artboard removes the active artboard', (await details()).artboards.length === 1 && (await details()).artboards[0].name !== 'Back')

  // ---------- History ----------
  await (await windowRow('history')).click()
  await kit.getByTestId('vector-history-panel').waitFor({ state: 'visible', timeout: 5000 })
  const done = await steps()
  await kit.getByTestId(`vector-history-${done - 2}`).click()
  await until(async () => (await steps()) === done - 2)
  step('a click on a History row goes back to it', (await steps()) === done - 2 && (await details()).redo.length >= 2)
  await shot('history')
  await kit.getByTestId(`vector-history-${done}`).click()
  await until(async () => (await steps()) === done)
  step('and a click on a dimmed row forward again', (await steps()) === done)

  // ---------- Rows that cannot run grey out ----------
  await run('edit.deselectAll')
  await kit.getByTestId('vector-menu-Object').click()
  await kit.locator('[data-kit-ui-id="menu:Object/Blend"]').first().hover()
  const make = kit.getByTestId('vector-cmd-object.blend.make')
  await make.waitFor({ state: 'visible', timeout: 5000 })
  const off = !(await make.isEnabled())
  await page.keyboard.press('Escape')
  await page.keyboard.press('Escape')
  step('Object › Blend › Make is grey with nothing selected, not an error', off)
} catch (error) {
  failures += 1
  console.log(`FAIL ${which} ${error instanceof Error ? error.message.split('\n').slice(0, 4).join(' | ') : String(error)}`)
} finally {
  if (errors.length) {
    failures += 1
    console.log(`FAIL ${which} page errors:\n${errors.slice(0, 8).join('\n')}`)
  }
  await browser.close()
  await server.close()
}
process.exit(failures ? 1 : 0)
