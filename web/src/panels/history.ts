/**
 * The History panel's rows and moves (plan 2026-11-25, stage 4). Pure, so `node --test` checks it.
 * Row 0 is the drawing as it was opened; row `n` is the state after the `n`-th step. The steps
 * after the current one (the redo list) follow it, dimmed.
 */

export type HistoryRow = { position: number; name: string | null; undone: boolean }

/** The rows: the opened state, the steps done (oldest first), then the steps undone (next first). */
export function historyRows(history: readonly string[], redo: readonly string[]): HistoryRow[] {
  return [
    { position: 0, name: null, undone: false },
    ...history.map((name, index) => ({ position: index + 1, name, undone: false })),
    ...redo.map((name, index) => ({ position: history.length + index + 1, name, undone: true })),
  ]
}

/** How to reach row `target` from the current state (`done` steps done, `undone` waiting to redo): undo or redo how many times. */
export function historyMove(done: number, undone: number, target: number): { command: 'edit.undo' | 'edit.redo'; times: number } | null {
  const to = Math.max(0, Math.min(done + undone, Math.round(target)))
  if (to === done) return null
  return to < done ? { command: 'edit.undo', times: done - to } : { command: 'edit.redo', times: to - done }
}
