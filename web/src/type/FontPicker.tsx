import { useState } from 'react'
import { ChevronDownIcon as ChevronDown, DownloadIcon as Download, LoaderCircleIcon as Loader } from '@kition/tessera/icons/react'
import { Command, CommandEmpty, CommandGroup, CommandInput, CommandList, CommandListItem, Popover, PopoverContent, PopoverTrigger, toast, useEngineQuery, useHostPick, useKitEditor, useKitStore } from '@kition/sdk/editor'
import { fontList, recentOf, withRecent, type FontState } from './catalog'
import { ensureFamily, loadFontFile, useServedFonts } from './served'

/** The font files "Load Font File…" offers. */
const FONT_FILES = ['ttf', 'otf', 'ttc', 'woff', 'woff2']

/** The families the engine draws (`text.fontList`), asked again when fonts were added. */
export function useFamilies(): string[] {
  const served = useServedFonts()
  return useEngineQuery<{ families?: string[] }>('text.fontList', {}, served.added)?.families ?? []
}

/** A family's styles (`text.fontList {family}`), or none for a family the engine does not have. */
export function useStyles(family: string | null): string[] {
  const served = useServedFonts()
  const answer = useEngineQuery<{ styles?: string[] }>('text.fontList', family ? { family } : {}, `${family}|${served.added}`)
  return family ? (answer?.styles ?? []) : []
}

/**
 * The font menu (plan 2026-11-25, stage 5): a field showing the family, which opens a searchable
 * list of every family the engine draws and the editor serves, each drawn in its own face where the
 * page has it, the recently used ones first. A served family loads when it is chosen (its state
 * shows beside it); "Load Font File…" gives the engine a font from the workspace through the host.
 */
export function FontPicker({ value, onPick, disabled, label, testId }: { value: string | null; onPick: (family: string) => void; disabled?: boolean; label: string; testId: string }) {
  const { session, t, report } = useKitEditor()
  const served = useServedFonts()
  const engine = useFamilies()
  const host = useHostPick()
  const [recent, setRecent] = useKitStore('kit.vector', 'recentFonts', { initial: [] as string[], validate: recentOf })
  const [open, setOpen] = useState(false)
  const list = fontList(engine, served.catalog, served.loading, value ?? '')
  const known = new Map(list.map((row) => [row.family.toLowerCase(), row]))
  const recentRows = recent.map((family) => known.get(family.toLowerCase())).filter((row): row is { family: string; state: FontState } => !!row)

  const choose = async (family: string, state: FontState) => {
    setOpen(false)
    const doc = session.doc
    if (state === 'served' && doc) {
      const loaded = await ensureFamily(doc, family)
      if (!loaded) return void toast('error', t('type.fontFailed', { family }))
    }
    setRecent(withRecent(recent, family))
    onPick(family)
  }
  const loadFile = async () => {
    setOpen(false)
    const doc = session.doc
    const file = await host.pick('font', FONT_FILES)
    if (!file || !doc) return
    try {
      const families = await loadFontFile(doc, file.name, new Uint8Array(file.bytes))
      if (!families.length) return void toast('error', t('type.notAFont', { name: file.name }))
      toast('success', t('type.fontLoaded', { families: families.join(', ') }))
      setRecent(withRecent(recent, families[0]))
      onPick(families[0])
    } catch (error) {
      report(t('type.notAFont', { name: file.name }), error)
    }
  }
  const row = (prefix: string) => (item: { family: string; state: FontState }) => (
    <CommandListItem
      key={`${prefix}:${item.family}`}
      value={`${prefix}:${item.family}`}
      keywords={[item.family]}
      onSelect={() => void choose(item.family, item.state)}
      className="vector-font-row"
      data-current={item.family === value || undefined}
      data-testid={`${testId}-${prefix}-${item.family.replace(/\s+/g, '-')}`}
    >
      <span className="vector-font-name" style={{ fontFamily: `"${item.family}", var(--kit-font, system-ui), sans-serif` }}>
        {item.family}
      </span>
      <span className="vector-font-sample" style={{ fontFamily: `"${item.family}", var(--kit-font, system-ui), sans-serif` }} aria-hidden="true">
        Aa 永
      </span>
      {item.state === 'served' ? <Download size={12} aria-label={t('type.loadsOnPick')} className="vector-font-state" /> : null}
      {item.state === 'loading' ? <Loader size={12} aria-label={t('type.loading')} className="vector-font-state" /> : null}
      {item.state === 'missing' ? <span className="vector-font-state">{t('type.missing')}</span> : null}
    </CommandListItem>
  )
  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <button type="button" className="kui-field vector-font-trigger" disabled={disabled} aria-label={label} title={label} data-testid={testId}>
          <span className="vector-font-current" style={value ? { fontFamily: `"${value}", var(--kit-font, system-ui), sans-serif` } : undefined}>
            {value ?? t('paint.mixed')}
          </span>
          <ChevronDown size={12} aria-hidden="true" />
        </button>
      </PopoverTrigger>
      <PopoverContent align="start" className="vector-font-menu" data-testid={`${testId}-menu`}>
        <Command loop>
          <CommandInput placeholder={t('type.searchFonts')} aria-label={t('type.searchFonts')} data-testid={`${testId}-search`} />
          <CommandList>
            <CommandEmpty>{t('type.noFonts')}</CommandEmpty>
            {recentRows.length ? <CommandGroup heading={t('type.recent')}>{recentRows.map(row('recent'))}</CommandGroup> : null}
            <CommandGroup heading={t('type.allFonts')}>{list.map(row('font'))}</CommandGroup>
          </CommandList>
        </Command>
        {host.can ? (
          <button type="button" className="vector-font-load" onClick={() => void loadFile()} data-testid={`${testId}-load`}>
            {t('type.loadFont')}
          </button>
        ) : null}
      </PopoverContent>
    </Popover>
  )
}
