"use strict";
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.ThemeProvider = ThemeProvider;
exports.useTheme = useTheme;
const jsx_runtime_1 = require("react/jsx-runtime");
const react_1 = require("react");
const types_1 = require("./types");
const bundled_1 = require("./bundled");
const electronAPI_1 = __importDefault(require("../electronAPI"));
const ThemeContext = (0, react_1.createContext)(null);
// Build theme lookup map
const themeMap = new Map();
bundled_1.bundledThemes.forEach(t => {
    const mapped = (0, types_1.mapVSCodeTheme)(t);
    themeMap.set(mapped.id, mapped);
});
function getThemeById(id) {
    return themeMap.get(id) || (0, types_1.mapVSCodeTheme)(bundled_1.bundledThemes[0]);
}
// Apply theme colors to CSS custom properties
function applyThemeToCSS(theme) {
    const root = document.documentElement;
    const c = theme.colors;
    // Map our theme colors to CSS custom properties
    root.style.setProperty('--bg-primary', c.bgPrimary);
    root.style.setProperty('--bg-secondary', c.bgSecondary);
    root.style.setProperty('--bg-tertiary', c.bgTertiary);
    root.style.setProperty('--bg-hover', c.bgHover);
    root.style.setProperty('--bg-active', c.bgActive);
    root.style.setProperty('--border-subtle', c.borderSubtle);
    root.style.setProperty('--border-default', c.borderDefault);
    root.style.setProperty('--text-primary', c.textPrimary);
    root.style.setProperty('--text-secondary', c.textSecondary);
    root.style.setProperty('--text-muted', c.textMuted);
    root.style.setProperty('--text-bright', c.textBright);
    root.style.setProperty('--accent-primary', c.accentPrimary);
    root.style.setProperty('--accent-secondary', c.accentSecondary);
    root.style.setProperty('--accent-muted', c.accentMuted);
    root.style.setProperty('--color-success', c.colorSuccess);
    root.style.setProperty('--color-warning', c.colorWarning);
    root.style.setProperty('--color-error', c.colorError);
    root.style.setProperty('--color-info', c.colorInfo);
    // Update color-scheme for scrollbars etc
    root.style.colorScheme = theme.type;
}
function ThemeProvider({ children }) {
    const [themes] = (0, react_1.useState)(() => Array.from(themeMap.values()));
    const [theme, setTheme] = (0, react_1.useState)(() => getThemeById('modular-dark'));
    const [cursorStyle, setCursorStyle] = (0, react_1.useState)('line');
    const [font, setFont] = (0, react_1.useState)('Fira Code');
    const [fontLigatures, setFontLigatures] = (0, react_1.useState)(true);
    const [fontSize, setFontSize] = (0, react_1.useState)(17);
    const [prettierConfig, setPrettierConfig] = (0, react_1.useState)({});
    // Load initial config and set up watcher
    (0, react_1.useEffect)(() => {
        let unsubscribe = null;
        async function init() {
            // Read initial config
            const config = await electronAPI_1.default.config.read();
            if (config.theme) {
                setTheme(getThemeById(config.theme));
            }
            if (config.cursorStyle) {
                setCursorStyle(config.cursorStyle);
            }
            if (config.font) {
                setFont(config.font);
            }
            if (config.fontLigatures != null) {
                setFontLigatures(config.fontLigatures);
            }
            if (config.fontSize != null) {
                setFontSize(config.fontSize);
            }
            if (config.prettier) {
                setPrettierConfig(config.prettier);
            }
            // Subscribe to config changes
            unsubscribe = electronAPI_1.default.config.onChange((newConfig) => {
                if (newConfig.theme) {
                    setTheme(getThemeById(newConfig.theme));
                }
                if (newConfig.cursorStyle) {
                    setCursorStyle(newConfig.cursorStyle);
                }
                if (newConfig.font) {
                    setFont(newConfig.font);
                }
                if (newConfig.fontLigatures != null) {
                    setFontLigatures(newConfig.fontLigatures);
                }
                if (newConfig.fontSize != null) {
                    setFontSize(newConfig.fontSize);
                }
                if (newConfig.prettier) {
                    setPrettierConfig(newConfig.prettier);
                }
            });
        }
        init();
        return () => {
            if (unsubscribe)
                unsubscribe();
        };
    }, []);
    // Apply theme to CSS whenever it changes
    (0, react_1.useEffect)(() => {
        applyThemeToCSS(theme);
    }, [theme]);
    return ((0, jsx_runtime_1.jsx)(ThemeContext.Provider, { value: { theme, themes, cursorStyle, font, fontLigatures, fontSize, prettierConfig }, children: children }));
}
function useTheme() {
    const ctx = (0, react_1.useContext)(ThemeContext);
    if (!ctx) {
        throw new Error('useTheme must be used within ThemeProvider');
    }
    return ctx;
}
//# sourceMappingURL=ThemeContext.js.map