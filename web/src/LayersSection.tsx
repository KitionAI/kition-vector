import { EyeIcon as Eye, EyeOffIcon as EyeOff, LockIcon as Lock, PlusIcon as Plus } from '@kition/tessera-icons/react'
import { IconButton, PanelSection, useKitEditor, useReadOnly } from '@kition/kit-editor'
import type { VectorInfo, VectorNode } from './vector'

function Row({ node, depth, selected }: { node: VectorNode; depth: number; selected: ReadonlySet<number> }) {
  const { run, t } = useKitEditor()
  const readOnly = useReadOnly()
  const layer = node.kind === 'Layer'
  return (
    <>
      <div className="vector-row" data-selected={selected.has(node.id)} style={{ paddingLeft: 6 + depth * 12 }} data-testid={`vector-layer-${node.id}`}>
        {layer ? (
          <IconButton
            icon={node.visible ? Eye : EyeOff}
            label={node.visible ? t('hideLayer') : t('showLayer')}
            pressed={!node.visible}
            disabled={readOnly}
            onClick={() => void run('layer.setProps', { id: node.id, visible: !node.visible })}
            data-testid={`vector-layer-visibility-${node.id}`}
          />
        ) : (
          <span className="vector-row-indent" />
        )}
        {layer && node.color ? <span className="vector-layer-color" style={{ background: node.color }} aria-hidden="true" /> : null}
        <button type="button" className="vector-row-name" onClick={() => void run(layer ? 'layer.selectAll' : 'select.set', layer ? { id: node.id } : { ids: [node.id] })}>
          {node.name}
        </button>
        {node.locked ? <Lock size={12} className="kui-muted" aria-label={t('locked')} /> : null}
      </div>
      {node.children.map((child) => (
        <Row key={child.id} node={child} depth={depth + 1} selected={selected} />
      ))}
    </>
  )
}

/** The drawing's layers and their objects; a click selects, the eye shows or hides a layer. */
export function LayersSection({ info }: { info: VectorInfo }) {
  const { phrase: p, run, t } = useKitEditor()
  const readOnly = useReadOnly()
  const selected = new Set(info.selection)
  return (
    <PanelSection title={p('Layers')} actions={<IconButton icon={Plus} label={t('newLayer')} disabled={readOnly} onClick={() => void run('layer.new', {})} data-testid="vector-new-layer" />} testId="vector-layers-panel">
      {info.layers.map((layer) => (
        <Row key={layer.id} node={layer} depth={0} selected={selected} />
      ))}
    </PanelSection>
  )
}
