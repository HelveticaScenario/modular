import type { VSCodeTheme } from '../types';

export const tokyoNight: VSCodeTheme = {
    colors: {
        'activityBar.background': '#16161e',
        'activityBar.foreground': '#a9b1d6',
        'button.background': '#3d59a1',
        'button.foreground': '#c0caf5',
        'button.hoverBackground': '#7aa2f7',
        descriptionForeground: '#7982a9',
        disabledForeground: '#3b4261',
        'editor.background': '#1a1b26',
        'editor.foreground': '#a9b1d6',
        'editor.inactiveSelectionBackground': '#292e42',
        'editor.lineHighlightBackground': '#1e2030',
        'editor.selectionBackground': '#33467c',
        'editorBracketMatch.background': '#33467c',
        'editorBracketMatch.border': '#7aa2f7',
        'editorCursor.foreground': '#c0caf5',
        'editorGutter.background': '#1a1b26',
        'editorHoverWidget.background': '#16161e',
        'editorHoverWidget.border': '#1a1b26',
        'editorIndentGuide.activeBackground': '#3b4261',
        'editorIndentGuide.background': '#292e42',
        'editorLineNumber.activeForeground': '#737aa2',
        'editorLineNumber.foreground': '#3b4261',

        'editorSuggestWidget.background': '#16161e',
        'editorSuggestWidget.border': '#1a1b26',
        'editorSuggestWidget.foreground': '#a9b1d6',
        'editorSuggestWidget.selectedBackground': '#33467c',
        'editorWhitespace.foreground': '#292e42',
        'editorWidget.background': '#16161e',
        'editorWidget.border': '#1a1b26',

        focusBorder: '#7aa2f7',
        foreground: '#a9b1d6',

        'input.background': '#16161e',
        'input.border': '#1a1b26',
        'input.foreground': '#a9b1d6',
        'input.placeholderForeground': '#3b4261',
        'list.activeSelectionBackground': '#33467c',

        'list.activeSelectionForeground': '#c0caf5',
        'list.focusBackground': '#33467c',
        'list.hoverBackground': '#1e2030',
        'scrollbar.shadow': '#00000000',

        'scrollbarSlider.activeBackground': '#7aa2f7',
        'scrollbarSlider.background': '#33467c80',
        'scrollbarSlider.hoverBackground': '#33467c',
        'sideBar.background': '#16161e',

        'sideBar.border': '#1a1b26',
        'sideBar.foreground': '#a9b1d6',
        'sideBarSectionHeader.background': '#1a1b26',

        'sideBarSectionHeader.foreground': '#7982a9',
        'statusBar.background': '#16161e',
        'statusBar.foreground': '#7982a9',
        'tab.activeBackground': '#1a1b26',

        'tab.activeForeground': '#a9b1d6',
        'tab.border': '#16161e',
        'tab.inactiveBackground': '#16161e',
        'tab.inactiveForeground': '#7982a9',

        'terminal.ansiBlue': '#7aa2f7',
        'terminal.ansiCyan': '#7dcfff',
        'terminal.ansiGreen': '#9ece6a',
        'terminal.ansiMagenta': '#bb9af7',
        'terminal.ansiRed': '#f7768e',
        'terminal.ansiYellow': '#e0af68',
    },
    name: 'Tokyo Night',
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
    type: 'dark',
};
