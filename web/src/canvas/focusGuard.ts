/**
 * The keyboard stays the editor's (plan 2026-11-25, stage 2): one guard for every way the focus is
 * lost, instead of a focus call after each control.
 *
 * - A control that goes away or turns disabled while it has the focus (a Pathfinder button that
 *   greys once its command ran, a field committed with Enter) leaves the focus on the page's body,
 *   where no editor key is heard. The guard gives it back to the workspace, and a key pressed
 *   before that (a disabled control loses the focus without any event) is passed on to the
 *   workspace as if it had been pressed there.
 * - When the last dialog closes, the focus goes to the workspace rather than back to the control
 *   that opened it (a menu title, where the next Enter would open the menu again).
 */

/** Where the focus is the person's own choice and must stay (typing somewhere). */
const TYPING = 'input, textarea, select, [contenteditable=""], [contenteditable="true"]'

const isPage = (element: Element | null) => element == null || element === document.body || element === document.documentElement

/** The dialogs open now. */
const dialogs = () => document.querySelectorAll('.kui-dialog').length

/**
 * Installs the guard on this document; `home` answers the element the keyboard goes back to (the
 * workspace). Answers the function that removes it.
 */
export function installFocusGuard(home: () => HTMLElement | null): () => void {
  let frame = 0
  const goHome = () => {
    const element = home()
    if (!element?.isConnected) return false
    element.focus({ preventScroll: true })
    return document.activeElement === element
  }
  // Focus left for nowhere (its control went away): back to the workspace, unless a dialog is open
  // (its own keys) or the window itself lost the focus.
  const lost = () => {
    frame = 0
    if (!document.hasFocus() || !isPage(document.activeElement) || dialogs()) return
    goHome()
  }
  const onFocusOut = (event: FocusEvent) => {
    if (event.relatedTarget == null && !frame) frame = requestAnimationFrame(lost)
  }

  // A key that found the focus on the page's body: the workspace takes the focus and hears the key.
  const onKeyDown = (event: KeyboardEvent) => {
    if (event.isComposing || event.keyCode === 229) return
    if (!isPage(event.target instanceof Element ? event.target : null) || dialogs() || !event.isTrusted) return
    if (!goHome()) return
    event.preventDefault()
    event.stopImmediatePropagation()
    home()?.dispatchEvent(
      new KeyboardEvent('keydown', {
        key: event.key,
        code: event.code,
        location: event.location,
        repeat: event.repeat,
        altKey: event.altKey,
        ctrlKey: event.ctrlKey,
        metaKey: event.metaKey,
        shiftKey: event.shiftKey,
        bubbles: true,
        cancelable: true,
        composed: true,
      }),
    )
  }

  // The last dialog closing: the keyboard goes to the workspace, unless the person is typing somewhere.
  let open = dialogs()
  const observer = new MutationObserver(() => {
    const now = dialogs()
    const closed = open > 0 && now === 0
    open = now
    if (!closed) return
    const active = document.activeElement
    if (active && !isPage(active) && active.matches(TYPING) && !active.closest('[role="menubar"], [role="menu"]')) return
    goHome()
  })
  observer.observe(document.body, { childList: true, subtree: true })

  document.addEventListener('focusout', onFocusOut, true)
  window.addEventListener('keydown', onKeyDown, true)
  return () => {
    if (frame) cancelAnimationFrame(frame)
    observer.disconnect()
    document.removeEventListener('focusout', onFocusOut, true)
    window.removeEventListener('keydown', onKeyDown, true)
  }
}
