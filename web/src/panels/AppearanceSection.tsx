import { Choice, PanelSection, useKitEditor, useReadOnly } from '@kition/sdk/editor'
import { paintView } from '../color/paint'
import type { VectorInfo } from '../vector'
import { PaintField, PanelNumber } from './fields'
import { BLEND_MODES } from './model'
import { usePaintPicker, useProxies, useTransparency } from './usePaint'
import { useLive } from './useLive'

/**
 * Properties › Appearance (plan 2026-11-25, stage 3): the fill and the stroke as they are (a colour,
 * a gradient's ramp, None, a pattern, or mixed), each chip opening the Color Picker and each hex
 * code typed; with objects selected, their opacity and blending mode. With nothing selected the fill
 * and stroke are what the next object drawn gets.
 */
export function AppearanceSection({ info }: { info: VectorInfo }) {
  const { run, phrase: p, t } = useKitEditor()
  const readOnly = useReadOnly()
  const proxies = useProxies(info)
  const transparency = useTransparency(info)
  const picker = usePaintPicker()
  const live = useLive()
  const some = info.selection.length > 0
  const fill = paintView(proxies?.fill, proxies?.fillMixed === true)
  const stroke = paintView(proxies?.stroke, proxies?.strokeMixed === true)
  const opacity = (value: number) => ({ opacity: Math.round(value * 100) / 100 })
  return (
    <PanelSection title={p('Appearance')} testId="vector-appearance">
      <div className="vector-fields">
        <PaintField label={t('fill')} view={fill} onPick={() => picker('fill', proxies?.fill)} onHex={(color) => void run('paint.setFill', { color })} testId="vector-fill" />
        <PaintField label={t('stroke')} view={stroke} onPick={() => picker('stroke', proxies?.stroke)} onHex={(color) => void run('paint.setStroke', { color })} testId="vector-stroke" />
        {some ? (
          <>
            <PanelNumber
              label={t('appearance.opacity')}
              value={transparency?.opacity ?? null}
              unit="%"
              decimals={0}
              min={0}
              max={100}
              wide
              onSet={(value) => void run('transparency.set', opacity(value))}
              onScrub={(value, final) => (final ? void live.commit('transparency.set', opacity(value)) : live.set('transparency.set', opacity(value)))}
              testId="vector-opacity"
            />
            <div className="vector-field" data-wide>
              <Choice
                label={t('appearance.blend')}
                value={transparency ? (transparency.blend ?? '') : 'Normal'}
                choices={[...(transparency && transparency.blend === null ? [['', t('paint.mixed')] as const] : []), ...BLEND_MODES.map((mode) => [mode, p(mode)] as const)]}
                disabled={readOnly}
                onChange={(blend) => {
                  if (blend) void run('transparency.set', { blend })
                }}
                testId="vector-blend"
              />
            </div>
          </>
        ) : null}
      </div>
    </PanelSection>
  )
}
