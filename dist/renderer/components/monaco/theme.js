"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.applyMonacoTheme = applyMonacoTheme;
function applyMonacoTheme(monaco, appTheme, monacoThemeId) {
    const raw = appTheme.raw;
    const rules = raw.tokenColors
        .map((tc) => {
        const scopes = Array.isArray(tc.scope) ? tc.scope : [tc.scope || ''];
        return scopes.map((scope) => ({
            token: scope.replace(/\./g, ' ').trim() || '',
            foreground: tc.settings.foreground?.replace('#', ''),
            background: tc.settings.background?.replace('#', ''),
            fontStyle: tc.settings.fontStyle,
        }));
    })
        .flat();
    monaco.editor.defineTheme(monacoThemeId, {
        base: appTheme.type === 'light' ? 'vs' : 'vs-dark',
        inherit: true,
        rules,
        colors: raw.colors,
    });
    monaco.editor.setTheme(monacoThemeId);
}
//# sourceMappingURL=theme.js.map