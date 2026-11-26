import { Button, Dialog, useKitEditor, useKitTools, useUi } from '@kition/sdk/editor'
import { isHiddenCommand } from './hidden'

export type HelpTopic = 'about' | 'shortcuts'

/** A shortcut of Kition's own menu rows (File › Document Setup…, Effect's first two), as Help lists them. */
export type OwnShortcut = { label: string; shortcut: string }

/**
 * Help's dialogs (plan 2026-11-25, stage 1): About (name and version) and Keyboard Shortcuts (the
 * tools' keys, and the shortcuts of the menu rows the editor shows, Kition's own included).
 */
export function HelpDialog({ topic, version, own, onClose }: { topic: HelpTopic; version: string; own: readonly OwnShortcut[]; onClose: () => void }) {
  const { t, phrase, commands, testId } = useKitEditor()
  const tools = useKitTools()
  const ui = useUi()
  const close = (
    <Button variant="primary" onClick={onClose} data-testid={`${testId}-help-close`}>
      {t('close')}
    </Button>
  )
  if (topic === 'about') {
    return (
      <Dialog title={t('help.about')} actions={close} onClose={onClose} testId={`${testId}-about`}>
        <p>
          <strong>Kition Vector</strong>
        </p>
        <p>{t('help.version', { version })}</p>
      </Dialog>
    )
  }
  // The tools by key, in the rail's order (a key that picks several lists them all).
  const byKey = new Map<string, string[]>()
  for (const tool of tools?.tools ?? []) {
    if (!tool.shortcut || ui.state(`tool:${tool.id}`).denied) continue
    byKey.set(tool.shortcut, [...(byKey.get(tool.shortcut) ?? []), phrase(tool.label)])
  }
  // The menu rows the editor shows that have a shortcut, under their menus.
  const rows = commands.filter((command) => command.shortcut && command.menu.length && !isHiddenCommand(command.id, command.menu) && !ui.state(`command:${command.id}`, command.menu).hidden)
  return (
    <Dialog title={t('help.shortcuts')} actions={close} onClose={onClose} testId={`${testId}-shortcuts`}>
      <div className="vector-shortcuts">
        <h3>{t('help.tools')}</h3>
        <dl>
          {[...byKey].map(([key, labels]) => (
            <div key={key}>
              <dt>{labels.join(', ')}</dt>
              <dd>
                <kbd>{key}</kbd>
              </dd>
            </div>
          ))}
        </dl>
        <h3>{t('help.menus')}</h3>
        <dl>
          {own.map((item) => (
            <div key={item.label}>
              <dt>{item.label}</dt>
              <dd>
                <kbd>{item.shortcut}</kbd>
              </dd>
            </div>
          ))}
          {rows.map((command) => (
            <div key={command.id}>
              <dt>{[...command.menu, command.title].map(phrase).join(' › ')}</dt>
              <dd>
                <kbd>{command.shortcut}</kbd>
              </dd>
            </div>
          ))}
        </dl>
      </div>
    </Dialog>
  )
}
