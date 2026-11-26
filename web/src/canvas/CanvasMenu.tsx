import { ArrowLeftIcon as ArrowLeft } from '@kition/tessera/icons/react'
import { ContextMenu, IconButton, MenuItem, MenuSeparator, useKitEditor, type ContextMenuPoint } from '@kition/sdk/editor'
import type { ContextRow } from './contextRows'

/**
 * The canvas's right-click menu (plan 2026-11-25, stage 2) at `at`: `rows` as `contextRows` gave
 * them, each label in the editor's language, a command the engine refuses now greyed. A choice
 * goes to `onChoose`; a press outside, Escape or a choice closes it.
 */
export function CanvasMenu({ at, rows, onChoose, onClose }: { at: ContextMenuPoint | null; rows: readonly ContextRow[]; onChoose: (row: Extract<ContextRow, { kind: 'item' }>) => void; onClose: () => void }) {
  const { enabled, phrase, t } = useKitEditor()
  return (
    <ContextMenu at={at} onClose={onClose} label={t('canvasMenu')} testId="vector-canvas-menu">
      {rows.map((row, index) =>
        row.kind === 'separator' ? (
          <MenuSeparator key={`sep-${index}`} />
        ) : (
          <MenuItem key={`${row.command}-${index}`} label={phrase(row.label)} disabled={!enabled(row.command)} onSelect={() => onChoose(row)} testId={`vector-canvas-menu-${row.command.replace(/[:.]/g, '-')}`} />
        ),
      )}
    </ContextMenu>
  )
}

/**
 * Isolation mode's bar over the canvas (plan 2026-11-25, stage 2): the path to the isolated group
 * (its layer, then each group), and a button back out; Escape and a double-click on the
 * pasteboard leave it too.
 */
export function IsolationBar({ path, onExit }: { path: readonly string[]; onExit: () => void }) {
  const { t } = useKitEditor()
  return (
    <div className="vector-isolation" data-testid="vector-isolation" role="navigation" aria-label={t('isolation.bar')}>
      <IconButton icon={ArrowLeft} label={t('isolation.exit')} onClick={onExit} data-testid="vector-isolation-exit" />
      <span className="vector-isolation-path">
        {path.map((name, index) => (
          <span key={`${name}-${index}`} className="vector-isolation-step" data-current={index === path.length - 1 || undefined}>
            {index ? <span className="vector-isolation-sep" aria-hidden="true">›</span> : null}
            {name}
          </span>
        ))}
      </span>
    </div>
  )
}
