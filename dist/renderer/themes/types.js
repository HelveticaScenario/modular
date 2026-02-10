"use strict";
// VS Code theme structure types
Object.defineProperty(exports, "__esModule", { value: true });
exports.mapVSCodeTheme = mapVSCodeTheme;
// Map VS Code workbench colors to our app theme colors
function mapVSCodeTheme(vscodeTheme) {
    const c = vscodeTheme.colors;
    const type = vscodeTheme.type === 'light' ? 'light' : 'dark';
    // Helper to get color with fallback
    const get = (key, fallback) => c[key] || fallback;
    // Default fallbacks based on theme type
    const defaults = type === 'dark' ? {
        bgPrimary: '#1e1e1e',
        bgSecondary: '#252526',
        bgTertiary: '#2d2d2d',
        textPrimary: '#cccccc',
        textSecondary: '#888888',
        textMuted: '#555555',
        accent: '#007acc',
    } : {
        bgPrimary: '#ffffff',
        bgSecondary: '#f3f3f3',
        bgTertiary: '#e8e8e8',
        textPrimary: '#333333',
        textSecondary: '#666666',
        textMuted: '#999999',
        accent: '#007acc',
    };
    return {
        id: vscodeTheme.name.toLowerCase().replace(/\s+/g, '-'),
        name: vscodeTheme.name,
        type,
        colors: {
            // Backgrounds - map from VS Code workbench colors
            bgPrimary: get('editor.background', defaults.bgPrimary),
            bgSecondary: get('sideBar.background', get('activityBar.background', defaults.bgSecondary)),
            bgTertiary: get('sideBarSectionHeader.background', get('tab.activeBackground', defaults.bgTertiary)),
            bgHover: get('list.hoverBackground', adjustAlpha(defaults.textPrimary, 0.1)),
            bgActive: get('list.activeSelectionBackground', get('editor.selectionBackground', defaults.accent)),
            // Borders
            borderSubtle: get('editorGroup.border', get('sideBar.border', adjustAlpha(defaults.textMuted, 0.3))),
            borderDefault: get('input.border', get('dropdown.border', adjustAlpha(defaults.textMuted, 0.5))),
            // Text
            textPrimary: get('editor.foreground', get('foreground', defaults.textPrimary)),
            textSecondary: get('descriptionForeground', get('sideBar.foreground', defaults.textSecondary)),
            textMuted: get('disabledForeground', get('editorLineNumber.foreground', defaults.textMuted)),
            textBright: get('editor.foreground', defaults.textPrimary),
            // Accent
            accentPrimary: get('focusBorder', get('button.background', get('activityBarBadge.background', defaults.accent))),
            accentSecondary: get('textLink.foreground', get('activityBarBadge.background', defaults.accent)),
            accentMuted: adjustAlpha(get('focusBorder', defaults.accent), 0.4),
            // Semantic
            colorSuccess: get('terminal.ansiGreen', get('gitDecoration.addedResourceForeground', '#3fb27f')),
            colorWarning: get('terminal.ansiYellow', get('editorWarning.foreground', '#d4a855')),
            colorError: get('terminal.ansiRed', get('editorError.foreground', '#e05561')),
            colorInfo: get('terminal.ansiBlue', get('editorInfo.foreground', '#61afef')),
            // Editor specific
            editorBackground: get('editor.background', defaults.bgPrimary),
            editorForeground: get('editor.foreground', defaults.textPrimary),
            editorLineHighlight: get('editor.lineHighlightBackground', adjustAlpha(defaults.textPrimary, 0.05)),
            editorSelection: get('editor.selectionBackground', defaults.accent),
            editorCursor: get('editorCursor.foreground', defaults.accent),
            lineNumberForeground: get('editorLineNumber.foreground', defaults.textMuted),
            lineNumberActiveForeground: get('editorLineNumber.activeForeground', defaults.textSecondary),
        },
        raw: vscodeTheme,
    };
}
// Helper to adjust alpha of a hex color
function adjustAlpha(hex, alpha) {
    // Handle already-rgba colors
    if (hex.startsWith('rgba'))
        return hex;
    if (hex.startsWith('rgb')) {
        const match = hex.match(/rgb\((\d+),\s*(\d+),\s*(\d+)\)/);
        if (match) {
            return `rgba(${match[1]}, ${match[2]}, ${match[3]}, ${alpha})`;
        }
    }
    // Handle hex colors
    const cleanHex = hex.replace('#', '');
    let r, g, b;
    if (cleanHex.length === 3) {
        r = parseInt(cleanHex[0] + cleanHex[0], 16);
        g = parseInt(cleanHex[1] + cleanHex[1], 16);
        b = parseInt(cleanHex[2] + cleanHex[2], 16);
    }
    else if (cleanHex.length === 6) {
        r = parseInt(cleanHex.slice(0, 2), 16);
        g = parseInt(cleanHex.slice(2, 4), 16);
        b = parseInt(cleanHex.slice(4, 6), 16);
    }
    else if (cleanHex.length === 8) {
        // Already has alpha
        r = parseInt(cleanHex.slice(0, 2), 16);
        g = parseInt(cleanHex.slice(2, 4), 16);
        b = parseInt(cleanHex.slice(4, 6), 16);
    }
    else {
        return hex;
    }
    return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}
//# sourceMappingURL=types.js.map