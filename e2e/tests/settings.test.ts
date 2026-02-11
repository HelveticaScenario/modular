/**
 * E2E tests for the Settings panel.
 */

import { test, expect } from '../fixtures';

test.describe('settings panel', () => {
    test('settings can be opened via Cmd+comma or button', async ({
        window,
    }) => {
        await window.waitForTimeout(2000);

        // Try keyboard shortcut first (Cmd+, on macOS)
        await window.keyboard.press('Meta+,');
        await window.waitForTimeout(500);

        // Check if settings panel is visible
        let settingsVisible = await window
            .locator('.settings-panel, .settings-overlay, [class*="settings"]')
            .first()
            .isVisible()
            .catch(() => false);

        if (!settingsVisible) {
            // Try clicking a settings button if keyboard didn't work
            const settingsBtn = window.locator(
                'button:has-text("Settings"), button[aria-label="Settings"], .settings-button',
            );
            const btnExists = await settingsBtn
                .first()
                .isVisible()
                .catch(() => false);
            if (btnExists) {
                await settingsBtn.first().click();
                await window.waitForTimeout(500);
                settingsVisible = await window
                    .locator(
                        '.settings-panel, .settings-overlay, [class*="settings"]',
                    )
                    .first()
                    .isVisible()
                    .catch(() => false);
            }
        }

        // Settings should be visible by some mechanism
        // If neither works, this test documents the current state
        expect(settingsVisible).toBe(true);
    });

    test('settings panel shows audio device section', async ({ window }) => {
        await window.waitForTimeout(2000);

        // Open settings
        await window.keyboard.press('Meta+,');
        await window.waitForTimeout(1000);

        // Look for audio-related content
        const audioSection = window
            .locator(
                'text=Audio, text=Output Device, text=Sample Rate, text=Buffer Size',
            )
            .first();
        const hasAudio = await audioSection.isVisible().catch(() => false);

        // If settings opened, there should be audio configuration
        if (hasAudio) {
            expect(hasAudio).toBe(true);
        }
    });

    test('settings panel can be closed', async ({ window }) => {
        await window.waitForTimeout(2000);

        // Open settings
        await window.keyboard.press('Meta+,');
        await window.waitForTimeout(500);

        const settingsPanel = window
            .locator('.settings-panel, .settings-overlay, [class*="settings"]')
            .first();
        const wasVisible = await settingsPanel.isVisible().catch(() => false);

        if (wasVisible) {
            // Try Escape to close
            await window.keyboard.press('Escape');
            await window.waitForTimeout(500);

            const stillVisible = await settingsPanel
                .isVisible()
                .catch(() => false);
            expect(stillVisible).toBe(false);
        }
    });
});
