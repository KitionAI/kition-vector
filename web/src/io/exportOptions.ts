/**
 * File › Export…'s settings (plan 2026-11-25, stage 7): the five formats a person exports a drawing
 * to, each with the options of the engine's `file.formats` that matter by hand (SVG: text as
 * outlines, embedded fonts, precision, minified, responsive; PNG, JPEG and WebP: the size, the
 * background, anti-aliasing, JPEG's quality; PDF as the engine's default preset, as the binding's
 * `file.exportBytes` takes no PDF options),
 * what to export (every artboard, one, the selection), and the `file.exportBytes` parameters they
 * make. Pure, so `node --test` checks it.
 */

import type { ExportFormat } from '../exporting.ts'

export type { ExportFormat }

export const EXPORT_FORMATS: readonly ExportFormat[] = ['svg', 'pdf', 'png', 'jpg', 'webp']

export const FORMAT_LABELS: Readonly<Record<ExportFormat, string>> = { svg: 'SVG', pdf: 'PDF', png: 'PNG', jpg: 'JPEG', webp: 'WebP' }

/** Every artboard, one (`artboard:<index>`), or the selection cropped to its art. */
export type ExportScope = 'all' | 'selection' | `artboard:${number}`

export type ExportSettings = {
  format: ExportFormat
  scope: ExportScope
  /** SVG: type as glyph outlines (viewable without the fonts), not text. */
  outlineText: boolean
  /** SVG: the fonts the type uses inside the file (when not outlined). */
  embedFonts: boolean
  /** SVG: coordinate precision, 1 to 7 decimals. */
  decimals: number
  minify: boolean
  /** SVG: no width and height, so it scales to its container. */
  responsive: boolean
  /** Raster: pixels per point (1 = 72 ppi). */
  scale: number
  /** PNG and WebP: transparent around the art (else on the background). */
  transparent: boolean
  /** JPEG (and PNG or WebP not transparent): the colour under the art. */
  background: string
  /** JPEG: 0 to 100. */
  quality: number
  /** Raster: anti-aliasing of the art (`art`), of type snapped to whole pixels (`type`), or none. */
  antiAlias: 'art' | 'type' | 'none'
}

export const SCALES = [0.5, 1, 2, 3, 4] as const
export const BACKGROUNDS = ['white', 'black'] as const

/** The settings an Export starts with: SVG of every artboard, text as the document's setup says (outlined when it keeps the appearance). */
export function defaultExport(outlineText = false): ExportSettings {
  return {
    format: 'svg',
    scope: 'all',
    outlineText,
    embedFonts: false,
    decimals: 3,
    minify: false,
    responsive: false,
    scale: 1,
    transparent: true,
    background: 'white',
    quality: 90,
    antiAlias: 'art',
  }
}

export const isRaster = (format: ExportFormat) => format === 'png' || format === 'jpg' || format === 'webp'

/** The artboard a scope names, or null. */
export function scopeArtboard(scope: ExportScope): number | null {
  return scope.startsWith('artboard:') ? Number(scope.slice('artboard:'.length)) : null
}

/** What part of the drawing: `file.exportBytes`'s scope parameters. */
function scopeParams(format: ExportFormat, scope: ExportScope): Record<string, unknown> {
  if (scope === 'selection') return { selectedOnly: true, useArtboards: false }
  const board = scopeArtboard(scope)
  if (board !== null) return { artboard: board }
  // Every artboard: a PDF has a page each; the other formats write a file each.
  return format === 'pdf' ? {} : { useArtboards: true, range: 'all' }
}

/** `file.exportBytes`'s parameters for `settings`. */
export function exportParams(settings: ExportSettings): Record<string, unknown> {
  const { format } = settings
  const params: Record<string, unknown> = { format, ...scopeParams(format, settings.scope) }
  if (format === 'svg') {
    // SVG's own options go in one object (`svg`), as `file.exportBytes` takes them.
    const svg: Record<string, unknown> = { outlineText: settings.outlineText, decimals: Math.min(7, Math.max(1, Math.round(settings.decimals))) }
    if (!settings.outlineText && settings.embedFonts) svg.embedFonts = true
    if (settings.minify) svg.minify = true
    if (settings.responsive) svg.responsive = true
    params.svg = svg
  } else if (isRaster(format)) {
    params.scale = settings.scale
    params.antiAlias = settings.antiAlias
    // JPEG has no transparency: it is always on its background.
    params.background = format !== 'jpg' && settings.transparent ? 'transparent' : settings.background
    if (format === 'jpg') params.quality = Math.min(100, Math.max(0, Math.round(settings.quality)))
  }
  return params
}

/**
 * The preview's parameters: the same part of the drawing as a small PNG (a PDF has no picture of
 * its own in a browser page), `width` the part's width in points, fitted into `box` pixels.
 */
export function previewParams(settings: ExportSettings, width: number, box = 320): Record<string, unknown> {
  const scale = Math.min(2, Math.max(0.05, box / Math.max(1, width)))
  return { format: 'png', ...scopeParams('png', settings.scope), scale: Math.round(scale * 1000) / 1000, background: 'transparent' }
}

/** The MIME type the preview shows a format's own bytes as, or null when it shows a PNG of it instead. */
export function previewMime(format: ExportFormat): string | null {
  return format === 'svg' ? 'image/svg+xml' : format === 'png' ? 'image/png' : format === 'jpg' ? 'image/jpeg' : format === 'webp' ? 'image/webp' : null
}

/** The bytes of a base64 string without decoding it (its length, less the padding). */
export function base64Size(data: string): number {
  const padding = data.endsWith('==') ? 2 : data.endsWith('=') ? 1 : 0
  return Math.max(0, Math.floor((data.length * 3) / 4) - padding)
}

/** The files an export answer holds and their total size. */
export function answerSize(answer: { data: string; files?: Array<{ data: string }> }): { files: number; bytes: number } {
  const files = answer.files && answer.files.length > 1 ? answer.files : [{ data: answer.data }]
  return { files: files.length, bytes: files.reduce((total, file) => total + base64Size(file.data), 0) }
}
