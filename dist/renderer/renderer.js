"use strict";
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
const jsx_runtime_1 = require("react/jsx-runtime");
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
const client_1 = require("react-dom/client");
require("./index.css");
const react_1 = require("react");
const App_1 = __importDefault(require("./App"));
const HelpWindow_1 = require("./components/HelpWindow");
const ThemeContext_1 = require("./themes/ThemeContext");
const editor_main_1 = __importDefault(require("monaco-editor/esm/vs/editor/editor.main"));
// Suppress Monaco Editor's harmless "Canceled" errors that occur during
// normal operations like dismissing the "Go to Symbol" dialog with Escape.
// These must be registered early before React/webpack error overlays catch them.
window.addEventListener('error', (event) => {
    if (event.error?.message === 'Canceled') {
        event.preventDefault();
        event.stopImmediatePropagation();
        return false;
    }
}, true); // Use capture phase to intercept before other handlers
window.addEventListener('unhandledrejection', (event) => {
    if (event.reason?.message === 'Canceled') {
        event.preventDefault();
        event.stopImmediatePropagation();
        return false;
    }
}, true);
// Make monaco available globally
window.monaco = editor_main_1.default;
// Set up main process log forwarding to renderer console
// This allows viewing main process logs in the renderer's DevTools
window.electronAPI.onMainLog((entry) => {
    const prefix = '[main]';
    const args = entry.args.map((arg) => {
        // Reconstruct Error objects
        if (arg && typeof arg === 'object' && '__error' in arg) {
            const errorLike = arg;
            const err = new Error(errorLike.message);
            err.name = errorLike.name;
            if (errorLike.stack) {
                err.stack = errorLike.stack;
            }
            return err;
        }
        return arg;
    });
    switch (entry.level) {
        case 'log':
            console.log(prefix, ...args);
            break;
        case 'info':
            console.info(prefix, ...args);
            break;
        case 'warn':
            console.warn(prefix, ...args);
            break;
        case 'error':
            console.error(prefix, ...args);
            break;
        case 'debug':
            console.debug(prefix, ...args);
            break;
    }
});
const root = document.getElementById('root');
if (!root) {
    throw new Error('Failed to find the root element');
}
const isHelpWindow = window.location.hash === '#help';
(0, client_1.createRoot)(root).render((0, jsx_runtime_1.jsx)(react_1.StrictMode, { children: (0, jsx_runtime_1.jsx)(ThemeContext_1.ThemeProvider, { children: isHelpWindow ? (0, jsx_runtime_1.jsx)(HelpWindow_1.HelpWindow, {}) : (0, jsx_runtime_1.jsx)(App_1.default, {}) }) }));
//# sourceMappingURL=renderer.js.map