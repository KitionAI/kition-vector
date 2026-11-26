/**
 * Files from the computer and the system clipboard (plan 2026-11-25, stage 7): which files the
 * editor opens and places (what the engine reads: drawings, SVG, PDF, EPS, AI, images), what a drag
 * carries, what a paste holds and which engine command takes it in (`clipboard.import*`), the
 * name a drawing saved from an SVG or PDF gets, and bytes as the base64 the commands take. Pure,
 * so `node --test` checks it; the handlers are in `VectorFileIo.tsx`.
 */

/** Drawings the engine opens as a document of their own (File › Open…). */
export const OPEN_EXTENSIONS = ['kvector', 'svg', 'svgz', 'pdf', 'ai', 'eps', 'vctemplate'] as const

/** Images the engine places (embedded at their own size). */
export const IMAGE_EXTENSIONS = ['png', 'jpg', 'jpeg', 'gif', 'webp', 'tif', 'tiff', 'bmp'] as const

/** What File › Place… and a drop take: the drawings (placed as one group of their art) and images. */
export const PLACE_EXTENSIONS = ['kvector', 'svg', 'svgz', 'pdf', 'ai', 'eps', 'dxf', 'emf', 'wmf', 'txt', ...IMAGE_EXTENSIONS] as const

/** File › New from Template…'s picker: templates, and any drawing as one. */
export const TEMPLATE_EXTENSIONS = ['vctemplate', 'kvector', 'svg', 'pdf', 'ai', 'eps'] as const

const dotted = (list: readonly string[]) => list.map((extension) => `.${extension}`)

/** The host picker's `accept` for each purpose. */
export const OPEN_ACCEPT: readonly string[] = dotted(OPEN_EXTENSIONS)
export const PLACE_ACCEPT: readonly string[] = dotted(PLACE_EXTENSIONS)
export const TEMPLATE_ACCEPT: readonly string[] = dotted(TEMPLATE_EXTENSIONS)

/** A file name's extension, lower case, without its dot ('' when it has none). */
export function extensionOf(name: string): string {
  return (/\.([^./\\]+)$/.exec(name)?.[1] ?? '').toLowerCase()
}

/** Whether `name` is a Kition Vector drawing (its own Save writes it). */
export function isNative(name: string): boolean {
  return extensionOf(name) === 'kvector'
}

/** Whether a file (its name and MIME type) is one to place: an image type, or a known extension. */
export function isPlaceable(file: { name: string; type?: string }): boolean {
  if (file.type && /^image\//i.test(file.type)) return true
  return (PLACE_EXTENSIONS as readonly string[]).includes(extensionOf(file.name))
}

/** Whether a drag carries files (not text or a link): only then is the canvas a drop target. */
export function dragHasFiles(types: readonly string[] | DOMStringList | null | undefined): boolean {
  if (!types) return false
  return Array.from(types as ArrayLike<string>).includes('Files')
}

/**
 * The drawing a document read from another format saves as (`Poster.svg` → `Poster.kvector`,
 * beside it): its own Save would write Kition Vector bytes over the SVG.
 */
export function nativeName(path: string): string {
  const slash = path.lastIndexOf('/')
  const folder = slash >= 0 ? path.slice(0, slash + 1) : ''
  const name = path.slice(slash + 1)
  const stem = name.replace(/\.[^.]+$/, '').trim() || 'Drawing'
  return `${folder}${stem}.kvector`
}

/** A copy's name (File › Save a Copy…): `Poster copy.kvector`, beside the drawing. */
export function copyName(path: string): string {
  return nativeName(path).replace(/\.kvector$/, ' copy.kvector')
}

/** Standard base64 of `bytes`, in chunks (a large image overflows one `String.fromCharCode` call). */
export function toBase64(bytes: Uint8Array): string {
  let binary = ''
  const chunk = 0x8000
  for (let at = 0; at < bytes.length; at += chunk) binary += String.fromCharCode(...bytes.subarray(at, at + chunk))
  return btoa(binary)
}

/** The bytes of standard base64 `text`. */
export function fromBase64(text: string): Uint8Array {
  const binary = atob(text)
  const bytes = new Uint8Array(binary.length)
  for (let at = 0; at < binary.length; at += 1) bytes[at] = binary.charCodeAt(at)
  return bytes
}

/** Whether `text` is SVG markup (pasted from a code editor, another vector app's text flavour). */
export function isSvgText(text: string): boolean {
  const start = text.trimStart().slice(0, 512)
  return /^(?:<\?xml[^>]*>\s*)?(?:<!--[\s\S]*?-->\s*)*(?:<!DOCTYPE svg[^>]*>\s*)?<svg[\s>]/i.test(start)
}

/** What a paste brings, as the system clipboard gives it (`DataTransfer` or `ClipboardItem` types). */
export type Pasted = {
  /** The `image/svg+xml` flavour, or text/plain that is SVG markup. */
  svg?: string
  /** A picture (PNG, JPEG, …): its bytes and type. */
  image?: { bytes: Uint8Array; type: string; name?: string }
  /** A PDF flavour (some apps copy vectors as one). */
  pdf?: Uint8Array
  /** Plain text. */
  text?: string
}

/**
 * The engine command that takes a paste in, best flavour first (as the reference reads the
 * system clipboard: vectors before pictures before text), with its parameters; null when nothing
 * in it is for the drawing. `center` is where it lands (the middle of the view).
 */
export function importOf(pasted: Pasted, center: [number, number]): { command: string; params: Record<string, unknown> } | null {
  if (pasted.svg) return { command: 'clipboard.importSvg', params: { svg: pasted.svg, center } }
  if (pasted.pdf) return { command: 'clipboard.importPdf', params: { dataBase64: toBase64(pasted.pdf), center } }
  if (pasted.image) return { command: 'clipboard.importImage', params: { dataBase64: toBase64(pasted.image.bytes), mime: pasted.image.type, center } }
  const text = pasted.text?.replace(/\r\n?/g, '\n')
  if (text && text.trim()) return { command: 'clipboard.importText', params: { text, center } }
  return null
}

/** What a copy holds for other applications, as the engine answers it (`clipboard.export*`). */
export type Copied = { svg: string | null; text: string | null; png: Uint8Array | null }

/** The types the editor offers the system clipboard, best first, of those the browser writes (`writable`). */
export function copyTypes(writable: (type: string) => boolean): string[] {
  return ['image/svg+xml', 'text/plain', 'image/png'].filter(writable)
}

/**
 * One type of a copy for the system clipboard, or null when the copy has none: the SVG markup as
 * `image/svg+xml`; as text/plain a type-only copy's words, else the SVG markup (vector apps and
 * code editors paste SVG markup as text); the PNG.
 */
export function flavourOf(type: string, copied: Copied): Blob | null {
  if (type === 'image/svg+xml') return copied.svg ? new Blob([copied.svg], { type }) : null
  if (type === 'text/plain') {
    const text = copied.text ?? copied.svg
    return text ? new Blob([text], { type }) : null
  }
  if (type === 'image/png') return copied.png ? new Blob([copied.png.slice().buffer], { type }) : null
  return null
}

/** The text/plain a copy puts on the system clipboard (to know it again on paste). */
export function copiedText(copied: Copied): string | null {
  return copied.text ?? copied.svg
}

/** A cheap fingerprint of text (length and an FNV-1a hash), to know the editor's own copy on the clipboard again. */
export function fingerprint(text: string): string {
  let hash = 0x811c9dc5
  for (let at = 0; at < text.length; at += 1) {
    hash ^= text.charCodeAt(at)
    hash = Math.imul(hash, 0x01000193) >>> 0
  }
  return `${text.length}:${hash.toString(16)}`
}

/**
 * Where dropped files go: the first at the drop point, the next ones 24 points further down and
 * right, as a stack the person can tell apart.
 */
export function dropCenters(point: { x: number; y: number }, count: number, step = 24): Array<[number, number]> {
  return Array.from({ length: count }, (_, index) => [Math.round(point.x + index * step), Math.round(point.y + index * step)])
}

/** A file size as a person reads it: `812 B`, `14.2 KB`, `3.1 MB`. */
export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
}
