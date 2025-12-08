import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import path from 'path'
import monacoEditorEsmPlugin from 'vite-plugin-monaco-editor-esm'

// https://vite.dev/config/
export default defineConfig({
  plugins: [react({
    babel: {
      plugins: ['babel-plugin-react-compiler'],
    },
  }), monacoEditorEsmPlugin()],
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
      '/lsp': {
        target: 'ws://127.0.0.1:7812',
        ws: true,
      },
    },
  },
})
