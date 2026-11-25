/**
 * File › Export: the drawing as SVG, PDF or PNG (at 1, 2 or 3 times its size, transparent or on
 * white): every artboard, one artboard, or the selection cropped to its art. The engine writes the
 * file into memory (`file.exportBytes`) and the host writes it as a new file next to the
 * drawing; success is shown only once the host answers.
 */
import { useState } from 'react'
import { Button, Checkbox, Choice, Dialog, toast, useKitEditor } from '@kition/sdk/editor'
import type { VectorInfo } from './vector'
import { exportFiles, writeExports, type ExportAnswer, type ExportFormat } from './exporting'

type Scope = 'all' | 'selection' | `artboard:${number}`

export function ExportDialog({ info, onClose }: { info: VectorInfo; onClose: () => void }) {
  const { query, host, t } = useKitEditor()
  const [format, setFormat] = useState<ExportFormat>('svg')
  const [scope, setScope] = useState<Scope>('all')
  const [scale, setScale] = useState('1')
  const [transparent, setTransparent] = useState(true)
  const [busy, setBusy] = useState(false)
  const scopes: Array<readonly [Scope, string]> = [
    ['all', t('exportAllArtboards')],
    ...info.artboards.map((artboard, index) => [`artboard:${index}` as Scope, artboard.name || t('exportArtboard', { number: index + 1 })] as const),
    ...(info.selection.length ? [['selection', t('exportSelection')] as const] : []),
  ]

  const run = async () => {
    setBusy(true)
    try {
      const params: Record<string, unknown> = { format }
      if (scope === 'selection') {
        params.selectedOnly = true
        params.useArtboards = false
      } else if (scope !== 'all') params.artboard = Number(scope.slice('artboard:'.length))
      // Every artboard: a PDF has a page each; SVG and PNG write a file each.
      else if (format !== 'pdf') {
        params.useArtboards = true
        params.range = 'all'
      }
      if (format === 'png') {
        params.scale = Number(scale)
        params.background = transparent ? 'transparent' : 'white'
      }
      const answer = await query<ExportAnswer>('file.exportBytes', params)
      if (!answer?.data) throw new Error(t('exportNoFile'))
      const names = scope === 'all' ? info.artboards.map((artboard) => artboard.name) : []
      const written = await writeExports(exportFiles(answer, host.init.document, format, names), host.exportFile)
      toast('success', t('exported'), written.join(', '))
      onClose()
    } catch (error) {
      toast('error', t('exportFailed'), error instanceof Error ? error.message : String(error))
    } finally {
      setBusy(false)
    }
  }

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
          <Button variant="primary" disabled={busy} onClick={() => void run()} data-testid="vector-export-run">
            {t('exportRun')}
          </Button>
        </>
      }
    >
      <div className="vector-export-form">
        <Choice label={t('exportFormat')} value={format} choices={[['svg', 'SVG'], ['pdf', 'PDF'], ['png', 'PNG']] as const} onChange={setFormat} testId="vector-export-format" />
        <Choice label={t('exportWhat')} value={scope} choices={scopes} onChange={setScope} testId="vector-export-scope" />
        {format === 'png' ? (
          <>
            <Choice label={t('exportScale')} value={scale} choices={[['1', '1×'], ['2', '2×'], ['3', '3×']] as const} onChange={setScale} testId="vector-export-scale" />
            <Checkbox label={t('exportTransparent')} checked={transparent} onChange={setTransparent} testId="vector-export-transparent" />
          </>
        ) : null}
      </div>
    </Dialog>
  )
}
