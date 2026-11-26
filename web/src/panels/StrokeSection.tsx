import { Checkbox, Choice, PanelSection, Segmented, useEngineQuery, useKitEditor, useReadOnly } from '@kition/sdk/editor'
import { selectedNodes, type StrokeOptions, type VectorInfo } from '../vector'
import { PanelNumber } from './fields'
import { ARROWHEADS, dashFields, withDash } from './model'
import { useLive } from './useLive'

type Profile = { id: string; label: string }

/**
 * Properties › Stroke (plan 2026-11-25, stage 3): weight, cap, corner and its limit, where the stroke
 * sits on the path, dashes and gaps (exact, or fitted to corners and ends), the two arrowheads and
 * where their tips go, and the width profile. With nothing selected only the weight shows (what
 * the next object drawn gets); the rest follow the first selected object.
 */
export function StrokeSection({ info }: { info: VectorInfo }) {
  const { run, phrase: p, t } = useKitEditor()
  const readOnly = useReadOnly()
  const live = useLive()
  const first = selectedNodes(info).find((node) => node.strokeOptions)
  const options: StrokeOptions | null = first?.strokeOptions ?? null
  const profiles = useEngineQuery<{ profiles?: Profile[] }>('stroke.widthProfile.list', {}, info.selection.join(','))?.profiles ?? []
  const set = (params: Record<string, unknown>) => void run('stroke.set', params)
  const weight = (value: number) => ({ weight: Math.round(value * 1000) / 1000 })
  const dash = options?.dash ?? null
  const dashed = dash !== null && dash.length > 0
  const arrows = [['', t('stroke.noArrow')] as const, ...ARROWHEADS.map((name) => [name, t(`arrow.${name}`)] as const)]
  return (
    <PanelSection title={p('Stroke')} testId="vector-stroke-panel">
      <div className="vector-fields">
        <PanelNumber
          label={t('strokeWeight')}
          value={info.paint.strokeWidth}
          unit="pt"
          decimals={3}
          min={0}
          max={1000}
          step={0.5}
          wide
          lengths
          onSet={(value) => set(weight(value))}
          onScrub={(value, final) => (final ? void live.commit('stroke.set', weight(value)) : live.set('stroke.set', weight(value)))}
          testId="vector-stroke-width"
        />
        {options ? (
          <>
            <div className="vector-field" data-wide>
              <span className="vector-field-label">{t('stroke.cap')}</span>
              <Segmented label={t('stroke.cap')} value={options.cap} disabled={readOnly} segments={['butt', 'round', 'square'].map((value) => ({ value, label: t(`stroke.cap.${value}`) }))} onChange={(cap) => set({ cap })} testId="vector-stroke-cap" />
            </div>
            <div className="vector-field" data-wide>
              <span className="vector-field-label">{t('stroke.join')}</span>
              <Segmented label={t('stroke.join')} value={options.join} disabled={readOnly} segments={['miter', 'round', 'bevel'].map((value) => ({ value, label: t(`stroke.join.${value}`) }))} onChange={(join) => set({ join })} testId="vector-stroke-join" />
            </div>
            <PanelNumber label={t('stroke.miterLimit')} value={options.miterLimit} unit="×" decimals={1} min={1} max={500} wide disabled={options.join !== 'miter'} onSet={(miterLimit) => set({ miterLimit })} testId="vector-stroke-miter" />
            <div className="vector-field" data-wide>
              <span className="vector-field-label">{t('stroke.align')}</span>
              <Segmented label={t('stroke.align')} value={options.align} disabled={readOnly} segments={['center', 'inside', 'outside'].map((value) => ({ value, label: t(`stroke.align.${value}`) }))} onChange={(align) => set({ align })} testId="vector-stroke-align" />
            </div>
            <div className="vector-field" data-wide>
              <Checkbox label={t('stroke.dashed')} checked={dashed} disabled={readOnly} onChange={(on) => set({ dash: on ? [12, 12] : null })} testId="vector-stroke-dashed" />
              {dashed ? (
                <Segmented
                  label={t('stroke.dashAlign')}
                  value={options.alignDashes ? 'corners' : 'exact'}
                  disabled={readOnly}
                  segments={[
                    { value: 'exact', label: t('stroke.dashExact') },
                    { value: 'corners', label: t('stroke.dashCorners') },
                  ]}
                  onChange={(value) => set({ alignDashes: value === 'corners' })}
                  testId="vector-stroke-dash-align"
                />
              ) : null}
            </div>
            {dashed ? (
              <div className="vector-dashes" data-wide>
                {dashFields(dash).map((value, index) => (
                  <PanelNumber
                    key={index}
                    label={t(index % 2 === 0 ? 'stroke.dash' : 'stroke.gap')}
                    value={value}
                    decimals={2}
                    min={0}
                    max={1000}
                    lengths
                    placeholder="—"
                    onSet={(next) => set({ dash: withDash(dash, index, next) })}
                    testId={`vector-stroke-dash-${index}`}
                  />
                ))}
              </div>
            ) : null}
            <div className="vector-field" data-wide>
              <Choice label={t('stroke.startArrow')} value={options.startArrow ?? ''} choices={arrows} disabled={readOnly} onChange={(name) => set({ startArrow: name || null })} testId="vector-stroke-start-arrow" />
            </div>
            <div className="vector-field" data-wide>
              <Choice label={t('stroke.endArrow')} value={options.endArrow ?? ''} choices={arrows} disabled={readOnly} onChange={(name) => set({ endArrow: name || null })} testId="vector-stroke-end-arrow" />
            </div>
            {options.startArrow || options.endArrow ? (
              <div className="vector-field" data-wide>
                <span className="vector-field-label">{t('stroke.arrowAlign')}</span>
                <Segmented
                  label={t('stroke.arrowAlign')}
                  value={options.arrowAlign === 'tip' ? 'tip' : 'extend'}
                  disabled={readOnly}
                  segments={[
                    { value: 'extend', label: t('stroke.arrowExtend') },
                    { value: 'tip', label: t('stroke.arrowTip') },
                  ]}
                  onChange={(arrowAlign) => set({ arrowAlign })}
                  testId="vector-stroke-arrow-align"
                />
              </div>
            ) : null}
            {profiles.length ? (
              <div className="vector-field" data-wide>
                <Choice
                  label={t('stroke.profile')}
                  value={profiles.some((profile) => profile.id === options.profile) ? options.profile : 'custom'}
                  choices={[...profiles.map((profile) => [profile.id, profile.label in PROFILE_KEYS ? t(`stroke.profile.${profile.id}`) : profile.label] as const), ...(profiles.some((profile) => profile.id === options.profile) ? [] : [['custom', t('stroke.profileCustom')] as const])]}
                  disabled={readOnly}
                  onChange={(profile) => {
                    if (profile !== 'custom') set({ profile })
                  }}
                  testId="vector-stroke-profile"
                />
              </div>
            ) : null}
          </>
        ) : null}
      </div>
    </PanelSection>
  )
}

/** The built-in profiles the editor's catalogs name (by the engine's English label). */
const PROFILE_KEYS: Readonly<Record<string, true>> = { Uniform: true, Lens: true, 'Taper Start': true, 'Taper End': true, Pinch: true, Teardrop: true, Wave: true }
