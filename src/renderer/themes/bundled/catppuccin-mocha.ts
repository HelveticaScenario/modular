import type { VSCodeTheme } from '../types';

export const catppuccinMocha: VSCodeTheme = {
    colors: {
        'activityBar.background': '#181825',
        'activityBar.foreground': '#cdd6f4',
        'button.background': '#cba6f7',
        'button.foreground': '#1e1e2e',
        'button.hoverBackground': '#b4befe',
        descriptionForeground: '#a6adc8',
        disabledForeground: '#6c7086',
        'editor.background': '#1e1e2e',
        'editor.foreground': '#cdd6f4',
        'editor.inactiveSelectionBackground': '#313244',
        'editor.lineHighlightBackground': '#313244',
        'editor.selectionBackground': '#45475a',
        'editorBracketMatch.background': '#45475a',
        'editorBracketMatch.border': '#cba6f7',
        'editorCursor.foreground': '#f5e0dc',
        'editorGutter.background': '#1e1e2e',
        'editorHoverWidget.background': '#181825',
        'editorHoverWidget.border': '#313244',
        'editorIndentGuide.activeBackground': '#45475a',
        'editorIndentGuide.background': '#313244',
        'editorLineNumber.activeForeground': '#cdd6f4',
        'editorLineNumber.foreground': '#6c7086',

        'editorSuggestWidget.background': '#181825',
        'editorSuggestWidget.border': '#313244',
        'editorSuggestWidget.foreground': '#cdd6f4',
        'editorSuggestWidget.selectedBackground': '#45475a',
        'editorWhitespace.foreground': '#313244',
        'editorWidget.background': '#181825',
        'editorWidget.border': '#313244',

        focusBorder: '#cba6f7',
        foreground: '#cdd6f4',

        'input.background': '#181825',
        'input.border': '#313244',
        'input.foreground': '#cdd6f4',
        'input.placeholderForeground': '#6c7086',
        'list.activeSelectionBackground': '#45475a',

        'list.activeSelectionForeground': '#cdd6f4',
        'list.focusBackground': '#45475a',
        'list.hoverBackground': '#313244',
        'scrollbar.shadow': '#00000000',

        'scrollbarSlider.activeBackground': '#6c7086',
        'scrollbarSlider.background': '#45475a80',
        'scrollbarSlider.hoverBackground': '#45475a',
        'sideBar.background': '#181825',

        'sideBar.border': '#313244',
        'sideBar.foreground': '#cdd6f4',
        'sideBarSectionHeader.background': '#1e1e2e',

        'sideBarSectionHeader.foreground': '#a6adc8',
        'statusBar.background': '#181825',
        'statusBar.foreground': '#a6adc8',
        'tab.activeBackground': '#1e1e2e',

        'tab.activeForeground': '#cdd6f4',
        'tab.border': '#313244',
        'tab.inactiveBackground': '#181825',
        'tab.inactiveForeground': '#a6adc8',

        'terminal.ansiBlue': '#89b4fa',
        'terminal.ansiCyan': '#94e2d5',
        'terminal.ansiGreen': '#a6e3a1',
        'terminal.ansiMagenta': '#cba6f7',
        'terminal.ansiRed': '#f38ba8',
        'terminal.ansiYellow': '#f9e2af',
    },
    name: 'Catppuccin Mocha',
    tokenColors: [
        {
            scope: ['comment'],
            settings: { foreground: '#6c7086', fontStyle: 'italic' },
        },
        {
            scope: ['keyword', 'storage.type', 'storage.modifier'],
            settings: { foreground: '#cba6f7' },
        },
        {
            scope: ['string', 'string.quoted'],
            settings: { foreground: '#a6e3a1' },
        },
        { scope: ['constant.numeric'], settings: { foreground: '#fab387' } },
        { scope: ['constant.language'], settings: { foreground: '#fab387' } },
        {
            scope: ['entity.name.function', 'support.function'],
            settings: { foreground: '#89b4fa' },
        },
        {
            scope: ['variable', 'variable.other'],
            settings: { foreground: '#cdd6f4' },
        },
        {
            scope: ['entity.name.type', 'support.type'],
            settings: { foreground: '#f9e2af' },
        },
        { scope: ['punctuation'], settings: { foreground: '#9399b2' } },
        {
            scope: ['constant.other', 'variable.other.constant'],
            settings: { foreground: '#fab387' },
        },
    ],
    type: 'dark',
};
