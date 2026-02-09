import React from 'react';
import type { AppConfig, PrettierConfig } from '../ipcTypes';

const TRAILING_COMMA_OPTIONS = ['all', 'es5', 'none'] as const;

interface FormatterSettingsTabProps {
    config: AppConfig;
    onConfigChange: (partial: Partial<AppConfig>) => void;
}

export function FormatterSettingsTab({ config, onConfigChange }: FormatterSettingsTabProps) {
    const prettier = config.prettier ?? {};

    const updatePrettier = (patch: Partial<PrettierConfig>) => {
        onConfigChange({ prettier: { ...prettier, ...patch } });
    };

    return (
        <div className="settings-tab-content">
            <p className="settings-description">
                Configure Prettier formatting options for the patch editor. These are merged with built-in defaults.
            </p>

            {/* Print Width */}
            <div className="settings-section">
                <h3>Print Width</h3>
                <div className="settings-row">
                    <input
                        type="number"
                        className="settings-number-input"
                        min={20}
                        max={200}
                        value={prettier.printWidth ?? 60}
                        onChange={(e) => updatePrettier({ printWidth: Number(e.target.value) })}
                    />
                    <span className="settings-hint">characters per line</span>
                </div>
            </div>

            {/* Tab Width */}
            <div className="settings-section">
                <h3>Tab Width</h3>
                <div className="settings-row">
                    <input
                        type="number"
                        className="settings-number-input"
                        min={1}
                        max={8}
                        value={prettier.tabWidth ?? 2}
                        onChange={(e) => updatePrettier({ tabWidth: Number(e.target.value) })}
                    />
                    <span className="settings-hint">spaces per indent</span>
                </div>
            </div>

            {/* Semicolons */}
            <div className="settings-section">
                <h3>Semicolons</h3>
                <label className="settings-toggle">
                    <input
                        type="checkbox"
                        checked={prettier.semi ?? false}
                        onChange={(e) => updatePrettier({ semi: e.target.checked })}
                    />
                    <span>Add semicolons at the end of statements</span>
                </label>
            </div>

            {/* Single Quotes */}
            <div className="settings-section">
                <h3>Single Quotes</h3>
                <label className="settings-toggle">
                    <input
                        type="checkbox"
                        checked={prettier.singleQuote ?? true}
                        onChange={(e) => updatePrettier({ singleQuote: e.target.checked })}
                    />
                    <span>Use single quotes instead of double quotes</span>
                </label>
            </div>

            {/* Trailing Commas */}
            <div className="settings-section">
                <h3>Trailing Commas</h3>
                <select
                    className="device-select"
                    value={(prettier.trailingComma as string) ?? 'all'}
                    onChange={(e) => updatePrettier({ trailingComma: e.target.value as 'all' | 'es5' | 'none' })}
                >
                    {TRAILING_COMMA_OPTIONS.map((opt) => (
                        <option key={opt} value={opt}>
                            {opt}
                        </option>
                    ))}
                </select>
            </div>
        </div>
    );
}
