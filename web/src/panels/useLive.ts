import { useMemo, useRef } from 'react'
import { useKitEditor } from '@kition/sdk/editor'
import { holdHistory } from './historyHold'
import { createLiveEdit, type LiveEdit } from './live'

/**
 * The editor's live gestures (`live.ts`) on its session: `set` begins one (or moves it on), `commit`
 * and `cancel` end it. A refused value is reported once per gesture, in the editor's words.
 */
export function useLive(): Pick<LiveEdit, 'set' | 'commit' | 'cancel'> {
  const editor = useKitEditor()
  const latest = useRef(editor)
  latest.current = editor
  const gesture = useRef<{ edit: LiveEdit; reported: boolean; release: () => void } | null>(null)
  return useMemo(() => {
    const current = () => {
      if (gesture.current) return gesture.current.edit
      // The History panel waits while the gesture's live steps come and go.
      const own = { edit: null as unknown as LiveEdit, reported: false, release: holdHistory() }
      own.edit = createLiveEdit(
        {
          run: async (command, params) => {
            const answer = await latest.current.session.run(command, params)
            if (answer === null) throw new Error(latest.current.t('dialog.refused'))
          },
          undo: () => latest.current.session.run('edit.undo', {}),
          revision: () => latest.current.session.doc?.info.revision ?? 0,
        },
        (error) => {
          if (own.reported) return
          own.reported = true
          latest.current.report('', error)
        },
      )
      gesture.current = own
      return own.edit
    }
    const end = () => {
      const own = gesture.current
      gesture.current = null
      return own
    }
    return {
      set: (command, params) => current().set(command, params),
      commit: async (command, params) => {
        if (command) current()
        const own = end()
        try {
          await own?.edit.commit(command, params)
        } finally {
          own?.release()
        }
      },
      cancel: async () => {
        const own = end()
        try {
          await own?.edit.cancel()
        } finally {
          own?.release()
        }
      },
    }
  }, [])
}
