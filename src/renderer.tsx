/**
 * This file will automatically be loaded by vite and run in the "renderer" context.
 * To learn more about the differences between the "main" and the "renderer" context in
 * Electron, visit:
 *
 * https://electronjs.org/docs/tutorial/process-model
 *
 * By default, Node.js integration in this file is disabled. When enabling Node.js integration
 * in a renderer process, please be aware of potential security implications. You can read
 * more about security risks here:
 *
 * https://electronjs.org/docs/tutorial/security
 *
 * To enable Node.js integration in this file, open up `main.ts` and enable the `nodeIntegration`
 * flag:
 *
 * ```
 *  // Create the browser window.
 *  mainWindow = new BrowserWindow({
 *    width: 800,
 *    height: 600,
 *    webPreferences: {
 *      nodeIntegration: true
 *    }
 *  });
 * ```
 */

import { createRoot } from 'react-dom/client';
import './index.css';
import { StrictMode } from 'react';
import App from './App';
import { HelpWindow } from './components/HelpWindow';
import { SchemasProvider } from './SchemaContext';
import monaco from 'monaco-editor/esm/vs/editor/editor.main';

// Make monaco available globally
(window as any).monaco = monaco;

const root = document.getElementById('root');
if (!root) {
    throw new Error('Failed to find the root element');
}

const isHelpWindow = window.location.hash === '#help';

createRoot(root).render(
    <StrictMode>
        <SchemasProvider>
            {isHelpWindow ? <HelpWindow /> : <App />}
        </SchemasProvider>
    </StrictMode>,
);
