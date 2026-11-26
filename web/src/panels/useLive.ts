import { useMemo, useRef } from 'react'
import { useKitEditor } from '@kition/sdk/editor'
import { createLiveEdit, type LiveEdit } from './live'

/**
 * The editor's live gestures (`live.ts`) on its session: `set` begins one (or moves it on), `commit`
 * and `cancel` end it. A refused value is reported once per gesture, in the editor's words.
 */
export function useLive(): Pick<LiveEdit, 'set' | 'commit' | 'cancel'> {
  const editor = useKitEditor()
  const latest = useRef(editor)
  latest.current = editor
  const gesture = useRef<{ edit: LiveEdit; reported: boolean } | null>(null)
  return useMemo(() => {
    const current = () => {
      if (gesture.current) return gesture.current.edit
      const own = { edit: null as unknown as LiveEdit, reported: false }
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
      const edit = gesture.current?.edit
      gesture.current = null
      return edit
    }
    return {
      set: (command, params) => current().set(command, params),
      commit: async (command, params) => {
        const edit = command ? current() : end()
        if (command) gesture.current = null
        await edit?.commit(command, params)
      },
      cancel: async () => {
        await end()?.cancel()
      },
    }
  }, [])
}
