import type { OverlayDrawer } from '@kition/sdk/editor'

/**
 * The text caret's blink (plan 2026-11-25, stage 5). The Type tool draws its caret as a solid black
 * line; while text is edited with a caret (no characters selected) the canvas redraws its overlays
 * every frame (`animated`), and this drawer leaves the caret out half of each second. Typing or
 * moving the caret shows it again at once (`restartBlink`), as text fields do.
 */

/** How long the caret shows, then hides, in milliseconds. */
export const BLINK_MS = 530

const blink = { on: false, since: 0 }

/** Whether the caret blinks now (a caret, no selection, no composition): set by the editor. */
export function setBlinking(on: boolean) {
  if (on && !blink.on) blink.since = performance.now()
  blink.on = on
}

/** The caret moved or text was typed: it shows for a whole period again. */
export function restartBlink() {
  blink.since = performance.now()
}

/** Whether the caret is in the hidden half of its blink at `now`. */
export function caretHidden(now: number, since: number) {
  return Math.floor(Math.max(0, now - since) / BLINK_MS) % 2 === 1
}

/** Leaves the caret (a solid black line) out while it blinks off; `next` draws everything else. */
export function blinkingCaret(next: OverlayDrawer): OverlayDrawer {
  return (overlay, draw) => {
    if (blink.on && overlay.kind === 'line' && !overlay.dashed && overlay.color[0] === 0 && overlay.color[1] === 0 && overlay.color[2] === 0 && caretHidden(performance.now(), blink.since)) return true
    return next(overlay, draw)
  }
}
