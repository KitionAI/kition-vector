import { useEffect, useMemo, useRef, useState } from 'react'
import { useKitEditor, type PreviewDialogPreview } from '@kition/sdk/editor'
import { createPreview, PREVIEW_DEBOUNCE_MS } from './preview'

/** `value`, once it has stayed the same (as JSON) for `ms`. */
export function useDebounced<T>(value: T, ms: number): T {
  const [settled, setSettled] = useState(value)
  const key = JSON.stringify(value)
  const latest = useRef(value)
  latest.current = value
  useEffect(() => {
    const timer = window.setTimeout(() => setSettled(latest.current), ms)
    return () => window.clearTimeout(timer)
  }, [key, ms])
  return settled
}

/**
 * A dialog's command, run through the editor's session: `preview` for `PreviewDialog` when the
 * dialog previews (run, and on a change take the preview's step away and run again, 150 ms after
 * the last change), `ok` that leaves one history step with `params` (or with `extra` over them:
 * the transform dialogs' Copy), and `cancel` that takes the preview's step away.
 *
 * The preview's step is taken away with `edit.undo` (plan 2026-11-25, stage 1, recorded in its
 * progress notes): the engine's own interaction preview is not exposed by the binding, and the
 * session's change tracking (revisions, the agent's view, the host's replica) follows only
 * commands. So after Cancel, Edit › Redo could bring the cancelled preview back. A refused or
 * failed run throws, so the dialog shows why and stays open.
 */
export function useCommandPreview(command: string, params: Record<string, unknown>, previewing: boolean) {
  const { session, t } = useKitEditor()
  const sessionNow = useRef(session)
  sessionNow.current = session
  const preview = useMemo(
    () =>
      createPreview({
        run: async (values) => {
          const result = await sessionNow.current.run(command, values)
          if (result === null) throw new Error(t('dialog.refused'))
        },
        undo: () => sessionNow.current.run('edit.undo', {}),
        revision: () => sessionNow.current.doc?.info.revision ?? 0,
      }),
    // One preview for the dialog's life; `t` only words an error.
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [command],
  )
  const settled = useDebounced(params, PREVIEW_DEBOUNCE_MS)
  const settledNow = useRef(settled)
  settledNow.current = settled
  const paramsNow = useRef(params)
  paramsNow.current = params
  const dialogPreview: PreviewDialogPreview | undefined = previewing
    ? {
        begin: () => preview.show(settledNow.current),
        update: () => preview.show(settledNow.current),
        end: () => preview.end(),
        draft: settled,
      }
    : undefined
  return {
    preview: dialogPreview,
    ok: (extra?: Record<string, unknown>) => preview.commit(extra ? { ...paramsNow.current, ...extra } : paramsNow.current),
    cancel: () => void preview.drop(),
  }
}
