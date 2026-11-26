import { useRef, useState, type PointerEvent as ReactPointerEvent } from 'react'
import { ChevronDownIcon as ChevronDown, ChevronUpIcon as ChevronUp, CopyIcon as Copy, EllipsisIcon as Ellipsis, PlusIcon as Plus, SettingsIcon as Settings, TrashIcon as Trash } from '@kition/tessera/icons/react'
import { IconButton, MenuButton, MenuItem, MenuSeparator, PanelFooter, RenameField, useKitEditor, useReadOnly } from '@kition/sdk/editor'
import { useDialogs } from '../dialogs/DialogHost'
import type { VectorInfo } from '../vector'
import { useArtboardNav } from './nav'
import { reorderTarget } from './order'

const ROW = 26

/**
 * The Artboards panel (plan 2026-11-25, stage 4): every artboard by number and name. A click makes
 * it the active artboard, a double-click on its number fits it in the window and on its name
 * renames it, the gear opens Artboard Options; a row dragged up or down (or the arrows) changes the
 * artboards' order; New, Duplicate and Delete below, and the panel menu.
 */
export function ArtboardsPanel({ info }: { info: VectorInfo }) {
  const { run, t, phrase, byId, enabled } = useKitEditor()
  const readOnly = useReadOnly()
  const dialogs = useDialogs()
  const nav = useArtboardNav()
  const [renaming, setRenaming] = useState<{ index: number; text: string } | null>(null)
  const [drop, setDrop] = useState<{ index: number; after: boolean } | null>(null)
  const press = useRef<{ pointer: number; y: number; index: number; moving: boolean } | null>(null)
  const list = useRef<HTMLDivElement>(null)
  const count = info.artboards.length
  const active = Math.min(nav.active, Math.max(0, count - 1))
  const title = (command: string) => phrase(byId.get(command)?.title ?? command)
  const can = (command: string) => !readOnly && enabled(command)

  const finishRename = (commit: boolean) => {
    const done = renaming
    setRenaming(null)
    const board = done ? info.artboards[done.index] : null
    if (commit && done && board && done.text.trim() && done.text.trim() !== board.name) void run('artboard.setProps', { index: done.index, name: done.text.trim() })
  }
  const reorder = (from: number, to: number) => {
    if (to < 0 || to >= count || to === from) return
    void run('artboard.reorder', { index: from, to }).then((done) => {
      if (done !== null) nav.setActive(to)
    })
  }
  const rowAt = (clientY: number) => {
    const box = list.current?.getBoundingClientRect()
    if (!box || !list.current) return null
    const y = clientY - box.top + list.current.scrollTop
    const index = Math.min(count - 1, Math.max(0, Math.floor(y / ROW)))
    return { index, fraction: Math.min(1, Math.max(0, (y - index * ROW) / ROW)) }
  }
  const onPointerDown = (event: ReactPointerEvent<HTMLDivElement>, index: number) => {
    if (event.button !== 0 || readOnly || renaming || (event.target instanceof Element && event.target.closest('button, input'))) return
    press.current = { pointer: event.pointerId, y: event.clientY, index, moving: false }
  }
  const onPointerMove = (event: ReactPointerEvent<HTMLDivElement>) => {
    const current = press.current
    if (!current || current.pointer !== event.pointerId) return
    if (!current.moving && Math.abs(event.clientY - current.y) < 4) return
    if (!current.moving) {
      current.moving = true
      event.currentTarget.setPointerCapture?.(event.pointerId)
    }
    const at = rowAt(event.clientY)
    setDrop(at ? { index: at.index, after: at.fraction >= 0.5 } : null)
  }
  const onPointerUp = (event: ReactPointerEvent<HTMLDivElement>) => {
    const current = press.current
    press.current = null
    setDrop(null)
    if (!current || current.pointer !== event.pointerId || !current.moving) return
    const at = rowAt(event.clientY)
    if (at) reorder(current.index, reorderTarget(current.index, at.index, at.fraction))
  }

  return (
    <div className="vector-artboards" data-testid="vector-artboards-panel">
      <div className="vector-artboards-list" ref={list} role="listbox" aria-label={phrase('Artboards')} data-testid="vector-artboards-list">
        {info.artboards.map((board, index) => (
          <div
            key={index}
            className="vector-artboard-row"
            role="option"
            aria-selected={index === active}
            data-selected={index === active}
            data-drop={drop?.index === index ? (drop.after ? 'below' : 'above') : undefined}
            style={{ height: ROW }}
            onPointerDown={(event) => onPointerDown(event, index)}
            onPointerMove={onPointerMove}
            onPointerUp={onPointerUp}
            onPointerCancel={() => {
              press.current = null
              setDrop(null)
            }}
            onClick={() => nav.setActive(index)}
            data-testid={`vector-artboard-row-${index}`}
          >
            <span className="vector-artboard-number" title={phrase('Fit Artboard in Window')} onDoubleClick={() => nav.show(index)} data-testid={`vector-artboard-number-${index}`}>
              {index + 1}
            </span>
            {renaming?.index === index ? (
              <span className="vector-layer-name">
                <RenameField value={renaming.text} label={t('layers.rename')} onChange={(text) => setRenaming({ index, text })} onDone={finishRename} testId={`vector-artboard-rename-${index}`} />
              </span>
            ) : (
              <span
                className="vector-layer-name"
                title={board.name}
                onDoubleClick={(event) => {
                  event.stopPropagation()
                  if (!readOnly) setRenaming({ index, text: board.name })
                }}
                data-testid={`vector-artboard-name-${index}`}
              >
                {board.name}
              </span>
            )}
            <IconButton icon={Settings} label={phrase('Artboard Options')} disabled={!can('artboard.setProps')} tabIndex={-1} onClick={() => dialogs.artboardOptions(index)} data-testid={`vector-artboard-options-${index}`} />
          </div>
        ))}
      </div>
      <PanelFooter
        start={
          <>
            <IconButton icon={ChevronUp} label={t('artboards.moveUp')} disabled={readOnly || active === 0} onClick={() => reorder(active, active - 1)} data-testid="vector-artboard-up" />
            <IconButton icon={ChevronDown} label={t('artboards.moveDown')} disabled={readOnly || active >= count - 1} onClick={() => reorder(active, active + 1)} data-testid="vector-artboard-down" />
          </>
        }
        end={
          <>
            <IconButton
              icon={Plus}
              label={title('artboard.new')}
              disabled={!can('artboard.new')}
              onClick={() => void run('artboard.new', {}).then((done) => done !== null && nav.setActive(count))}
              data-testid="vector-artboard-new"
            />
            <IconButton
              icon={Copy}
              label={title('artboard.duplicate')}
              disabled={!can('artboard.duplicate')}
              onClick={() => void run('artboard.duplicate', { index: active }).then((done) => done !== null && nav.setActive(count))}
              data-testid="vector-artboard-duplicate"
            />
            <IconButton
              icon={Trash}
              label={title('artboard.delete')}
              disabled={!can('artboard.delete') || count < 2}
              onClick={() => void run('artboard.delete', { index: active }).then((done) => done !== null && nav.setActive(Math.max(0, active - 1)))}
              data-testid="vector-artboard-delete"
            />
            <MenuButton align="end" trigger={({ toggle }) => <IconButton icon={Ellipsis} label={t('artboards.menu')} onClick={toggle} data-testid="vector-artboards-menu" />}>
              <MenuItem label={title('artboard.new')} disabled={!can('artboard.new')} onSelect={() => void run('artboard.new', {})} testId="vector-artboards-menu-new" />
              <MenuItem label={title('artboard.duplicate')} disabled={!can('artboard.duplicate')} onSelect={() => void run('artboard.duplicate', { index: active })} testId="vector-artboards-menu-duplicate" />
              <MenuItem label={title('artboard.delete')} disabled={!can('artboard.delete') || count < 2} onSelect={() => void run('artboard.delete', { index: active })} testId="vector-artboards-menu-delete" />
              <MenuItem label={t('layers.rename')} disabled={readOnly} onSelect={() => setRenaming({ index: active, text: info.artboards[active]?.name ?? '' })} testId="vector-artboards-menu-rename" />
              <MenuSeparator />
              <MenuItem label={title('artboard.convertToArtboards')} disabled={!can('artboard.convertToArtboards')} onSelect={() => void run('artboard.convertToArtboards', {})} testId="vector-artboards-menu-convert" />
              <MenuItem label={title('artboard.rearrange')} disabled={!can('artboard.rearrange')} onSelect={() => dialogs.invoke('artboard.rearrange')} testId="vector-artboards-menu-rearrange" />
              <MenuItem label={title('artboard.fitToArt')} disabled={!can('artboard.fitToArt')} onSelect={() => void run('artboard.fitToArt', { index: active })} testId="vector-artboards-menu-fit-art" />
              <MenuItem label={title('artboard.fitToSelection')} disabled={!can('artboard.fitToSelection')} onSelect={() => void run('artboard.fitToSelection', { index: active })} testId="vector-artboards-menu-fit-selection" />
              <MenuSeparator />
              <MenuItem label={phrase('Artboard Options')} disabled={!can('artboard.setProps')} onSelect={() => dialogs.artboardOptions(active)} testId="vector-artboards-menu-options" />
            </MenuButton>
          </>
        }
      />
    </div>
  )
}
