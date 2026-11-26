// Type by hand in a real browser, through the development host: node tests/type.mjs [chromium|webkit]
// Starts the Vite dev server itself, opens dev.html, and prints PASS or FAIL per step (plan
// 2026-11-25, stage 5, feature vector.editor.type): point type by a click (its placeholder selected),
// typing, the caret keys and Shift selection, a double-click selecting a word and a quick triple-click
// its paragraph, Cmd+A selecting the text only, copy and paste of characters, area type by a drag and
// type on a path by a click; a Selection-tool double-click putting the caret where it was clicked;
// size and colour of a few characters only, one step each; the Character panel, the font menu (a
// served family loaded on choosing, recently used first, a font file loaded through the host) and the
// Paragraph panel; what new type is set in; an input method's composition shown at the caret and
// committed once (Chinese, drawn with the lazily loaded Chinese face); the typing session one undo
// step. SHOTS=<folder> saves screenshots.
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
  await page.goto(`${origin}/dev.html?locale=${locale}`)
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
  const shot = async (name) => {
    if (shots) await page.screenshot({ path: `${shots}/${name}.png` })
  }
  const details = async () => (await page.evaluate(async () => window.devHost.connection().ai.inspect())).details
  const run = async (command, params = {}) => (await page.evaluate(([c, p]) => window.devHost.connection().ai.run([{ command: c, params: p }], false), [command, params])).completed?.[0]?.result
  const batch = (steps) => page.evaluate((s) => window.devHost.connection().ai.run(s, false), steps)
  const nodes = async () => (await details()).layers.flatMap((layer) => layer.children)
  const node = async (id) => (await nodes()).find((item) => item.id === id)
  const typing = async () => (await details()).typing
  const steps = async () => (await details()).history.length
  const textOf = async (id) => (await node(id))?.text ?? null
  const runs = async (id) => (await run('text.getRange', { id }))?.runs ?? []
  const tool = (id) => kit.getByTestId(`vector-tool-${id}`)
  const pressed = async (id) => (await tool(id).getAttribute('aria-pressed').catch(() => null)) === 'true'
  const primary = (await page.evaluate(() => /Mac|iPhone|iPad/i.test(navigator.platform))) ? 'Meta' : 'Control'
  const sink = kit.locator('textarea.kui-text-sink')

  // A clean artboard: everything deleted (one agent batch, which keeps its own selection).
  await batch([{ command: 'edit.selectAll', params: {} }, { command: 'edit.delete', params: {} }])
  await until(async () => (await nodes()).length === 0)
  const board = (await details()).artboards[0]
  const surface = kit.getByTestId('kit-canvas-surface')
  // Where a drawing's point is on the screen: from the white artboard's pixels (the art is gone).
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
    return (x, y) => ({ x: left + x * zoom, y: top + y * zoom })
  }
  await page.waitForTimeout(500)
  const screen = await calibrate()
  const toScreen = async (x, y) => screen(x, y)
  await kit.getByTestId('vector-canvas-region').click({ position: { x: 5, y: 5 } }).catch(() => undefined)

  // ---------- point type: a click places it with its placeholder selected; typing replaces it ----------
  await page.keyboard.press('t')
  await until(() => pressed('type'), 30)
  const at = await toScreen(board.x + 60, board.y + 120)
  await page.mouse.click(at.x, at.y)
  await until(async () => (await typing()).editing !== null)
  const fresh = await typing()
  const id = fresh.editing
  const placeholder = await textOf(id)
  step('a click with the Type tool places point type, its placeholder selected', id !== null && placeholder.length > 0 && fresh.start === 0 && fresh.end === new TextEncoder().encode(placeholder).length, JSON.stringify({ fresh, placeholder }))
  await page.keyboard.type('Hello world', { delay: 20 })
  await until(async () => (await textOf(id)) === 'Hello world')
  step('typing replaces the placeholder', (await textOf(id)) === 'Hello world', await textOf(id))
  await shot('typed')

  // ---------- the caret keys, Shift selection, Home and End, word jumps ----------
  await page.keyboard.press('Home')
  await until(async () => (await typing()).start === 0 && (await typing()).end === 0)
  await page.keyboard.press('Shift+ArrowRight')
  await page.keyboard.press('Shift+ArrowRight')
  await until(async () => (await typing()).end === 2)
  step('Home, then Shift+Right twice selects two characters', (await typing()).start === 0 && (await typing()).end === 2, JSON.stringify(await typing()))
  await page.keyboard.press('End')
  await page.keyboard.press(`${primary}+ArrowLeft`)
  await until(async () => (await typing()).start === 6)
  step('End then Cmd+Left jumps a word back', (await typing()).start === 6 && (await typing()).end === 6, JSON.stringify(await typing()))

  // ---------- the caret blinks ----------
  step('the caret has its line (where the input method\'s window goes)', (await typing()).caret !== null, JSON.stringify((await typing()).caret))

  // ---------- double-click a word, triple-click its paragraph ----------
  const rect = (await node(id)).bounds
  const word = await toScreen(rect.x + rect.width * 0.8, rect.y + rect.height * 0.5)
  await page.mouse.dblclick(word.x, word.y)
  await until(async () => (await typing()).start === 6 && (await typing()).end === 11)
  step('a double-click selects the word', (await typing()).start === 6 && (await typing()).end === 11, JSON.stringify(await typing()))
  await page.mouse.click(word.x, word.y, { clickCount: 3 })
  await until(async () => (await typing()).start === 0 && (await typing()).end === 11)
  step('a quick triple-click selects the paragraph', (await typing()).start === 0 && (await typing()).end === 11, JSON.stringify(await typing()))
  await page.waitForTimeout(700)
  await page.mouse.click(word.x, word.y)
  await until(async () => (await typing()).start === (await typing()).end)
  step('a click after a pause puts the caret there (no paragraph)', (await typing()).start === (await typing()).end, JSON.stringify(await typing()))

  // ---------- Cmd+A selects the text only ----------
  await page.keyboard.press(`${primary}+a`)
  await until(async () => (await typing()).start === 0 && (await typing()).end === 11)
  step('Cmd+A while editing selects all the text, the selection stays the one object', (await typing()).end === 11 && JSON.stringify((await details()).selection) === JSON.stringify([id]), JSON.stringify({ typing: await typing(), selection: (await details()).selection }))

  // ---------- copy and paste of characters ----------
  await page.keyboard.press('Home')
  await page.keyboard.press('Shift+ArrowRight')
  await page.keyboard.press('Shift+ArrowRight')
  await page.keyboard.press('Shift+ArrowRight')
  await page.keyboard.press('Shift+ArrowRight')
  await page.keyboard.press('Shift+ArrowRight')
  await until(async () => (await typing()).end === 5)
  await page.keyboard.press(`${primary}+c`)
  await page.waitForTimeout(200)
  await page.keyboard.press('End')
  await page.keyboard.press(`${primary}+v`)
  await until(async () => (await textOf(id)) === 'Hello worldHello')
  step('Cmd+C and Cmd+V copy and paste characters, not the object', (await textOf(id)) === 'Hello worldHello' && (await nodes()).length === 1, JSON.stringify({ text: await textOf(id), count: (await nodes()).length }))
  for (let i = 0; i < 5; i += 1) await page.keyboard.press('Backspace')
  await until(async () => (await textOf(id)) === 'Hello world')

  // ---------- range formatting: size and colour of a few characters only ----------
  await page.keyboard.press('End')
  for (let i = 0; i < 5; i += 1) await page.keyboard.press('Shift+ArrowLeft')
  await until(async () => (await typing()).start === 6 && (await typing()).end === 11)
  let before = await steps()
  await kit.getByTestId('vector-control-size').fill('36')
  await kit.getByTestId('vector-control-size').press('Enter')
  await until(async () => (await runs(id)).length === 2)
  const sized = await runs(id)
  step('a size set with "world" selected sizes "world" only', sized.length === 2 && sized[0].text === 'Hello ' && sized[1].text === 'world' && sized[1].style.size === 36 && sized[0].style.size !== 36, JSON.stringify(sized.map((r) => [r.text, r.style.size])))
  await kit.getByTestId('vector-fill').fill('#e03131')
  const red = (run) => Math.abs((run?.style.fill?.color?.r ?? 0) - 0xe0 / 255) < 0.01 && Math.abs((run?.style.fill?.color?.g ?? 1) - 0x31 / 255) < 0.01
  await until(async () => (await runs(id)).some(red))
  await kit.getByTestId('vector-fill').press('Enter')
  const coloured = await runs(id)
  step('a fill set with "world" selected colours "world" only (not the whole text)', coloured.length === 2 && red(coloured[1]) && !red(coloured[0]), JSON.stringify(coloured.map((r) => [r.text, r.style.fill])))
  step('size and colour are one history step each', (await steps()) >= before + 2, JSON.stringify({ before, now: await steps() }))
  step('the Character panel reads the selection: size 36 shown in the control bar', (await kit.getByTestId('vector-control-size').inputValue()) === '36')
  await shot('range')

  // ---------- Escape commits; V picks the Selection tool ----------
  await page.keyboard.press('Escape')
  await until(async () => (await typing()).editing === null)
  await page.keyboard.press('v')
  await until(() => pressed('selection'), 30)
  step('Escape ends editing, the text stays selected, V picks the Selection tool', (await typing()).editing === null && (await pressed('selection')) && JSON.stringify((await details()).selection) === JSON.stringify([id]))

  // ---------- a Selection-tool double-click puts the caret where it was clicked ----------
  const between = await toScreen(rect.x + rect.width * 0.08, rect.y + rect.height * 0.5)
  await page.mouse.dblclick(between.x, between.y)
  await until(async () => (await typing()).editing === id)
  const placed = await typing()
  step('a Selection-tool double-click on type edits it with the caret there, nothing selected', placed.editing === id && placed.start === placed.end && placed.start > 0 && placed.start < 6, JSON.stringify(placed))
  await page.keyboard.type('!')
  await until(async () => ((await textOf(id)) ?? '').includes('!'))
  const typedIn = await textOf(id)
  step('typing then inserts at the caret instead of replacing a word', typedIn.length === 12 && typedIn.replace('!', '') === 'Hello world', typedIn)
  await page.keyboard.press('Backspace')
  await until(async () => (await textOf(id)) === 'Hello world')

  // ---------- the Character panel, the font menu ----------
  await page.keyboard.press(`${primary}+a`)
  await until(async () => (await typing()).end === 11)
  await kit.getByTestId('vector-type-more-character').click()
  const panel = await until(async () => (await kit.getByTestId('vector-character-panel').count()) === 1)
  step('Properties › Character panel opens the Character panel', panel)
  await kit.getByTestId('vector-char-font').click()
  await until(async () => (await kit.getByTestId('vector-char-font-menu').count()) === 1)
  await kit.getByTestId('vector-char-font-search').fill('Lora')
  await shot('font-menu')
  await kit.getByTestId('vector-char-font-font-Lora').click()
  const lora = await until(async () => (await typing()).character?.font === 'Lora', 100)
  step('the font menu searches, and a served family loads when chosen and is set', lora, JSON.stringify((await typing()).character))
  await kit.getByTestId('vector-char-font').click()
  const recent = await until(async () => (await kit.getByTestId('vector-char-font-recent-Lora').count()) === 1)
  step('the family is then among the recently used', recent)
  await page.keyboard.press('Escape')
  await kit.getByTestId('vector-char-tracking').fill('200')
  await kit.getByTestId('vector-char-tracking').press('Enter')
  await until(async () => (await typing()).character?.tracking === 200)
  await kit.getByTestId('vector-char-underline').click()
  await until(async () => (await typing()).character?.underline === true)
  step('tracking and underline from the Character panel', (await typing()).character?.tracking === 200 && (await typing()).character?.underline === true, JSON.stringify((await typing()).character))

  // ---------- the Paragraph panel ----------
  await kit.getByTestId('vector-dock').getByText(locale === 'zh-CN' ? '段落' : 'Paragraph', { exact: true }).click()
  await until(async () => (await kit.getByTestId('vector-paragraph-panel').count()) === 1)
  await kit.getByTestId('vector-para-align-center').click()
  await until(async () => (await typing()).paragraph?.justify === 'center')
  await kit.getByTestId('vector-para-space-before').fill('12')
  await kit.getByTestId('vector-para-space-before').press('Enter')
  await until(async () => (await typing()).paragraph?.spaceBefore === 12)
  step('the Paragraph panel aligns centre and sets space before', (await typing()).paragraph?.justify === 'center' && (await typing()).paragraph?.spaceBefore === 12, JSON.stringify((await typing()).paragraph))
  await shot('paragraph')

  // ---------- Load Font File…: a font from the workspace, through the host ----------
  await page.evaluate(async (url) => {
    const bytes = await (await fetch(url)).arrayBuffer()
    window.devHost.files.set('Cousine-Bold.ttf', { bytes, version: 1 })
    window.devHost.scriptHostPick('Cousine-Bold.ttf')
  }, `${origin}/fonts/cousine/Cousine-Bold.ttf`)
  await kit.getByTestId('vector-control-font').click()
  await until(async () => (await kit.getByTestId('vector-control-font-load').count()) === 1)
  await kit.getByTestId('vector-control-font-load').click()
  const loadedFile = await until(async () => (await typing()).character?.font === 'Cousine', 100)
  step('Load Font File… gives the engine a font from the workspace and sets it', loadedFile && ((await run('text.fontList', {}))?.families ?? []).includes('Cousine'), JSON.stringify({ font: (await typing()).character?.font }))
  await page.keyboard.press('Escape')
  await until(async () => (await typing()).editing === null)

  // ---------- an input method: composition shown at the caret, committed once ----------
  await page.keyboard.press('Escape')
  await page.keyboard.press('t')
  await until(() => pressed('type'), 30)
  const ime = await toScreen(board.x + 60, board.y + 260)
  await page.mouse.click(ime.x, ime.y)
  await until(async () => (await typing()).editing !== null && (await typing()).editing !== id)
  const chinese = (await typing()).editing
  before = await steps()
  await page.keyboard.type('A')
  await until(async () => (await textOf(chinese)) === 'A')
  const compose = (type, data, value) =>
    sink.evaluate(
      (field, [kind, text, now]) => {
        if (now !== null) field.value = now
        field.dispatchEvent(new CompositionEvent(kind, { data: text, bubbles: true, cancelable: true }))
        if (kind === 'compositionupdate') field.dispatchEvent(new InputEvent('input', { data: text, inputType: 'insertCompositionText', isComposing: true, bubbles: true }))
      },
      [type, data, value],
    )
  await compose('compositionstart', '', '')
  await compose('compositionupdate', 'zhong', 'zhong')
  const composing = await until(async () => (await textOf(chinese)) === 'Azhong' && (await typing()).composing)
  const caretBox = await sink.boundingBox()
  const textBox = (await node(chinese)).bounds
  const near = await toScreen(textBox.x, textBox.y)
  step('an input method\'s composition shows in the text at the caret, marked', composing, JSON.stringify({ text: await textOf(chinese), typing: await typing() }))
  step('the hidden text field (the candidate window\'s anchor) sits at the caret', caretBox && Math.abs(caretBox.y - near.y) < 60 && caretBox.x > near.x - 5 && caretBox.x < near.x + 200, JSON.stringify({ caretBox, near }))
  await shot('ime-composing')
  await compose('compositionupdate', '中文', '中文')
  await compose('compositionend', '中文', '中文')
  await until(async () => (await textOf(chinese)) === 'A中文' && !(await typing()).composing)
  step('the commit lands once and the marked text goes', (await textOf(chinese)) === 'A中文' && !(await typing()).composing, JSON.stringify({ text: await textOf(chinese) }))
  // Composing again and giving up leaves nothing.
  await compose('compositionstart', '', '')
  await compose('compositionupdate', 'wen', 'wen')
  await until(async () => (await textOf(chinese)) === 'A中文wen')
  await compose('compositionend', '', '')
  await until(async () => (await textOf(chinese)) === 'A中文')
  step('a composition given up leaves no marked text', (await textOf(chinese)) === 'A中文', await textOf(chinese))
  // The Chinese face comes lazily once the text needs it: then no character is missing.
  const drawn = await until(async () => ((await run('text.fonts', {})) ?? []).every((row) => (row.noGlyph ?? 0) === 0), 150)
  step('Chinese draws with the lazily loaded Chinese face (no missing glyphs)', drawn, JSON.stringify(await run('text.fonts', {})))
  await shot('chinese')
  await page.keyboard.press('Escape')
  await until(async () => (await typing()).editing === null)
  step('the typing session is one undo step', (await steps()) === before + 1, JSON.stringify({ before, now: await steps(), history: (await details()).history.slice(-3) }))
  await page.keyboard.press(`${primary}+z`)
  await until(async () => (await textOf(chinese)) !== 'A中文')
  step('one Undo takes the whole typing back', (await textOf(chinese)) !== 'A中文', String(await textOf(chinese)))

  // ---------- new type takes the family and size set with nothing selected ----------
  await page.keyboard.press(`${primary}+Shift+a`)
  await until(async () => (await details()).selection.length === 0)
  await page.keyboard.press('t')
  await until(() => pressed('type'), 30)
  await kit.getByTestId('vector-control-size').fill('30')
  await kit.getByTestId('vector-control-size').press('Enter')
  await until(async () => (await typing()).newType.size === 30)
  const third = await toScreen(board.x + 300, board.y + 380)
  await page.mouse.click(third.x, third.y)
  await until(async () => (await typing()).editing !== null)
  await page.keyboard.type('Big')
  const big = (await typing()).editing
  await until(async () => (await textOf(big)) === 'Big')
  step('with nothing selected the size sets what new type is set in (30 pt)', (await runs(big))[0]?.style.size === 30, JSON.stringify((await runs(big))[0]?.style))
  await page.keyboard.press('Escape')

  // ---------- area type by a drag, type on a path by a click ----------
  await page.keyboard.press('t')
  const a0 = await toScreen(board.x + 320, board.y + 60)
  const a1 = await toScreen(board.x + 520, board.y + 160)
  await page.mouse.move(a0.x, a0.y)
  await page.mouse.down()
  for (let i = 1; i <= 8; i += 1) await page.mouse.move(a0.x + ((a1.x - a0.x) * i) / 8, a0.y + ((a1.y - a0.y) * i) / 8)
  await page.mouse.up()
  await until(async () => (await typing()).editing !== null)
  await page.keyboard.type('Area type wraps in its frame when a line is too long.')
  const area = (await typing()).editing
  await until(async () => ((await textOf(area)) ?? '').startsWith('Area'))
  const areaKind = (await run('text.areaOptions', {}))
  step('a drag with the Type tool makes area type', ((await textOf(area)) ?? '').startsWith('Area') && areaKind !== undefined && areaKind !== null, JSON.stringify(areaKind))
  await page.keyboard.press('Escape')
  const ellipse = (await run('shape.ellipse', { x: board.x + 120, y: board.y + 420, width: 160, height: 100 }))?.id
  await page.keyboard.press('Escape')
  await kit.getByTestId('vector-tool-type').click({ button: 'right' })
  await kit.getByTestId('vector-tool-typeOnPath').click()
  await until(() => pressed('typeOnPath'), 30)
  const onPath = await toScreen(board.x + 120, board.y + 470)
  await page.mouse.click(onPath.x, onPath.y)
  await until(async () => (await typing()).editing !== null)
  await page.keyboard.type('Round we go')
  const pathText = (await typing()).editing
  await until(async () => (await textOf(pathText)) === 'Round we go')
  const pathOptions = await run('text.pathOptions', {})
  step('a click on a path with Type on a Path sets type along it (the path becomes its baseline)', (await textOf(pathText)) === 'Round we go' && pathOptions?.effect !== undefined && (await node(ellipse)) === undefined, JSON.stringify({ pathOptions, ellipse: await node(ellipse) }))
  await page.keyboard.press('Escape')
  await shot('kinds')
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
