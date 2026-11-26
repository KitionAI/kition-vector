import { useRef, useState } from 'react'
import { AlignCenterHorizontalIcon as AlignCenterHorizontal, AlignCenterVerticalIcon as AlignCenterVertical, AlignEndHorizontalIcon as AlignEndHorizontal, AlignEndVerticalIcon as AlignEndVertical, AlignStartHorizontalIcon as AlignStartHorizontal, AlignStartVerticalIcon as AlignStartVertical } from '@kition/tessera/icons/react'
import { Choice, IconButton, PanelSection, toast, useKitEditor, useReadOnly } from '@kition/sdk/editor'
import { selectedNodes, type VectorInfo } from '../vector'
import { PanelNumber } from './fields'
import { alignTo, type AlignTo } from './model'
import { DistributeBottom, DistributeLeft, DistributeMiddleH, DistributeMiddleV, DistributeRight, DistributeTop, SpaceHorizontal, SpaceVertical } from './icons'

export const ALIGN = [
  { icon: AlignStartVertical, label: 'Horizontal Align Left', params: { horizontal: 'left' } },
  { icon: AlignCenterVertical, label: 'Horizontal Align Center', params: { horizontal: 'center' } },
  { icon: AlignEndVertical, label: 'Horizontal Align Right', params: { horizontal: 'right' } },
  { icon: AlignStartHorizontal, label: 'Vertical Align Top', params: { vertical: 'top' } },
  { icon: AlignCenterHorizontal, label: 'Vertical Align Center', params: { vertical: 'center' } },
  { icon: AlignEndHorizontal, label: 'Vertical Align Bottom', params: { vertical: 'bottom' } },
]

const DISTRIBUTE = [
  { icon: DistributeTop, label: 'Vertical Distribute Top', params: { vertical: 'top' }, id: 'top' },
  { icon: DistributeMiddleV, label: 'Vertical Distribute Center', params: { vertical: 'center' }, id: 'vcenter' },
  { icon: DistributeBottom, label: 'Vertical Distribute Bottom', params: { vertical: 'bottom' }, id: 'bottom' },
  { icon: DistributeLeft, label: 'Horizontal Distribute Left', params: { horizontal: 'left' }, id: 'left' },
  { icon: DistributeMiddleH, label: 'Horizontal Distribute Center', params: { horizontal: 'center' }, id: 'hcenter' },
  { icon: DistributeRight, label: 'Horizontal Distribute Right', params: { horizontal: 'right' }, id: 'right' },
]

/** Where the selected objects are, as one string to compare. */
const placesOf = (info: VectorInfo) => JSON.stringify(selectedNodes(info).map((node) => (node.bounds ? [node.bounds.x, node.bounds.y, node.bounds.width, node.bounds.height].map((value) => Math.round(value * 100) / 100) : null)))

/**
 * Properties › Align (plan 2026-11-25, stage 3): align (six), distribute objects (six), distribute
 * spacing across or down (by the gap typed, from the key object, or evenly when empty), and what
 * they align to: the selection, its key object (a click on one of the selected objects with the
 * Selection tool makes it the key) or the artboard. A button that moved nothing says so.
 */
export function AlignSection({ info }: { info: VectorInfo }) {
  const { run, enabled, phrase: p, t } = useKitEditor()
  const readOnly = useReadOnly()
  const [to, setTo] = useState<AlignTo>('selection')
  const [spacing, setSpacing] = useState<number | null>(null)
  const count = info.selection.length
  const infoNow = useRef(info)
  infoNow.current = info
  /** Runs `command`, and says so when no selected object moved (the engine keeps a step either way). */
  const act = (command: string, params: Record<string, unknown>) => {
    const before = placesOf(infoNow.current)
    void run(command, params).then(async (done) => {
      if (done === null) return
      // The panel's state follows the run a moment later.
      for (let tries = 0; tries < 20 && placesOf(infoNow.current) === before; tries += 1) await new Promise((resolve) => window.setTimeout(resolve, 25))
      if (placesOf(infoNow.current) === before) toast('info', t('align.nothingMoved'))
    })
  }
  const align = (params: Record<string, unknown>) => act('object.align', { ...params, ...alignTo(to, count) })
  const off = readOnly || count === 0
  return (
    <PanelSection title={p('Align')} testId="vector-align">
      <div className="vector-align">
        <span className="vector-align-caption">{t('align.objects')}</span>
        <div className="vector-buttons">
          {ALIGN.map((item) => (
            <IconButton key={item.label} icon={item.icon} label={p(item.label)} disabled={off || !enabled('object.align')} onClick={() => align(item.params)} data-testid={`vector-align-${Object.values(item.params)[0]}`} />
          ))}
        </div>
        <span className="vector-align-caption">{t('align.distribute')}</span>
        <div className="vector-buttons">
          {DISTRIBUTE.map((item) => (
            <IconButton key={item.id} icon={item.icon} label={t(`align.distribute.${item.id}`)} disabled={readOnly || count < 3 || !enabled('object.distribute')} onClick={() => act('object.distribute', item.params)} data-testid={`vector-distribute-${item.id}`} />
          ))}
        </div>
        <span className="vector-align-caption">{t('align.spacing')}</span>
        <div className="vector-buttons vector-align-spacing">
          <IconButton icon={SpaceVertical} label={t('align.spacing.vertical')} disabled={readOnly || count < 2 || !enabled('object.distributeSpacing')} onClick={() => act('object.distributeSpacing', { axis: 'vertical', ...(spacing !== null ? { spacing } : {}) })} data-testid="vector-spacing-vertical" />
          <IconButton icon={SpaceHorizontal} label={t('align.spacing.horizontal')} disabled={readOnly || count < 2 || !enabled('object.distributeSpacing')} onClick={() => act('object.distributeSpacing', { axis: 'horizontal', ...(spacing !== null ? { spacing } : {}) })} data-testid="vector-spacing-horizontal" />
          <PanelNumber label={t('align.gap')} title={t('align.gapHint')} value={spacing} unit="pt" decimals={2} min={0} lengths placeholder={t('dialog.auto')} onSet={(value) => setSpacing(value)} onClear={() => setSpacing(null)} testId="vector-spacing-value" />
        </div>
        <div className="vector-field" data-wide>
          <Choice
            label={t('align.to')}
            value={to}
            choices={[
              ['selection', t('align.to.selection')],
              ['key', t('align.to.key')],
              ['artboard', t('align.to.artboard')],
            ]}
            onChange={setTo}
            testId="vector-align-to"
          />
        </div>
      </div>
    </PanelSection>
  )
}
