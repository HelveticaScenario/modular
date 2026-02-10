"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.EditorSettingsTab = EditorSettingsTab;
const jsx_runtime_1 = require("react/jsx-runtime");
const react_1 = require("react");
const BUNDLED_FONTS = [
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
const SYSTEM_FONTS = [
    'SF Mono',
    'Monaco',
    'Menlo',
    'Consolas',
];
const CURSOR_STYLES = [
    'line',
    'block',
    'underline',
    'line-thin',
    'block-outline',
    'underline-thin',
];
/**
 * Detect whether a font is installed by measuring text against baseline fonts.
 * If the candidate font renders at a different width than all baselines, it's installed.
 */
function isFontInstalled(fontName) {
    const canvas = document.createElement('canvas');
    const ctx = canvas.getContext('2d');
    if (!ctx)
        return false;
    const testString = 'mmmmmmmmmmlli1WWW@#$';
    const baselines = ['monospace', 'sans-serif', 'serif'];
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
function EditorSettingsTab({ config, themes, onConfigChange }) {
    const [availableSystemFonts, setAvailableSystemFonts] = (0, react_1.useState)([]);
    (0, react_1.useEffect)(() => {
        // Detect which system fonts are actually installed via canvas measurement
        const available = SYSTEM_FONTS.filter(isFontInstalled);
        setAvailableSystemFonts(available);
    }, []);
    return ((0, jsx_runtime_1.jsxs)("div", { className: "settings-tab-content", children: [(0, jsx_runtime_1.jsxs)("div", { className: "settings-section", children: [(0, jsx_runtime_1.jsx)("h3", { children: "Color Theme" }), (0, jsx_runtime_1.jsx)("select", { className: "device-select", value: config.theme || 'modular-dark', onChange: (e) => onConfigChange({ theme: e.target.value }), children: themes.map((t) => ((0, jsx_runtime_1.jsx)("option", { value: t.id, children: t.name }, t.id))) })] }), (0, jsx_runtime_1.jsxs)("div", { className: "settings-section", children: [(0, jsx_runtime_1.jsx)("h3", { children: "Font" }), (0, jsx_runtime_1.jsxs)("select", { className: "device-select", value: config.font || 'Fira Code', onChange: (e) => onConfigChange({ font: e.target.value }), children: [(0, jsx_runtime_1.jsx)("optgroup", { label: "Bundled", children: BUNDLED_FONTS.map((f) => ((0, jsx_runtime_1.jsx)("option", { value: f, children: f }, f))) }), availableSystemFonts.length > 0 && ((0, jsx_runtime_1.jsx)("optgroup", { label: "System", children: availableSystemFonts.map((f) => ((0, jsx_runtime_1.jsx)("option", { value: f, children: f }, f))) }))] })] }), (0, jsx_runtime_1.jsxs)("div", { className: "settings-section", children: [(0, jsx_runtime_1.jsx)("h3", { children: "Font Size" }), (0, jsx_runtime_1.jsxs)("div", { className: "settings-row", children: [(0, jsx_runtime_1.jsx)("input", { type: "range", className: "settings-range", min: 8, max: 72, step: 1, value: config.fontSize ?? 17, onChange: (e) => onConfigChange({ fontSize: Number(e.target.value) }) }), (0, jsx_runtime_1.jsxs)("span", { className: "settings-range-value", children: [config.fontSize ?? 17, "px"] })] })] }), (0, jsx_runtime_1.jsxs)("div", { className: "settings-section", children: [(0, jsx_runtime_1.jsx)("h3", { children: "Font Ligatures" }), (0, jsx_runtime_1.jsx)("label", { className: "settings-toggle-label", children: (0, jsx_runtime_1.jsx)("input", { type: "checkbox", className: "settings-toggle", checked: config.fontLigatures ?? true, onChange: (e) => onConfigChange({ fontLigatures: e.target.checked }) }) })] }), (0, jsx_runtime_1.jsxs)("div", { className: "settings-section", children: [(0, jsx_runtime_1.jsx)("h3", { children: "Cursor Style" }), (0, jsx_runtime_1.jsx)("select", { className: "device-select", value: config.cursorStyle || 'block', onChange: (e) => onConfigChange({ cursorStyle: e.target.value }), children: CURSOR_STYLES.map((s) => ((0, jsx_runtime_1.jsx)("option", { value: s, children: s }, s))) })] })] }));
}
//# sourceMappingURL=EditorSettingsTab.js.map