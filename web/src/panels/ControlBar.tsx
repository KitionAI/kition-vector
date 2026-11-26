import { IconButton, useKitEditor, useReadOnly } from '@kition/sdk/editor'
import { paintView } from '../color/paint'
import type { VectorInfo } from '../vector'
import { ALIGN } from './AlignSection'
import { alignTo } from './model'
import { PaintChip, PanelNumber } from './fields'
import { usePaintPicker, useProxies, useTransparency } from './usePaint'
import { useLive } from './useLive'

/**
 * The control bar (plan 2026-11-25, stage 3), in the frame's tool-options row: the fill and stroke
 * (each chip opens the Color Picker), the stroke weight, and with objects selected their opacity
 * and the six Align buttons (one object aligns to its artboard). With nothing selected the fill,
 * stroke and weight are what the next object drawn gets.
 */
export function ControlBar({ info }: { info: VectorInfo }) {
  const { run, phrase: p, t } = useKitEditor()
  const readOnly = useReadOnly()
  const proxies = useProxies(info)
  const transparency = useTransparency(info)
  const picker = usePaintPicker()
  const live = useLive()
  const count = info.selection.length
  const weight = (value: number) => ({ weight: Math.round(value * 1000) / 1000 })
  const opacity = (value: number) => ({ opacity: Math.round(value * 100) / 100 })
  return (
    <div className="vector-control-bar" data-testid="vector-control-bar">
      <span className="vector-control-what">{count ? t('selectedCount', { count }) : t('control.newArt')}</span>
      <span className="vector-control-item">
        <span className="vector-field-label">{t('fill')}</span>
        <PaintChip view={paintView(proxies?.fill, proxies?.fillMixed === true)} label={t('fill')} onOpen={() => picker('fill', proxies?.fill)} disabled={readOnly} testId="vector-control-fill" />
      </span>
      <span className="vector-control-item">
        <span className="vector-field-label">{t('stroke')}</span>
        <PaintChip view={paintView(proxies?.stroke, proxies?.strokeMixed === true)} label={t('stroke')} onOpen={() => picker('stroke', proxies?.stroke)} disabled={readOnly} stroke testId="vector-control-stroke" />
      </span>
      <PanelNumber
        label={t('control.weight')}
        title={t('strokeWeight')}
        value={info.paint.strokeWidth}
        unit="pt"
        decimals={3}
        min={0}
        max={1000}
        step={0.5}
        lengths
        onSet={(value) => void run('stroke.set', weight(value))}
        onScrub={(value, final) => (final ? void live.commit('stroke.set', weight(value)) : live.set('stroke.set', weight(value)))}
        testId="vector-control-weight"
      />
      {count ? (
        <>
          <PanelNumber
            label={t('appearance.opacity')}
            value={transparency?.opacity ?? null}
            unit="%"
            decimals={0}
            min={0}
            max={100}
            onSet={(value) => void run('transparency.set', opacity(value))}
            onScrub={(value, final) => (final ? void live.commit('transparency.set', opacity(value)) : live.set('transparency.set', opacity(value)))}
            testId="vector-control-opacity"
          />
          <span className="vector-control-align">
            {ALIGN.map((item) => (
              <IconButton key={item.label} icon={item.icon} label={p(item.label)} disabled={readOnly} onClick={() => void run('object.align', { ...item.params, ...alignTo('selection', count) })} data-testid={`vector-control-align-${Object.values(item.params)[0]}`} />
            ))}
          </span>
        </>
      ) : null}
    </div>
  )
}
