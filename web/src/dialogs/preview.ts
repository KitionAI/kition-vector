/**
 * A dialog's live preview on the canvas, by running its command and taking its step away again
 * (plan 2026-11-25, stage 1): `show` runs the command with the dialog's values (dropping the last
 * preview first), `drop` takes the preview's step away, and `commit` leaves exactly one history
 * step with the final values, keeping the preview when it already shows them, so OK does not run
 * a slow effect twice.
 *
 * Every call runs after the one before (any dialog's). A preview is undone only while the document
 * is still at the revision it left (nobody edited since): otherwise the undo would take someone
 * else's step.
 * A run that changed nothing (no new revision) has nothing to undo.
 *
 * `end` is the dialog's "the preview is over" (`PreviewDialog` calls it before OK and when Preview
 * is turned off): it drops the preview a moment later unless `commit` claims it first, which OK does
 * right after.
 */

export type PreviewEngine = {
  /** Runs the command with `params`; throws when the engine refuses it. */
  run: (params: Record<string, unknown>) => Promise<unknown>
  /** Takes the last step away (`edit.undo`: the engine keeps no other way to drop a step). */
  undo: () => Promise<unknown>
  /** The document's revision now (it moves with every change, an undo too). */
  revision: () => number
}

export type Preview = {
  show: (params: Record<string, unknown>) => Promise<void>
  end: () => void
  drop: () => Promise<void>
  commit: (params: Record<string, unknown>) => Promise<void>
  /** Whether a preview is on the canvas (for tests). */
  readonly shown: boolean
}

/** How long `end` waits for a `commit` before it drops the preview. */
export const END_GRACE_MS = 40

/** The identity of a set of values: their JSON with the keys sorted. */
export function paramsKey(params: Record<string, unknown>) {
  return JSON.stringify(Object.keys(params).sort().map((key) => [key, params[key]]))
}

/**
 * Every preview's calls, one after another, across dialogs: a dialog closed while its preview runs
 * drops it before the next dialog's preview runs, so a drop never takes the next one's step.
 */
let queue: Promise<unknown> = Promise.resolve()

export function createPreview(engine: PreviewEngine, schedule: (task: () => void, ms: number) => unknown = setTimeout, unschedule: (handle: unknown) => void = (handle) => clearTimeout(handle as ReturnType<typeof setTimeout>)): Preview {
  // The preview on the canvas: its values and the revision its run left.
  let applied: { key: string; revision: number } | null = null
  let pendingEnd: unknown = null
  const serial = <T>(task: () => Promise<T>): Promise<T> => {
    const next = queue.then(task, task)
    queue = next.catch(() => undefined)
    return next
  }
  const cancelEnd = () => {
    if (pendingEnd !== null) unschedule(pendingEnd)
    pendingEnd = null
  }
  const undoApplied = async () => {
    const mine = applied
    applied = null
    if (mine && engine.revision() === mine.revision) await engine.undo()
  }
  const runOnce = async (params: Record<string, unknown>) => {
    const before = engine.revision()
    await engine.run(params)
    const after = engine.revision()
    return after !== before ? after : null
  }
  return {
    get shown() {
      return applied !== null
    },
    show(params) {
      cancelEnd()
      return serial(async () => {
        await undoApplied()
        const revision = await runOnce(params)
        applied = revision === null ? null : { key: paramsKey(params), revision }
      })
    },
    end() {
      cancelEnd()
      pendingEnd = schedule(() => {
        pendingEnd = null
        void serial(undoApplied)
      }, END_GRACE_MS)
    },
    drop() {
      cancelEnd()
      return serial(undoApplied)
    },
    commit(params) {
      cancelEnd()
      return serial(async () => {
        const kept = applied
        if (kept && kept.key === paramsKey(params) && engine.revision() === kept.revision) {
          applied = null
          return
        }
        await undoApplied()
        await engine.run(params)
      })
    },
  }
}

/** How long the values must stay the same before the preview runs again. */
export const PREVIEW_DEBOUNCE_MS = 150
