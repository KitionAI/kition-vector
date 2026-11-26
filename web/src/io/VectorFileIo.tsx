import { useEffect, useLayoutEffect, useRef, useState, type RefObject } from 'react'
import { KitEngine, toast, useHostPick, useKitEditor, useUi, type HostConnection, type KitCanvasHandle, type Strings } from '@kition/sdk/editor'
import { engineManifest } from '../engine'
import { errorMessage } from './errors'
import type { Busy } from './busy'
import {
  OPEN_ACCEPT,
  PLACE_ACCEPT,
  TEMPLATE_ACCEPT,
  copiedText,
  copyName,
  copyTypes,
  dragHasFiles,
  dropCenters,
  extensionOf,
  fingerprint,
  flavourOf,
  fromBase64,
  importOf,
  isNative,
  isPlaceable,
  isSvgText,
  nativeName,
  toBase64,
  type Copied,
  type Pasted,
} from './files'

/**
 * Files in and out (plan 2026-11-25, stage 7), every one through the host or the browser's own
 * clipboard and drag (Kit UI protocol UI-K2): File › Open… (the host's picker, then `host/open`),
 * Save (a drawing read from SVG, PDF or EPS is saved as a `.kvector` beside it, never over it),
 * Save a Copy…, New from Template…, Place… (the host's picker, placed in the middle of the view);
 * the system clipboard (Copy and Cut put the copy there as SVG, text and PNG as well as in the
 * engine's clipboard; Paste takes in SVG markup, pictures, PDF and text other applications put
 * there); and files dragged from the computer onto the canvas, placed where they drop. App
 * reaches it through `api`; it draws the drop mark and the paste catcher.
 */

export type FileIo = {
  open: () => void
  /** File › Save: the host's save for a drawing, a `.kvector` beside the file for one read from another format. */
  save: () => void
  saveCopy: () => void
  newFromTemplate: () => void
  place: () => void
  /** Whether the editor runs `command` itself (Copy, Cut and Paste from a menu, with the system clipboard). */
  handles: (command: string) => boolean
  run: (command: string) => void
}

/** What the host lets the File menu do (protocol capabilities and the document's permissions). */
export function fileAbilities(host: HostConnection) {
  const caps = host.init.capabilities
  return {
    open: caps.includes('open-file') && caps.includes('pick-file'),
    pick: caps.includes('pick-file'),
    edit: host.readOnly !== true,
  }
}

/** Asks the host to open the workspace file at `path` (from a person's gesture: a menu row, a notice's button). */
export async function openInHost(host: HostConnection, path: string): Promise<void> {
  await host.peer.request('host/open', { link: { path } })
}

/**
 * Says a new file was written beside the drawing, with Open on the notice where the host opens
 * files (in Kition a new tab; the development host opens it in place).
 */
export function announceWritten(host: HostConnection, t: Strings, title: string, path: string) {
  const open = fileAbilities(host).open
  toast.success(title, {
    duration: open ? 10_000 : undefined,
    action: open ? { label: t('file.openIt'), onClick: () => void openInHost(host, path).catch((error) => toast('error', t('file.openFailed'), errorMessage(t, error))) } : undefined,
  })
}

const EDITABLE = 'input, textarea, select, [contenteditable=""], [contenteditable="true"]'
const isEditable = (target: EventTarget | null) => target instanceof Element && target.closest(EDITABLE) != null
const dialogOpen = () => document.querySelector('.kui-dialog, .kui-dialog-backdrop') != null
/** How long Cmd+V waits for the browser's paste before it pastes the engine's clipboard. */
const PASTE_WAIT_MS = 600
const COPY_COMMANDS = new Set(['edit.copy', 'edit.cut'])

/** Whether the person was told this session that the browser keeps copies off the system clipboard. */
let toldLocal = false

export function VectorFileIo({
  api,
  area,
  canvas,
  origin,
  busy,
  editing,
}: {
  api: RefObject<FileIo | null>
  /** The canvas area: where files drop and the drop mark shows. */
  area: RefObject<HTMLDivElement | null>
  canvas: RefObject<KitCanvasHandle | null>
  /** The extent's origin (document pixels are the drawing's points less it). */
  origin: readonly [number, number] | null
  busy: Busy
  /** Text is being typed on the canvas: its own clipboard keys win. */
  editing: () => boolean
}) {
  const editor = useKitEditor()
  const { t, host } = editor
  const ui = useUi()
  const hostPick = useHostPick()
  const [dropping, setDropping] = useState(false)
  const catcher = useRef<HTMLTextAreaElement>(null)
  // The fingerprint of the text the editor last put on the system clipboard ('' when that write
  // failed): a paste of it is the engine's own paste, which keeps swatches, styles and layers.
  const ownCopy = useRef<string | null>(null)
  const now = useRef({ editor, ui, hostPick, editing, origin })
  now.current = { editor, ui, hostPick, editing, origin }

  useLayoutEffect(() => {
    const surfaceBox = () => area.current?.querySelector('[data-testid="kit-canvas-surface"]')?.getBoundingClientRect() ?? null
    /** The drawing's point (in points) under a client point. */
    const docPoint = (clientX: number, clientY: number): [number, number] | null => {
      const view = canvas.current?.view()
      const box = surfaceBox()
      const [ox, oy] = now.current.origin ?? [0, 0]
      if (!view || !box) return null
      return [view.x + (clientX - box.left) / (view.zoom || 1) + ox, view.y + (clientY - box.top) / (view.zoom || 1) + oy]
    }
    /** The middle of the view, where a paste or a place lands. */
    const viewCenter = (): [number, number] => {
      const box = surfaceBox()
      return (box ? docPoint(box.left + box.width / 2, box.top + box.height / 2) : null) ?? [0, 0]
    }
    const allowed = (command: string) => {
      const { editor: e, ui: u } = now.current
      return e.enabled(command) && !u.state(`command:${command}`, e.byId.get(command)?.menu).denied
    }
    const run = (command: string, params: Record<string, unknown> = {}) => now.current.editor.run(command, params)
    const query = <R,>(command: string, params: Record<string, unknown> = {}) => now.current.editor.query<R>(command, params)
    const report = (title: string, error: unknown) => toast('error', title, errorMessage(t, error))
    const backToCanvas = () => document.querySelector<HTMLElement>('.kui-app')?.focus({ preventScroll: true })

    /** Runs `work` as a slow job the person can cancel: undone when it ends after Cancel. */
    const job = async (label: string, work: () => Promise<string | null>) => {
      const started = busy.begin(label)
      let answer: string | null = null
      try {
        answer = await work()
      } finally {
        if (busy.end(started) && answer !== null) await run('edit.undo')
      }
      return answer
    }

    /** Places a file's bytes centred on `at` (drawing points): one step, the placed art selected. */
    const placeBytes = (name: string, bytes: Uint8Array, at: [number, number]) =>
      job(t('busy.place', { name }), () => run('file.place', { name, dataBase64: toBase64(bytes), at: [Math.round(at[0]), Math.round(at[1])] }))
    const placeFiles = async (files: readonly File[], at: [number, number]) => {
      const placeable = files.filter((file) => isPlaceable(file))
      for (const file of files) if (!placeable.includes(file)) toast('error', t('file.placeFailed'), t('drop.notPlaceable', { name: file.name }))
      const centers = dropCenters({ x: at[0], y: at[1] }, placeable.length)
      for (const [index, file] of placeable.entries()) await placeBytes(file.name, new Uint8Array(await file.arrayBuffer()), centers[index])
      backToCanvas()
    }

    // ---------- the system clipboard ----------
    /** The copy as other applications take it: SVG markup, a type-only copy's words, a PNG at twice its size. */
    const copied = async (): Promise<Copied> => {
      const [svg, text, png] = await Promise.all([
        query<{ svg?: string | null }>('clipboard.exportSvg'),
        query<{ text?: string | null }>('clipboard.exportText'),
        query<{ dataBase64?: string | null }>('clipboard.exportPng', { scale: 2 }),
      ])
      return { svg: svg?.svg ?? null, text: text?.text ?? null, png: png?.dataBase64 ? fromBase64(png.dataBase64) : null }
    }
    const writable = (type: string) => {
      const supports = (globalThis.ClipboardItem as unknown as { supports?: (type: string) => boolean } | undefined)?.supports
      // Without `supports` (older browsers) only the types every browser writes.
      return supports ? supports(type) : type !== 'image/svg+xml'
    }
    const copy = (command: string) => {
      const done = run(command)
      const content = done.then(async (answer) => {
        if (answer === null) throw new Error('nothing copied')
        const copy = await copied()
        const text = copiedText(copy)
        ownCopy.current = text ? fingerprint(text) : ''
        return copy
      })
      content.catch(() => undefined)
      // The write starts in the key's (or click's) gesture with the content to come: Safari asks for that.
      const types = typeof ClipboardItem !== 'undefined' && typeof navigator.clipboard?.write === 'function' ? copyTypes(writable) : []
      const blob = (type: string) =>
        content.then((copy) => {
          const found = flavourOf(type, copy)
          if (!found) throw new Error(`no ${type}`)
          return found
        })
      const write = types.length ? navigator.clipboard.write([new ClipboardItem(Object.fromEntries(types.map((type) => [type, blob(type)])))]) : Promise.reject(new Error('no clipboard'))
      void write.catch(async () => {
        if ((await content.catch(() => null)) === null) return
        // Pasted again, the copy comes from the engine's clipboard; other applications do not get it.
        ownCopy.current = ''
        if (!toldLocal) {
          toldLocal = true
          toast('info', t('clipboard.copyLocal'))
        }
      })
    }

    const enginePaste = () => {
      if (allowed('edit.paste')) void run('edit.paste')
    }
    /** Takes in what another application put on the clipboard; the editor's own copy is the engine's paste. */
    const pasteFrom = async (pasted: Pasted) => {
      const text = pasted.text ?? pasted.svg
      const own = ownCopy.current
      if (own === '' || (own && text && fingerprint(text) === own)) return enginePaste()
      const taken = importOf(pasted, viewCenter())
      if (!taken) return enginePaste()
      // Paste in Place reads as disabled while the engine's clipboard is empty, which the import fills.
      if (host.readOnly || now.current.ui.state('command:edit.pasteInPlace', now.current.editor.byId.get('edit.pasteInPlace')?.menu).denied) return
      try {
        await job(t('busy.paste'), async () => {
          // Loading the clipboard is no step; Paste in Place is the one.
          if ((await run(taken.command, taken.params)) === null) return null
          return run('edit.pasteInPlace')
        })
      } finally {
        backToCanvas()
      }
    }
    const pastedOf = async (data: DataTransfer | null): Promise<Pasted> => {
      if (!data) return {}
      const pasted: Pasted = {}
      const svg = data.getData('image/svg+xml')
      const text = data.getData('text/plain')
      if (svg && isSvgText(svg)) pasted.svg = svg
      else if (text && isSvgText(text)) pasted.svg = text
      if (text) pasted.text = text
      for (const item of Array.from(data.items ?? [])) {
        if (item.kind !== 'file') continue
        const file = item.getAsFile()
        if (!file) continue
        if (!pasted.image && /^image\/(png|jpeg|gif|webp|tiff|bmp)$/i.test(file.type)) pasted.image = { bytes: new Uint8Array(await file.arrayBuffer()), type: file.type, name: file.name }
        else if (!pasted.svg && (file.type === 'image/svg+xml' || extensionOf(file.name) === 'svg')) pasted.svg = await file.text()
        else if (!pasted.pdf && (file.type === 'application/pdf' || extensionOf(file.name) === 'pdf')) pasted.pdf = new Uint8Array(await file.arrayBuffer())
      }
      return pasted
    }
    /** Edit › Paste from a menu: the browser lets the editor read the clipboard there, else the engine's paste. */
    const menuPaste = () => {
      if (!navigator.clipboard?.read) return enginePaste()
      void navigator.clipboard
        .read()
        .then(async (items) => {
          const pasted: Pasted = {}
          for (const item of items) {
            const read = async (type: string) => (item.types.includes(type) ? await item.getType(type) : null)
            const svg = await read('image/svg+xml')
            if (svg && !pasted.svg) pasted.svg = await svg.text()
            const text = await read('text/plain')
            if (text && pasted.text === undefined) {
              pasted.text = await text.text()
              if (!pasted.svg && isSvgText(pasted.text)) pasted.svg = pasted.text
            }
            const picture = item.types.find((type) => /^image\/(png|jpeg|gif|webp)$/.test(type))
            if (picture && !pasted.image) pasted.image = { bytes: new Uint8Array(await (await item.getType(picture)).arrayBuffer()), type: picture }
          }
          await pasteFrom(pasted)
        })
        .catch(() => enginePaste())
    }

    // Cmd+V: the browser's paste, caught by a text box the keyboard is sent to for a moment (a
    // browser fires it only there), carries what other applications copied; Cmd+C and Cmd+X copy
    // onto the system clipboard too.
    let pending: number | null = null
    const settlePaste = () => {
      if (pending !== null) window.clearTimeout(pending)
      pending = null
    }
    const idle = () => !dialogOpen() && !now.current.editing() && now.current.editor.session.doc != null
    const key = (event: KeyboardEvent) => {
      const command = event.metaKey || event.ctrlKey
      if (!command || event.altKey || event.defaultPrevented || isEditable(event.target) || !idle()) return
      if ((event.code === 'KeyC' || event.code === 'KeyX') && !event.shiftKey) {
        const id = event.code === 'KeyC' ? 'edit.copy' : 'edit.cut'
        if (!allowed(id)) return
        event.preventDefault()
        event.stopPropagation()
        copy(id)
        return
      }
      if (event.code === 'KeyV' && !event.shiftKey && !host.readOnly && catcher.current) {
        // Not prevented: the browser's own paste follows, into the catcher.
        event.stopPropagation()
        catcher.current.value = ''
        catcher.current.focus({ preventScroll: true })
        settlePaste()
        pending = window.setTimeout(() => {
          pending = null
          backToCanvas()
          enginePaste()
        }, PASTE_WAIT_MS)
      }
    }
    const pasted = (event: ClipboardEvent) => {
      const mine = event.target === catcher.current
      if (!mine && (isEditable(event.target) || !idle())) return
      if (host.readOnly) return
      event.preventDefault()
      settlePaste()
      if (mine) backToCanvas()
      void pastedOf(event.clipboardData).then(pasteFrom)
    }

    // ---------- files dragged in from the computer ----------
    let leaving: number | null = null
    const onCanvas = (event: DragEvent) => {
      if (host.readOnly || dialogOpen()) return false
      const target = event.target instanceof Element ? event.target : null
      return target != null && area.current?.contains(target) === true
    }
    const over = (event: DragEvent) => {
      if (!dragHasFiles(event.dataTransfer?.types)) return
      // Taken everywhere in the frame, so a drop never navigates it away (UI-K4).
      event.preventDefault()
      const here = onCanvas(event)
      if (event.dataTransfer) event.dataTransfer.dropEffect = here ? 'copy' : 'none'
      setDropping(here)
      if (leaving !== null) window.clearTimeout(leaving)
      leaving = window.setTimeout(() => setDropping(false), 200)
    }
    const dropped = (event: DragEvent) => {
      if (!dragHasFiles(event.dataTransfer?.types)) return
      event.preventDefault()
      setDropping(false)
      const files = Array.from(event.dataTransfer?.files ?? [])
      if (!onCanvas(event) || !files.length) return
      const at = docPoint(event.clientX, event.clientY)
      if (at) void placeFiles(files, at)
    }

    // ---------- the File menu ----------
    const pickOpen = async () => {
      if (!fileAbilities(host).open) return
      try {
        const path = await host.pick('open', [...OPEN_ACCEPT])
        if (path) await openInHost(host, path)
      } catch (error) {
        report(t('file.openFailed'), error)
      }
    }
    /** The drawing as a `.kvector` file's bytes, whatever it was read from. */
    const nativeBytes = async () => {
      const answer = await query<{ dataBase64?: string }>('file.serialize', { format: 'kvector' })
      if (!answer?.dataBase64) throw new Error('the drawing could not be written as a file')
      return fromBase64(answer.dataBase64)
    }
    const writeBeside = async (name: string, title: (path: string) => string) => {
      const started = busy.begin(t('busy.save'))
      try {
        const bytes = await nativeBytes()
        if (busy.end(started)) return
        const path = await host.exportFile(name, bytes)
        announceWritten(host, t, title(path), path)
      } catch (error) {
        if (!busy.end(started)) report(t('file.saveFailed'), error)
      }
    }
    const save = () => {
      const { editor: e } = now.current
      if (host.readOnly) return
      if (isNative(e.session.path)) return void e.save()
      void writeBeside(nativeName(e.session.path), (path) => t('file.savedNative', { path }))
    }
    const saveCopy = () => void writeBeside(copyName(now.current.editor.session.path), (path) => t('file.savedCopy', { path }))
    const newFromTemplate = async () => {
      const { hostPick: picker } = now.current
      if (!picker.can) return
      try {
        const file = await picker.pick('template', [...TEMPLATE_ACCEPT])
        if (!file) return
        const engine = await KitEngine.open(engineManifest(), file.name, file.bytes)
        try {
          const answer = JSON.parse(await engine.execute('file.serialize', { format: 'kvector' })) as { dataBase64?: string }
          if (!answer.dataBase64) throw new Error('the template could not be written as a drawing')
          const path = await host.exportFile(`${t('dialog.untitled')}.kvector`, fromBase64(answer.dataBase64))
          announceWritten(host, t, t('dialog.created', { path }), path)
        } finally {
          engine.dispose()
        }
      } catch (error) {
        report(t('dialog.createFailed'), error)
      }
    }
    const place = async () => {
      const { hostPick: picker } = now.current
      if (!picker.can || host.readOnly) return
      try {
        const file = await picker.pick('place', [...PLACE_ACCEPT])
        if (file) await placeBytes(file.name, new Uint8Array(file.bytes), viewCenter())
      } catch (error) {
        report(t('file.placeFailed'), error)
      } finally {
        backToCanvas()
      }
    }

    api.current = {
      open: () => void pickOpen(),
      save,
      saveCopy,
      newFromTemplate: () => void newFromTemplate(),
      place: () => void place(),
      handles: (command) => COPY_COMMANDS.has(command) || command === 'edit.paste',
      run: (command) => {
        if (COPY_COMMANDS.has(command)) copy(command)
        else if (command === 'edit.paste' && !host.readOnly) menuPaste()
      },
    }
    window.addEventListener('keydown', key, true)
    document.addEventListener('paste', pasted, true)
    window.addEventListener('dragover', over, true)
    window.addEventListener('drop', dropped, true)
    return () => {
      settlePaste()
      if (leaving !== null) window.clearTimeout(leaving)
      window.removeEventListener('keydown', key, true)
      document.removeEventListener('paste', pasted, true)
      window.removeEventListener('dragover', over, true)
      window.removeEventListener('drop', dropped, true)
    }
    // The handlers read the editor through `now`; they are set up once per host.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [host, t, busy])

  // The drop target's mark: the canvas outlined, with what a drop does.
  useEffect(() => {
    const element = area.current
    if (!element) return
    if (dropping) {
      element.dataset.drop = 'true'
      element.dataset.dropLabel = t('drop.place')
    } else {
      delete element.dataset.drop
      delete element.dataset.dropLabel
    }
  }, [dropping, t, area])

  return <textarea ref={catcher} className="vector-paste-catcher" aria-hidden="true" tabIndex={-1} data-testid="vector-paste-catcher" />
}
