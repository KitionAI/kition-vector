/**
 * Export's file handling, apart from the dialog: the engine answers the file as base64
 * (`file.exportBytes`), and the host writes it as a new file next to the drawing
 * (`host/write`, mode `export`). Nothing here reaches the engine or the host, so it is tested on
 * its own.
 */

export type ExportFormat = 'svg' | 'pdf' | 'png' | 'jpg' | 'webp'

/** What `file.exportBytes` answers: the first file, and every file when the format writes several. */
export type ExportAnswer = { data: string; format?: string; files?: Array<{ name: string; data: string }> }

/** One file to write: its workspace path and bytes. */
export type ExportFile = { path: string; bytes: Uint8Array }

export function decodeBase64(base64: string): Uint8Array {
  return Uint8Array.from(atob(base64), (char) => char.charCodeAt(0))
}

/** A name safe in a file name ('' stays ''). */
const safe = (name: string) => name.replace(/[\\/:*?"<>|]/g, '_').trim()

/** The document's name without its extension, safe in a file name. */
export function exportStem(name: string): string {
  return safe(name.replace(/\.[^.]+$/, '')) || 'Drawing'
}

/** The folder of the document's workspace path, with its trailing slash ('' at the top). */
export function exportFolder(path: string): string {
  return path.includes('/') ? path.slice(0, path.lastIndexOf('/') + 1) : ''
}

/**
 * The files an answer makes: one (`Name.svg`), or, when the format writes one file per artboard,
 * one each named after its artboard (`Name - Artboard 1.png`; numbered when the names are not
 * known or not unique).
 */
export function exportFiles(answer: ExportAnswer, document: { name: string; path: string }, format: ExportFormat, artboards: readonly string[] = []): ExportFile[] {
  const at = `${exportFolder(document.path)}${exportStem(document.name)}`
  const files = answer.files ?? []
  if (files.length < 2) return [{ path: `${at}.${format}`, bytes: decodeBase64(answer.data) }]
  const names = artboards.map(safe)
  const named = names.length === files.length && names.every((name, index) => name && names.indexOf(name) === index)
  return files.map((file, index) => ({ path: `${at} - ${named ? names[index] : index + 1}.${format}`, bytes: decodeBase64(file.data) }))
}

/**
 * Writes each file through the host, in order, and answers the paths the host wrote (it may pick
 * an unused name). The host's refusal ends the export with its own error.
 */
export async function writeExports(files: readonly ExportFile[], write: (path: string, bytes: Uint8Array) => Promise<string>): Promise<string[]> {
  const written: string[] = []
  for (const file of files) written.push(await write(file.path, file.bytes))
  return written
}
