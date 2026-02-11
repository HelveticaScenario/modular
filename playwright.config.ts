/**
 * Playwright E2E test configuration for the Operator Electron app.
 *
 * Usage:
 *   yarn test:e2e           — run all E2E tests
 *   yarn test:e2e:update    — run with snapshot update
 *
 * Prerequisites:
 *   The webpack build must exist in .webpack/ (run `yarn start` once to create it).
 *   The webServer config below will automatically serve the renderer files on
 *   port 3000 (or reuse your existing `yarn start` dev server if it's already running).
 */

import { defineConfig } from '@playwright/test';

export default defineConfig({
    testDir: './e2e',
    timeout: 60_000,
    retries: 0,
    workers: 1, // Electron tests must run serially (one app instance at a time)
    reporter: [['list'], ['html', { open: 'never' }]],
    use: {
        trace: 'retain-on-failure',
        screenshot: 'only-on-failure',
    },

    /**
     * Serve the webpack renderer output on port 3000.
     *
     * The compiled main process loads the renderer from
     * http://localhost:3000/main_window/index.html (baked in by electron-forge's
     * WebpackPlugin). In dev mode (`yarn start`), this is handled by webpack-dev-server.
     * For E2E tests we serve the static build with `serve`.
     *
     * If `yarn start` is already running, `reuseExistingServer: true` skips this.
     */
    webServer: {
        command:
            'python3 -m http.server 3000 --directory .webpack/renderer --bind 127.0.0.1',
        port: 3000,
        reuseExistingServer: true,
        timeout: 15_000,
    },
});
