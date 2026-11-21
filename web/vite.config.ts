import { fileURLToPath } from 'node:url'
import react from '@vitejs/plugin-react'
import { defineConfig } from 'vite'

// The kit package is served from wherever a host puts it, so every asset path is relative.
export default defineConfig({
  base: './',
  plugins: [react()],
  resolve: {
    // KITION_LINK=1 may link @kition/kit-editor from a kition-sdk checkout (.pnpmfile.cjs); one React for the page.
    dedupe: ['react', 'react-dom'],
  },
  optimizeDeps: {
    exclude: ['@kition/kit-editor', '@kition/kit-host', '@kition/kit-ui-protocol', '@kition/kit-dev-host'],
    // The published kit editor reaches a CommonJS shim (through its UI library); excluded above, it
    // is not scanned, so the shim is named here to be converted for the browser.
    include: ['@kition/kit-editor > use-sync-external-store/shim'],
  },
  worker: { format: 'es' },
  // The development host (dev.html) and the kition-sdk and Tessera checkouts KITION_LINK=1 links.
  server: {
    port: 5185,
    fs: { allow: [fileURLToPath(new URL('..', import.meta.url)), fileURLToPath(new URL('../../kition-sdk', import.meta.url)), fileURLToPath(new URL('../../kition-tessera', import.meta.url))] },
  },
  build: {
    outDir: 'dist',
    rollupOptions: { input: { index: fileURLToPath(new URL('index.html', import.meta.url)), dev: fileURLToPath(new URL('dev.html', import.meta.url)) } },
  },
})
