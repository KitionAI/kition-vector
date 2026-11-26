import { useEffect, useLayoutEffect, useMemo, useRef, useState, type MouseEvent as ReactMouseEvent, type PointerEvent as ReactPointerEvent } from 'react'
import {
  ChevronDownIcon as ChevronDown,
  ChevronRightIcon as ChevronRight,
  CrosshairIcon as Crosshair,
  EllipsisIcon as Ellipsis,
  LockIcon as Lock,
  LockOpenIcon as LockOpen,
  PlusIcon as Plus,
  SquareDashedIcon as SquareDashed,
  SquarePlusIcon as SquarePlus,
  TrashIcon as Trash,
} from '@kition/tessera/icons/react'
import { ContextMenu, EyeButton, IconButton, MenuButton, MenuItem, MenuSeparator, PanelFooter, RenameField, useKitEditor, useReadOnly } from '@kition/sdk/editor'
import { useDialogs } from '../dialogs/DialogHost'
import type { VectorInfo, VectorNode } from '../vector'
import { ancestorsOf, canDrop, dropPlace, findNode, objectsAmong, openByDefault, rowRange, rowWindow, selectionMarks, visibleRows, type Row } from './tree'
import { layerOptionsDialog } from './options'

/** A row's height in CSS pixels (the list draws only the rows in view). */
export const ROW_HEIGHT = 26

type Drop = { target: number; place: 'above' | 'inside' | 'below' }

/** The panel menu's rows: engine commands, in the reference's order; `---` separates. */
const PANEL_MENU = [
  'layer.new',
  'layer.newSublayer',
  'layer.duplicate',
  'layer.delete',
  'layer.setProps',
  '---',
  'layer.clippingMask.toggle',
  'layer.locate',
  'layer.collectInNew',
  'layer.releaseToLayers',
  'layer.releaseToLayersBuild',
  'layer.reverse',
  '---',
  'layer.template',
  'layer.merge',
  'layer.flatten',
  '---',
  'layer.pasteRemembersLayers',
] as const

/** A row's right-click menu: what acts on it and the rows around it (short enough for a small window). */
const ROW_MENU = [
  'layer.setProps',
  'layer.duplicate',
  'layer.delete',
  '---',
  'layer.new',
  'layer.newSublayer',
  'layer.collectInNew',
  '---',
  'layer.hideOthers',
  'layer.lockOthers',
  'layer.outlineOthers',
  'layer.showAll',
  'layer.unlockAll',
  'layer.previewAll',
] as const

/**
 * The Layers panel (plan 2026-11-25, stage 4): the drawing's tree (layer → sublayer → group →
 * object) with an eye, a lock and a disclosure triangle on every row, the layer's colour, the
 * name (double-click renames it), the target circle and the selection square in the layer's
 * colour. A click highlights a row (a layer becomes the current layer; an object is selected too),
 * Shift a range and Cmd one more; a row dragged lands above, below or inside another (one undo
 * step, Alt copies); right-click opens the panel's menu on it. Only the rows in view are drawn,
 * and a layer of hundreds of objects opens closed.
 */
export function LayersPanel({ info }: { info: VectorInfo }) {
  const editor = useKitEditor()
  const { run, t, phrase, byId, enabled } = editor
  const readOnly = useReadOnly()
  const dialogs = useDialogs()
  const [open, setOpen] = useState<ReadonlyMap<number, boolean>>(() => new Map())
  const [renaming, setRenaming] = useState<{ id: number; text: string } | null>(null)
  const [menuAt, setMenuAt] = useState<{ x: number; y: number } | null>(null)
  const [drop, setDrop] = useState<Drop | null>(null)
  const [dragging, setDragging] = useState<number[] | null>(null)
  const anchor = useRef<number | null>(null)
  const list = useRef<HTMLDivElement>(null)
  const [view, setView] = useState({ top: 0, height: 400 })

  // A row the person has not opened or closed keeps the state it had when it first showed: a layer
  // that grows past hundreds of objects while open stays open.
  const firstSeen = useRef(new Map<number, boolean>())
  const rows = useMemo(() => {
    const opened = new Map(firstSeen.current)
    const remember = (nodes: readonly VectorNode[]) => {
      for (const node of nodes) {
        if (!opened.has(node.id) && node.children.length) {
          opened.set(node.id, openByDefault(node))
          firstSeen.current.set(node.id, openByDefault(node))
        }
        remember(node.children)
      }
    }
    remember(info.layers)
    for (const [id, state] of open) opened.set(id, state)
    return visibleRows(info.layers, opened)
  }, [info.layers, open])
  const marks = useMemo(() => selectionMarks(info.layers, info.selection), [info.layers, info.selection])
  const highlighted = useMemo(() => new Set(info.layerRows), [info.layerRows])
  const { start, end } = rowWindow(rows.length, view.top, view.height, ROW_HEIGHT)

  // The list's scroll and height decide which rows are drawn.
  useLayoutEffect(() => {
    const element = list.current
    if (!element) return undefined
    const measure = () => setView({ top: element.scrollTop, height: element.clientHeight })
    measure()
    const observer = typeof ResizeObserver === 'undefined' ? null : new ResizeObserver(measure)
    observer?.observe(element)
    return () => observer?.disconnect()
  }, [])

  // Locate Object (the engine highlights the row): the layers around it open, and it scrolls into view.
  const located = useRef<number | null>(null)
  useEffect(() => {
    const id = located.current
    if (id === null) return
    const index = rows.findIndex((row) => row.node.id === id)
    if (index < 0) return
    located.current = null
    const element = list.current
    if (element && (index * ROW_HEIGHT < element.scrollTop || (index + 1) * ROW_HEIGHT > element.scrollTop + element.clientHeight)) element.scrollTop = Math.max(0, index * ROW_HEIGHT - element.clientHeight / 2)
  }, [rows])
  const reveal = (id: number) => {
    const above = ancestorsOf(info.layers, id) ?? []
    located.current = id
    setOpen((current) => {
      const next = new Map(current)
      for (const ancestor of above) next.set(ancestor, true)
      return next
    })
  }

  const toggleOpen = (row: Row, deep: boolean) => {
    const to = !row.expanded
    setOpen((current) => {
      const next = new Map(current)
      next.set(row.node.id, to)
      if (deep) {
        const walk = (node: VectorNode) => {
          for (const child of node.children) {
            if (child.children.length) next.set(child.id, to)
            walk(child)
          }
        }
        walk(row.node as VectorNode)
      }
      return next
    })
  }

  /** A click on a row: highlight it (and select it on the canvas when it is an object), Shift a range, Cmd one more. */
  const choose = async (row: Row, event: { shiftKey: boolean; metaKey: boolean; ctrlKey: boolean }) => {
    const id = row.node.id
    const toggle = event.metaKey || event.ctrlKey
    if (event.shiftKey) {
      const range = rowRange(rows, anchor.current, id)
      await run('layer.highlight', { ids: range, mode: 'set' })
      const objects = objectsAmong(info.layers, range)
      if (objects.length) await run('select.set', { ids: objects })
      return
    }
    anchor.current = id
    if (toggle) {
      await run('layer.highlight', { ids: [id], mode: 'toggle' })
      if (row.node.kind !== 'Layer') {
        const selected = info.selection.includes(id) ? info.selection.filter((other) => other !== id) : [...info.selection, id]
        await run(selected.length ? 'select.set' : 'edit.deselectAll', selected.length ? { ids: selected } : {})
      }
      return
    }
    if (row.node.kind !== 'Layer') await run('select.set', { ids: [id] })
    await run('layer.setCurrent', { id })
  }

  // Dragging a row: past a few pixels it moves; dropped on a row it lands above, below or inside it.
  const press = useRef<{ pointer: number; y: number; row: Row; moving: number[] | null } | null>(null)
  const dropAt = (clientY: number, moving: readonly number[]): Drop | null => {
    const element = list.current
    if (!element) return null
    const box = element.getBoundingClientRect()
    const y = clientY - box.top + element.scrollTop
    const index = Math.floor(y / ROW_HEIGHT)
    const row = rows[Math.min(rows.length - 1, Math.max(0, index))]
    if (!row) return null
    const fraction = index >= rows.length ? 1 : index < 0 ? 0 : (y - index * ROW_HEIGHT) / ROW_HEIGHT
    const place = dropPlace(row.node, fraction)
    return canDrop(info.layers, moving, row.node.id, place) ? { target: row.node.id, place } : null
  }
  const justDragged = useRef(false)
  const onPointerDown = (event: ReactPointerEvent<HTMLDivElement>, row: Row) => {
    justDragged.current = false
    // The row's own controls (eye, lock, triangle, target, selection square) are not handles.
    if (event.button !== 0 || readOnly || renaming || (event.target instanceof Element && event.target.closest('button, input'))) return
    press.current = { pointer: event.pointerId, y: event.clientY, row, moving: null }
  }
  const onPointerMove = (event: ReactPointerEvent<HTMLDivElement>) => {
    const current = press.current
    if (!current || current.pointer !== event.pointerId) return
    if (!current.moving) {
      if (Math.abs(event.clientY - current.y) < 4) return
      const id = current.row.node.id
      // The highlighted rows move together when the one dragged is among them.
      current.moving = highlighted.has(id) && highlighted.size > 1 ? [...highlighted] : [id]
      event.currentTarget.setPointerCapture?.(event.pointerId)
      setDragging(current.moving)
    }
    setDrop(dropAt(event.clientY, current.moving))
  }
  const onPointerUp = (event: ReactPointerEvent<HTMLDivElement>) => {
    const current = press.current
    if (!current || current.pointer !== event.pointerId) return
    press.current = null
    if (!current.moving) return
    justDragged.current = true
    const landing = dropAt(event.clientY, current.moving)
    setDrop(null)
    setDragging(null)
    if (landing) {
      void run('layer.move', { ids: current.moving, target: landing.target, place: landing.place, ...(event.altKey ? { copy: true } : {}) }).then((done) => {
        if (done !== null && landing.place === 'inside') reveal(landing.target)
      })
    }
  }
  const onPointerCancel = () => {
    press.current = null
    setDrop(null)
    setDragging(null)
  }
  // A click right after a drag is the drag's end, not a click.
  const wasDrag = () => {
    const dragged = justDragged.current
    justDragged.current = false
    return dragged
  }

  // The eye column: a press shows or hides a row, a sweep down the column gives the others the same.
  const sweepTo = useRef(true)
  const setVisible = (node: VectorNode, visible: boolean) => {
    if (node.visible !== visible) void run('layer.setProps', { id: node.id, visible })
  }

  const rename = (node: VectorNode) => {
    if (!readOnly) setRenaming({ id: node.id, text: node.name })
  }
  const finishRename = (commit: boolean) => {
    const done = renaming
    setRenaming(null)
    const node = done ? findNode(info.layers, done.id) : null
    if (commit && done && node && done.text.trim() && done.text !== node.name) void run('layer.setProps', { id: done.id, name: done.text.trim() })
  }

  /** Layer Options… (a layer) or the object's options (name, show, lock) for the highlighted row. */
  const options = (id: number | null = info.layerRows.at(-1) ?? info.currentLayer) => {
    const node = id === null ? null : findNode(info.layers, id)
    if (!node || readOnly) return
    dialogs.ask(layerOptionsDialog(node as VectorNode, phrase(node.kind === 'Layer' ? 'Layer Options' : 'Options'), 'layer.options'))
  }

  const runRow = (command: string) => {
    switch (command) {
      case 'layer.setProps':
        return options()
      case 'layer.new':
      case 'layer.newSublayer':
        return void run(command, {}).then((done) => {
          const made = (done as { id?: unknown } | null)?.id
          if (typeof made === 'number') reveal(made)
        })
      case 'layer.locate':
        return void run(command, {}).then((done) => {
          const id = (done as { id?: unknown } | null)?.id
          if (typeof id === 'number') reveal(id)
        })
      default:
        return void run(command, {})
    }
  }
  const optionsLabel = () => {
    const id = info.layerRows.at(-1) ?? info.currentLayer
    const node = id === null ? null : findNode(info.layers, id)
    return node && node.kind !== 'Layer' ? `${phrase('Options')}…` : phrase(byId.get('layer.setProps')?.title ?? 'Layer Options…')
  }
  const menuRows = (rowsOf: readonly string[], testPrefix: string) =>
    rowsOf.map((command, index) =>
      command === '---' ? (
        <MenuSeparator key={`sep-${index}`} />
      ) : (
        <MenuItem
          key={command}
          label={command === 'layer.setProps' ? optionsLabel() : phrase(byId.get(command)?.title ?? command)}
          disabled={readOnly || !enabled(command)}
          checked={command === 'layer.pasteRemembersLayers' ? info.pasteRemembersLayers : command === 'layer.template' ? isTemplate(info) : undefined}
          onSelect={() => runRow(command)}
          testId={`${testPrefix}-${command}`}
        />
      ),
    )

  const layerCount = info.layers.length
  const selectedRow = info.layerRows.at(-1) ?? null
  return (
    <div className="vector-layers" data-testid="vector-layers-panel">
      <div
        className="vector-layers-list"
        ref={list}
        role="tree"
        aria-label={phrase('Layers')}
        aria-multiselectable="true"
        onScroll={(event) => setView({ top: event.currentTarget.scrollTop, height: event.currentTarget.clientHeight })}
        data-dragging={dragging ? 'true' : undefined}
        data-testid="vector-layers-list"
      >
        <div style={{ height: rows.length * ROW_HEIGHT, position: 'relative' }}>
          {rows.slice(start, end).map((row, offset) => {
            const { node } = row
            const index = start + offset
            const mark = marks.get(node.id)
            const isLayer = node.kind === 'Layer'
            const chosen = highlighted.has(node.id) || mark === 'self'
            const targeted = info.target === node.id
            return (
              <div
                key={node.id}
                className="vector-layer-row"
                role="treeitem"
                aria-level={row.depth + 1}
                aria-selected={chosen}
                aria-expanded={row.expandable ? row.expanded : undefined}
                data-selected={chosen}
                data-current={isLayer && info.currentLayer === node.id ? 'true' : undefined}
                data-drop={drop?.target === node.id ? drop.place : undefined}
                data-hidden={!node.visible || row.hiddenAbove ? 'true' : undefined}
                style={{ top: index * ROW_HEIGHT, height: ROW_HEIGHT }}
                onPointerDown={(event) => onPointerDown(event, row)}
                onPointerMove={onPointerMove}
                onPointerUp={onPointerUp}
                onPointerCancel={onPointerCancel}
                onClick={(event) => {
                  if (!wasDrag()) void choose(row, event)
                }}
                onContextMenu={(event: ReactMouseEvent) => {
                  event.preventDefault()
                  if (!chosen) void choose(row, { shiftKey: false, metaKey: false, ctrlKey: false })
                  setMenuAt({ x: event.clientX, y: event.clientY })
                }}
                data-testid={`vector-layer-${node.id}`}
              >
                <span className="vector-layer-eye" data-above={row.hiddenAbove ? 'true' : undefined} onClick={(event) => event.stopPropagation()}>
                  <EyeButton
                    visible={node.visible}
                    label={node.visible ? t('hideLayer') : t('showLayer')}
                    onPress={(alt) => {
                      if (readOnly) return
                      if (alt && isLayer) return void run('layer.hideOthers', { ids: [node.id] })
                      sweepTo.current = !node.visible
                      setVisible(node as VectorNode, sweepTo.current)
                    }}
                    sweep={{
                      id: String(node.id),
                      group: 'vector-layers',
                      onOver: (other) => {
                        const target = findNode(info.layers, Number(other))
                        if (target && !readOnly) setVisible(target as VectorNode, sweepTo.current)
                      },
                    }}
                    tabIndex={-1}
                    testId={`vector-layer-visibility-${node.id}`}
                  />
                </span>
                <span className="vector-layer-lock" data-above={row.lockedAbove ? 'true' : undefined} onClick={(event) => event.stopPropagation()}>
                  <IconButton
                    icon={node.locked ? Lock : LockOpen}
                    label={node.locked ? t('layers.unlock') : t('layers.lock')}
                    pressed={node.locked}
                    disabled={readOnly}
                    tabIndex={-1}
                    onClick={(event) => {
                      if (event.altKey && isLayer) void run('layer.lockOthers', { ids: [node.id] })
                      else void run('layer.setProps', { id: node.id, locked: !node.locked })
                    }}
                    data-testid={`vector-layer-lock-${node.id}`}
                  />
                </span>
                <span className="vector-layer-color" style={{ background: row.color ?? 'transparent' }} aria-hidden="true" />
                <span className="vector-layer-indent" style={{ width: row.depth * 12 }} />
                {row.expandable ? (
                  <IconButton
                    icon={row.expanded ? ChevronDown : ChevronRight}
                    label={row.expanded ? t('layers.collapse') : t('layers.expand')}
                    tabIndex={-1}
                    onClick={(event) => {
                      event.stopPropagation()
                      toggleOpen(row, event.altKey)
                    }}
                    data-testid={`vector-layer-toggle-${node.id}`}
                  />
                ) : (
                  <span className="vector-layer-twisty" />
                )}
                {renaming?.id === node.id ? (
                  <span className="vector-layer-name">
                    <RenameField value={renaming.text} label={t('layers.rename')} onChange={(text) => setRenaming({ id: node.id, text })} onDone={finishRename} testId={`vector-layer-rename-${node.id}`} />
                  </span>
                ) : (
                  <span
                    className="vector-layer-name"
                    data-layer={isLayer ? 'true' : undefined}
                    onDoubleClick={(event) => {
                      event.stopPropagation()
                      rename(node as VectorNode)
                    }}
                    title={node.name}
                    data-testid={`vector-layer-name-${node.id}`}
                  >
                    {node.name}
                  </span>
                )}
                <button
                  type="button"
                  className="vector-layer-target"
                  data-targeted={targeted || mark === 'self' ? 'true' : undefined}
                  aria-label={t('layers.target')}
                  title={t('layers.target')}
                  tabIndex={-1}
                  disabled={readOnly}
                  onClick={(event) => {
                    event.stopPropagation()
                    void run('layer.target', { id: node.id, ...(event.shiftKey ? { add: true } : {}) })
                  }}
                  data-testid={`vector-layer-target-${node.id}`}
                />
                <button
                  type="button"
                  className="vector-layer-select"
                  data-mark={mark}
                  style={mark ? { background: row.color ?? 'var(--kit-primary)' } : undefined}
                  aria-label={t('layers.selectArt')}
                  title={t('layers.selectArt')}
                  tabIndex={-1}
                  onClick={(event) => {
                    event.stopPropagation()
                    void run('layer.selectAll', { id: node.id, ...(event.shiftKey ? { add: true } : {}) })
                  }}
                  data-testid={`vector-layer-select-${node.id}`}
                />
              </div>
            )
          })}
        </div>
      </div>
      <PanelFooter
        start={
          <span className="vector-layers-count" data-testid="vector-layers-count">
            {t(layerCount === 1 ? 'layers.countOne' : 'layers.count', { count: layerCount })}
          </span>
        }
        end={
          <>
            <IconButton icon={Crosshair} label={phrase('Locate Object')} disabled={readOnly || !enabled('layer.locate')} onClick={() => runRow('layer.locate')} data-testid="vector-layer-locate" />
            <IconButton icon={SquareDashed} label={phrase(byId.get('layer.clippingMask.toggle')?.title ?? 'Make/Release Clipping Mask')} disabled={readOnly || !enabled('layer.clippingMask.toggle')} onClick={() => runRow('layer.clippingMask.toggle')} data-testid="vector-layer-clip" />
            <IconButton icon={SquarePlus} label={t('layers.newSublayer')} disabled={readOnly || !enabled('layer.newSublayer')} onClick={() => runRow('layer.newSublayer')} data-testid="vector-new-sublayer" />
            <IconButton icon={Plus} label={t('newLayer')} disabled={readOnly || !enabled('layer.new')} onClick={() => runRow('layer.new')} data-testid="vector-new-layer" />
            <IconButton icon={Trash} label={t('layers.delete')} disabled={readOnly || !enabled('layer.delete') || (selectedRow === null && layerCount < 2)} onClick={() => runRow('layer.delete')} data-testid="vector-delete-layer" />
            <MenuButton align="end" trigger={({ toggle }) => <IconButton icon={Ellipsis} label={t('layers.menu')} onClick={toggle} data-testid="vector-layers-menu" />}>
              {menuRows(PANEL_MENU, 'vector-layers-menu')}
            </MenuButton>
          </>
        }
      />
      <ContextMenu at={menuAt} onClose={() => setMenuAt(null)} label={phrase('Layers')} testId="vector-layers-context">
        <MenuItem label={t('layers.rename')} disabled={readOnly || selectedRow === null} onSelect={() => {
          const node = selectedRow === null ? null : findNode(info.layers, selectedRow)
          if (node) rename(node as VectorNode)
        }} testId="vector-layers-context-rename" />
        <MenuSeparator />
        {menuRows(ROW_MENU, 'vector-layers-context')}
      </ContextMenu>
    </div>
  )
}

/** Whether the highlighted rows' layer (else the current layer) is a template: Template's check mark. */
function isTemplate(info: VectorInfo) {
  const id = info.layerRows.at(-1) ?? info.currentLayer
  const node = id === null ? null : findNode(info.layers, id)
  return node?.kind === 'Layer' ? (node as VectorNode).template : false
}
