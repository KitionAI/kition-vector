/**
 * The engine's menu rows the editor does not show, each with why (plan 2026-11-25, stage 1): a
 * row that would only fail with the engine's English error, or apply defaults nobody asked for,
 * is left out until the dialog or panel it needs is built. Agents still run every command; only
 * the menus leave them out. Pure, so `node --test` checks it.
 */

/** A menu row as this table reads it: the command and the menu it is drawn under. */
export type HiddenRow = { command: string; path: readonly string[] }

const HOST_FILE = 'The host opens, saves, and closes documents.'
const HOST_WRITES = 'Writes a file the page cannot keep; File › Export… writes through the host instead.'
const LOCAL_FILE = 'Reads a file from a local folder, which an editor in the browser cannot reach.'

/** Commands a browser editor never shows. */
const NEVER: Readonly<Record<string, string>> = {
  'file.new': "The host makes documents; Kition's own File › New… row asks for the size and writes the file through the host.",
  'file.close': HOST_FILE,
  'file.closeAll': HOST_FILE,
  'file.saveAs': HOST_FILE,
  'file.saveCopy': HOST_FILE,
  'file.revert': 'The host reverts a document to its saved copy.',
  'file.exportForScreens': HOST_WRITES,
  'file.exportSelection': HOST_WRITES,
  'file.exportForOffice': HOST_WRITES,
  'file.saveAsTemplate': HOST_WRITES,
  'plugin.reload': 'Reloads plug-ins from a local folder, which an editor in the browser cannot reach.',
}

/**
 * Rows hidden until what they need is built, with the stage of plan 2026-11-25 that brings them
 * back where one does.
 */
const NOT_BUILT: Readonly<Record<string, string>> = {
  'file.place': 'Needs a file picked through the host (stage 7); the engine refuses it without one.',
  'view.printTiling': 'Shows the pages of the print settings on the canvas, which the canvas does not draw (the Print Tiling tool is off the rail); printing goes through the host.',
  'file.newFromTemplate': 'Needs a template file picked through the host; the engine refuses it without one.',
  'file.setUnits': "Kition's own File › Document Setup… row sets the units with the rest of the document's settings.",
  'recolor.apply': 'Recolor Artwork needs a colour-mapping dialog of its own, cut for the launch (plan section 1).',
  'edit.colorSettings': 'Needs a list of colour profiles to choose from; the engine has none to offer in the browser.',
  'edit.assignProfile': 'Needs a list of colour profiles to choose from; the engine has none to offer in the browser.',
  'edit.findNext': 'Finds what Find and Replace… last searched for, which the editor does not keep yet; Find and Replace… finds it.',
  'text.insert': 'Inserts a glyph into the text being edited: it needs the Glyphs panel, not built yet; type it with the Type tool.',
  'text.insertInline': 'Inserts a symbol into text: symbols are cut for the launch (plan section 1).',
  'graph.setData': "Graphs are cut for the launch (their tools are off the rail); the data sheet is not built.",
  'graph.setType': "Graphs are cut for the launch (their tools are off the rail); the graph type dialog is not built.",
  'view.saved.new': 'Saves the current view, which the editor does not hand to the engine yet: saved views are not built in the editor (plan 2026-11-25, stage 6 left them out).',
  'view.saved.edit': 'Needs the list of saved views, which the editor does not show: saved views are not built in the editor (plan 2026-11-25, stage 6 left them out).',
  'select.editSaved': 'Needs the list of saved selections to choose from, not built yet; Select › Recall Selection recalls the last one.',
}

/**
 * The Window menu's panel commands (the engine lists each panel's own menu under Window): they act
 * on a panel's current row or option, and run from that panel's own buttons and menu once the panel
 * is built (stages 3 and 4); Window lists the panels instead (`panels/dockModel.ts`). Pathfinder's rows stay: they act on the
 * selection, and stage 1's menu test keeps them under Window (the side panel has all ten since
 * stage 3).
 */
const PANELS: Readonly<Record<string, string>> = {
  Align: 'Align panel commands: Properties › Align aligns, distributes, spaces and chooses what to align to.',
  'Magic Wand': 'Magic Wand panel options: the panel is not built.',
  Transparency: 'Transparency panel commands: Properties › Appearance sets the opacity and blending mode; opacity masks need a Transparency panel of their own, not built yet.',
  Stroke: 'Stroke panel options: Properties › Stroke sets every one of them on the selection.',
  Appearance: 'Appearance panel commands: they act on the panel’s current fill or stroke, which comes with the Appearance panel.',
  'Graphic Styles': 'Graphic Styles panel commands: they act on the panel’s current style; graphic styles come after the launch.',
  Swatches: 'Swatches panel commands: they act on the panel’s picked swatches; the Swatches panel’s own buttons (new, options, colour group, delete) run them.',
  Gradient: 'Gradient panel options: Properties › Gradient edits the gradient’s type, angle and stops.',
  Layers: 'Layers panel commands: the Layers panel’s buttons, its menu and a row’s right-click menu run them on the rows it highlights.',
  Artboards: 'Artboards panel commands: the Artboards panel’s buttons and menu run them on its active artboard; Artboard Options opens from its gear.',
  Type: 'Character and paragraph style commands: styles come after the launch.',
  Tabs: 'Tabs panel commands: the Tabs panel is not built.',
  Variables: 'Variables and data sets are cut for the launch (plan section 1).',
  'Document Info': 'Document Info is a panel of its own, not built yet.',
  'Pattern Options': 'Pattern Options acts on the pattern being edited, in a panel not built yet.',
  'Swatch Libraries': 'Swatch libraries are cut for the launch (plan section 1).',
  'Graphic Style Libraries': 'Graphic style libraries are cut for the launch (plan section 1).',
  'Color Themes': 'Color themes are a panel of their own, not built yet.',
}
const KEPT_PANELS = new Set(['Pathfinder'])

/** Why the menus leave out `row`, or null when they show it. */
export function hiddenReason(row: HiddenRow): string | null {
  const own = NEVER[row.command] ?? NOT_BUILT[row.command]
  if (own) return own
  const [top, panel] = row.path
  if (top === 'Window' && panel && !KEPT_PANELS.has(panel)) return PANELS[panel] ?? 'A panel command: it comes back in its panel’s menu when the panel is built.'
  return null
}

/** Whether the menus leave out `row`. */
export function isHiddenRow(row: HiddenRow) {
  return hiddenReason(row) !== null
}

/** Whether the menus leave out `command` (drawn under its engine menu `menu`). */
export function isHiddenCommand(command: string, menu: readonly string[] | null | undefined) {
  return isHiddenRow({ command, path: menu ?? [] })
}

/**
 * Whether `command` is left out wherever a person could reach it, its shortcut included: a host
 * command, or one that fails without what is not built. Panel commands are only off the menus:
 * their shortcuts (New Layer's Cmd+L) still run them.
 */
export function isHiddenEverywhere(command: string) {
  return NEVER[command] !== undefined || NOT_BUILT[command] !== undefined
}
