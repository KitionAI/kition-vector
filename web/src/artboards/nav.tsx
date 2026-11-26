import { createContext, useContext } from 'react'
import { ChevronLeftIcon as ChevronLeft, ChevronRightIcon as ChevronRight, FrameIcon as Frame, MaximizeIcon as Maximize } from '@kition/tessera/icons/react'
import { IconButton, useKitEditor } from '@kition/sdk/editor'
import type { VectorInfo } from '../vector'
import { stepArtboard } from './order'

/**
 * The active artboard and the view's moves to it (plan 2026-11-25, stage 4): the Artboards panel,
 * the status bar's artboard navigation, Cmd+0 and the artboard marks on the canvas share it.
 */
export type ArtboardNav = {
  /** The active artboard's index. */
  active: number
  /** Makes artboard `index` the active one. */
  setActive: (index: number) => void
  /** Makes artboard `index` active and fits it in the window. */
  show: (index: number) => void
  /** Fits every artboard in the window. */
  fitAll: () => void
}

const NavContext = createContext<ArtboardNav | null>(null)
export const ArtboardNavProvider = NavContext.Provider

/** The editor's artboard navigation; outside the editor, one that does nothing. */
export function useArtboardNav(): ArtboardNav {
  return useContext(NavContext) ?? { active: 0, setActive: () => undefined, show: () => undefined, fitAll: () => undefined }
}

/** An artboard's label in lists: its number and name ("2  Back"). */
export const artboardLabel = (index: number, name: string) => `${index + 1}  ${name}`

/**
 * The status bar's artboard navigation, as the reference's: previous, a list of every artboard
 * (choosing one fits it in the window), next, then Fit Artboard in Window and Fit All in Window.
 */
export function ArtboardNavigator({ info }: { info: VectorInfo }) {
  const { t, phrase } = useKitEditor()
  const nav = useArtboardNav()
  const count = info.artboards.length
  if (!count) return null
  const active = Math.min(nav.active, count - 1)
  return (
    <span className="vector-artboard-nav" data-testid="vector-artboard-nav">
      <IconButton icon={ChevronLeft} label={t('artboards.previous')} disabled={count < 2} onClick={() => nav.show(stepArtboard(active, -1, count))} data-testid="vector-artboard-previous" />
      <select className="vector-artboard-pick" aria-label={phrase('Artboards')} value={active} onChange={(event) => nav.show(Number(event.target.value))} data-testid="vector-artboard-pick">
        {info.artboards.map((board, index) => (
          <option key={index} value={index}>
            {artboardLabel(index, board.name)}
          </option>
        ))}
      </select>
      <IconButton icon={ChevronRight} label={t('artboards.next')} disabled={count < 2} onClick={() => nav.show(stepArtboard(active, 1, count))} data-testid="vector-artboard-next" />
      <IconButton icon={Frame} label={phrase('Fit Artboard in Window')} onClick={() => nav.show(active)} data-testid="vector-artboard-fit" />
      <IconButton icon={Maximize} label={phrase('Fit All in Window')} onClick={nav.fitAll} data-testid="vector-artboard-fit-all" />
    </span>
  )
}
