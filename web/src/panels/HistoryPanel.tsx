import { useEffect, useRef, useState } from 'react'
import { Redo2Icon as Redo, Undo2Icon as Undo } from '@kition/tessera/icons/react'
import { IconButton, PanelFooter, toast, useKitEditor, useReadOnly } from '@kition/sdk/editor'
import type { VectorInfo } from '../vector'
import { historyMove, historyRows } from './history'
import { historyHeld } from './historyHold'

/**
 * The History panel (plan 2026-11-25, stage 4): the drawing as opened, every step done and the
 * steps undone after them (dimmed). A click on a row goes back or forward to it by undoing or
 * redoing step by step (the engine has no jump); Undo and Redo below. While a live gesture or a
 * dialog's preview is under way it waits, and says so: those take their own steps away with Undo.
 */
export function HistoryPanel({ info }: { info: VectorInfo }) {
  const { run, t, phrase, enabled } = useKitEditor()
  const readOnly = useReadOnly()
  const [busy, setBusy] = useState(false)
  const rows = historyRows(info.history, info.redo)
  const current = info.history.length
  const list = useRef<HTMLDivElement>(null)
  // The current row stays in view as steps are added.
  useEffect(() => {
    const row = list.current?.querySelector<HTMLElement>('[data-current="true"]')
    row?.scrollIntoView?.({ block: 'nearest' })
  }, [current, info.redo.length])

  const go = async (target: number) => {
    if (readOnly || busy) return
    if (historyHeld()) {
      toast('info', t('history.wait'))
      return
    }
    const move = historyMove(current, info.redo.length, target)
    if (!move) return
    setBusy(true)
    try {
      for (let step = 0; step < move.times; step += 1) {
        if (historyHeld()) break
        if ((await run(move.command, {})) === null) break
      }
    } finally {
      setBusy(false)
    }
  }
  return (
    <div className="vector-history" data-testid="vector-history-panel">
      <div className="vector-history-list" ref={list} role="listbox" aria-label={phrase('History')} aria-busy={busy}>
        {rows.map((row) => (
          <button
            type="button"
            key={row.position}
            role="option"
            aria-selected={row.position === current}
            className="vector-history-row"
            data-current={row.position === current}
            data-undone={row.undone || undefined}
            disabled={readOnly}
            onClick={() => void go(row.position)}
            data-testid={`vector-history-${row.position}`}
          >
            {row.name === null ? t('history.opened') : phrase(row.name)}
          </button>
        ))}
      </div>
      <PanelFooter
        start={<span className="vector-layers-count">{t('history.count', { count: current })}</span>}
        end={
          <>
            <IconButton icon={Undo} label={phrase('Undo')} disabled={readOnly || busy || !enabled('edit.undo')} onClick={() => void go(current - 1)} data-testid="vector-history-undo" />
            <IconButton icon={Redo} label={phrase('Redo')} disabled={readOnly || busy || !enabled('edit.redo')} onClick={() => void go(current + 1)} data-testid="vector-history-redo" />
          </>
        }
      />
    </div>
  )
}
