import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
// import path from 'path'
// import monacoEditorEsmPlugin from 'vite-plugin-monaco-editor-esm'

const prefix = `monaco-editor/esm/vs`;

// https://vite.dev/config/
export default defineConfig({
  plugins: [
    react({
      babel: {
        plugins: ['babel-plugin-react-compiler'],
      },
    }),
    // monacoEditorEsmPlugin(),
  ],
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
  optimizeDeps: {
    include: [
      `${prefix}/language/json/json.worker`,
      `${prefix}/language/css/css.worker`,
      `${prefix}/language/html/html.worker`,
      `${prefix}/language/typescript/ts.worker`,
      `${prefix}/editor/editor.worker`,
    ],
  },
})
