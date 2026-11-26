import { useCallback, useMemo } from 'react'
import {
  CaseSensitiveIcon as CaseSensitive,
  CaseUpperIcon as CaseUpper,
  StrikethroughIcon as Strikethrough,
  SubscriptIcon as Subscript,
  SuperscriptIcon as Superscript,
  TextAlignCenterIcon as AlignCenter,
  TextAlignEndIcon as AlignRight,
  TextAlignJustifyIcon as JustifyAll,
  TextAlignStartIcon as AlignLeft,
  TextJustifyCenterIcon as JustifyCenter,
  TextJustifyEndIcon as JustifyRight,
  TextJustifyStartIcon as JustifyLeft,
  UnderlineIcon as Underline,
} from '@kition/tessera/icons/react'
import { Checkbox, Choice, IconButton, PanelSection, useKitEditor, useKitTools, useReadOnly } from '@kition/sdk/editor'
import { paintView } from '../color/paint'
import { PaintChip, PanelNumber } from '../panels/fields'
import { usePaintPicker } from '../panels/usePaint'
import { useLive } from '../panels/useLive'
import { selectedNodes, type VectorInfo } from '../vector'
import { FontPicker, useStyles } from './FontPicker'
import { ALIGNMENTS, characterStep, labelPaint, paragraphStep, typeRequest, type CharacterStyle } from './typing'

/** Whether the selection holds type (a type object, or a group with some). */
export function selectsText(info: VectorInfo) {
  const holds = (nodes: VectorInfo['layers']): boolean => nodes.some((node) => node.kind === 'Type' || holds(node.children))
  return holds(selectedNodes(info))
}

/**
 * What the type controls set (plan 2026-11-25, stage 5): the selected characters of the type being
 * edited, else the whole type object(s), else what new type is set in (`typing.ts`). `live` moves a
 * value as a field's label is dragged: one history step when it ends.
 */
export function useTypeSetters(info: VectorInfo) {
  const { run, session } = useKitEditor()
  const tools = useKitTools()
  const live = useLive()
  const typing = info.typing
  const selected = typing.editing !== null || selectsText(info)
  const setCharacter = useCallback(
    (values: Record<string, unknown>, scrub?: boolean | 'final') => {
      const step = characterStep(typing, selected, values)
      if (!step) return
      if ('request' in step) {
        void session.tool('toolKey', [typeRequest(step.request), '{}', 1], true).then(() => tools?.refresh())
        return
      }
      if (scrub === true) live.set(step.command, step.params)
      else if (scrub === 'final') void live.commit(step.command, step.params)
      else void run(step.command, step.params)
    },
    [typing, selected, session, tools, live, run],
  )
  const setParagraph = useCallback(
    (values: Record<string, unknown>, scrub?: boolean | 'final') => {
      const step = paragraphStep(typing, values)
      if (scrub === true) live.set(step.command, step.params)
      else if (scrub === 'final') void live.commit(step.command, step.params)
      else void run(step.command, step.params)
    },
    [typing, live, run],
  )
  /** The attributes shown: the edited or selected type's, else new type's family, style and size. */
  const shown: CharacterStyle | null = typing.character
  return { selected, setCharacter, setParagraph, shown }
}

/** The font family and its style, as the Character panel and the control bar show them. */
export function FamilyAndStyle({ info, compact = false, prefix = 'vector-char' }: { info: VectorInfo; compact?: boolean; prefix?: string }) {
  const { t } = useKitEditor()
  const readOnly = useReadOnly()
  const { selected, setCharacter, shown } = useTypeSetters(info)
  const family = selected ? (shown?.font ?? null) : (info.typing.newType.font ?? 'Source Sans 3')
  const style = selected ? (shown?.style ?? null) : (info.typing.newType.style ?? 'Regular')
  const styles = useStyles(family)
  const choices = useMemo(() => {
    const list = styles.map((name) => [name, name] as const)
    return style && !styles.includes(style) ? [[style, style] as const, ...list] : list
  }, [styles, style])
  return (
    <>
      <FontPicker value={family} label={t('type.font')} disabled={readOnly} onPick={(font) => setCharacter({ font })} testId={`${prefix}-font`} />
      <span className="vector-type-style">
        <Choice label={t('type.fontStyle')} quiet={compact} value={style ?? ''} choices={style ? choices : [['', t('paint.mixed')] as const, ...choices]} disabled={readOnly || choices.length === 0} onChange={(next) => next && setCharacter({ style: next })} testId={`${prefix}-style`} />
      </span>
    </>
  )
}

/** The font size field. */
export function SizeField({ info, prefix = 'vector-char' }: { info: VectorInfo; prefix?: string }) {
  const { t } = useKitEditor()
  const { selected, setCharacter, shown } = useTypeSetters(info)
  const size = selected ? (shown?.size ?? null) : (info.typing.newType.size ?? 12)
  return (
    <PanelNumber
      label={t('type.size')}
      title={t('type.sizeTitle')}
      value={size}
      unit="pt"
      decimals={2}
      min={0.1}
      max={1296}
      step={1}
      lengths
      onSet={(value) => setCharacter({ size: value })}
      onScrub={selected ? (value, final) => setCharacter({ size: value }, final ? 'final' : true) : undefined}
      testId={`${prefix}-size`}
    />
  )
}

/** A number field of a character attribute (`key` as `text.setRangeStyle` names it). */
function CharNumber({ info, attribute, label, title, unit, min, max, step = 1, decimals = 2, auto, testId }: { info: VectorInfo; attribute: keyof CharacterStyle; label: string; title: string; unit?: string; min: number; max: number; step?: number; decimals?: number; auto?: boolean; testId: string }) {
  const { t } = useKitEditor()
  const { selected, setCharacter, shown } = useTypeSetters(info)
  const raw = shown?.[attribute]
  const value = typeof raw === 'number' ? raw : null
  return (
    <PanelNumber
      label={label}
      title={title}
      value={value}
      unit={unit}
      decimals={decimals}
      min={min}
      max={max}
      step={step}
      placeholder={raw === 'auto' ? t('type.auto') : undefined}
      disabled={!selected}
      onSet={(next) => setCharacter({ [attribute]: next })}
      onClear={auto ? () => setCharacter({ [attribute]: 'auto' }) : undefined}
      onScrub={value !== null ? (next, final) => setCharacter({ [attribute]: next }, final ? 'final' : true) : undefined}
      testId={testId}
    />
  )
}

/** A toggle of a character attribute; pressed when every selected character has it. */
function CharToggle({ info, icon, label, pressed, values, testId }: { info: VectorInfo; icon: typeof Underline; label: string; pressed: boolean; values: (on: boolean) => Record<string, unknown>; testId: string }) {
  const readOnly = useReadOnly()
  const { selected, setCharacter } = useTypeSetters(info)
  return <IconButton icon={icon} label={label} aria-pressed={pressed} data-active={pressed || undefined} disabled={readOnly || !selected} onClick={() => setCharacter(values(!pressed))} data-testid={testId} />
}

/**
 * The Character panel (plan 2026-11-25, stage 5): font family (searchable, previewed, recently used
 * first), style, size, leading, kerning, tracking, vertical and horizontal scale, baseline shift,
 * character rotation, the characters' colour, All Caps, Small Caps, superscript, subscript,
 * underline and strikethrough. It acts on the selected characters while some are selected in the
 * type being edited, else on the whole type object(s); with no type, family, style and size set
 * what new type is set in.
 */
export function CharacterPanel({ info }: { info: VectorInfo }) {
  const { t } = useKitEditor()
  const readOnly = useReadOnly()
  const picker = usePaintPicker()
  const { selected, shown } = useTypeSetters(info)
  const fill = labelPaint(shown?.fill ?? null)
  const toggles = (
    [
      ['allCaps', CaseUpper, t('type.allCaps'), shown?.allCaps === true, (on: boolean) => ({ allCaps: on })],
      ['smallCaps', CaseSensitive, t('type.smallCaps'), shown?.smallCaps === true, (on: boolean) => ({ smallCaps: on })],
      ['superscript', Superscript, t('type.superscript'), shown?.position === 'superscript', (on: boolean) => ({ position: on ? 'superscript' : 'normal' })],
      ['subscript', Subscript, t('type.subscript'), shown?.position === 'subscript', (on: boolean) => ({ position: on ? 'subscript' : 'normal' })],
      ['underline', Underline, t('type.underline'), shown?.underline === true, (on: boolean) => ({ underline: on })],
      ['strikethrough', Strikethrough, t('type.strikethrough'), shown?.strikethrough === true, (on: boolean) => ({ strikethrough: on })],
    ] as const
  ).map(([key, icon, label, pressed, values]) => <CharToggle key={key} info={info} icon={icon} label={label} pressed={pressed} values={values} testId={`vector-char-${key}`} />)
  return (
    <div className="vector-panel-scroll" data-testid="vector-character-panel">
      <PanelSection title={t('type.characterTitle')} testId="vector-character">
        {selected ? null : <p className="vector-hint">{t('type.newTypeHint')}</p>}
        <div className="vector-type-family">
          <FamilyAndStyle info={info} />
        </div>
        <div className="vector-fields">
          <SizeField info={info} />
          <CharNumber info={info} attribute="leading" label={t('type.leading')} title={t('type.leadingTitle')} unit="pt" min={0} max={5000} auto testId="vector-char-leading" />
          <CharNumber info={info} attribute="kerning" label={t('type.kerning')} title={t('type.kerningTitle')} min={-1000} max={10000} step={10} decimals={0} auto testId="vector-char-kerning" />
          <CharNumber info={info} attribute="tracking" label={t('type.tracking')} title={t('type.trackingTitle')} min={-1000} max={10000} step={10} decimals={0} testId="vector-char-tracking" />
          <CharNumber info={info} attribute="vScale" label={t('type.vScale')} title={t('type.vScaleTitle')} unit="%" min={1} max={10000} decimals={1} testId="vector-char-vscale" />
          <CharNumber info={info} attribute="hScale" label={t('type.hScale')} title={t('type.hScaleTitle')} unit="%" min={1} max={10000} decimals={1} testId="vector-char-hscale" />
          <CharNumber info={info} attribute="baselineShift" label={t('type.baselineShift')} title={t('type.baselineShiftTitle')} unit="pt" min={-1296} max={1296} testId="vector-char-baseline" />
          <CharNumber info={info} attribute="rotation" label={t('type.rotation')} title={t('type.rotationTitle')} unit="°" min={-180} max={180} decimals={1} testId="vector-char-rotation" />
        </div>
        <div className="vector-type-row">
          <span className="vector-field-label">{t('type.color')}</span>
          <PaintChip view={paintView(fill, selected && shown?.fill === null)} label={t('type.color')} disabled={readOnly || !selected} onOpen={() => picker('fill', fill)} testId="vector-char-fill" />
          <span className="vector-type-toggles">{toggles}</span>
        </div>
      </PanelSection>
    </div>
  )
}

const ALIGN_ICONS = { left: AlignLeft, center: AlignCenter, right: AlignRight, justifyLeft: JustifyLeft, justifyCenter: JustifyCenter, justifyRight: JustifyRight, justifyAll: JustifyAll } as const

/**
 * The Paragraph panel (plan 2026-11-25, stage 5): the seven alignments, left, right and first-line
 * indents, space before and after, and hyphenation, for the paragraphs the selection (or the caret)
 * touches in the type being edited, else for every paragraph of the selected type.
 */
export function ParagraphPanel({ info }: { info: VectorInfo }) {
  const { t } = useKitEditor()
  const readOnly = useReadOnly()
  const { selected, setParagraph } = useTypeSetters(info)
  const para = info.typing.paragraph
  // "auto" (new type's) aligns to the start of each paragraph's direction: left for most text.
  const justify = para?.justify === 'auto' ? 'left' : (para?.justify ?? null)
  const number = (key: 'leftIndent' | 'rightIndent' | 'firstLineIndent' | 'spaceBefore' | 'spaceAfter', label: string, title: string, testId: string) => (
    <PanelNumber
      label={label}
      title={title}
      value={para?.[key] ?? null}
      unit="pt"
      min={-1296}
      max={1296}
      disabled={!selected}
      onSet={(value) => setParagraph({ [key]: value })}
      onScrub={para?.[key] != null ? (value, final) => setParagraph({ [key]: value }, final ? 'final' : true) : undefined}
      testId={testId}
    />
  )
  return (
    <div className="vector-panel-scroll" data-testid="vector-paragraph-panel">
      <PanelSection title={t('type.paragraphTitle')} testId="vector-paragraph">
        {selected ? null : <p className="vector-hint">{t('type.selectTypeHint')}</p>}
        <div className="vector-type-align" role="group" aria-label={t('para.alignment')}>
          {ALIGNMENTS.map((value) => (
            <IconButton key={value} icon={ALIGN_ICONS[value]} label={t(`para.align.${value}`)} aria-pressed={justify === value} data-active={justify === value || undefined} disabled={readOnly || !selected} onClick={() => setParagraph({ justify: value })} data-testid={`vector-para-align-${value}`} />
          ))}
        </div>
        <div className="vector-fields">
          {number('leftIndent', t('para.leftIndent'), t('para.leftIndentTitle'), 'vector-para-left-indent')}
          {number('rightIndent', t('para.rightIndent'), t('para.rightIndentTitle'), 'vector-para-right-indent')}
          {number('firstLineIndent', t('para.firstLineIndent'), t('para.firstLineIndentTitle'), 'vector-para-first-indent')}
          {number('spaceBefore', t('para.spaceBefore'), t('para.spaceBeforeTitle'), 'vector-para-space-before')}
          {number('spaceAfter', t('para.spaceAfter'), t('para.spaceAfterTitle'), 'vector-para-space-after')}
        </div>
        <Checkbox label={t('para.hyphenate')} checked={para?.hyphenate === true} disabled={readOnly || !selected} onChange={(hyphenate) => setParagraph({ hyphenate })} testId="vector-para-hyphenate" />
      </PanelSection>
    </div>
  )
}
