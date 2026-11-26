/**
 * Escape for dialogs over dialogs (plan 2026-11-25, stage 3): the Color Picker over a command's
 * dialog or over the swatch dialog. Kit UI's dialogs each close on any Escape the window sees, so
 * the one on top takes it first, in the capture phase, and no dialog under it hears it. The last
 * one pushed is on top.
 */
const stack: (() => void)[] = []

const onKey = (event: KeyboardEvent) => {
  if (event.key !== 'Escape' || event.isComposing) return
  const top = stack.at(-1)
  if (!top) return
  event.preventDefault()
  event.stopPropagation()
  event.stopImmediatePropagation()
  top()
}

/** Gives Escape to `cancel` while it is the latest one pushed; call the answer to let go. */
export function pushEscape(cancel: () => void): () => void {
  if (stack.length === 0) window.addEventListener('keydown', onKey, true)
  stack.push(cancel)
  return () => {
    const at = stack.lastIndexOf(cancel)
    if (at >= 0) stack.splice(at, 1)
    if (stack.length === 0) window.removeEventListener('keydown', onKey, true)
  }
}
