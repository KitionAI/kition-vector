// An agent's draft and a restored version in the editor (Kit UI protocol sections 7d and 7m),
// through the development host with the replica and checkpoints kept (`?replica=1&checkpoints=1`)
// and the sample drawing (case vector.collab.drafts/editor): node tests/drafts.mjs [chromium|webkit]
// a. the person adds a layer with the Layers panel's New Layer button; an agent's draft draws an
//    ellipse on the first layer; the draft's comparison lists it and the drawing does not hold it;
//    the person renames the new layer meanwhile (a double-click on its row); the draft is accepted:
//    the Layers panel shows the ellipse's row and the new name, and the inspect answer holds both;
// b. one Undo (Edit › Undo) takes back the ellipse only; Redo (Edit › Redo) brings it back;
// d. the drawing saved, a checkpoint recorded with a label, a layer added after it, the checkpoint
//    restored through the host: the drawing has the checkpoint's layers and the ellipse; one Undo
//    brings the later layer back.
import assert from 'node:assert/strict'
import { fileURLToPath } from 'node:url'
import { chromium, webkit } from 'playwright'
import { createServer } from 'vite'

const which = process.argv[2] ?? 'chromium'
const PATH = 'Sample.kvector'
const server = await createServer({ configFile: fileURLToPath(new URL('../vite.config.ts', import.meta.url)), server: { port: 0, host: '127.0.0.1' }, logLevel: 'error' })
await server.listen()
const browser = await (which === 'webkit' ? webkit : chromium).launch()
const context = await browser.newContext({ viewport: { width: 1440, height: 900 } })
// Every click, wait and host call has a time limit.
context.setDefaultTimeout(10000)
const page = await context.newPage()
const kit = page.frameLocator('#kit')
const errors = []
page.on('pageerror', (error) => errors.push(error.message))
const until = async (check, message) => {
  for (let n = 0; n < 200; n += 1) {
    if (await check().catch(() => false)) return
    await page.waitForTimeout(100)
  }
  assert.fail(typeof message === 'function' ? await message() : message)
}
/** A host call that fails with `what` instead of hanging. */
const host = (what, work, arg) =>
  Promise.race([page.evaluate(work, arg), new Promise((_, reject) => setTimeout(() => reject(new Error(`${what} did not answer in 20 s`)), 20000))])
/** The drawing as the editor answers it (`ai/inspect`). */
const details = async () => (await host('ai/inspect', async () => window.devHost.connection().ai.inspect())).details
const layers = async () => (await details()).layers
/** Every object id of the drawing, through layers and groups. */
const ids = async () => {
  const out = []
  const walk = (nodes) => {
    for (const node of nodes ?? []) {
      out.push(node.id)
      walk(node.children)
    }
  }
  walk(await layers())
  return out
}
const layerNamed = async (name) => (await layers()).find((layer) => layer.name === name)
/** Edit › `id` (Undo or Redo), the editor's own menu row. */
const editMenu = async (id) => {
  await kit.getByTestId('vector-menu-Edit').click()
  const row = kit.locator(`[data-testid="vector-cmd-${id}"], [data-testid="vector-control-${id}"]`).first()
  await row.waitFor({ state: 'visible' })
  await until(() => row.isEnabled(), `Edit › ${id} is enabled`)
  await row.click()
}
/** The person adds a layer with the Layers panel's New Layer button; answers its id. */
const newLayer = async () => {
  const before = (await layers()).map((layer) => layer.id)
  await kit.getByTestId('vector-new-layer').click()
  await until(async () => (await layers()).length === before.length + 1, 'New Layer adds a layer')
  return (await layers()).find((layer) => !before.includes(layer.id)).id
}
const version = () => page.evaluate((path) => window.devHost.files.get(path)?.version ?? 0, PATH)
let failures = 0
let step = 'opening'

try {
  await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/dev.html?modes=menus&locale=en-US&replica=1&checkpoints=1`)
  await page.waitForFunction(() => document.getElementById('status')?.textContent?.includes(' open in '), null, { timeout: 90000 })
  await kit.getByTestId('vector-dock').waitFor({ state: 'visible', timeout: 30000 })
  await until(async () => (await page.evaluate(() => window.devHost.connection() != null)) && (await ids()).length > 1, 'the sample drawing opens')
  await kit.getByTestId('vector-dock-tab-layers').click()
  await kit.getByTestId('vector-layers-panel').waitFor({ state: 'visible' })
  const first = (await layers())[0].id

  step = '(a) the draft accepted'
  // The person adds a layer.
  const added = await newLayer()
  // The agent's draft draws an ellipse on the first layer; the drawing does not hold it.
  const ran = await host(
    'the draft run',
    async (layer) =>
      window.devHost.connection().ai.run(
        [
          { command: 'layer.setCurrent', params: { id: layer } },
          { command: 'shape.ellipse', params: { x: 420, y: 320, width: 60, height: 40 } },
        ],
        false,
        { draft: true },
      ),
    first,
  )
  const drafted = ran.completed?.[1]?.result?.id
  assert.ok(drafted != null, `the draft answers the ellipse's id: ${JSON.stringify(ran)}`)
  const compared = await host('draft/compare', () => window.devHost.connection().draft.compare({ maxSize: 256 }))
  assert.ok(compared.summary.created.length >= 1, `the comparison lists the ellipse: ${JSON.stringify(compared.summary)}`)
  assert.ok(!(await ids()).includes(drafted), 'comparing changes nothing in the drawing')
  // The person renames the new layer meanwhile.
  await kit.getByTestId(`vector-layer-name-${added}`).dblclick()
  await kit.getByTestId(`vector-layer-rename-${added}`).waitFor({ state: 'visible' })
  await page.keyboard.press(`${(await page.evaluate(() => /Mac|iPhone|iPad/i.test(navigator.platform))) ? 'Meta' : 'Control'}+a`)
  await page.keyboard.type('Meanwhile')
  await page.keyboard.press('Enter')
  await until(async () => (await layerNamed('Meanwhile'))?.id === added, 'the person renames the new layer')
  const layerCount = (await layers()).length
  // The person accepts the draft (the host passes the agent's origin).
  const merged = await host('draft/merge', () => window.devHost.connection().draft.merge({ kind: 'agent', principal: 'agent:test' }))
  assert.equal(merged.merged, true, `the draft merged: ${JSON.stringify(merged)}`)
  await until(async () => (await ids()).includes(drafted), 'the drawing holds the merged ellipse')
  await kit.getByTestId(`vector-layer-${drafted}`).waitFor({ state: 'visible' })
  assert.equal((await layers()).find((layer) => layer.id === first).children.some((node) => node.id === drafted), true, 'the ellipse is on the first layer')
  assert.equal((await layerNamed('Meanwhile'))?.id, added, 'the name given meanwhile stays')
  assert.equal((await layers()).length, layerCount, 'the person\'s layer stays')
  assert.match(await kit.getByTestId(`vector-layer-name-${added}`).textContent(), /Meanwhile/, 'the Layers panel shows the new name')
  console.log(`PASS ${which}: a draft accepted over the person's edits keeps both`)

  step = '(b) Undo and Redo'
  await editMenu('edit.undo')
  await until(async () => !(await ids()).includes(drafted), 'Undo takes the ellipse back')
  await kit.getByTestId(`vector-layer-${drafted}`).waitFor({ state: 'detached' })
  assert.equal((await layerNamed('Meanwhile'))?.id, added, 'the person\'s rename stays')
  assert.equal((await layers()).length, layerCount, 'the person\'s layer stays')
  await editMenu('edit.redo')
  await until(async () => (await ids()).includes(drafted), 'Redo brings the ellipse back')
  await kit.getByTestId(`vector-layer-${drafted}`).waitFor({ state: 'visible' })
  console.log(`PASS ${which}: one Undo takes back the draft only, Redo brings it back`)

  step = '(d) the checkpoint restored'
  const saved = await version()
  await kit.getByTestId('vector-save').click()
  await until(async () => (await version()) > saved, 'saved through the host')
  await until(() => page.evaluate((path) => window.devHost.history.replica(path) != null, PATH), 'the replica is stored with the save')
  const checkpoint = await host('recordCheckpoint', () => window.devHost.recordCheckpoint({ origin: 'save', label: 'Before the third layer' }))
  assert.ok(checkpoint?.id, 'a checkpoint is recorded')
  const listed = await host('the checkpoint list', (path) => window.devHost.history.checkpoints(path), PATH)
  assert.ok(listed.some((entry) => entry.id === checkpoint.id && entry.label === 'Before the third layer'), `the checkpoint is listed with its label: ${JSON.stringify(listed)}`)
  const later = await newLayer()
  await host('restoreCheckpoint', (id) => window.devHost.restoreCheckpoint(id), checkpoint.id)
  await until(async () => (await layers()).length === layerCount && !(await ids()).includes(later), 'the restore gives back the checkpoint\'s layers')
  await kit.getByTestId(`vector-layer-${later}`).waitFor({ state: 'detached' })
  assert.ok((await ids()).includes(drafted), 'the checkpoint holds the merged ellipse')
  assert.equal((await layerNamed('Meanwhile'))?.id, added)
  await editMenu('edit.undo')
  await until(async () => (await ids()).includes(later), 'Undo brings the later layer back')
  await kit.getByTestId(`vector-layer-${later}`).waitFor({ state: 'visible' })
  console.log(`PASS ${which}: a restored checkpoint is one step of the person's Undo`)
  assert.deepEqual(errors, [], 'no page errors')
} catch (error) {
  failures += 1
  console.log(`FAIL ${which} at ${step}: ${error instanceof Error ? error.message.split('\n').slice(0, 4).join(' | ') : String(error)}`)
} finally {
  if (errors.length) console.log(`page errors:\n${errors.slice(0, 8).join('\n')}`)
  await browser.close()
  await server.close()
}
process.exit(failures ? 1 : 0)
