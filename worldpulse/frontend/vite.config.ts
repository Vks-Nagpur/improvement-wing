import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

// WORLDPULSE_BASE is set by the Pages workflow to "/<repo>/".
export default defineConfig({
  base: process.env.WORLDPULSE_BASE || './',
  plugins: [react()],
  server: { host: '127.0.0.1' },
})
