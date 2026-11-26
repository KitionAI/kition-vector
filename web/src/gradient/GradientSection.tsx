import { useRef, useState, type PointerEvent as ReactPointerEvent } from 'react'
import { ArrowLeftRightIcon as ArrowLeftRight, TrashIcon as Trash } from '@kition/tessera/icons/react'
import { Button, IconButton, PanelSection, Segmented, useKitEditor, useReadOnly } from '@kition/sdk/editor'
import { useColorPicker } from '../color/ColorPickerHost'
import { colorCmyk, colorParam, type Proxies } from '../color/paint'
import { PaintChip, PanelNumber } from '../panels/fields'
import { useProxies } from '../panels/usePaint'
import { useLive } from '../panels/useLive'
import type { VectorInfo } from '../vector'
import { addStop, engineStops, fromPaint, midpointPlaces, moveStop, rampCss, removeStop, reversed, setMidpoint, setStop, type GradientDraft, type GradientKind } from './gradient'

/** How far (px) a stop dragged off the bar is taken away on release, as the reference's. */
const DRAG_OFF = 28

/** The gradient behind the proxy in front, as `paint.proxies` answers it. */
export function activeGradient(proxies: Proxies | null) {
  const stroke = proxies?.fillActive === false
  return { stroke, draft: fromPaint(stroke ? proxies?.stroke : proxies?.fill) }
}

/**
 * Properties › Gradient (plan 2026-11-25, stage 3): the gradient behind the Fill or Stroke proxy in
 * front, of the selection (or of what new art gets). Its type, angle, a ramp with its colour stops
 * (a press under the ramp adds one there, a stop drags along it and off it to go, a diamond moves
 * the midpoint between two), the selected stop's colour (the Color Picker), opacity and location,
 * and Reverse. Every edit is `paint.editGradient`, which keeps the gradient where it is on the
 * objects; a drag or a colour chosen in the picker shows live and leaves one history step. Setting a
 * stop's colour never replaces the gradient with a flat colour.
 */
export function GradientSection({ info }: { info: VectorInfo }) {
  const { run, phrase: p, t } = useKitEditor()
  const readOnly = useReadOnly()
  const proxies = useProxies(info)
  const pick = useColorPicker()
  const live = useLive()
  const { stroke, draft: engineDraft } = activeGradient(proxies)
  const [dragging, setDragging] = useState<GradientDraft | null>(null)
  const [selected, setSelected] = useState(0)
  const bar = useRef<HTMLDivElement>(null)
  const draft = dragging ?? engineDraft
  const index = draft ? Math.min(selected, draft.stops.length - 1) : 0
  const stop = draft?.stops[index]

  const edit = (params: Record<string, unknown>) => void run('paint.editGradient', { stroke, ...params })
  const stopsOf = (next: GradientDraft) => ({ stroke, stops: engineStops(next) })
  const fraction = (clientX: number) => {
    const box = bar.current?.getBoundingClientRect()
    return box ? Math.min(1, Math.max(0, (clientX - box.left) / Math.max(1, box.width))) : 0
  }

  /** A press on the stop row: on a stop, select and drag it; elsewhere add one there and drag it. */
  const pressRow = (event: ReactPointerEvent<HTMLDivElement>) => {
    if (event.button !== 0 || readOnly || !draft) return
    event.preventDefault()
    const target = (event.target as HTMLElement).closest<HTMLElement>('[data-stop]')
    let working = draft
    let at: number
    if (target) at = Number(target.dataset.stop)
    else {
      const added = addStop(working, fraction(event.clientX))
      working = added.draft
      at = added.index
      setDragging(working)
      live.set('paint.editGradient', stopsOf(working))
    }
    setSelected(at)
    const row = event.currentTarget
    row.setPointerCapture?.(event.pointerId)
    const startY = event.clientY
    const startX = event.clientX
    let moved = !target
    const move = (next: PointerEvent) => {
      if (!moved && Math.abs(next.clientX - startX) < 2 && Math.abs(next.clientY - startY) < 2) return
      moved = true
      row.dataset.removing = Math.abs(next.clientY - startY) > DRAG_OFF && working.stops.length > 2 ? 'true' : 'false'
      const shifted = moveStop(working, at, fraction(next.clientX))
      working = shifted.draft
      at = shifted.index
      setSelected(at)
      setDragging(working)
      live.set('paint.editGradient', stopsOf(working))
    }
    const up = (released: PointerEvent) => {
      row.removeEventListener('pointermove', move)
      row.removeEventListener('pointerup', up)
      row.removeEventListener('pointercancel', up)
      delete row.dataset.removing
      if (moved && Math.abs(released.clientY - startY) > DRAG_OFF) {
        const fewer = removeStop(working, at)
        if (fewer) {
          working = fewer
          setSelected(0)
        }
      }
      if (moved) void live.commit('paint.editGradient', stopsOf(working)).finally(() => setDragging(null))
      else setDragging(null)
    }
    row.addEventListener('pointermove', move)
    row.addEventListener('pointerup', up)
    row.addEventListener('pointercancel', up)
  }
  const pressMidpoint = (segment: number) => (event: ReactPointerEvent<HTMLButtonElement>) => {
    if (event.button !== 0 || readOnly || !draft) return
    event.preventDefault()
    event.stopPropagation()
    const handle = event.currentTarget
    handle.setPointerCapture?.(event.pointerId)
    let working = draft
    const move = (moved: PointerEvent) => {
      const a = working.stops[segment]?.offset ?? 0
      const b = working.stops[segment + 1]?.offset ?? 1
      working = setMidpoint(working, segment, b - a <= 0 ? 0.5 : (fraction(moved.clientX) - a) / (b - a))
      setDragging(working)
      live.set('paint.editGradient', stopsOf(working))
    }
    const up = () => {
      handle.removeEventListener('pointermove', move)
      handle.removeEventListener('pointerup', up)
      handle.removeEventListener('pointercancel', up)
      if (working !== draft) void live.commit('paint.editGradient', stopsOf(working)).finally(() => setDragging(null))
    }
    handle.addEventListener('pointermove', move)
    handle.addEventListener('pointerup', up)
    handle.addEventListener('pointercancel', up)
  }
  const pickStop = () => {
    if (!pick || !draft || !stop) return
    const at = index
    const start = draft
    const cmyk = colorCmyk(typeof stop.color === 'string' ? null : stop.color)
    void pick({
      title: t('gradient.stopColor'),
      value: { kind: 'color', hex: stop.hex, ...(cmyk ? { cmyk } : {}) },
      onLive: (picked) => {
        if (picked.kind !== 'color') return
        live.set('paint.editGradient', stopsOf(setStop(start, at, { color: colorParam(picked), hex: picked.hex })))
      },
    }).then((picked) => {
      if (picked?.kind === 'color') void live.commit('paint.editGradient', stopsOf(setStop(start, at, { color: colorParam(picked), hex: picked.hex })))
      else void live.cancel()
    })
  }

  const kinds: { value: GradientKind; label: string }[] = [
    { value: 'linear', label: t('gradient.linear') },
    { value: 'radial', label: t('gradient.radial') },
    { value: 'freeform', label: t('gradient.freeform') },
  ]
  return (
    <PanelSection title={p('Gradient')} testId="vector-gradient">
      {draft ? (
        <div className="vector-gradient" data-kind={draft.kind}>
          <div className="vector-field" data-wide>
            <span className="vector-field-label">{t('gradient.type')}</span>
            <Segmented label={t('gradient.type')} value={draft.kind} segments={kinds} disabled={readOnly} onChange={(kind) => edit({ kind })} testId="vector-gradient-kind" />
          </div>
          <div className="vector-fields">
            {draft.kind === 'linear' ? (
              <PanelNumber
                label="∠"
                title={t('dialog.angle')}
                value={Math.round(draft.angle * 100) / 100}
                unit="°"
                decimals={2}
                onSet={(angle) => edit({ angle })}
                onScrub={(angle, final) => (final ? void live.commit('paint.editGradient', { stroke, angle }) : live.set('paint.editGradient', { stroke, angle }))}
                testId="vector-gradient-angle"
              />
            ) : (
              <span />
            )}
            <IconButton icon={ArrowLeftRight} label={t('gradient.reverse')} disabled={readOnly} onClick={() => edit({ stops: engineStops(reversed(draft)) })} data-testid="vector-gradient-reverse" />
          </div>
          <div className="vector-gradient-ramp">
            <div ref={bar} className="vector-gradient-bar" style={{ backgroundImage: `${rampCss(draft)}, var(--vector-checker)` }} data-testid="vector-gradient-bar" />
            <div className="vector-gradient-row" onPointerDown={pressRow} title={t('gradient.addStop')} data-testid="vector-gradient-stops">
              {draft.stops.map((item, at) => (
                <span key={at} className="vector-gradient-stop" data-stop={at} data-selected={at === index} style={{ left: `${item.offset * 100}%`, background: item.hex }} data-color={item.hex} data-testid={`vector-gradient-stop-${at}`} />
              ))}
              {midpointPlaces(draft).map((place, segment) => (
                <button key={`mid-${segment}`} type="button" className="vector-gradient-midpoint" style={{ left: `${place * 100}%` }} aria-label={t('gradient.midpoint')} title={t('gradient.midpoint')} onPointerDown={pressMidpoint(segment)} data-testid={`vector-gradient-midpoint-${segment}`} />
              ))}
            </div>
          </div>
          {stop ? (
            <div className="vector-fields">
              <div className="vector-field">
                <span className="vector-field-label">{t('gradient.stopColor')}</span>
                <PaintChip view={{ kind: 'color', hex: stop.hex }} label={t('gradient.stopColor')} onOpen={pickStop} disabled={readOnly} testId="vector-gradient-stop-color" />
              </div>
              <IconButton icon={Trash} label={t('gradient.deleteStop')} disabled={readOnly || draft.stops.length <= 2} onClick={() => {
                const fewer = removeStop(draft, index)
                if (fewer) {
                  setSelected(0)
                  edit({ stops: engineStops(fewer) })
                }
              }} data-testid="vector-gradient-delete-stop" />
              <PanelNumber label={t('gradient.opacity')} value={Math.round(stop.opacity * 100)} unit="%" decimals={0} min={0} max={100} onSet={(value) => edit({ stops: engineStops(setStop(draft, index, { opacity: value / 100 })) })} testId="vector-gradient-stop-opacity" />
              <PanelNumber
                label={t('gradient.location')}
                value={Math.round(stop.offset * 1000) / 10}
                unit="%"
                decimals={1}
                min={0}
                max={100}
                onSet={(value) => {
                  const shifted = moveStop(draft, index, value / 100)
                  setSelected(shifted.index)
                  edit({ stops: engineStops(shifted.draft) })
                }}
                testId="vector-gradient-stop-location"
              />
            </div>
          ) : null}
        </div>
      ) : (
        <div className="vector-gradient-none">
          <p className="vector-empty kui-muted">{t(stroke ? 'gradient.noneStroke' : 'gradient.noneFill')}</p>
          <Button variant="secondary" disabled={readOnly} onClick={() => edit({})} data-testid="vector-gradient-apply">
            {t('gradient.apply')}
          </Button>
        </div>
      )}
    </PanelSection>
  )
}
