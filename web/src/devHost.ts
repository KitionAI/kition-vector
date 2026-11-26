/**
 * The development host (dev.html): opens `public/sample/Sample.kvector`, or a file chosen in the
 * page. It serves opening (`open-file`, plan 2026-11-25, stage 7): Kition opens the file
 * `host/open` names in a tab of its own; this page has one editor, so it opens the file in it,
 * as a person running the editor alone expects of File › Open… and a new document's Open.
 * Picks offer the drawings and pictures the editor opens and places.
 */
import { startDevHost } from '@kition/sdk/dev-host'

startDevHost({ sample: './sample/Sample.kvector', open: true, pickable: /\.(kvector|vctemplate|svgz?|pdf|ai|eps|dxf|png|jpe?g|gif|webp|tiff?|bmp)$/i })

type DevOpen = { method: string; params: { link?: { path?: string; name?: string } } }
type DevHost = { files: Map<string, { bytes: ArrayBuffer; version: number }>; open: (path: string, bytes: ArrayBuffer) => Promise<void>; opens: DevOpen[] }

const host = (window as unknown as { devHost?: DevHost }).devHost
if (host) {
  const record = host.opens.push.bind(host.opens)
  host.opens.push = (...requests: DevOpen[]) => {
    const count = record(...requests)
    for (const request of requests) {
      const link = request.method === 'host/open' ? request.params.link : undefined
      const path = link?.path ?? [...host.files.keys()].find((file) => file.split('/').pop() === link?.name)
      const file = path ? host.files.get(path) : undefined
      // After the host's answer has gone back to the editor that asked.
      if (path && file && file.bytes.byteLength > 0) window.setTimeout(() => void host.open(path, file.bytes.slice(0)), 50)
    }
    return count
  }
}
