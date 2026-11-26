import { useMemo, useState } from 'react'
import { useKitEditor } from '@kition/sdk/editor'
import { DialogFrame } from './DialogFrame'
import { CheckRow, ChoiceRow, ColorRow, NumberRow, SliderRow, TextRow } from './controls'
import { optionLabel, paramLabel } from './labels'
import { initialValues, paramsOf, type ParamField, type ParamValues } from './params'

/** The values each dialog was last OK'd with, for the session (the reference's dialogs remember theirs), by command or effect. */
const remembered = new Map<string, ParamValues>()

/** The values `key`'s dialog was last OK'd with (Effect › Apply Last Effect runs them again). */
export function rememberedValues(key: string): ParamValues | undefined {
  return remembered.get(key)
}

/**
 * The generic parameter dialog (plan 2026-11-25, stage 1): a control per parameter a person sets,
 * labelled in the editor's language, with a live preview where the command reshapes or recolours
 * art. It opens with the values given (a tool's request, the document's current settings) over
 * those last OK'd over each field's own; Alt+Cancel resets them to the fields' own.
 *
 * `memory` names what it remembers by (the command, or `effect:<id>`); `build` turns the fields'
 * parameters into the command's (an effect's go under `params`).
 */
export function ParamsDialog({
  command,
  memory,
  scope,
  title,
  fields,
  start,
  build = (params) => params,
  previewing,
  onClose,
}: {
  command: string
  memory: string
  /** What the labels are looked up for: the command, or the effect's id. */
  scope: string
  title: string
  fields: readonly ParamField[]
  start?: Record<string, unknown>
  build?: (params: Record<string, unknown>) => Record<string, unknown>
  previewing: boolean
  /** Called once the dialog is done, with whether OK ran it. */
  onClose: (committed: boolean) => void
}) {
  const words = useKitEditor()
  const defaults = useMemo(() => initialValues(fields), [fields])
  const [values, setValues] = useState<ParamValues>(() => initialValues(fields, { ...remembered.get(memory), ...start }))
  const set = (key: string, value: ParamValues[string]) => setValues((current) => ({ ...current, [key]: value }))
  const label = (key: string) => paramLabel(words, key, scope)
  const testId = (key: string) => `vector-dialog-param-${key}`
  // Keys the dialog was given but does not show (a tool's `origin`, an effect's id) go to the command as they are.
  const passed = Object.fromEntries(Object.entries(start ?? {}).filter(([key]) => !fields.some((field) => field.key === key)))
  const params = build({ ...passed, ...paramsOf(fields, values) })
  return (
    <DialogFrame
      command={command}
      title={title}
      params={params}
      previewing={previewing}
      onReset={() => setValues(defaults)}
      onClose={(committed) => {
        if (committed) remembered.set(memory, values)
        onClose(committed)
      }}
    >
      <div className="vector-dialog-fields">
        {fields.map((field) => {
          const value = values[field.key]
          switch (field.kind) {
            case 'slider':
              return <SliderRow key={field.key} label={label(field.key)} value={typeof value === 'number' ? value : field.initial} min={field.min} max={field.max} softMax={field.softMax} decimals={field.decimals} fallback={field.initial} onChange={(next) => set(field.key, next)} testId={testId(field.key)} />
            case 'number':
              return <NumberRow key={field.key} label={label(field.key)} value={typeof value === 'number' ? value : null} min={field.min} max={field.max} decimals={field.decimals} placeholder={words.t('dialog.auto')} onChange={(next) => set(field.key, next)} testId={testId(field.key)} />
            case 'choice':
              return <ChoiceRow key={field.key} label={label(field.key)} value={typeof value === 'string' ? value : field.initial} choices={field.options.map((option) => [option, optionLabel(words, option)] as const)} onChange={(next) => set(field.key, next)} testId={testId(field.key)} />
            case 'check':
              return <CheckRow key={field.key} label={label(field.key)} checked={value === true} onChange={(next) => set(field.key, next)} testId={testId(field.key)} />
            case 'color':
              return <ColorRow key={field.key} label={label(field.key)} value={typeof value === 'string' ? value : field.initial} onChange={(next) => set(field.key, next)} testId={testId(field.key)} />
            case 'text':
              return <TextRow key={field.key} label={label(field.key)} value={typeof value === 'string' ? value : ''} onChange={(next) => set(field.key, next)} testId={testId(field.key)} />
          }
        })}
      </div>
    </DialogFrame>
  )
}
