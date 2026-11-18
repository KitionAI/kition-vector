import { createRoot } from 'react-dom/client'
import { connectHost, createKitStrings, loadPhrases, withPhrases } from '@kition/kit-editor'
import '@kition/kit-editor/styles.css'
import './app.css'
import { App } from './App'
import { CATALOGS, PHRASES } from './strings'
import manifest from '../../kit.json'
import './commandIcons'

const root = createRoot(document.getElementById('root')!)

// Opened outside a host, the page says where it belongs instead of waiting forever.
const alone = window.setTimeout(() => {
  const t = createKitStrings(CATALOGS, navigator.language)
  root.render(<div className="kui-alone">{t('noHost')}</div>)
}, 3000)

// The translation catalogs of the English labels and command titles, and this editor's own.
const UPSTREAM = import.meta.glob('./i18n/upstream/*.tsv', { query: '?raw', import: 'default' })

void connectHost({ id: 'kit.vector', version: '0.1.0', capabilities: ['ai', 'replica'], brand: manifest.ui.brand, display: manifest.ui.display }).then(async (host) => {
  window.clearTimeout(alone)
  const phrase = withPhrases(await loadPhrases(UPSTREAM, host.init.locale), PHRASES, host.init.locale)
  // No StrictMode: its doubled effects would open the document twice, in two engine workers.
  root.render(<App host={host} phrase={phrase} />)
})
