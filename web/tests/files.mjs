// Files in and out by hand in a real browser, through the development host: node tests/files.mjs [chromium|webkit]
// Starts the Vite dev server itself, opens dev.html, and prints PASS or FAIL per step (plan
// 2026-11-25, stage 7, feature vector.editor.files): File › New… makes a drawing and its notice
// opens it; a rectangle, a word and a guide drawn by hand are saved (Cmd+S) and come back equal
// when the drawing is opened again; File › Place… places a picture picked through the host; SVG
// markup on the system clipboard pastes as editable art; a copy goes onto the system clipboard as
// SVG text and PNG and pastes back as the engine's own copy; a picture dropped on the canvas is
// placed where it drops; all of it is still there once saved and opened again; File › Export…
// writes an SVG whose text is outlines (no <text>) and a PNG at 2x twice the size of one at 1x;
// File › Open… opens an SVG, and Save keeps it as a .kvector beside it. SHOTS=<folder> saves
// screenshots.
import { mkdirSync } from 'node:fs'
import { deflateSync } from 'node:zlib'
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

/** A small opaque PNG (`size` square, one colour), written from scratch. */
function png(size, [r, g, b]) {
  const crcTable = Array.from({ length: 256 }, (_, n) => {
    let c = n
    for (let k = 0; k < 8; k += 1) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1
    return c >>> 0
  })
  const crc = (bytes) => {
    let c = 0xffffffff
    for (const byte of bytes) c = crcTable[(c ^ byte) & 0xff] ^ (c >>> 8)
    return (c ^ 0xffffffff) >>> 0
  }
  const chunk = (type, data) => {
    const out = Buffer.alloc(12 + data.length)
    out.writeUInt32BE(data.length, 0)
    out.write(type, 4, 'ascii')
    data.copy(out, 8)
    out.writeUInt32BE(crc(out.subarray(4, 8 + data.length)), 8 + data.length)
    return out
  }
  const header = Buffer.alloc(13)
  header.writeUInt32BE(size, 0)
  header.writeUInt32BE(size, 4)
  header[8] = 8
  header[9] = 2
  const rows = Buffer.alloc(size * (1 + size * 3))
  for (let y = 0; y < size; y += 1) for (let x = 0; x < size; x += 1) rows.set([r, g, b], y * (1 + size * 3) + 1 + x * 3)
  return Buffer.concat([Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]), chunk('IHDR', header), chunk('IDAT', deflateSync(rows)), chunk('IEND', Buffer.alloc(0))])
}
/** A PNG's width and height, from its header. */
const pngSize = (bytes) => [((bytes[16] << 24) | (bytes[17] << 16) | (bytes[18] << 8) | bytes[19]) >>> 0, ((bytes[20] << 24) | (bytes[21] << 16) | (bytes[22] << 8) | bytes[23]) >>> 0]

try {
  const context = await browser.newContext({ viewport: { width: 1440, height: 1000 } })
  if (which === 'chromium') await context.grantPermissions(['clipboard-read', 'clipboard-write'])
  const page = await context.newPage()
  page.on('pageerror', (error) => errors.push(error.message))
  await page.goto(`${origin}/dev.html?locale=${locale}`)
  const openIn = (name) => page.waitForFunction((file) => document.getElementById('status')?.textContent?.includes(`${file} open in `), name, { timeout: 60000 })
  await openIn('Sample.kvector')
  const kit = page.frameLocator('#kit')
  const frame = () => page.frames().find((item) => item !== page.mainFrame())
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
  const steps = async () => (await details()).history.length
  const primary = (await page.evaluate(() => /Mac|iPhone|iPad/i.test(navigator.platform))) ? 'Meta' : 'Control'
  const file = (name) => page.evaluate((path) => {
    const entry = window.devHost.files.get(path)
    return entry ? { version: entry.version, bytes: [...new Uint8Array(entry.bytes)] } : null
  }, name)
  const text = async (name) => Buffer.from((await file(name))?.bytes ?? []).toString('utf8')
  const toast = () => kit.locator('.kui-toast').last().textContent().catch(() => '')

  // ---------- File › New…, and its notice's Open ----------
  await kit.getByTestId('vector-menu-File').click()
  await kit.getByTestId('vector-menu-new').click()
  const dialog = kit.getByTestId('vector-dialog')
  await dialog.waitFor({ state: 'visible', timeout: 10000 })
  await kit.getByTestId('vector-dialog-name').locator('input').fill('Card').catch(() => kit.getByTestId('vector-dialog-name').fill('Card'))
  await kit.getByTestId('vector-dialog-ok').click()
  const created = await until(async () => ((await file('Card.kvector'))?.bytes.length ?? 0) > 0, 100)
  const openButton = kit.locator('.kui-toast button', { hasText: locale === 'zh-CN' ? '打开' : 'Open' }).last()
  const offered = await until(async () => (await openButton.count()) > 0, 50)
  await shot('files-new-notice')
  step('File › New… writes Card.kvector and offers to open it', created && offered, await toast())
  await openButton.click()
  await openIn('Card.kvector')
  await page.waitForTimeout(1200)
  step('Open on the notice opens the new drawing in the editor', (await nodes()).length === 0 && (await details()).artboards.length === 1)

  // ---------- Draw by hand: a rectangle, a word, a guide ----------
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
  let screen = await calibrate()
  const drag = async (from, to, moves = 8) => {
    await page.mouse.move(from.x, from.y)
    await page.mouse.down()
    for (let i = 1; i <= moves; i += 1) await page.mouse.move(from.x + ((to.x - from.x) * i) / moves, from.y + ((to.y - from.y) * i) / moves)
    await page.mouse.up()
  }
  await page.keyboard.press('m')
  await drag(screen.at(board.x + 100, board.y + 100), screen.at(board.x + 300, board.y + 250))
  await until(async () => (await nodes()).length === 1)
  await page.keyboard.press('t')
  await until(async () => (await kit.getByTestId('vector-tool-type').getAttribute('aria-pressed').catch(() => null)) === 'true', 30)
  await page.mouse.click(screen.at(board.x + 100, board.y + 400).x, screen.at(board.x + 100, board.y + 400).y)
  // The click places point type with its placeholder selected; typing replaces it.
  await until(async () => (await details()).typing?.editing != null, 30)
  await page.keyboard.type('Kition', { delay: 20 })
  await until(async () => (await nodes()).some((node) => node.kind === 'Type' && node.text === 'Kition'), 30)
  await page.keyboard.press('Escape')
  await until(async () => (await details()).typing?.editing == null, 30)
  await page.waitForTimeout(200)
  await page.keyboard.press('v')
  await until(async () => (await kit.getByTestId('vector-tool-selection').getAttribute('aria-pressed').catch(() => null)) === 'true', 30)
  await page.keyboard.press(`${primary}+r`)
  const ruler = kit.getByTestId('vector-ruler-x')
  await until(async () => (await ruler.count()) === 1)
  await page.waitForTimeout(300)
  screen = await calibrate()
  const rulerBox = await ruler.boundingBox()
  await drag({ x: screen.at(board.x + 450, 0).x, y: rulerBox.y + rulerBox.height / 2 }, screen.at(board.x + 450, board.y + 520))
  await until(async () => ((await run('guide.list', {})) ?? []).length === 1, 30)
  const drawn = await nodes()
  const guides = (await run('guide.list', {})) ?? []
  await shot('files-drawn')
  step('a rectangle, a word and a guide drawn by hand', drawn.length === 2 && guides.length === 1, JSON.stringify({ kinds: drawn.map((node) => node.kind), guides: guides.length }))

  // ---------- Save, open again: the same drawing, the guide included ----------
  const before = (await file('Card.kvector')).version
  await kit.getByTestId('vector-canvas-region').click({ position: { x: 5, y: 5 } }).catch(() => undefined)
  await page.keyboard.press(`${primary}+s`)
  const saved = await until(async () => (await file('Card.kvector')).version > before, 100)
  step('Cmd+S saves the drawing through the host', saved)
  const shape = (list) => JSON.stringify(list.map((node) => ({ kind: node.kind, name: node.name, bounds: node.bounds && Object.fromEntries(Object.entries(node.bounds).map(([key, value]) => [key, Math.round(value * 100) / 100])) })))
  await page.evaluate(() => window.devHost.reload())
  await openIn('Card.kvector')
  await page.waitForTimeout(1200)
  const reopened = await nodes()
  const guidesAgain = (await run('guide.list', {})) ?? []
  step('opened again, the drawing is the same, its guide included', shape(reopened) === shape(drawn) && guidesAgain.length === 1 && Math.abs(guidesAgain[0].pos - guides[0].pos) < 0.01, JSON.stringify({ before: shape(drawn), after: shape(reopened), guides: guidesAgain }))

  // ---------- File › Place…: a picture picked through the host ----------
  await page.evaluate((bytes) => {
    window.devHost.files.set('Dot.png', { bytes: new Uint8Array(bytes).buffer, version: 1 })
    window.devHost.scriptHostPick('Dot.png')
  }, [...png(40, [0, 128, 255])])
  let count = (await nodes()).length
  let history = await steps()
  await kit.getByTestId('vector-menu-File').click()
  await kit.getByTestId('vector-menu-place').click()
  const placed = await until(async () => (await nodes()).length === count + 1, 80)
  const image = (await nodes()).find((node) => !reopened.some((old) => old.id === node.id))
  await shot('files-placed')
  step('File › Place… places a picture picked through the host, as one step', placed && /image/i.test(image?.kind ?? '') && (await steps()) === history + 1, JSON.stringify({ kind: image?.kind, steps: (await steps()) - history }))

  // ---------- Paste SVG markup from the system clipboard ----------
  const svgMarkup = '<svg xmlns="http://www.w3.org/2000/svg" width="80" height="60"><rect x="0" y="0" width="80" height="60" fill="#ff0000"/><circle cx="40" cy="30" r="20" fill="#00aa00"/></svg>'
  count = (await nodes()).length
  history = await steps()
  await kit.getByTestId('vector-canvas-region').click({ position: { x: 5, y: 5 } }).catch(() => undefined)
  if (which === 'chromium') {
    // Another application's copy: the system clipboard holds the markup as text.
    await frame().evaluate((markup) => navigator.clipboard.writeText(markup), svgMarkup)
    await page.keyboard.press(`${primary}+v`)
  } else {
    // WebKit's clipboard is not written from a script: the browser's paste event, as Cmd+V brings it.
    await frame().evaluate((markup) => {
      const data = new DataTransfer()
      data.setData('text/plain', markup)
      document.dispatchEvent(new ClipboardEvent('paste', { clipboardData: data, bubbles: true, cancelable: true }))
    }, svgMarkup)
  }
  const pastedSvg = await until(async () => (await nodes()).length === count + 2, 80)
  const added = (await nodes()).filter((node) => !reopened.some((old) => old.id === node.id) && node.id !== image?.id).map((node) => node.kind)
  await shot('files-pasted-svg')
  step('SVG markup on the system clipboard pastes as editable art (a rectangle and a circle), one step', pastedSvg && added.includes('Rectangle') && added.includes('Ellipse') && (await steps()) === history + 1, JSON.stringify({ added, steps: (await steps()) - history }))

  // ---------- Copy out: SVG text and PNG on the system clipboard; pasted back as the engine's copy ----------
  if (which === 'chromium') {
    // The rectangle, selected with a click on its edge (as a person selects: the editor's own selection).
    screen = await calibrate()
    const edge = screen.at(board.x + 100, board.y + 175)
    await page.keyboard.press('v')
    await page.mouse.click(edge.x, edge.y)
    await until(async () => (await kit.getByTestId('vector-selection-size').count()) === 1, 30)
    await page.keyboard.press(`${primary}+c`)
    const types = await until(async () => {
      const items = await frame().evaluate(async () => (await navigator.clipboard.read()).map((item) => item.types))
      return items.flat().includes('image/png') && items.flat().includes('text/plain')
    }, 60)
    const copiedText = await frame().evaluate(() => navigator.clipboard.readText())
    const flavours = await frame().evaluate(async () => (await navigator.clipboard.read()).flatMap((item) => item.types))
    step('Copy puts SVG text and a PNG on the system clipboard', types && copiedText.includes('<svg'), JSON.stringify(flavours))
    const known = (await nodes()).map((node) => node.id)
    await page.keyboard.press(`${primary}+v`)
    const back = await until(async () => (await nodes()).length === known.length + 1, 60)
    const pasted = (await nodes()).filter((node) => !known.includes(node.id))
    step("pasting the editor's own copy is the engine's paste (one rectangle, as copied)", back && pasted.length === 1 && pasted[0].kind === 'Rectangle', JSON.stringify(pasted.map((node) => node.kind)))
  }

  // ---------- A picture dropped on the canvas from the computer ----------
  count = (await nodes()).length
  screen = await calibrate()
  const dropAt = screen.at(board.x + 450, board.y + 300)
  const frameBox = await page.locator('#kit').boundingBox()
  await frame().evaluate(
    ([bytes, point]) => {
      const data = new DataTransfer()
      data.items.add(new File([new Uint8Array(bytes)], 'Drop.png', { type: 'image/png' }))
      const target = document.elementFromPoint(point.x, point.y)
      for (const type of ['dragenter', 'dragover', 'drop']) target.dispatchEvent(new DragEvent(type, { dataTransfer: data, clientX: point.x, clientY: point.y, bubbles: true, cancelable: true }))
    },
    [[...png(30, [255, 160, 0])], { x: dropAt.x - frameBox.x, y: dropAt.y - frameBox.y }],
  )
  const dropped = await until(async () => (await nodes()).length === count + 1, 80)
  const dropImage = (await details()).selectionBounds
  const centre = dropImage ? [dropImage.x + dropImage.width / 2, dropImage.y + dropImage.height / 2] : null
  await shot('files-dropped')
  step('a picture dropped on the canvas is placed where it drops', dropped && centre !== null && Math.abs(centre[0] - (board.x + 450)) < 3 && Math.abs(centre[1] - (board.y + 300)) < 3, JSON.stringify({ dropped, centre }))

  // ---------- Placed, pasted and dropped art survives a save and opening again ----------
  const withArt = await nodes()
  const version = (await file('Card.kvector')).version
  await kit.getByTestId('vector-canvas-region').click({ position: { x: 5, y: 5 } }).catch(() => undefined)
  await page.keyboard.press(`${primary}+s`)
  await until(async () => (await file('Card.kvector')).version > version, 100)
  await page.evaluate(() => window.devHost.reload())
  await openIn('Card.kvector')
  await page.waitForTimeout(1200)
  const kept = await nodes()
  step('placed, pasted and dropped art is still there after saving and opening again', shape(kept) === shape(withArt) && kept.filter((node) => node.kind === 'Image').length === 2, JSON.stringify(kept.map((node) => node.kind)))

  // ---------- Export: SVG with outlined text; PNG at 2x is twice the size of 1x ----------
  const exportWith = async (setup) => {
    await kit.getByTestId('vector-menu-File').click()
    await kit.getByTestId('vector-menu-export').click()
    await kit.getByTestId('vector-export-dialog').waitFor({ state: 'visible', timeout: 10000 })
    await setup()
    await until(async () => /\d/.test((await kit.getByTestId('vector-export-size').textContent()) ?? ''), 60)
    await kit.getByTestId('vector-export-run').click()
    await until(async () => (await kit.getByTestId('vector-export-dialog').count()) === 0, 100)
  }
  await exportWith(async () => {
    await kit.getByTestId('vector-export-format').selectOption('svg')
    const box = kit.getByTestId('vector-export-outline-text')
    if (!(await box.isChecked())) await box.check()
    await shot('files-export-svg')
  })
  await until(async () => (await text('Card.svg')).includes('</svg>'), 100)
  const outlined = await text('Card.svg')
  step('Export SVG with text as outlines has no <text>', outlined.includes('<svg') && !outlined.includes('<text'), `${outlined.length} bytes`)
  await exportWith(async () => {
    await kit.getByTestId('vector-export-format').selectOption('png')
    await kit.getByTestId('vector-export-scope').selectOption('artboard:0')
    await kit.getByTestId('vector-export-scale').selectOption('1')
  })
  await until(async () => ((await file('Card.png'))?.bytes.length ?? 0) > 0, 100)
  await exportWith(async () => {
    await kit.getByTestId('vector-export-format').selectOption('png')
    await kit.getByTestId('vector-export-scope').selectOption('artboard:0')
    await kit.getByTestId('vector-export-scale').selectOption('2')
    await shot('files-export-png')
  })
  await until(async () => ((await file('Card 2.png'))?.bytes.length ?? 0) > 0, 100)
  const one = pngSize((await file('Card.png')).bytes)
  const two = pngSize((await file('Card 2.png')).bytes)
  step('Export PNG at 2x is twice the size of 1x', two[0] === one[0] * 2 && two[1] === one[1] * 2 && one[0] === Math.round(board.width), JSON.stringify({ one, two, board: [board.width, board.height] }))

  // ---------- File › Open… an SVG; Save keeps it as a .kvector beside it, the SVG untouched ----------
  await page.evaluate((markup) => {
    window.devHost.files.set('Badge.svg', { bytes: new TextEncoder().encode(markup).buffer, version: 1 })
    window.devHost.scriptHostPick('Badge.svg')
  }, svgMarkup)
  await kit.getByTestId('vector-menu-File').click()
  await kit.getByTestId('vector-menu-open').click()
  await openIn('Badge.svg')
  await page.waitForTimeout(1200)
  const badge = await nodes()
  step('File › Open… opens an SVG picked through the host, as editable art', badge.flatMap((node) => [node.kind, ...(node.children ?? []).map((child) => child.kind)]).filter((kind) => kind === 'Rectangle' || kind === 'Ellipse').length === 2, JSON.stringify(badge.map((node) => node.kind)))
  await kit.getByTestId('vector-menu-File').click()
  const saveLabel = (await kit.getByTestId('vector-menu-save').textContent()) ?? ''
  await kit.getByTestId('vector-menu-save').click()
  const native = await until(async () => ((await file('Badge.kvector'))?.bytes.length ?? 0) > 0, 100)
  await shot('files-saved-native')
  step('Save on a drawing read from SVG writes Badge.kvector beside it and leaves the SVG as it was', native && saveLabel.includes('.kvector') && (await text('Badge.svg')) === svgMarkup && (await text('Badge.kvector')).includes('"kvector"'), JSON.stringify({ saveLabel, toast: await toast() }))
} catch (error) {
  failures += 1
  console.log(`FAIL ${which} ${error instanceof Error ? error.message.split('\n').slice(0, 4).join(' | ') : String(error)}`)
} finally {
  if (errors.length) console.log(`page errors:\n${errors.slice(0, 8).join('\n')}`)
  await browser.close()
  await server.close()
}
process.exit(failures ? 1 : 0)
