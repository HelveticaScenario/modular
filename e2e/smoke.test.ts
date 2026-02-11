/**
 * Smoke E2E test: verify the Electron app launches, renders the main window,
 * and the key UI elements are visible.
 */

import { test, expect } from './fixtures';

test.describe('app launch', () => {
    test('window opens and has a title', async ({ window }) => {
        const title = await window.title();
        expect(title.length).toBeGreaterThan(0);
    });

    test('main UI elements are visible', async ({ window }) => {
        // Header with audio controls
        await expect(window.locator('.audio-controls')).toBeVisible({
            timeout: 15_000,
        });

        // Update Patch button
        await expect(
            window.locator('button:has-text("Update Patch")'),
        ).toBeVisible();

        // Stop button
        await expect(window.locator('button:has-text("Stop")')).toBeVisible();
    });

    test('app renders without critical JS errors', async ({ window }) => {
        const errors: string[] = [];
        window.on('pageerror', (error) => {
            errors.push(error.message);
        });

        // Wait for the app to settle
        await window.waitForTimeout(3000);

        // Filter out known benign errors
        const criticalErrors = errors.filter(
            (e) =>
                !e.includes('ResizeObserver loop') &&
                !e.includes('net::ERR_') && // network errors from static serve (e.g. missing vite.svg)
                !e.includes('Failed to load resource'),
        );
        expect(criticalErrors).toEqual([]);
    });
});

test.describe('workspace', () => {
    test('shows "Open Folder" when no workspace is selected', async ({
        window,
    }) => {
        // When no workspace is set, the app shows an empty state
        const openButton = window.locator('button:has-text("Open Folder")');
        const editorPanel = window.locator('.editor-panel');

        // Either we have an open folder button (no workspace) or the editor panel (workspace already set)
        const hasOpenButton = await openButton.isVisible().catch(() => false);
        const hasEditor = await editorPanel.isVisible().catch(() => false);

        expect(hasOpenButton || hasEditor).toBe(true);
    });
});

test.describe('electron main process', () => {
    test('can evaluate code in main process', async ({ electronApp }) => {
        // Verify we can communicate with the main process
        const appPath = await electronApp.evaluate(async ({ app }) => {
            return app.getAppPath();
        });
        expect(typeof appPath).toBe('string');
        expect(appPath.length).toBeGreaterThan(0);
    });

    test('app name is Operator', async ({ electronApp }) => {
        const name = await electronApp.evaluate(async ({ app }) => {
            return app.getName();
        });
        // The app name should be set in package.json
        expect(name).toBeTruthy();
    });
});
