import { AlignCenterHorizontalIcon as AlignCenterHorizontal, AlignCenterVerticalIcon as AlignCenterVertical, AlignEndHorizontalIcon as AlignEndHorizontal, AlignEndVerticalIcon as AlignEndVertical, AlignStartHorizontalIcon as AlignStartHorizontal, AlignStartVerticalIcon as AlignStartVertical, CombineIcon as Combine, SquircleIcon as Squircle, SquaresExcludeIcon as SquaresExclude, SquaresIntersectIcon as SquaresIntersect, SquaresSubtractIcon as SquaresSubtract, SquareSplitHorizontalIcon as SquareSplitHorizontal } from '@kition/tessera/icons/react'
import { IconButton, PanelSection, useEngineQuery, useKitEditor, useReadOnly } from '@kition/sdk/editor'
import { LayersSection } from './LayersSection'
import { boundsParams, type BoundsKey } from './canvas/bounds'
import type { Swatch, VectorInfo } from './vector'

const round = (value: number) => Math.round(value * 100) / 100

/** The editor's workspace, where the keyboard goes back to once a field is done. */
export const toWorkspace = (from: Element) => (from.closest('.kui-app') as HTMLElement | null)?.focus({ preventScroll: true })

/**
 * A number field that sets the selection's bounds when it changes (X, Y, W, H). Enter commits it
 * and gives the keyboard back to the canvas; a value refused (W or H not above 0) shows the old one.
 */
function BoundsField({ label, value, onSet }: { label: string; value: number; onSet: (value: number) => boolean }) {
  const readOnly = useReadOnly()
  return (
    <label className="vector-field">
      <span>{label}</span>
      <input
        key={value}
        className="kui-field"
        type="number"
        defaultValue={round(value)}
        disabled={readOnly}
        onKeyDown={(event) => {
          if (event.key === 'Enter') toWorkspace(event.currentTarget)
        }}
        onBlur={(event) => {
          const field = event.currentTarget
          const next = Number(field.value)
          if (field.value.trim() === '' || next === round(value)) return
          if (!onSet(next)) field.value = String(round(value))
        }}
        data-testid={`vector-bounds-${label}`}
      />
    </label>
  )
}

/** The selection's place and size (Properties > Transform). */
function TransformSection({ info }: { info: VectorInfo }) {
  const { run, phrase: p, t } = useKitEditor()
  const bounds = info.selectionBounds
  return (
    <PanelSection title={p('Transform')} testId="vector-transform">
      {bounds ? (
        <div className="vector-fields">
          {(['x', 'y', 'width', 'height'] as const satisfies readonly BoundsKey[]).map((key) => (
            <BoundsField
              key={key}
              label={key === 'width' ? 'W' : key === 'height' ? 'H' : key.toUpperCase()}
              value={bounds[key]}
              onSet={(value) => {
                const params = boundsParams(key, value)
                if (params) void run('object.setBounds', params)
                return params !== null
              }}
            />
          ))}
        </div>
      ) : (
        <p className="vector-empty kui-muted">{t('noSelection')}</p>
      )}
    </PanelSection>
  )
}

/** Fill, stroke, and stroke weight of the selection (or of what comes next). */
function AppearanceSection({ info }: { info: VectorInfo }) {
  const { run, phrase: p, t } = useKitEditor()
  const readOnly = useReadOnly()
  const { fill, stroke, strokeWidth } = info.paint
  return (
    <PanelSection title={p('Appearance')} testId="vector-appearance">
      <div className="vector-fields">
        <label className="vector-field">
          <span>{t('fill')}</span>
          <input type="color" value={fill ?? '#ffffff'} disabled={readOnly} onChange={(event) => void run('paint.setFill', { color: event.target.value })} data-testid="vector-fill" />
        </label>
        <label className="vector-field">
          <span>{t('stroke')}</span>
          <input type="color" value={stroke ?? '#000000'} disabled={readOnly} onChange={(event) => void run('paint.setStroke', { color: event.target.value })} data-testid="vector-stroke" />
        </label>
        <label className="vector-field vector-field-wide">
          <span>{t('strokeWeight')}</span>
          <input
            key={strokeWidth}
            className="kui-field"
            type="number"
            min={0}
            step={0.5}
            defaultValue={strokeWidth}
            disabled={readOnly}
            onBlur={(event) => {
              // The Stroke Weight field: `stroke.set` takes the weight as `weight` (it never read `width`).
              const weight = Number(event.currentTarget.value)
              if (Number.isFinite(weight) && weight !== strokeWidth) void run('stroke.set', { weight })
            }}
            onKeyDown={(event) => {
              if (event.key === 'Enter') toWorkspace(event.currentTarget)
            }}
            data-testid="vector-stroke-width"
          />
        </label>
      </div>
    </PanelSection>
  )
}

function swatchColor(swatch: Swatch): string | null {
  const color = swatch.color as { r?: number; g?: number; b?: number; c?: number; m?: number; y?: number; k?: number; gray?: number } | undefined
  if (!color) return null
  if (color.r != null && color.g != null && color.b != null) return `rgb(${Math.round(color.r * 255)}, ${Math.round(color.g * 255)}, ${Math.round(color.b * 255)})`
  if (color.c != null && color.m != null && color.y != null && color.k != null) {
    const k = 1 - color.k
    return `rgb(${Math.round(255 * (1 - color.c) * k)}, ${Math.round(255 * (1 - color.m) * k)}, ${Math.round(255 * (1 - color.y) * k)})`
  }
  if (color.gray != null) return `rgb(${Math.round(255 * (1 - color.gray))}, ${Math.round(255 * (1 - color.gray))}, ${Math.round(255 * (1 - color.gray))})`
  return null
}

/** The document's swatches; a click paints the fill (or the stroke, when it is the active proxy). */
function SwatchesSection({ info }: { info: VectorInfo }) {
  const { run, phrase: p } = useKitEditor()
  const readOnly = useReadOnly()
  const swatches = useEngineQuery<{ swatches: Swatch[] }>('swatch.list')?.swatches ?? []
  return (
    <PanelSection title={p('Swatches')} defaultOpen={false} testId="vector-swatches">
      <div className="vector-swatches">
        {swatches.map((swatch) => {
          const color = swatchColor(swatch)
          return (
            <button
              key={swatch.name}
              type="button"
              className="kui-swatch"
              title={swatch.name}
              aria-label={swatch.name}
              disabled={readOnly}
              style={{ background: color ?? 'repeating-linear-gradient(45deg, #fff 0 3px, #d33 3px 4px)' }}
              onClick={() => void (swatch.kind === 'none' ? run('paint.none', {}) : run(info.paint.fillActive ? 'paint.setFill' : 'paint.setStroke', { color: swatch.color }))}
            />
          )
        })}
      </div>
    </PanelSection>
  )
}

export const ALIGN = [
  { icon: AlignStartVertical, label: 'Horizontal Align Left', params: { horizontal: 'left' } },
  { icon: AlignCenterVertical, label: 'Horizontal Align Center', params: { horizontal: 'center' } },
  { icon: AlignEndVertical, label: 'Horizontal Align Right', params: { horizontal: 'right' } },
  { icon: AlignStartHorizontal, label: 'Vertical Align Top', params: { vertical: 'top' } },
  { icon: AlignCenterHorizontal, label: 'Vertical Align Center', params: { vertical: 'center' } },
  { icon: AlignEndHorizontal, label: 'Vertical Align Bottom', params: { vertical: 'bottom' } },
]

const PATHFINDER = [
  { icon: Combine, command: 'object.pathfinder.unite' },
  { icon: SquaresSubtract, command: 'object.pathfinder.minusFront' },
  { icon: SquaresIntersect, command: 'object.pathfinder.intersect' },
  { icon: SquaresExclude, command: 'object.pathfinder.exclude' },
  { icon: SquareSplitHorizontal, command: 'object.pathfinder.divide' },
  { icon: Squircle, command: 'object.pathfinder.trim' },
]

/** Align: several objects to their selection (or key object); one object alone to its artboard, as the reference does. */
function AlignSection({ info }: { info: VectorInfo }) {
  const { run, enabled, phrase: p, byId } = useKitEditor()
  const some = info.selection.length > 0
  return (
    <>
      <PanelSection title={p('Align')} testId="vector-align">
        <div className="vector-buttons">
          {ALIGN.map((item) => (
            <IconButton key={item.label} icon={item.icon} label={p(item.label)} disabled={!some || !enabled('object.align')} onClick={() => void run('object.align', info.selection.length === 1 ? { ...item.params, to: 'artboard' } : item.params)} data-testid={`vector-align-${Object.values(item.params)[0]}`} />
          ))}
        </div>
      </PanelSection>
      <PanelSection title={p('Pathfinder')} defaultOpen={false} testId="vector-pathfinder">
        <div className="vector-buttons">
          {PATHFINDER.filter((item) => byId.has(item.command)).map((item) => (
            <IconButton key={item.command} icon={item.icon} label={p(byId.get(item.command)?.title ?? item.command)} disabled={info.selection.length < 2 || !enabled(item.command)} onClick={() => void run(item.command, {})} />
          ))}
        </div>
      </PanelSection>
    </>
  )
}

/** Kition Vector's panels beside the artboard: Properties, Layers, Swatches, Align, Pathfinder. */
export function SidePanel({ info }: { info: VectorInfo }) {
  return (
    <aside className="kui-panel vector-side" data-side="right" data-testid="vector-side">
      <div className="kui-panel-body">
        <TransformSection info={info} />
        <AppearanceSection info={info} />
        <AlignSection info={info} />
        <LayersSection info={info} />
        <SwatchesSection info={info} />
      </div>
    </aside>
  )
}
