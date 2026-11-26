/**
 * Whether a live gesture or a dialog's preview is under way (plan 2026-11-25, stage 4): both take
 * their in-between steps away with Edit › Undo, so the History panel waits until none is, rather
 * than undo or redo across them. Pure, so `node --test` checks it.
 */

let holds = 0

/** Holds the History panel back until the returned release runs (once, however often it is called). */
export function holdHistory(): () => void {
  holds += 1
  let released = false
  return () => {
    if (released) return
    released = true
    holds -= 1
  }
}

/** Whether a gesture or a preview holds the History panel back now. */
export function historyHeld() {
  return holds > 0
}
