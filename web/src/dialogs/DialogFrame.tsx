import { useState, type ReactNode } from 'react'
import { Button, PreviewDialog, useKitEditor } from '@kition/sdk/editor'
import { useCommandPreview } from './useCommandPreview'

/** The dialogs' OK, Cancel and Preview: the Kit UI ids every kit's dialogs share. */
export const DIALOG_BUTTONS = { ok: 'control:dialog.ok', cancel: 'control:dialog.cancel', preview: 'control:dialog.preview' }

/** The test id every command dialog has (its buttons: `-ok`, `-cancel`, `-preview`; the transform dialogs' Copy: `-copy`). */
export const DIALOG_TEST_ID = 'vector-dialog'

/** Where the person last moved a dialog (by its title), for the session: the next opens there too, off the drawing. */
let lastPlace: { x: number; y: number } | null = null

/** A dialog's place: where the person last moved one, kept as they move this one. */
export function useDialogPlace() {
  const [place, setPlace] = useState(lastPlace)
  return {
    position: place,
    onMove: (at: { x: number; y: number }) => {
      lastPlace = at
      setPlace(at)
    },
  }
}

/**
 * A command's dialog: `PreviewDialog` running `command` with `params` (previewing it on the canvas
 * when `previewing`), OK leaving one history step and Cancel undoing the preview, then `onClose`.
 * Alt turns Cancel into Reset when the dialog has `onReset`. With `copy`, a Copy button runs the
 * command on a copy of the selection (the transform dialogs). Its title bar moves it, and the
 * next dialog opens where it was left.
 */
export function DialogFrame({
  command,
  title,
  params,
  previewing,
  onClose,
  onReset,
  okLabel,
  okDisabled,
  copy = false,
  children,
}: {
  command: string
  title: string
  params: Record<string, unknown>
  previewing: boolean
  onClose: (committed: boolean) => void
  onReset?: () => void
  okLabel?: string
  okDisabled?: boolean
  copy?: boolean
  children: ReactNode
}) {
  const { t } = useKitEditor()
  const run = useCommandPreview(command, params, previewing)
  const place = useDialogPlace()
  const [copying, setCopying] = useState(false)
  return (
    <PreviewDialog
      position={place.position}
      onMove={place.onMove}
      title={title}
      preview={run.preview}
      okLabel={okLabel}
      okDisabled={okDisabled || copying}
      onOk={async () => {
        await run.ok()
        onClose(true)
      }}
      onCancel={() => {
        run.cancel()
        onClose(false)
      }}
      onReset={onReset}
      ids={DIALOG_BUTTONS}
      testId={DIALOG_TEST_ID}
    >
      <div className="vector-dialog" data-command={command}>
        {children}
        {copy ? (
          <div className="vector-dialog-copy">
            <Button
              variant="secondary"
              disabled={okDisabled || copying}
              onClick={() => {
                setCopying(true)
                void run
                  .ok({ copy: true })
                  .then(() => onClose(true))
                  .catch(() => setCopying(false))
              }}
              data-testid={`${DIALOG_TEST_ID}-copy`}
            >
              {t('dialog.copy')}
            </Button>
          </div>
        ) : null}
      </div>
    </PreviewDialog>
  )
}
