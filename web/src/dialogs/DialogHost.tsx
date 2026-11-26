import { createContext, Fragment, useContext, useLayoutEffect, useRef, useState, type ReactNode, type RefObject } from 'react'
import { useKitEditor, useUi, type KitCommand } from '@kition/sdk/editor'
import type { VectorInfo } from '../vector'
import { DocumentSetupDialog, type DocumentSetup } from './DocumentSetupDialog'
import { effectOfRow, effectRoute } from './effects'
import { dialogTitle } from './labels'
import { NewDocumentDialog, type NewPresets } from './NewDocumentDialog'
import { effectFields, paramFields, type EffectEntry, type ParamField } from './params'
import { ParamsDialog, rememberedValues } from './ParamsDialog'
import { dialogFor, previews, type DialogKind } from './routes'
import { ColorBalanceDialog } from './ColorBalanceDialog'
import { ArtboardOptionsDialog, type ArtboardStart } from '../artboards/ArtboardOptionsDialog'
import { MoveDialog, ReflectDialog, RotateDialog, ScaleDialog, ShearDialog, TransformEachDialog } from './TransformDialogs'

/**
 * A dialog made for a tool's request (plan 2026-11-25, stage 2, `canvas/toolDialogs.ts`): its
 * fields, the values they start from, and how their values become `command`'s parameters.
 */
export type AskedDialog = {
  command: string
  /** The dialog's title, in the editor's language. */
  title: string
  /** What the dialog remembers its last OK'd values by. */
  memory: string
  fields: readonly ParamField[]
  start: Record<string, unknown>
  build: (values: Record<string, unknown>) => Record<string, unknown>
  previewing: boolean
}

/**
 * The dialog host (plan 2026-11-25, stage 1). Every way to run a command as a person does (a menu
 * row, a ribbon button, Home's buttons, a shortcut, a panel's "…" row, and from stage 2 a tool's
 * dialog request) goes through `invoke`, which opens the command's dialog when it has one
 * (`routes.ts`) instead of running it with the engine's defaults. One dialog is open at a time.
 *
 * Stage 2 opens a tool's dialog with `open(command, values)`: `values` are the dialog's starting
 * values (a field's key, as `{ dx, dy }` for Move) and any parameter it passes on as given (the
 * point a Rotate tool click chose, `{ origin: [x, y] }`). Effects open with
 * `open('effect.apply', { effect: '<catalogue id>' })`.
 */
export type Dialogs = {
  /** Runs `command` as a click does: opens its dialog when it has one, else runs it. */
  invoke: (command: string) => void
  /** Opens `command`'s dialog (starting from `values`); false when it has none, true when it has one (open, or refused as disabled or denied). */
  open: (command: string, values?: Record<string, unknown>) => boolean
  /** Whether `command` opens a dialog. */
  asks: (command: string) => boolean
  /** Opens a dialog made for a tool's request; false when a dialog is open already or the command cannot run now. */
  ask: (dialog: AskedDialog) => boolean
  /** Runs an Effect menu row (`kit:effect.<id>`, Apply Last Effect, Last Effect…). */
  effectRow: (row: string) => void
  /** File › New…'s dialog. */
  newDocument: () => void
  /** File › Document Setup…'s dialog. */
  documentSetup: () => void
  /** Artboard Options for artboard `index` (the Artboards panel, the Artboard tool's double-click); `values` start some fields (a tool's name, x, y, width, height). */
  artboardOptions: (index: number, values?: Partial<ArtboardStart>) => void
}

const DialogsContext = createContext<Dialogs | null>(null)
export const DialogsProvider = DialogsContext.Provider

/** The editor's dialog host; outside one (a test's tree), commands run at once. */
export function useDialogs(): Dialogs {
  const own = useContext(DialogsContext)
  const { run } = useKitEditor()
  return own ?? { invoke: (command) => void run(command), open: () => false, asks: () => false, ask: () => false, effectRow: () => undefined, newDocument: () => undefined, documentSetup: () => undefined, artboardOptions: () => undefined }
}

/** The dialog of `command`, or null: its purpose-built one, or the generic one when it has parameters to ask. */
export function routeOf(command: KitCommand | undefined): DialogKind | null {
  return command ? dialogFor({ id: command.id, title: command.title, menu: command.menu, asks: paramFields(command).length > 0 }) : null
}

/**
 * Commands whose dialog opens on the document's current settings: what the command (or its
 * reading twin) answers when given nothing, without a step.
 */
const READS: Readonly<Record<string, string>> = {
  'file.rasterEffectsSettings': 'file.rasterEffectsSettings',
  'file.setInfo': 'file.setInfo',
  'object.envelope.options': 'object.envelope.info',
  'object.textWrap.options': 'object.textWrap.options',
  'text.pathOptions': 'text.pathOptions',
  'text.areaOptions': 'text.areaOptions',
  'view.proofSetup': 'view.proofSetup',
}

/** The transform options the preferences hold, as the transform dialogs' boxes start. */
const PREFERENCE_OPTIONS = { patterns: 'transformPatternTiles', strokes: 'scaleStrokes', corners: 'scaleCorners' } as const

type Open =
  | { serial: number; kind: DialogKind; command: KitCommand; values?: Record<string, unknown> }
  | { serial: number; kind: 'effect'; effect: EffectEntry; values?: Record<string, unknown> }
  | { serial: number; kind: 'newDocument'; presets: NewPresets | null }
  | { serial: number; kind: 'documentSetup'; setup: DocumentSetup }
  | { serial: number; kind: 'asked'; dialog: AskedDialog }
  | { serial: number; kind: 'artboard'; start: ArtboardStart; presets: NewPresets | null }

/** A dialog to show, before the host numbers it. */
type Unserialed<T> = T extends unknown ? Omit<T, 'serial'> : never

/** The values of `answer` the dialog's fields show, leaving out what the read answers beside them. */
function fieldValues(answer: unknown, fields: readonly ParamField[]): Record<string, unknown> {
  if (!answer || typeof answer !== 'object') return {}
  const record = answer as Record<string, unknown>
  return Object.fromEntries(fields.filter((field) => field.key in record).map((field) => [field.key, record[field.key]]))
}

/**
 * Holds the open dialog and answers `api` (which App passes to the menus, built outside the
 * editor's provider). Dialogs that start from the document read it before they open, so the
 * preview never runs before the values it starts from are known.
 */
export function VectorDialogs({ api, info, effects, onEffectApplied }: { api: RefObject<Dialogs | null>; info: VectorInfo | null; effects: readonly EffectEntry[]; onEffectApplied: (effect: EffectEntry) => void }) {
  const editor = useKitEditor()
  const ui = useUi()
  const [open, setOpen] = useState<Open | null>(null)
  const serial = useRef(0)
  const busy = useRef(false)
  const lastEffect = useRef<EffectEntry | null>(null)
  const latest = useRef({ editor, ui, open, effects, onEffectApplied, info })
  latest.current = { editor, ui, open, effects, onEffectApplied, info }

  useLayoutEffect(() => {
    const show = (next: Unserialed<Open>) => {
      serial.current += 1
      setOpen({ ...next, serial: serial.current })
    }
    /** Reads what the dialog starts from, then shows it; one read at a time, and none while a dialog is open. */
    const after = <T,>(read: () => Promise<T>, then: (answer: T) => void) => {
      busy.current = true
      void read()
        .then(then)
        .finally(() => {
          busy.current = false
        })
    }
    const blocked = () => latest.current.open !== null || busy.current
    const openEffect = (entry: EffectEntry, values?: Record<string, unknown>) => {
      const { editor: now, ui: uiNow } = latest.current
      if (blocked() || !now.enabled('effect.apply') || uiNow.state('command:effect.apply').denied) return
      const route = effectRoute(entry)
      if (route.kind === 'hidden') return
      if (route.kind === 'apply') {
        void now.run('effect.apply', { effect: entry.id }).then((done) => {
          if (done !== null) {
            lastEffect.current = entry
            latest.current.onEffectApplied(entry)
          }
        })
        return
      }
      show({ kind: 'effect', effect: entry, values })
    }
    const openDialog: Dialogs['open'] = (id, values) => {
      const { editor: now, ui: uiNow, effects: catalog } = latest.current
      if (id === 'effect.apply') {
        const entry = catalog.find((item) => item.id === values?.effect)
        if (!entry) return false
        const { effect: _effect, ...rest } = values ?? {}
        openEffect(entry, rest)
        return true
      }
      const command = now.byId.get(id)
      const kind = routeOf(command)
      if (!command || !kind) return false
      if (blocked() || !now.enabled(id) || uiNow.state(`command:${id}`, command.menu).denied) return true
      if (kind !== 'params' && kind !== 'colorBalance') {
        // The transform dialogs' option boxes start as the preferences have them.
        after(
          () => now.query<Array<{ key?: string; value?: unknown }>>('prefs.list', {}),
          (prefs) => {
            const options = Object.fromEntries(Object.entries(PREFERENCE_OPTIONS).map(([key, pref]) => [key, prefs?.find((item) => item.key === pref)?.value === true]))
            show({ kind, command, values: { ...options, ...values } })
          },
        )
        return true
      }
      const read = READS[id]
      if (read) {
        after(
          () => now.query(read, {}),
          (answer) => show({ kind, command, values: { ...fieldValues(answer, paramFields(command)), ...values } }),
        )
        return true
      }
      show({ kind, command, values })
      return true
    }
    api.current = {
      open: openDialog,
      invoke: (id) => {
        if (!openDialog(id)) void latest.current.editor.run(id)
      },
      asks: (id) => routeOf(latest.current.editor.byId.get(id)) !== null,
      ask: (dialog) => {
        const { editor: now, ui: uiNow } = latest.current
        const command = now.byId.get(dialog.command)
        if (blocked() || !command || !now.enabled(dialog.command) || uiNow.state(`command:${dialog.command}`, command.menu).denied) return false
        show({ kind: 'asked', dialog })
        return true
      },
      effectRow: (row) => {
        const catalog = latest.current.effects
        const id = effectOfRow(row)
        if (id) {
          const entry = catalog.find((item) => item.id === id)
          if (entry) openEffect(entry)
          return
        }
        const last = lastEffect.current
        if (!last) return
        // Apply Last Effect runs it again with the values it was last OK'd with; Last Effect… opens its dialog on them.
        if (row.endsWith('applyLast')) void latest.current.editor.run('effect.apply', { effect: last.id, params: rememberedValues(`effect:${last.id}`) ?? {} })
        else openEffect(last)
      },
      newDocument: () => {
        if (blocked()) return
        after(
          () => latest.current.editor.query<NewPresets>('file.newPresets', {}),
          (presets) => show({ kind: 'newDocument', presets }),
        )
      },
      artboardOptions: (index, values) => {
        const board = latest.current.info?.artboards[index]
        if (blocked() || !board || !latest.current.editor.enabled('artboard.setProps')) return
        const start: ArtboardStart = { index, name: board.name, x: board.x, y: board.y, width: board.width, height: board.height, ...values }
        after(
          () => latest.current.editor.query<NewPresets>('file.newPresets', {}),
          (presets) => show({ kind: 'artboard', start, presets }),
        )
      },
      documentSetup: () => {
        if (blocked() || !latest.current.editor.enabled('file.setup')) return
        after(
          () => latest.current.editor.query<DocumentSetup>('file.setup', {}),
          (setup) => {
            if (setup) show({ kind: 'documentSetup', setup })
          },
        )
      },
    }
  })

  if (!open) return null
  const close = () => setOpen(null)
  const words = editor
  const units = info?.units ?? 'Points'
  const dialog = ((): ReactNode => {
    switch (open.kind) {
      case 'newDocument':
        return <NewDocumentDialog presets={open.presets} onClose={close} />
      case 'documentSetup':
        return <DocumentSetupDialog setup={open.setup} title={words.t('dialog.documentSetup')} onClose={close} />
      case 'artboard':
        return <ArtboardOptionsDialog start={open.start} presets={open.presets} units={units} onClose={close} />
      case 'asked': {
        const { dialog: asked } = open
        return <ParamsDialog command={asked.command} memory={asked.memory} scope={asked.command} title={asked.title} fields={asked.fields} start={asked.start} build={asked.build} previewing={asked.previewing} onClose={close} />
      }
      case 'effect': {
        const { effect } = open
        return (
          <ParamsDialog
            command="effect.apply"
            memory={`effect:${effect.id}`}
            scope={effect.id}
            title={dialogTitle(words, effect.label)}
            fields={effectFields(effect)}
            start={open.values}
            build={(params) => ({ effect: effect.id, params })}
            previewing
            onClose={(committed) => {
              close()
              // An effect OK'd is the one Apply Last Effect runs again.
              if (committed) {
                lastEffect.current = effect
                onEffectApplied(effect)
              }
            }}
          />
        )
      }
      default: {
        const { command } = open
        const props = { title: dialogTitle(words, command.title), units, start: open.values, onClose: close }
        switch (open.kind) {
          case 'move':
            return <MoveDialog {...props} />
          case 'rotate':
            return <RotateDialog {...props} />
          case 'scale':
            return <ScaleDialog {...props} />
          case 'reflect':
            return <ReflectDialog {...props} />
          case 'shear':
            return <ShearDialog {...props} />
          case 'transformEach':
            return <TransformEachDialog {...props} />
          case 'colorBalance':
            return <ColorBalanceDialog title={props.title} onClose={close} />
          case 'params':
            return <ParamsDialog command={command.id} memory={command.id} scope={command.id} title={props.title} fields={paramFields(command)} start={open.values} previewing={previews(command.id)} onClose={close} />
        }
      }
    }
  })()
  // A new dialog (its serial) starts from its own values, never the last one's.
  return <Fragment key={open.serial}>{dialog}</Fragment>
}
