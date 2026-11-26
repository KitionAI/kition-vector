/**
 * Slow file jobs (plan 2026-11-25, stage 7): an export, an open, a place or a paste still running
 * after `delay` (half a second) shows the progress bar with Cancel; a quicker one never flashes it.
 * The engine gives no fraction done and runs a command to its end in its worker (the web build has
 * no way in to stop it), so Cancel keeps the editor's promise instead: the indicator closes at
 * once, an export cancelled before it is written writes nothing, and a place or paste that ends
 * after Cancel is undone. Pure, with the clock passed in, so `node --test` checks it.
 */

export type BusyJob = {
  readonly id: number
  /** What runs, as the person reads it (the command's title, "Export"). */
  readonly label: string
  readonly started: number
  /** Cancel was pressed: the job's result is to be dropped when it ends. */
  cancelled: boolean
  /** Whether Cancel is offered (a step that can be taken away, an export not written yet). */
  readonly cancellable: boolean
}

export type BusyClock = {
  now: () => number
  schedule: (task: () => void, ms: number) => unknown
  unschedule: (handle: unknown) => void
}

const systemClock: BusyClock = {
  now: () => Date.now(),
  schedule: (task, ms) => setTimeout(task, ms),
  unschedule: (handle) => clearTimeout(handle as ReturnType<typeof setTimeout>),
}

/** How long a command runs before the indicator shows. */
export const BUSY_DELAY_MS = 500

export class Busy {
  private jobs: BusyJob[] = []
  private shown: BusyJob | null = null
  private timers = new Map<number, unknown>()
  private listeners = new Set<() => void>()
  private serial = 0
  private readonly clock: BusyClock
  private readonly delay: number

  constructor(clock: BusyClock = systemClock, delay = BUSY_DELAY_MS) {
    this.clock = clock
    this.delay = delay
  }

  /** The job the indicator shows (the oldest one running past the delay and not cancelled), or null. */
  get current(): BusyJob | null {
    return this.shown
  }

  subscribe(listener: () => void): () => void {
    this.listeners.add(listener)
    return () => this.listeners.delete(listener)
  }

  /** A job starts; `end` it whatever happens. */
  begin(label: string, cancellable = true): BusyJob {
    this.serial += 1
    const job: BusyJob = { id: this.serial, label, started: this.clock.now(), cancelled: false, cancellable }
    this.jobs.push(job)
    this.timers.set(
      job.id,
      this.clock.schedule(() => {
        this.timers.delete(job.id)
        this.update()
      }, this.delay),
    )
    return job
  }

  /** The job ended; answers whether it was cancelled (its result is to be dropped). */
  end(job: BusyJob): boolean {
    const timer = this.timers.get(job.id)
    if (timer !== undefined) this.clock.unschedule(timer)
    this.timers.delete(job.id)
    this.jobs = this.jobs.filter((item) => item !== job)
    this.update()
    return job.cancelled
  }

  /** Cancel on the indicator: the job it shows is dropped when it ends; the indicator moves on. */
  cancel(): BusyJob | null {
    const job = this.shown
    if (!job || !job.cancellable) return null
    job.cancelled = true
    this.update()
    return job
  }

  private update() {
    const next = this.jobs.find((job) => !job.cancelled && !this.timers.has(job.id)) ?? null
    if (next === this.shown) return
    this.shown = next
    for (const listener of [...this.listeners]) listener()
  }
}
