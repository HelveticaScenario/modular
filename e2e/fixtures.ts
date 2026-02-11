/**
 * Shared Playwright fixtures for Electron E2E tests.
 *
 * Provides `electronApp` and `window` fixtures that launch the app once per
 * test file and expose the first BrowserWindow as a Playwright Page.
 *
 * Requirements:
 *   - The webpack build must exist (.webpack/main and .webpack/renderer).
 *     Run `yarn start` once, or `npx electron-forge build` before running E2E.
 *   - Set E2E_TEST=1 env var so the renderer exposes window.__TEST_API__.
 */

import { test as base, type Page } from '@playwright/test';
import { _electron as electron, type ElectronApplication } from 'playwright';
import * as path from 'path';
import * as fs from 'fs';
import * as os from 'os';

// Resolve paths relative to the project root
const projectRoot = path.resolve(__dirname, '..');
const electronBin = path.join(projectRoot, 'node_modules', '.bin', 'electron');
const mainEntry = path.join(projectRoot, '.webpack', 'main');

export type TestFixtures = {
    electronApp: ElectronApplication;
    window: Page;
};

/**
 * Extended Playwright test with `electronApp` and `window` fixtures.
 *
 * Usage:
 *   import { test, expect } from './fixtures';
 *   test('my test', async ({ window }) => { ... });
 */
export const test = base.extend<TestFixtures>({
    // eslint-disable-next-line no-empty-pattern
    electronApp: async ({}, use) => {
        // Create a temp workspace directory so the app doesn't show the
        // "Open Folder" empty-state screen during tests.
        const tmpWorkspace = fs.mkdtempSync(
            path.join(os.tmpdir(), 'modular-e2e-'),
        );

        const app = await electron.launch({
            args: [mainEntry],
            executablePath: electronBin,
            env: {
                ...process.env,
                E2E_TEST: '1',
                E2E_WORKSPACE: tmpWorkspace,
                // Disable hardware acceleration for CI/headless stability
                ELECTRON_DISABLE_GPU: '1',
                // Prevent the app from trying to restore window positions
                NODE_ENV: 'test',
            },
        });

        // Override the workspace IPC handler in the main process so the
        // renderer sees an open workspace (avoids the "Open Folder" screen).
        // This works even against an already-built webpack bundle.
        await app.evaluate(({ ipcMain }, workspace) => {
            ipcMain.removeHandler('modular:fs:get-workspace');
            ipcMain.handle('modular:fs:get-workspace', () => ({
                path: workspace,
            }));

            ipcMain.removeHandler('modular:fs:list-files');
            ipcMain.handle('modular:fs:list-files', () => []);
        }, tmpWorkspace);

        await use(app);
        await app.close();

        // Clean up the temp workspace
        fs.rmSync(tmpWorkspace, { recursive: true, force: true });
    },

    window: async ({ electronApp }, use) => {
        const window = await electronApp.firstWindow();
        // Wait for the renderer to be fully loaded
        await window.waitForLoadState('domcontentloaded');

        // Reload the page so the renderer picks up the overridden workspace
        // IPC handler (the initial load may have already queried before the
        // override was installed).
        await window.reload();
        await window.waitForLoadState('domcontentloaded');
        // Give React time to mount and render the UI
        await window.waitForLoadState('networkidle');
        await use(window);
    },
});

export { expect } from '@playwright/test';
