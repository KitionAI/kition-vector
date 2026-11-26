/**
 * File › Export… (plan 2026-11-25, stage 7): the drawing as SVG, PDF, PNG, JPEG or WebP with each
 * format's options (`io/exportOptions.ts`): SVG with text as outlines (so another computer without
 * the font sees it as here), embedded fonts, precision; PNG, JPEG and WebP at a size, on a
 * background, JPEG's quality; PDF with its fonts embedded. Every artboard, one, or the selection. The preview shows what the
 * export holds (the file itself for SVG and the pictures, a picture of it for PDF) and its exact
 * size; Export writes those bytes beside the drawing through the host (`host/write`, as an export).
 * A slow export shows the progress bar; Cancel there writes nothing.
 */
import { useEffect, useMemo, useRef, useState } from 'react'
import { Button, Dialog, toast, useKitEditor } from '@kition/sdk/editor'
import type { VectorInfo } from './vector'
import { exportFiles, writeExports, type ExportAnswer } from './exporting'
import { CheckRow, ChoiceRow, NumberRow, SliderRow } from './dialogs/controls'
import { useDebounced } from './dialogs/useCommandPreview'
import { errorMessage } from './io/errors'
import { formatBytes } from './io/files'
import type { Busy } from './io/busy'
import {
  BACKGROUNDS,
  EXPORT_FORMATS,
  FORMAT_LABELS,
  SCALES,
  answerSize,
  defaultExport,
  exportParams,
  isRaster,
  previewMime,
  previewParams,
  scopeArtboard,
  type ExportFormat,
  type ExportScope,
  type ExportSettings,
} from './io/exportOptions'

const PREVIEW_MS = 300
/** The settings of the last export this session: the next Export… opens on them. */
let remembered: ExportSettings | null = null

type Preview = { key: string; answer: ExportAnswer; url: string | null; files: number; bytes: number }

export function ExportDialog({ info, busy, onClose }: { info: VectorInfo; busy: Busy; onClose: () => void }) {
  const { query, host, t } = useKitEditor()
  const [settings, setSettings] = useState<ExportSettings>(() => {
    const start = remembered ?? defaultExport()
    // A scope that no longer exists (an artboard deleted, nothing selected) starts as every artboard.
    const board = scopeArtboard(start.scope)
    const valid = start.scope === 'all' || (start.scope === 'selection' ? info.selection.length > 0 : board !== null && board < info.artboards.length)
    return valid ? start : { ...start, scope: 'all' }
  })
  const set = (patch: Partial<ExportSettings>) => setSettings((now) => ({ ...now, ...patch }))
  // Text as outlines starts as Document Setup › Export Text says, until the person chooses.
  const touched = useRef(remembered !== null)
  useEffect(() => {
    if (touched.current) return
    let live = true
    void query<{ exportText?: string }>('file.setup', {}).then((setup) => {
      if (live && !touched.current && setup?.exportText === 'appearance') setSettings((now) => ({ ...now, outlineText: true }))
    })
    return () => {
      live = false
    }
  }, [query])

  const { format } = settings
  const scopes: Array<readonly [ExportScope, string]> = [
    ['all', t('exportAllArtboards')],
    ...info.artboards.map((artboard, index) => [`artboard:${index}` as ExportScope, artboard.name || t('exportArtboard', { number: index + 1 })] as const),
    ...(info.selection.length ? [['selection', t('exportSelection')] as const] : []),
  ]
  const params = useMemo(() => exportParams(settings), [settings])
  const key = JSON.stringify(params)
  const settled = useDebounced(settings, PREVIEW_MS)

  // The preview: the export itself (its bytes are what Export writes while the settings stay these).
  const [preview, setPreview] = useState<Preview | null>(null)
  const [updating, setUpdating] = useState(false)
  const [previewError, setPreviewError] = useState<string | null>(null)
  const serial = useRef(0)
  useEffect(() => {
    serial.current += 1
    const mine = serial.current
    const settledParams = exportParams(settled)
    const settledKey = JSON.stringify(settledParams)
    setUpdating(true)
    void (async () => {
      try {
        const answer = await query<ExportAnswer>('file.exportBytes', settledParams)
        if (!answer?.data) throw new Error(t('exportNoFile'))
        let url: string | null = null
        const mime = previewMime(settled.format)
        if (mime) {
          const first = answer.files && answer.files.length > 1 ? answer.files[0].data : answer.data
          url = URL.createObjectURL(new Blob([Uint8Array.from(atob(first), (char) => char.charCodeAt(0))], { type: mime }))
        } else {
          const board = scopeArtboard(settled.scope) ?? 0
          const width = settled.scope === 'selection' ? (info.selectionBounds?.width ?? 320) : (info.artboards[board]?.width ?? 320)
          const picture = await query<ExportAnswer>('file.exportBytes', previewParams(settled, width))
          const data = picture?.files && picture.files.length > 1 ? picture.files[0].data : picture?.data
          if (data) url = URL.createObjectURL(new Blob([Uint8Array.from(atob(data), (char) => char.charCodeAt(0))], { type: 'image/png' }))
        }
        if (mine !== serial.current) {
          if (url) URL.revokeObjectURL(url)
          return
        }
        setPreview((before) => {
          if (before?.url) URL.revokeObjectURL(before.url)
          return { key: settledKey, answer, url, ...answerSize(answer) }
        })
        setPreviewError(null)
      } catch (error) {
        if (mine === serial.current) setPreviewError(errorMessage(t, error))
      } finally {
        if (mine === serial.current) setUpdating(false)
      }
    })()
    // `settled` holds every setting; the drawing does not change while the dialog is open.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [settled])
  // The preview's picture is let go with the dialog.
  const shownUrl = useRef<string | null>(null)
  shownUrl.current = preview?.url ?? null
  useEffect(
    () => () => {
      if (shownUrl.current) URL.revokeObjectURL(shownUrl.current)
    },
    [],
  )

  const [running, setRunning] = useState(false)
  const run = async () => {
    setRunning(true)
    const job = busy.begin(t('busy.export'))
    try {
      const answer = preview?.key === key ? preview.answer : await query<ExportAnswer>('file.exportBytes', params)
      if (!answer?.data) throw new Error(t('exportNoFile'))
      if (busy.end(job)) return
      const names = settings.scope === 'all' ? info.artboards.map((artboard) => artboard.name) : []
      const written = await writeExports(exportFiles(answer, host.init.document, format, names), host.exportFile)
      remembered = settings
      toast('success', t('exported'), written.join(', '))
      onClose()
    } catch (error) {
      busy.end(job)
      toast('error', t('exportFailed'), error instanceof Error ? error.message : String(error))
    } finally {
      setRunning(false)
    }
  }

  const raster = isRaster(format)
  const size = preview && preview.key === key ? preview : null
  return (
    <Dialog
      title={t('export')}
      onClose={onClose}
      testId="vector-export-dialog"
      actions={
        <>
          <Button variant="ghost" onClick={onClose} data-testid="vector-export-cancel">
            {t('cancel')}
          </Button>
          <Button variant="primary" disabled={running} onClick={() => void run()} data-testid="vector-export-run">
            {t('exportRun')}
          </Button>
        </>
      }
    >
      <div className="vector-export">
        <div className="vector-dialog vector-dialog-fields vector-export-form">
          <ChoiceRow label={t('exportFormat')} value={format} choices={EXPORT_FORMATS.map((item) => [item, FORMAT_LABELS[item]] as const)} onChange={(next: ExportFormat) => set({ format: next })} testId="vector-export-format" />
          <ChoiceRow label={t('exportWhat')} value={settings.scope} choices={scopes} onChange={(scope) => set({ scope })} testId="vector-export-scope" />
          {format === 'svg' ? (
            <CheckRow
              label={t('exportOutlineText')}
              checked={settings.outlineText}
              onChange={(outlineText) => {
                touched.current = true
                set({ outlineText })
              }}
              testId="vector-export-outline-text"
            />
          ) : null}
          {format === 'svg' ? (
            <>
              <CheckRow label={t('exportEmbedFonts')} checked={settings.embedFonts && !settings.outlineText} disabled={settings.outlineText} onChange={(embedFonts) => set({ embedFonts })} testId="vector-export-embed-fonts" />
              <NumberRow label={t('exportDecimals')} value={settings.decimals} min={1} max={7} decimals={0} onChange={(decimals) => set({ decimals: decimals ?? 3 })} testId="vector-export-decimals" />
              <CheckRow label={t('exportMinify')} checked={settings.minify} onChange={(minify) => set({ minify })} testId="vector-export-minify" />
              <CheckRow label={t('exportResponsive')} checked={settings.responsive} onChange={(responsive) => set({ responsive })} testId="vector-export-responsive" />
            </>
          ) : null}
          {raster ? (
            <>
              <ChoiceRow label={t('exportScale')} value={String(settings.scale)} choices={SCALES.map((scale) => [String(scale), `${scale}× (${Math.round(scale * 72)} ppi)`] as const)} onChange={(scale) => set({ scale: Number(scale) })} testId="vector-export-scale" />
              {format !== 'jpg' ? <CheckRow label={t('exportTransparent')} checked={settings.transparent} onChange={(transparent) => set({ transparent })} testId="vector-export-transparent" /> : null}
              {format === 'jpg' || !settings.transparent ? (
                <ChoiceRow label={t('exportBackground')} value={settings.background} choices={BACKGROUNDS.map((item) => [item, t(`exportBackground.${item}`)] as const)} onChange={(background) => set({ background })} testId="vector-export-background" />
              ) : null}
              {format === 'jpg' ? <SliderRow label={t('exportQuality')} value={settings.quality} min={0} max={100} fallback={90} onChange={(quality) => set({ quality })} testId="vector-export-quality" /> : null}
              <ChoiceRow
                label={t('exportAntiAlias')}
                value={settings.antiAlias}
                choices={(['art', 'type', 'none'] as const).map((item) => [item, t(`exportAntiAlias.${item}`)] as const)}
                onChange={(antiAlias) => set({ antiAlias })}
                testId="vector-export-antialias"
              />
              {format === 'webp' ? <p className="vector-export-note">{t('exportWebpLossless')}</p> : null}
            </>
          ) : null}
        </div>
        <figure className="vector-export-preview" data-testid="vector-export-preview" data-updating={updating || undefined}>
          {preview?.url ? <img src={preview.url} alt={t('exportPreview')} data-testid="vector-export-preview-image" /> : <span className="vector-export-note">{previewError ?? t('exportPreviewing')}</span>}
          <figcaption className="tabular-nums" data-testid="vector-export-size">
            {size ? (size.files > 1 ? t('exportSizeFiles', { size: formatBytes(size.bytes), files: size.files }) : t('exportSize', { size: formatBytes(size.bytes) })) : t('exportPreviewing')}
          </figcaption>
        </figure>
      </div>
    </Dialog>
  )
}
