// Takes the README's pictures of the Vector editor (assets/readme/*.png at the repository root):
// the editor in the development host, on showcase samples ("Alpine Lake Sunrise"; for the SVG
// picture, "Weather Icon Set"), at 1440 x 900, device scale 2, Classic display mode, light theme,
// English. Not a test; re-run it by hand when the editor changes:
//
//   cd web && pnpm install && pnpm run engine      # the engine in public/
//   node scripts/readme-shots.mjs [name ...]       # every shot, or only the named ones
//
// It needs kition-sdk (the shared runner, tools/readme-shots/) and kition-showcase (the samples)
// checked out beside this repository, or KITION_SDK and KITION_SHOWCASE naming them. Run it with
// no HTTP proxy set; CHROME may name a Chrome or Chromium executable. Every step is what a person
// does in the editor: clicks on the drawing, the ribbon, the zoom buttons and the Export dialog.
// When pngquant is installed, each picture is then shrunk with `pngquant --quality 70-90` (a
// README picture is at most 400 KB).
import { execFileSync } from 'node:child_process'
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'

const web = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const sdk = path.resolve(process.env.KITION_SDK ?? path.join(web, '..', '..', 'kition-sdk'))
const showcase = path.resolve(process.env.KITION_SHOWCASE ?? path.join(web, '..', '..', 'kition-showcase'))
const { openFile, takeShots } = await import(pathToFileURL(path.join(sdk, 'tools/readme-shots/shots.mjs')).href)

const sample = (name) => path.join(showcase, 'vector', name, `${name}.kvector`)
const LAKE = sample('alpine-lake-sunrise')
const ICONS = sample('weather-icon-set')

/** A click at a point of the kit's frame (CSS pixels from its top left corner), measured on the pictures. */
async function click(page, x, y) {
  const box = await page.locator('#kit').boundingBox()
  await page.mouse.click(box.x + x, box.y + y)
  await page.waitForTimeout(600)
}

/** Opens File › Export… from the ribbon's File tab. */
async function openExport(page, kit) {
  await kit.getByTestId('ribbon-tab-File').click()
  await page.waitForTimeout(300)
  await kit.getByTestId('vector-menu-export').click()
  await kit.getByTestId('vector-export-dialog').waitFor()
}

const report = await takeShots({
  web,
  sample: async (page, shot) => {
    const file = shot.name === 'see-4' ? ICONS : LAKE
    await openFile(page, { name: path.basename(file), bytes: fs.readFileSync(file) })
  },
  query: { modes: 'classic' },
  ready: '[data-testid="vector-status"]',
  settle: { open: 4000, shot: 1000 },
  shots: [
    // The illustration as it opens: the tools, the ribbon, and its layers in the side panel.
    { name: 'hero' },
    {
      // The sun selected with a click: its handles on the drawing, its size and paint in the panels.
      name: 'see-1',
      steps: async (page) => click(page, 619, 426),
    },
    {
      // New Artboard (artboard.new) on the ribbon, then zoomed out to show both artboards.
      name: 'see-2',
      steps: async (page, { kit }) => {
        await kit.getByTestId('vector-cmd-artboard.new').first().click()
        await page.waitForTimeout(1000)
        for (let step = 0; step < 3; step += 1) {
          await kit.getByRole('button', { name: /zoom out/i }).first().click()
          await page.waitForTimeout(400)
        }
      },
    },
    {
      // File › Export: a PNG of the first artboard at twice its size.
      name: 'see-3',
      steps: async (page, { kit }) => {
        await openExport(page, kit)
        await kit.getByTestId('vector-export-format').selectOption('png')
        await kit.getByTestId('vector-export-scope').selectOption('artboard:0')
        await kit.getByTestId('vector-export-scale').selectOption('2')
        await page.waitForTimeout(500)
      },
    },
    {
      // An .svg opened in the editor: the icon set exported as SVG (File › Export), then that file
      // opened through the host.
      name: 'see-4',
      steps: async (page, { kit }) => {
        await openExport(page, kit)
        await kit.getByTestId('vector-export-format').selectOption('svg')
        await kit.getByTestId('vector-export-run').click()
        const name = 'weather-icon-set.svg'
        await page.waitForFunction((file) => window.devHost.files.has(file), name, { timeout: 60_000 })
        const bytes = Buffer.from(await page.evaluate((file) => [...new Uint8Array(window.devHost.files.get(file).bytes)], name))
        await openFile(page, { name: 'Weather icons.svg', bytes })
        await kit.locator('[data-testid="vector-status"]').waitFor()
        await page.waitForTimeout(2500)
      },
    },
  ],
})

// Shrinks the pictures when pngquant is there (it keeps a file it cannot make smaller).
const repo = path.resolve(web, '..')
for (const { file } of report) {
  try {
    execFileSync('pngquant', ['--quality', '70-90', '--force', '--skip-if-larger', '--output', file, file], { cwd: repo, stdio: 'ignore' })
  } catch (error) {
    if (error.code === 'ENOENT') break
  }
  console.log(`${file}  ${Math.ceil(fs.statSync(path.join(repo, file)).size / 1024)} KB`)
}
