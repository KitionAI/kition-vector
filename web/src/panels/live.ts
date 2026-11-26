/**
 * A gesture that edits the drawing live and leaves one history step (plan 2026-11-25, stage 3): a
 * colour dragged in the picker, a number field's label scrubbed, a gradient stop dragged. The
 * engine keeps no coalescing of steps, so each live value takes the last live step away (`edit.undo`,
 * as the dialogs' preview does, `dialogs/preview.ts`) before it runs; `commit` keeps the last one,
 * or runs the final value once when it differs; `cancel` takes it away. Values that arrive while
 * one runs wait, and only the latest of them runs: a fast drag never queues a step per move.
 *
 * A live step is undone only while the document is still at the revision it left: someone else's
 * edit in between is never taken away (the gesture then simply keeps its steps). Pure, so
 * `node --test` checks it with a fake engine.
 */

export type LiveEngine = {
  /** Runs `command` with `params`; throws when the engine refuses it. */
  run: (command: string, params: Record<string, unknown>) => Promise<unknown>
  /** Takes the last step away. */
  undo: () => Promise<unknown>
  /** The document's revision now. */
  revision: () => number
}

export type LiveEdit = {
  /** Shows `command` with `params` on the canvas now, in place of the gesture's last live value. */
  set: (command: string, params: Record<string, unknown>) => void
  /** Ends the gesture with one step: the final value (`command`, `params`), or the last live one. */
  commit: (command?: string, params?: Record<string, unknown>) => Promise<void>
  /** Ends the gesture leaving the drawing as it was before it. */
  cancel: () => Promise<void>
  /** Whether a live value is on the canvas (for tests). */
  readonly live: boolean
}

type Call = { command: string; params: Record<string, unknown> }
const keyOf = (call: Call) => JSON.stringify([call.command, call.params])

export function createLiveEdit(engine: LiveEngine, onError: (error: unknown) => void = () => undefined): LiveEdit {
  let applied: { key: string; revision: number } | null = null
  let pending: Call | null = null
  let running: Promise<void> | null = null
  let last: Call | null = null

  const undoApplied = async () => {
    const mine = applied
    applied = null
    if (mine && engine.revision() === mine.revision) await engine.undo()
  }
  const runOnce = async (call: Call) => {
    const before = engine.revision()
    await engine.run(call.command, call.params)
    const after = engine.revision()
    return after !== before ? after : null
  }
  const pump = () => {
    if (running) return running
    running = (async () => {
      while (pending) {
        const next = pending
        pending = null
        try {
          await undoApplied()
          const revision = await runOnce(next)
          applied = revision === null ? null : { key: keyOf(next), revision }
        } catch (error) {
          onError(error)
        }
      }
      running = null
    })()
    return running
  }
  return {
    get live() {
      return applied !== null
    },
    set(command, params) {
      last = { command, params }
      pending = last
      void pump()
    },
    async commit(command, params) {
      if (command) last = { command, params: params ?? {} }
      pending = null
      await running
      const final = last
      last = null
      if (!final) return
      if (applied && applied.key === keyOf(final) && engine.revision() === applied.revision) {
        applied = null
        return
      }
      try {
        await undoApplied()
        await engine.run(final.command, final.params)
      } catch (error) {
        onError(error)
      }
    },
    async cancel() {
      pending = null
      last = null
      await running
      await undoApplied()
    },
  }
}
