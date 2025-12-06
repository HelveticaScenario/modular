import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import path from 'path'

// https://vite.dev/config/
export default defineConfig({
  plugins: [react({
    babel: {
      plugins: ['babel-plugin-react-compiler'],
    },
  })],
  build: {
    outDir: path.resolve(__dirname, '../modular_server/static'),
    emptyOutDir: true,
  },
  server: {
    proxy: {
      '/ws': {
        target: 'ws://127.0.0.1:7812',
        ws: true
      },
    },
  },
})
