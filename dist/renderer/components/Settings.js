"use strict";
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.Settings = Settings;
const jsx_runtime_1 = require("react/jsx-runtime");
const react_1 = require("react");
const electronAPI_1 = __importDefault(require("../electronAPI"));
const ThemeContext_1 = require("../themes/ThemeContext");
const AudioSettings_1 = require("./AudioSettings");
const EditorSettingsTab_1 = require("./EditorSettingsTab");
const FormatterSettingsTab_1 = require("./FormatterSettingsTab");
require("./Settings.css");
const TABS = [
    { id: 'editor', label: 'Editor' },
    { id: 'audio', label: 'Audio' },
    { id: 'formatter', label: 'Formatter' },
];
function Settings({ isOpen, onClose }) {
    const [activeTab, setActiveTab] = (0, react_1.useState)('editor');
    // Saved config (what's on disk)
    const [savedConfig, setSavedConfig] = (0, react_1.useState)({});
    // Draft config (local edits, not yet persisted)
    const [draftConfig, setDraftConfig] = (0, react_1.useState)({});
    const [saving, setSaving] = (0, react_1.useState)(false);
    const { themes } = (0, ThemeContext_1.useTheme)();
    const audioRef = (0, react_1.useRef)(null);
    const panelRef = (0, react_1.useRef)(null);
    // Load config when opened
    (0, react_1.useEffect)(() => {
        if (!isOpen)
            return;
        electronAPI_1.default.config.read().then((cfg) => {
            setSavedConfig(cfg);
            setDraftConfig(cfg);
        }).catch(console.error);
    }, [isOpen]);
    // Listen for external config changes (e.g. manual JSON edits)
    (0, react_1.useEffect)(() => {
        if (!isOpen)
            return;
        const unsubscribe = electronAPI_1.default.config.onChange((newConfig) => {
            setSavedConfig(newConfig);
            setDraftConfig(newConfig);
        });
        return unsubscribe;
    }, [isOpen]);
    const handleDraftChange = (0, react_1.useCallback)((partial) => {
        setDraftConfig((prev) => {
            const updated = { ...prev, ...partial };
            // Auto-save editor/formatter config changes immediately
            electronAPI_1.default.config.write(updated).then(() => {
                setSavedConfig(updated);
            }).catch(console.error);
            return updated;
        });
    }, []);
    const handleAudioSave = (0, react_1.useCallback)(async () => {
        setSaving(true);
        try {
            // Apply audio device changes
            await audioRef.current?.apply();
            onClose();
        }
        catch (err) {
            console.error('Failed to apply audio settings:', err);
        }
        finally {
            setSaving(false);
        }
    }, [onClose]);
    // Close on Escape key
    (0, react_1.useEffect)(() => {
        if (!isOpen)
            return;
        const handleKeyDown = (e) => {
            if (e.key === 'Escape') {
                onClose();
            }
        };
        window.addEventListener('keydown', handleKeyDown);
        return () => window.removeEventListener('keydown', handleKeyDown);
    }, [isOpen, onClose]);
    // Focus the panel when opened so the editor loses focus
    (0, react_1.useEffect)(() => {
        if (isOpen) {
            // Use rAF to ensure the DOM has rendered before focusing
            requestAnimationFrame(() => {
                panelRef.current?.focus();
            });
        }
    }, [isOpen]);
    if (!isOpen)
        return null;
    return ((0, jsx_runtime_1.jsx)("div", { className: "settings-overlay", onClick: onClose, children: (0, jsx_runtime_1.jsxs)("div", { className: "settings-panel", ref: panelRef, tabIndex: -1, onClick: (e) => e.stopPropagation(), children: [(0, jsx_runtime_1.jsxs)("div", { className: "settings-header", children: [(0, jsx_runtime_1.jsx)("h2", { children: "Settings" }), (0, jsx_runtime_1.jsx)("button", { className: "close-btn", onClick: onClose, children: "\u00D7" })] }), (0, jsx_runtime_1.jsx)("div", { className: "settings-tabs", children: TABS.map((tab) => ((0, jsx_runtime_1.jsx)("button", { className: `settings-tab-btn${activeTab === tab.id ? ' active' : ''}`, onClick: () => setActiveTab(tab.id), children: tab.label }, tab.id))) }), (0, jsx_runtime_1.jsxs)("div", { className: "settings-body", children: [activeTab === 'editor' && ((0, jsx_runtime_1.jsx)(EditorSettingsTab_1.EditorSettingsTab, { config: draftConfig, themes: themes, onConfigChange: handleDraftChange })), activeTab === 'audio' && ((0, jsx_runtime_1.jsx)(AudioSettings_1.AudioSettingsTab, { isActive: activeTab === 'audio', ref: audioRef })), activeTab === 'formatter' && ((0, jsx_runtime_1.jsx)(FormatterSettingsTab_1.FormatterSettingsTab, { config: draftConfig, onConfigChange: handleDraftChange }))] }), activeTab === 'audio' && ((0, jsx_runtime_1.jsx)("div", { className: "settings-footer", children: (0, jsx_runtime_1.jsx)("button", { className: "btn btn-primary", onClick: handleAudioSave, disabled: saving, children: saving ? 'Saving...' : 'Save' }) }))] }) }));
}
//# sourceMappingURL=Settings.js.map