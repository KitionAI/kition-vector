import { useState, type MouseEvent as ReactMouseEvent } from 'react'
import { FolderPlusIcon as FolderPlus, PencilIcon as Pencil, PlusIcon as Plus, TrashIcon as Trash } from '@kition/tessera/icons/react'
import { IconButton, PanelSection, useEngineQuery, useKitEditor, useReadOnly } from '@kition/sdk/editor'
import { useDialogs } from '../dialogs/DialogHost'
import { paintHex } from '../color/paint'
import type { VectorInfo } from '../vector'
import { PaintChip } from './fields'
import { swatchParams, swatchView, type Swatch, type SwatchList } from './model'
import { useProxies } from './usePaint'

/** The built-in swatches nobody edits or deletes. */
const BUILT_IN = new Set(['[None]', '[Registration]'])

/**
 * The Swatches panel (plan 2026-11-25, stage 3): every swatch drawn as what it is, the colour groups
 * under their names, a global swatch marked by its corner. A click paints the selection's fill, or
 * its stroke when the Stroke proxy is in front, and picks the swatch; Shift or Cmd adds swatches
 * to the picked ones. New Swatch saves the colour in front (asking its name, colour and whether it
 * is global), Swatch Options edits the picked one, New Color Group groups the picked colours, Delete
 * deletes them.
 */
export function SwatchesSection({ info }: { info: VectorInfo }) {
  const { run, phrase: p, t, enabled } = useKitEditor()
  const readOnly = useReadOnly()
  const dialogs = useDialogs()
  const list = useEngineQuery<SwatchList>('swatch.list', {}, info.selection.join(','))
  const proxies = useProxies(info)
  const [picked, setPicked] = useState<string[]>([])
  const swatches = list?.swatches ?? []
  const named = new Map(swatches.map((swatch) => [swatch.name, swatch]))
  const live = picked.filter((name) => named.has(name))
  const stroke = info.paint.fillActive === false

  const apply = (swatch: Swatch) => void run(stroke ? 'paint.setStroke' : 'paint.setFill', swatchParams(swatch))
  const click = (swatch: Swatch, event: ReactMouseEvent) => {
    if (event.shiftKey || event.metaKey || event.ctrlKey) {
      setPicked((now) => (now.includes(swatch.name) ? now.filter((name) => name !== swatch.name) : [...now, swatch.name]))
      return
    }
    setPicked([swatch.name])
    if (!readOnly) apply(swatch)
  }
  const current = paintHex(stroke ? proxies?.stroke : proxies?.fill) || '#000000'
  const newSwatch = () =>
    dialogs.ask({
      command: 'swatch.new',
      title: t('swatch.newTitle'),
      // Its own memory each time: a new swatch starts from the colour in front, not the last one's name.
      memory: `swatch.new:${Date.now()}`,
      fields: [
        { key: 'name', kind: 'text', initial: '' },
        { key: 'color', kind: 'color', initial: current },
        { key: 'global', kind: 'check', initial: false },
      ],
      start: { name: '', color: current },
      build: (values) => ({ ...(typeof values.name === 'string' && values.name.trim() ? { name: values.name.trim() } : {}), color: values.color, global: values.global === true }),
      previewing: false,
    })
  const editSwatch = (swatch: Swatch) => {
    const solid = swatch.kind === 'color'
    const view = swatchView(swatch)
    const hex = view.kind === 'color' ? view.hex : '#000000'
    dialogs.ask({
      command: 'swatch.edit',
      title: t('swatch.optionsTitle'),
      memory: `swatch.edit:${swatch.name}`,
      fields: [{ key: 'newName', kind: 'text', initial: swatch.name }, ...(solid && !swatch.tintOf ? [{ key: 'color', kind: 'color' as const, initial: hex }, { key: 'global', kind: 'check' as const, initial: swatch.global === true }] : [])],
      // The swatch as it is now, over what the dialog last OK'd.
      start: { newName: swatch.name, ...(solid && !swatch.tintOf ? { color: hex, global: swatch.global === true } : {}) },
      build: (values) => {
        const out: Record<string, unknown> = { name: swatch.name }
        if (typeof values.newName === 'string' && values.newName.trim() && values.newName.trim() !== swatch.name) out.newName = values.newName.trim()
        if (solid && !swatch.tintOf) {
          out.color = values.color
          out.global = values.global === true
        }
        return out
      },
      previewing: false,
    })
  }
  const newGroup = () => {
    const solid = live.filter((name) => named.get(name)?.kind === 'color' && !BUILT_IN.has(name))
    dialogs.ask({
      command: 'swatch.newGroup',
      title: t('swatch.groupTitle'),
      memory: `swatch.newGroup:${Date.now()}`,
      fields: [{ key: 'name', kind: 'text', initial: '' }],
      start: { name: '' },
      build: (values) => ({ ...(typeof values.name === 'string' && values.name.trim() ? { name: values.name.trim() } : {}), swatches: solid }),
      previewing: false,
    })
  }
  const deletable = live.filter((name) => !BUILT_IN.has(name))
  const one = live.length === 1 ? named.get(live[0]) : undefined

  const chip = (swatch: Swatch) => (
    <span key={swatch.name} className="vector-swatch" data-global={swatch.global || undefined} data-spot={swatch.spot || undefined} data-picked={live.includes(swatch.name) || undefined}>
      <PaintChip
        view={swatchView(swatch)}
        label={p(swatch.name)}
        title={swatch.global ? `${p(swatch.name)} · ${t('swatch.global')}` : p(swatch.name)}
        pressed={live.includes(swatch.name)}
        onClick={(event) => click(swatch, event)}
        onOpen={() => {
          if (!readOnly && !BUILT_IN.has(swatch.name)) editSwatch(swatch)
        }}
        testId={`vector-swatch-${swatch.name}`}
      />
    </span>
  )
  const ungrouped = swatches.filter((swatch) => !swatch.group)
  return (
    <PanelSection
      title={p('Swatches')}
      testId="vector-swatches"
      actions={
        <span className="vector-swatch-actions">
          <IconButton icon={Plus} label={t('swatch.new')} disabled={readOnly || !enabled('swatch.new')} onClick={newSwatch} data-testid="vector-swatch-new" />
          <IconButton icon={FolderPlus} label={t('swatch.newGroup')} disabled={readOnly || !enabled('swatch.newGroup')} onClick={newGroup} data-testid="vector-swatch-new-group" />
          <IconButton icon={Pencil} label={t('swatch.options')} disabled={readOnly || !one || BUILT_IN.has(one.name)} onClick={() => one && editSwatch(one)} data-testid="vector-swatch-edit" />
          <IconButton
            icon={Trash}
            label={t('swatch.delete')}
            disabled={readOnly || deletable.length === 0}
            onClick={() => {
              void run('swatch.delete', { names: deletable }).then((done) => {
                if (done !== null) setPicked([])
              })
            }}
            data-testid="vector-swatch-delete"
          />
        </span>
      }
    >
      <div className="vector-swatches">{ungrouped.map(chip)}</div>
      {(list?.groups ?? []).map((group) => (
        <div key={group.name} className="vector-swatch-group" data-testid={`vector-swatch-group-${group.name}`}>
          <span className="vector-swatch-group-name">{p(group.name)}</span>
          <div className="vector-swatches">{swatches.filter((swatch) => swatch.group === group.name).map(chip)}</div>
        </div>
      ))}
    </PanelSection>
  )
}
