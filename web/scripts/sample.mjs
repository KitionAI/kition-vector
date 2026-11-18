// Makes public/sample/Sample.kvector with the engine itself: a few colored shapes on two layers.
import fs from 'node:fs/promises'
import { fileURLToPath, pathToFileURL } from 'node:url'

const web = fileURLToPath(new URL('..', import.meta.url))
const engine = await import(pathToFileURL(`${web}public/engine/kition_vector_kit.js`).href)
await engine.default({ module_or_path: await fs.readFile(`${web}public/engine/kition_vector_kit_bg.wasm`) })
const doc = engine.VectorDocument.create('Sample.kvector', 800, 600)
const run = (command, params = {}) => doc.execute(command, JSON.stringify(params))
run('shape.rectangle', { x: 80, y: 120, width: 260, height: 180, radius: 24 })
run('paint.setFill', { color: '#5645d4' })
run('paint.toggleActive')
run('paint.none')
run('paint.toggleActive')
run('shape.ellipse', { x: 400, y: 140, width: 180, height: 180 })
run('paint.setFill', { color: '#f2a93b' })
run('layer.new', { name: 'Details' })
run('shape.star', { cx: 640, cy: 420, radius1: 70, radius2: 32 })
run('paint.setFill', { color: '#2e9e6b' })
run('select.none')
await fs.writeFile(`${web}public/sample/Sample.kvector`, doc.save())
console.log('wrote public/sample/Sample.kvector')
