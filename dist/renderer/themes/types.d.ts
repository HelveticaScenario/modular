export interface VSCodeTokenColor {
    name?: string;
    scope?: string | string[];
    settings: {
        foreground?: string;
        background?: string;
        fontStyle?: string;
    };
}
export interface VSCodeTheme {
    name: string;
    type: 'dark' | 'light' | 'hc';
    colors: Record<string, string>;
    tokenColors: VSCodeTokenColor[];
}
export interface AppTheme {
    id: string;
    name: string;
    type: 'dark' | 'light';
    colors: {
        bgPrimary: string;
        bgSecondary: string;
        bgTertiary: string;
        bgHover: string;
        bgActive: string;
        borderSubtle: string;
        borderDefault: string;
        textPrimary: string;
        textSecondary: string;
        textMuted: string;
        textBright: string;
        accentPrimary: string;
        accentSecondary: string;
        accentMuted: string;
        colorSuccess: string;
        colorWarning: string;
        colorError: string;
        colorInfo: string;
        editorBackground: string;
        editorForeground: string;
        editorLineHighlight: string;
        editorSelection: string;
        editorCursor: string;
        lineNumberForeground: string;
        lineNumberActiveForeground: string;
    };
    raw: VSCodeTheme;
}
export declare function mapVSCodeTheme(vscodeTheme: VSCodeTheme): AppTheme;
