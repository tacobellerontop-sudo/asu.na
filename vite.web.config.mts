// Runs the renderer in a normal browser with a mock API (no Electron),
// handy for UI work: `npm run web`.
import { resolve } from 'node:path'
import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

export default defineConfig({
  root: resolve(import.meta.dirname, 'src/renderer'),
  plugins: [react()]
})
