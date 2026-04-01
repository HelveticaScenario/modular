import type { VSCodeTheme } from '../types';

export const modularDark: VSCodeTheme = {
    colors: {
        'activityBar.background': '#0a0a0a',
        'activityBar.foreground': '#cccccc',
        'button.background': '#2d6b5f',
        'button.foreground': '#ffffff',
        'button.hoverBackground': '#3d7b6f',
        descriptionForeground: '#888888',
        disabledForeground: '#555555',
        'editor.background': '#0a0a0a',
        'editor.foreground': '#cccccc',
        'editor.inactiveSelectionBackground': '#1f3a3a',
        'editor.lineHighlightBackground': '#111111',
        'editor.selectionBackground': '#2d6b5f',
        'editorBracketMatch.background': '#2d6b5f44',
        'editorBracketMatch.border': '#4ec9b0',
        'editorCursor.foreground': '#4ec9b0',
        'editorGutter.background': '#0a0a0a',
        'editorHoverWidget.background': '#111111',
        'editorHoverWidget.border': '#222222',
        'editorIndentGuide.activeBackground': '#2a2a2a',
        'editorIndentGuide.background': '#222222',
        'editorLineNumber.activeForeground': '#888888',
        'editorLineNumber.foreground': '#555555',

        'editorSuggestWidget.background': '#111111',
        'editorSuggestWidget.border': '#222222',
        'editorSuggestWidget.foreground': '#cccccc',
        'editorSuggestWidget.selectedBackground': '#1f3a3a',
        'editorWhitespace.foreground': '#222222',
        'editorWidget.background': '#111111',
        'editorWidget.border': '#222222',

        focusBorder: '#4ec9b0',
        foreground: '#cccccc',

        'input.background': '#111111',
        'input.border': '#2a2a2a',
        'input.foreground': '#cccccc',
        'input.placeholderForeground': '#555555',
        'list.activeSelectionBackground': '#1f3a3a',

        'list.activeSelectionForeground': '#4ec9b0',
        'list.focusBackground': '#1f3a3a',
        'list.hoverBackground': '#1a1a1a',
        'scrollbar.shadow': '#00000000',

        'scrollbarSlider.activeBackground': '#555555',
        'scrollbarSlider.background': '#2a2a2a',
        'scrollbarSlider.hoverBackground': '#555555',
        'sideBar.background': '#111111',

        'sideBar.border': '#222222',
        'sideBar.foreground': '#cccccc',
        'sideBarSectionHeader.background': '#161616',

        'sideBarSectionHeader.foreground': '#888888',
        'statusBar.background': '#111111',
        'statusBar.foreground': '#888888',
        'tab.activeBackground': '#0a0a0a',

        'tab.activeForeground': '#cccccc',
        'tab.border': '#222222',
        'tab.inactiveBackground': '#111111',
        'tab.inactiveForeground': '#888888',

        'terminal.ansiBlue': '#61afef',
        'terminal.ansiCyan': '#5ce1e6',
        'terminal.ansiGreen': '#3fb27f',
        'terminal.ansiMagenta': '#c678dd',
        'terminal.ansiRed': '#e05561',
        'terminal.ansiYellow': '#d4a855',
    },
    name: 'Modular Dark',
    tokenColors: [
        {
            scope: ['comment'],
            settings: { foreground: '#555555', fontStyle: 'italic' },
        },
        {
            scope: ['keyword', 'storage.type', 'storage.modifier'],
            settings: { foreground: '#4ec9b0' },
        },
        {
            scope: ['string', 'string.quoted'],
            settings: { foreground: '#d4a855' },
        },
        {
            scope: ['constant.numeric', 'constant.language'],
            settings: { foreground: '#5ce1e6' },
        },
        {
            scope: ['entity.name.function', 'support.function'],
            settings: { foreground: '#61afef' },
        },
        {
            scope: ['variable', 'variable.other'],
            settings: { foreground: '#cccccc' },
        },
        {
            scope: ['entity.name.type', 'support.type'],
            settings: { foreground: '#4ec9b0' },
        },
        {
            scope: ['punctuation', 'meta.brace'],
            settings: { foreground: '#888888' },
        },
        {
            scope: ['constant.other', 'variable.other.constant'],
            settings: { foreground: '#5ce1e6' },
        },
    ],
    type: 'dark',
};
