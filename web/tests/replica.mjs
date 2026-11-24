// A drawing changed outside Kition keeps its replica (Kit UI protocol UI-R1, `replicaMergesFile`):
// node tests/replica.mjs [chromium|webkit]. Opens dev.html with the replica and checkpoints kept,
// edits and saves, then gives the file new bytes from elsewhere (`devHost.reopenChanged`). The
// engine merges the newer file into the stored replica as one `import` change: the outside change
// shows, the replica is the same one (its id, its version history, its checkpoints), and the
// object ids from before stay. Undo steps live in the session, so they are not asserted here.
import { chromium, webkit } from 'playwright'
import { createServer } from 'vite'

const which = process.argv[2] ?? 'chromium'
const server = await createServer({ configFile: new URL('../vite.config.ts', import.meta.url).pathname, server: { port: 0, host: '127.0.0.1' }, logLevel: 'error' })
await server.listen()
const origin = `http://127.0.0.1:${server.httpServer.address().port}`
const browser = await (which === 'webkit' ? webkit : chromium).launch()
const page = await (await browser.newContext({ viewport: { width: 1440, height: 900 } })).newPage()
const errors = []
let failures = 0
page.on('pageerror', (error) => errors.push(error.message))
const step = (name, ok, detail = '') => {
  if (!ok) failures += 1
  console.log(`${ok ? 'PASS' : 'FAIL'} ${which}: ${name}${detail ? ` - ${detail}` : ''}`)
}
const until = async (check, tries = 100) => {
  for (let i = 0; i < tries; i += 1) {
    if (await check().catch(() => false)) return true
    await page.waitForTimeout(100)
  }
  return false
}
const PATH = 'Sample.kvector'
const details = async () => (await page.evaluate(async () => window.devHost.connection().ai.inspect())).details
/** Every object of the drawing, id → fill, through nested groups. */
const objects = async () => {
  const out = {}
  const walk = (nodes) => {
    for (const node of nodes ?? []) {
      out[node.id] = node.fill
      walk(node.children)
    }
  }
  for (const layer of (await details()).layers) walk(layer.children)
  return out
}
const opened = () => page.waitForFunction(() => document.getElementById('status')?.textContent?.includes(' open in '), null, { timeout: 60000 })

try {
  await page.goto(`${origin}/dev.html?replica=1&checkpoints=1&replica-merges-file=1`)
  await opened()
  await until(async () => (await page.evaluate(() => window.devHost.connection() != null)) && Object.keys(await objects()).length > 0)
  const rect = (await details()).layers.flatMap((layer) => layer.children).find((node) => node.kind.includes('Rect'))
  step('the sample has a rectangle', rect != null)

  // An edit, saved: the host keeps the file and the replica beside it.
  const run = await page.evaluate(async () => window.devHost.connection().ai.run([{ command: 'shape.rectangle', params: { x: 20, y: 20, width: 40, height: 40 } }], false))
  step('edits the drawing', run.completed[0].changed)
  await page.frameLocator('#kit').getByTestId('vector-save').click()
  await until(async () => (await page.evaluate((path) => window.devHost.files.get(path).version > 1 && window.devHost.history.replica(path) != null, PATH)) === true)
  step('saves the file and its replica', await page.evaluate((path) => window.devHost.files.get(path).version > 1 && window.devHost.history.replica(path) != null, PATH))
  const before = await objects()
  const checkpoint = await page.evaluate(() => window.devHost.recordCheckpoint({ origin: 'save' }))
  step('the host records a checkpoint', checkpoint != null && checkpoint.available === true)
  const docId = await page.evaluate(async () => {
    const connection = window.devHost.connection()
    window.versionBefore = await connection.replica.version()
    return connection.replica.info()
  })
  step('the replica has an id', typeof docId === 'string' && docId.length > 0, String(docId))

  // Another program recolours the rectangle in the saved file.
  const changed = await page.evaluate(
    ([path, id]) => {
      const text = new TextDecoder().decode(window.devHost.files.get(path).bytes)
      const file = JSON.parse(text)
      let hit = false
      const walk = (nodes) => {
        for (const node of nodes ?? []) {
          if (node.id === id && node.appearance) {
            for (const item of node.appearance.items) {
              if (item.kind === 'fill') {
                item.paint = { type: 'solid', color: { model: 'rgb', r: 1, g: 0, b: 0 } }
                hit = true
              }
            }
          }
          walk(node.kind?.children ?? node.children)
        }
      }
      for (const layer of file.document.layers) walk(layer.kind?.children)
      return hit ? JSON.stringify(file) : null
    },
    [PATH, rect.id],
  )
  step('the outside program changes the file', changed != null)
  await page.evaluate((text) => window.devHost.reopenChanged(new TextEncoder().encode(text).buffer), changed)
  await opened()
  await until(async () => (await objects())[rect.id] === '#ff0000')
  const after = await objects()
  step('the outside change shows', after[rect.id] === '#ff0000', String(after[rect.id]))
  step('the drawing opens clean', (await page.evaluate(async () => window.devHost.connection().ai.inspect())).dirty === false)
  step(
    'every object keeps its id',
    JSON.stringify(Object.keys(after).sort()) === JSON.stringify(Object.keys(before).sort()),
    `${Object.keys(before)} -> ${Object.keys(after)}`,
  )
  step(
    'the other objects are unchanged',
    Object.keys(before).every((id) => Number(id) === rect.id || before[id] === after[id]),
  )
  const kept = await page.evaluate(async () => {
    const connection = window.devHost.connection()
    const { changes } = await connection.replica.exportChanges(window.versionBefore)
    return { docId: await connection.replica.info(), since: changes.byteLength }
  })
  step('the stored replica opened: the same replica id', kept.docId === docId, `${docId} -> ${kept.docId}`)
  step('its history holds the version from before, and the import after it', kept.since > 0, String(kept.since))
  step('the stored replica is still kept', await page.evaluate((path) => window.devHost.history.replica(path) != null, PATH))
  const checkpoints = await page.evaluate((path) => window.devHost.history.checkpoints(path), PATH)
  step('the checkpoint from before stays available', checkpoints.some((entry) => entry.id === checkpoint?.id && entry.available === true))
} catch (error) {
  failures += 1
  console.log(`FAIL ${which}: ${error instanceof Error ? error.message.split('\n')[0] : String(error)}`)
} finally {
  if (errors.length) console.log(`page errors:\n${errors.slice(0, 8).join('\n')}`)
  await browser.close()
  await server.close()
}
process.exit(failures ? 1 : 0)
