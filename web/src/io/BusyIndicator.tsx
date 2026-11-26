import { useEffect, useState, useSyncExternalStore } from 'react'
import { Button, type Strings } from '@kition/sdk/editor'
import type { Busy } from './busy'

/**
 * A slow file job's progress (plan 2026-11-25, stage 7): after half a second, a bar at the foot of
 * the canvas with what runs, how long it has run, and Cancel. The engine gives no fraction done,
 * so the bar moves without a percentage. It blocks nothing: panels, menus and the view keep working.
 */
export function BusyIndicator({ busy, t }: { busy: Busy; t: Strings }) {
  const job = useSyncExternalStore(
    (listener) => busy.subscribe(listener),
    () => busy.current,
  )
  const [now, setNow] = useState(() => Date.now())
  useEffect(() => {
    if (!job) return
    const timer = window.setInterval(() => setNow(Date.now()), 250)
    return () => window.clearInterval(timer)
  }, [job])
  if (!job) return null
  const seconds = Math.max(0, Math.floor((now - job.started) / 1000))
  return (
    <div className="vector-busy" role="status" aria-live="polite" data-testid="vector-busy" data-label={job.label}>
      <div className="vector-busy-text">
        <span className="vector-busy-label">{t('busy.working', { label: job.label })}</span>
        <span className="vector-busy-time tabular-nums">{t('busy.seconds', { seconds })}</span>
      </div>
      <div className="vector-busy-bar" role="progressbar" aria-label={job.label} aria-busy="true">
        <span />
      </div>
      {job.cancellable ? (
        <Button size="sm" onClick={() => busy.cancel()} data-testid="vector-busy-cancel">
          {t('cancel')}
        </Button>
      ) : null}
    </div>
  )
}
