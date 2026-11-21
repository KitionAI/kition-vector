// Installs the published @kition/sdk and @kition/tessera packages. KITION_LINK=1 links the
// kition-sdk and kition-tessera checkouts instead (KITION_SDK, KITION_TESSERA; default
// ../kition-sdk and ../kition-tessera) for development; do not commit the lockfile it writes.
// Both packages' `exports` point at their built `dist/`, so build them in the checkouts first
// (`pnpm build` in kition-sdk/packages/sdk and in kition-tessera/packages/tessera).
const fs = require('node:fs')
const path = require('node:path')

const repo = path.resolve(__dirname, '..')
const sdk = path.resolve(process.env.KITION_SDK ?? path.join(repo, '..', 'kition-sdk'))
const tessera = path.resolve(process.env.KITION_TESSERA ?? path.join(repo, '..', 'kition-tessera'))

const local = {
  '@kition/sdk': path.join(sdk, 'packages/sdk'),
  '@kition/tessera': path.join(tessera, 'packages/tessera'),
}

function readPackage(pkg) {
  if (pkg.name !== 'kition-vector-ui' || process.env.KITION_LINK !== '1') return pkg
  for (const field of ['dependencies', 'devDependencies']) {
    for (const name of Object.keys(pkg[field] ?? {})) {
      const dir = local[name]
      if (dir && fs.existsSync(path.join(dir, 'package.json'))) {
        pkg[field][name] = `link:${path.relative(__dirname, dir).split(path.sep).join('/')}`
      }
    }
  }
  return pkg
}

module.exports = { hooks: { readPackage } }
