import type { VSCodeTheme } from '../types';

export const tokyoNight: VSCodeTheme = {
    name: 'Tokyo Night',
    type: 'dark',
    colors: {
        'editor.background': '#1a1b26',
        'editor.foreground': '#a9b1d6',
        'editor.lineHighlightBackground': '#1e2030',
        'editor.selectionBackground': '#33467c',
        'editor.inactiveSelectionBackground': '#292e42',
        'editorCursor.foreground': '#c0caf5',
        'editorLineNumber.foreground': '#3b4261',
        'editorLineNumber.activeForeground': '#737aa2',
        'editorIndentGuide.background': '#292e42',
        'editorIndentGuide.activeBackground': '#3b4261',
        'editorWhitespace.foreground': '#292e42',
        'editorBracketMatch.background': '#33467c',
        'editorBracketMatch.border': '#7aa2f7',
        'editorGutter.background': '#1a1b26',
        'editorWidget.background': '#16161e',
        'editorWidget.border': '#1a1b26',
        'editorSuggestWidget.background': '#16161e',
        'editorSuggestWidget.border': '#1a1b26',
        'editorSuggestWidget.foreground': '#a9b1d6',
        'editorSuggestWidget.selectedBackground': '#33467c',
        'editorHoverWidget.background': '#16161e',
        'editorHoverWidget.border': '#1a1b26',

        'activityBar.background': '#16161e',
        'activityBar.foreground': '#a9b1d6',
        'sideBar.background': '#16161e',
        'sideBar.foreground': '#a9b1d6',
        'sideBar.border': '#1a1b26',
        'sideBarSectionHeader.background': '#1a1b26',
        'sideBarSectionHeader.foreground': '#7982a9',

        'statusBar.background': '#16161e',
        'statusBar.foreground': '#7982a9',

        'tab.activeBackground': '#1a1b26',
        'tab.inactiveBackground': '#16161e',
        'tab.activeForeground': '#a9b1d6',
        'tab.inactiveForeground': '#7982a9',
        'tab.border': '#16161e',

        'list.activeSelectionBackground': '#33467c',
        'list.activeSelectionForeground': '#c0caf5',
        'list.hoverBackground': '#1e2030',
        'list.focusBackground': '#33467c',

        'input.background': '#16161e',
        'input.foreground': '#a9b1d6',
        'input.border': '#1a1b26',
        'input.placeholderForeground': '#3b4261',

        'button.background': '#3d59a1',
        'button.foreground': '#c0caf5',
        'button.hoverBackground': '#7aa2f7',

        focusBorder: '#7aa2f7',
        foreground: '#a9b1d6',
        descriptionForeground: '#7982a9',
        disabledForeground: '#3b4261',

        'scrollbar.shadow': '#00000000',
        'scrollbarSlider.background': '#33467c80',
        'scrollbarSlider.hoverBackground': '#33467c',
        'scrollbarSlider.activeBackground': '#7aa2f7',

        'terminal.ansiGreen': '#9ece6a',
        'terminal.ansiYellow': '#e0af68',
        'terminal.ansiRed': '#f7768e',
        'terminal.ansiBlue': '#7aa2f7',
        'terminal.ansiCyan': '#7dcfff',
        'terminal.ansiMagenta': '#bb9af7',
    },
    tokenColors: [
        {
            scope: ['comment'],
            settings: { foreground: '#565f89', fontStyle: 'italic' },
        },
        {
            scope: ['keyword', 'storage.type', 'storage.modifier'],
            settings: { foreground: '#bb9af7' },
        },
        {
            scope: ['string', 'string.quoted'],
            settings: { foreground: '#9ece6a' },
        },
        { scope: ['constant.numeric'], settings: { foreground: '#ff9e64' } },
        { scope: ['constant.language'], settings: { foreground: '#ff9e64' } },
        {
            scope: ['entity.name.function', 'support.function'],
            settings: { foreground: '#7aa2f7' },
        },
        {
            scope: ['variable', 'variable.other'],
            settings: { foreground: '#c0caf5' },
        },
        {
            scope: ['entity.name.type', 'support.type'],
            settings: { foreground: '#2ac3de' },
        },
        { scope: ['punctuation'], settings: { foreground: '#9abdf5' } },
        {
            scope: ['constant.other', 'variable.other.constant'],
            settings: { foreground: '#ff9e64' },
        },
    ],
};
