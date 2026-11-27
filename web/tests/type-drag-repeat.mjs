// test:type's "a drag with the Type tool makes area type" again and again (case
// regressions/issue-type-drag-repeat): in CI the WebKit step failed three times since kition-sdk
// d2b39aa (the text typed after the drag did not start with "Area"), gone on rerun. Each round runs
// that step as test:type runs it, from a fresh page load: a clean artboard, the step before it (new
// type in 30 pt with nothing selected: Cmd+Shift+A, T, the size field, a click, "Big", Escape), then
// T, a drag from (320, 60) to (520, 160) on the artboard, the text typed, and the same check: the
// new type's text starts with "Area" and Area Type Options answer. A failing round prints the tool
// and the editor's state before the press, every pointer and key event the editor's frame received,
// every message to and from the engine's worker (the commands and tool events the editor sent) and
// when the Type tool's button showed it chosen, each with its time.
// node tests/type-drag-repeat.mjs [webkit,chromium] [rounds per browser, default 20]
// Rounds stop early when a browser's time (BUDGET_S, default 130 s) is spent.
import { fileURLToPath } from 'node:url'
import { chromium, webkit } from 'playwright'
import { createServer } from 'vite'

const browsers = (process.argv[2] ?? 'webkit,chromium').split(',')
const rounds = Number(process.argv[3] ?? 20)
const budget = Number(process.env.BUDGET_S ?? 130) * 1000
const server = await createServer({ configFile: fileURLToPath(new URL('../vite.config.ts', import.meta.url)), server: { port: 0, host: '127.0.0.1' }, logLevel: 'error' })
await server.listen()
const origin = `http://127.0.0.1:${server.httpServer.address().port}`

/** Records, in every frame, the input events, the engine worker's messages and the tool buttons' state, with times. */
function recorder() {
  const log = []
  window.__roundLog = log
  window.__mark = (what) => log.push([performance.now(), 'mark', what])
  const brief = (data) => {
    try {
      const text = JSON.stringify(data, (key, value) => (value instanceof ArrayBuffer || ArrayBuffer.isView(value) || (typeof ImageBitmap !== 'undefined' && value instanceof ImageBitmap) ? '[bytes]' : value))
      return text && text.length > 300 ? `${text.slice(0, 300)}…` : text
    } catch {
      return String(data)
    }
  }
  for (const type of ['pointerdown', 'pointermove', 'pointerup', 'pointercancel', 'keydown', 'keyup', 'beforeinput', 'focusin']) {
    window.addEventListener(
      type,
      (event) => {
        const target = event.target instanceof Element ? `${event.target.tagName.toLowerCase()}${event.target.getAttribute('data-testid') ? `[${event.target.getAttribute('data-testid')}]` : ''}${event.target.className && typeof event.target.className === 'string' ? `.${event.target.className.split(' ')[0]}` : ''}` : String(event.target)
        const detail = event.key !== undefined ? `key=${event.key}` : event.clientX !== undefined ? `at=${Math.round(event.clientX)},${Math.round(event.clientY)} buttons=${event.buttons}` : event.inputType ? `input=${event.inputType} data=${event.data}` : ''
        log.push([performance.now(), type, `${detail} on ${target}`])
      },
      true,
    )
  }
  const Base = window.Worker
  if (Base) {
    const post = Base.prototype.postMessage
    Base.prototype.postMessage = function (data, ...rest) {
      log.push([performance.now(), 'to worker', brief(data)])
      return post.call(this, data, ...rest)
    }
    window.Worker = class extends Base {
      constructor(...args) {
        super(...args)
        this.addEventListener('message', (event) => log.push([performance.now(), 'from worker', brief(event.data)]))
      }
    }
  }
  const watch = () =>
    new MutationObserver((changes) => {
      for (const change of changes) {
        const id = change.target.getAttribute?.('data-testid') ?? ''
        if (id.startsWith('vector-tool-')) log.push([performance.now(), 'tool button', `${id} aria-pressed=${change.target.getAttribute('aria-pressed')}`])
      }
    }).observe(document, { subtree: true, attributes: true, attributeFilter: ['aria-pressed'] })
  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', watch)
  else watch()
}

/** One round from a fresh page load; answers whether it passed and, if not, what to print. */
async function round(browser, which) {
  const context = await browser.newContext({ viewport: { width: 1440, height: 1000 } })
  context.setDefaultTimeout(10000)
  await context.addInitScript(recorder)
  const page = await context.newPage()
  const errors = []
  page.on('pageerror', (error) => errors.push(error.message))
  try {
    await page.goto(`${origin}/dev.html?locale=en-US`)
    await page.waitForFunction(() => document.getElementById('status')?.textContent?.includes(' open in '), null, { timeout: 60000 })
    const kit = page.frameLocator('#kit')
    const frame = await (await page.$('#kit')).contentFrame()
    await page.waitForTimeout(1200)
    const until = async (check, tries = 60) => {
      for (let i = 0; i < tries; i += 1) {
        if (await check().catch(() => false)) return true
        await page.waitForTimeout(100)
      }
      return false
    }
    const details = async () => (await page.evaluate(async () => window.devHost.connection().ai.inspect())).details
    const run = async (command, params = {}) => (await page.evaluate(([c, p]) => window.devHost.connection().ai.run([{ command: c, params: p }], false), [command, params])).completed?.[0]?.result
    const batch = (steps) => page.evaluate((s) => window.devHost.connection().ai.run(s, false), steps)
    const nodes = async () => (await details()).layers.flatMap((layer) => layer.children)
    const node = async (id) => (await nodes()).find((item) => item.id === id)
    const typing = async () => (await details()).typing
    const textOf = async (id) => (await node(id))?.text ?? null
    const tool = (id) => kit.getByTestId(`vector-tool-${id}`)
    const pressed = async (id) => (await tool(id).getAttribute('aria-pressed').catch(() => null)) === 'true'
    const primary = (await page.evaluate(() => /Mac|iPhone|iPad/i.test(navigator.platform))) ? 'Meta' : 'Control'
    const mark = (what) => frame.evaluate((w) => window.__mark?.(w), what)

    // A clean artboard, as test:type makes it.
    await batch([{ command: 'edit.selectAll', params: {} }, { command: 'edit.delete', params: {} }])
    await until(async () => (await nodes()).length === 0)
    const board = (await details()).artboards[0]
    const surface = kit.getByTestId('kit-canvas-surface')
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
    const toScreen = (x, y) => ({ x: left + x * zoom, y: top + y * zoom })
    await kit.getByTestId('vector-canvas-region').click({ position: { x: 5, y: 5 } }).catch(() => undefined)

    // The step before, as test:type runs it: new type in 30 pt with nothing selected.
    await page.keyboard.press(`${primary}+Shift+a`)
    await until(async () => (await details()).selection.length === 0)
    await page.keyboard.press('t')
    await until(() => pressed('type'), 30)
    await kit.getByTestId('vector-control-size').fill('30')
    await kit.getByTestId('vector-control-size').press('Enter')
    await until(async () => (await typing()).newType.size === 30)
    const third = toScreen(board.x + 300, board.y + 380)
    await page.mouse.click(third.x, third.y)
    await until(async () => (await typing()).editing !== null)
    await page.keyboard.type('Big')
    const big = (await typing()).editing
    await until(async () => (await textOf(big)) === 'Big')
    await page.keyboard.press('Escape')

    // The step: T, a drag, the text typed.
    const before = { tool: (await details()).tool, typeButton: await pressed('type'), typing: await typing(), selection: (await details()).selection, focused: await frame.evaluate(() => document.activeElement?.getAttribute('data-testid') ?? document.activeElement?.tagName ?? null) }
    await frame.evaluate(() => window.__roundLog.splice(0))
    await mark('press t')
    await page.keyboard.press('t')
    const a0 = toScreen(board.x + 320, board.y + 60)
    const a1 = toScreen(board.x + 520, board.y + 160)
    await mark('drag starts')
    await page.mouse.move(a0.x, a0.y)
    await page.mouse.down()
    for (let i = 1; i <= 8; i += 1) await page.mouse.move(a0.x + ((a1.x - a0.x) * i) / 8, a0.y + ((a1.y - a0.y) * i) / 8)
    await page.mouse.up()
    await mark('drag released')
    await until(async () => (await typing()).editing !== null)
    const editingAfterDrag = await typing()
    await mark('typing starts')
    await page.keyboard.type('Area type wraps in its frame when a line is too long.')
    await mark('typing done')
    const area = (await typing()).editing
    await until(async () => ((await textOf(area)) ?? '').startsWith('Area'))
    const areaKind = await run('text.areaOptions', {})
    const text = await textOf(area)
    const ok = (text ?? '').startsWith('Area') && areaKind !== undefined && areaKind !== null && errors.length === 0
    if (ok) return { ok }
    const log = await frame.evaluate(() => window.__roundLog.map(([t, kind, what]) => `${t.toFixed(1)} ${kind} ${what}`))
    const all = (await nodes()).map((item) => ({ id: item.id, kind: item.kind, text: item.text }))
    return {
      ok,
      report: [
        `before the press: ${JSON.stringify(before)}`,
        `editing after the drag: ${JSON.stringify(editingAfterDrag)}; editing at the check: ${area}; its text: ${JSON.stringify(text)}; areaOptions: ${JSON.stringify(areaKind)}`,
        `objects: ${JSON.stringify(all)}`,
        `typing now: ${JSON.stringify(await typing())}; tool now: ${(await details()).tool}`,
        `page errors: ${JSON.stringify(errors)}`,
        `events, worker messages and tool button changes (ms since the frame loaded):`,
        ...log.slice(-600),
      ].join('\n'),
    }
  } catch (error) {
    return { ok: false, report: `the round did not run to its check: ${error instanceof Error ? error.message.split('\n').slice(0, 3).join(' | ') : String(error)}; page errors: ${JSON.stringify(errors)}` }
  } finally {
    await context.close().catch(() => {})
  }
}

let failed = 0
for (const which of browsers) {
  const browser = await (which === 'webkit' ? webkit : chromium).launch()
  const started = Date.now()
  let ran = 0
  let lost = 0
  for (let n = 1; n <= rounds; n += 1) {
    if (Date.now() - started > budget) break
    const at = Date.now()
    const result = await round(browser, which)
    ran += 1
    if (!result.ok) lost += 1
    console.log(`${result.ok ? 'PASS' : 'FAIL'} ${which} round ${n}/${rounds} (${((Date.now() - at) / 1000).toFixed(1)} s)`)
    if (!result.ok) console.log(result.report)
  }
  console.log(`${which}: the area type drag passed ${ran - lost} of ${ran} rounds in ${((Date.now() - started) / 1000).toFixed(0)} s${ran < rounds ? ` (stopped at the time budget, ${rounds} asked)` : ''}`)
  failed += lost
  await browser.close()
}
await server.close()
process.exit(failed ? 1 : 0)
