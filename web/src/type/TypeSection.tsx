import { createContext, useContext } from 'react'
import { Button, PanelSection, useKitEditor } from '@kition/sdk/editor'
import type { VectorInfo } from '../vector'
import { FamilyAndStyle, SizeField, useTypeSetters } from './TypePanels'
import { PanelNumber } from '../panels/fields'

/** Brings a dock panel forward (its group shown, its tab in front): the editor's. */
export const RevealPanel = createContext<(tab: string) => void>(() => undefined)

/**
 * Properties › Character (plan 2026-11-25, stage 5), shown while type is selected or edited: font
 * family and style, size, leading and tracking, and a way to the full Character and Paragraph
 * panels. It acts as the Character panel does (the selected characters, else the whole type).
 */
export function TypeSection({ info }: { info: VectorInfo }) {
  const { t } = useKitEditor()
  const reveal = useContext(RevealPanel)
  const { setCharacter, shown } = useTypeSetters(info)
  if (!shown) return null
  const leading = shown.leading
  return (
    <PanelSection title={t('type.characterTitle')} testId="vector-type-section">
      <div className="vector-type-family">
        <FamilyAndStyle info={info} prefix="vector-type" />
      </div>
      <div className="vector-fields">
        <SizeField info={info} prefix="vector-type" />
        <PanelNumber
          label={t('type.leading')}
          title={t('type.leadingTitle')}
          value={typeof leading === 'number' ? leading : null}
          placeholder={leading === 'auto' ? t('type.auto') : undefined}
          unit="pt"
          min={0}
          max={5000}
          onSet={(value) => setCharacter({ leading: value })}
          onClear={() => setCharacter({ leading: 'auto' })}
          testId="vector-type-leading"
        />
        <PanelNumber label={t('type.tracking')} title={t('type.trackingTitle')} value={shown.tracking} min={-1000} max={10000} step={10} decimals={0} onSet={(value) => setCharacter({ tracking: value })} testId="vector-type-tracking" />
      </div>
      <div className="vector-type-more">
        <Button size="sm" variant="ghost" onClick={() => reveal('character')} data-testid="vector-type-more-character">
          {t('type.moreCharacter')}
        </Button>
        <Button size="sm" variant="ghost" onClick={() => reveal('paragraph')} data-testid="vector-type-more-paragraph">
          {t('type.moreParagraph')}
        </Button>
      </div>
    </PanelSection>
  )
}
