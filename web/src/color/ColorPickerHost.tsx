import { createContext, useCallback, useContext, useMemo, useRef, useState, type ReactNode } from 'react'
import { ColorPickerDialog } from './ColorPickerDialog'
import type { Picked } from './paint'

/**
 * The Color Picker as the editor's one way to choose a colour (plan 2026-11-25, stage 3): the Fill
 * and Stroke proxies, the panels' fill and stroke chips, the control bar, gradient stops (in the
 * Gradient panel and from the canvas's gradient annotator), the swatch dialog and the command
 * dialogs' colour rows all open it through `useColorPicker`. One picker is open at a time, over
 * whatever dialog asked for it.
 */
export type PickRequest = {
  /** The dialog's title (the colour being chosen: "Fill", "Stroke", "Gradient Stop"). */
  title: string
  /** The colour it starts from, shown as "current". */
  value: Picked
  /** Whether None is a choice (a fill or a stroke; not a gradient stop). */
  allowNone?: boolean
  /** Each colour as the person moves through them (the canvas shows them at once); the start colour again on Cancel. */
  onLive?: (picked: Picked) => void
}

/** Opens the picker: the chosen colour on OK, null on Cancel. */
export type PickColor = (request: PickRequest) => Promise<Picked | null>

type Open = PickRequest & { serial: number; resolve: (picked: Picked | null) => void }
type Host = { pick: PickColor; open: Open | null; close: (picked: Picked | null) => void }

const HostContext = createContext<Host | null>(null)

/** The picker of the editor around (null outside one). */
export function useColorPicker(): PickColor | null {
  return useContext(HostContext)?.pick ?? null
}

/** Holds the open picker; `ColorPickerOutlet`, inside the editor's frame, draws it. */
export function ColorPickerProvider({ children }: { children: ReactNode }) {
  const [open, setOpen] = useState<Open | null>(null)
  const openNow = useRef<Open | null>(null)
  const serial = useRef(0)
  const pick = useCallback<PickColor>(
    (request) =>
      new Promise((resolve) => {
        // A second request replaces the first, which answers Cancel.
        openNow.current?.resolve(null)
        serial.current += 1
        const next = { ...request, serial: serial.current, resolve }
        openNow.current = next
        setOpen(next)
      }),
    [],
  )
  const close = useCallback((picked: Picked | null) => {
    const current = openNow.current
    openNow.current = null
    setOpen(null)
    current?.resolve(picked)
  }, [])
  const host = useMemo<Host>(() => ({ pick, open, close }), [pick, open, close])
  return <HostContext.Provider value={host}>{children}</HostContext.Provider>
}

/** Draws the open picker where the editor's dialogs are drawn. */
export function ColorPickerOutlet() {
  const host = useContext(HostContext)
  const open = host?.open
  if (!host || !open) return null
  return <ColorPickerDialog key={open.serial} title={open.title} value={open.value} allowNone={open.allowNone === true} onLive={open.onLive} onDone={host.close} />
}
