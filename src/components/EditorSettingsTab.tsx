import React, { useState, useEffect } from 'react';
import type {
    AppConfig,
    MonospaceFont,
    BundledFont,
    SystemFont,
} from '../ipcTypes';
import type { AppTheme } from '../themes/types';

type CursorStyle =
    | 'line'
    | 'block'
    | 'underline'
    | 'line-thin'
    | 'block-outline'
    | 'underline-thin';

const BUNDLED_FONTS: BundledFont[] = [
    'Fira Code',
    'JetBrains Mono',
    'Cascadia Code',
    'Source Code Pro',
    'IBM Plex Mono',
    'Hack',
    'Inconsolata',
    'Monaspace Neon',
    'Monaspace Argon',
    'Monaspace Xenon',
    'Monaspace Krypton',
    'Monaspace Radon',
    'Geist Mono',
    'Iosevka',
    'Victor Mono',
    'Roboto Mono',
    'Maple Mono',
    'Commit Mono',
    '0xProto',
    'Intel One Mono',
    'Mononoki',
    'Anonymous Pro',
    'Recursive',
];

const SYSTEM_FONTS: SystemFont[] = ['SF Mono', 'Monaco', 'Menlo', 'Consolas'];

const CURSOR_STYLES: CursorStyle[] = [
    'line',
    'block',
    'underline',
    'line-thin',
    'block-outline',
    'underline-thin',
];

interface EditorSettingsTabProps {
    config: AppConfig;
    themes: AppTheme[];
    onConfigChange: (partial: Partial<AppConfig>) => void;
}

/**
 * Detect whether a font is installed by measuring text against baseline fonts.
 * If the candidate font renders at a different width than all baselines, it's installed.
 */
function isFontInstalled(fontName: string): boolean {
    const canvas = document.createElement('canvas');
    const ctx = canvas.getContext('2d');
    if (!ctx) return false;

    const testString = 'mmmmmmmmmmlli1WWW@#$';
    const baselines = ['monospace', 'sans-serif', 'serif'] as const;
    const size = '72px';

    for (const baseline of baselines) {
        ctx.font = `${size} ${baseline}`;
        const baselineWidth = ctx.measureText(testString).width;

        ctx.font = `${size} "${fontName}", ${baseline}`;
        const candidateWidth = ctx.measureText(testString).width;

        if (candidateWidth !== baselineWidth) {
            return true;
        }
    }
    return false;
}

export function EditorSettingsTab({
    config,
    themes,
    onConfigChange,
}: EditorSettingsTabProps) {
    const [availableSystemFonts, setAvailableSystemFonts] = useState<
        SystemFont[]
    >([]);

    useEffect(() => {
        // Detect which system fonts are actually installed via canvas measurement
        const available = SYSTEM_FONTS.filter(isFontInstalled);
        setAvailableSystemFonts(available);
    }, []);

    return (
        <div className="settings-tab-content">
            {/* Theme */}
            <div className="settings-section">
                <h3>Color Theme</h3>
                <select
                    className="device-select"
                    value={config.theme || 'modular-dark'}
                    onChange={(e) => onConfigChange({ theme: e.target.value })}
                >
                    {themes.map((t) => (
                        <option key={t.id} value={t.id}>
                            {t.name}
                        </option>
                    ))}
                </select>
            </div>

            {/* Font */}
            <div className="settings-section">
                <h3>Font</h3>
                <select
                    className="device-select"
                    value={config.font || 'Fira Code'}
                    onChange={(e) =>
                        onConfigChange({
                            font: e.target.value as MonospaceFont,
                        })
                    }
                >
                    <optgroup label="Bundled">
                        {BUNDLED_FONTS.map((f) => (
                            <option key={f} value={f}>
                                {f}
                            </option>
                        ))}
                    </optgroup>
                    {availableSystemFonts.length > 0 && (
                        <optgroup label="System">
                            {availableSystemFonts.map((f) => (
                                <option key={f} value={f}>
                                    {f}
                                </option>
                            ))}
                        </optgroup>
                    )}
                </select>
            </div>

            {/* Font Size */}
            <div className="settings-section">
                <h3>Font Size</h3>
                <div className="settings-row">
                    <input
                        type="range"
                        className="settings-range"
                        min={8}
                        max={72}
                        step={1}
                        value={config.fontSize ?? 17}
                        onChange={(e) =>
                            onConfigChange({ fontSize: Number(e.target.value) })
                        }
                    />
                    <span className="settings-range-value">
                        {config.fontSize ?? 17}px
                    </span>
                </div>
            </div>

            {/* Font Ligatures */}
            <div className="settings-section">
                <h3>Font Ligatures</h3>
                <label className="settings-toggle-label">
                    <input
                        type="checkbox"
                        className="settings-toggle"
                        checked={config.fontLigatures ?? true}
                        onChange={(e) =>
                            onConfigChange({ fontLigatures: e.target.checked })
                        }
                    />
                </label>
            </div>

            {/* Cursor Style */}
            <div className="settings-section">
                <h3>Cursor Style</h3>
                <select
                    className="device-select"
                    value={config.cursorStyle || 'block'}
                    onChange={(e) =>
                        onConfigChange({
                            cursorStyle: e.target.value as CursorStyle,
                        })
                    }
                >
                    {CURSOR_STYLES.map((s) => (
                        <option key={s} value={s}>
                            {s}
                        </option>
                    ))}
                </select>
            </div>
        </div>
    );
}
