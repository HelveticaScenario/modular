import type { VSCodeTheme } from '../types';

export const gruvboxDark: VSCodeTheme = {
    colors: {
        'activityBar.background': '#1d2021',
        'activityBar.foreground': '#ebdbb2',
        'button.background': '#504945',
        'button.foreground': '#ebdbb2',
        'button.hoverBackground': '#665c54',
        descriptionForeground: '#a89984',
        disabledForeground: '#665c54',
        'editor.background': '#282828',
        'editor.foreground': '#ebdbb2',
        'editor.inactiveSelectionBackground': '#3c3836',
        'editor.lineHighlightBackground': '#3c3836',
        'editor.selectionBackground': '#504945',
        'editorBracketMatch.background': '#504945',
        'editorBracketMatch.border': '#fe8019',
        'editorCursor.foreground': '#ebdbb2',
        'editorGutter.background': '#282828',
        'editorHoverWidget.background': '#1d2021',
        'editorHoverWidget.border': '#3c3836',
        'editorIndentGuide.activeBackground': '#504945',
        'editorIndentGuide.background': '#3c3836',
        'editorLineNumber.activeForeground': '#ebdbb2',
        'editorLineNumber.foreground': '#665c54',

        'editorSuggestWidget.background': '#1d2021',
        'editorSuggestWidget.border': '#3c3836',
        'editorSuggestWidget.foreground': '#ebdbb2',
        'editorSuggestWidget.selectedBackground': '#504945',
        'editorWhitespace.foreground': '#3c3836',
        'editorWidget.background': '#1d2021',
        'editorWidget.border': '#3c3836',

        focusBorder: '#fe8019',
        foreground: '#ebdbb2',

        'input.background': '#1d2021',
        'input.border': '#3c3836',
        'input.foreground': '#ebdbb2',
        'input.placeholderForeground': '#665c54',
        'list.activeSelectionBackground': '#504945',

        'list.activeSelectionForeground': '#ebdbb2',
        'list.focusBackground': '#504945',
        'list.hoverBackground': '#3c3836',
        'scrollbar.shadow': '#00000000',

        'scrollbarSlider.activeBackground': '#665c54',
        'scrollbarSlider.background': '#50494580',
        'scrollbarSlider.hoverBackground': '#504945',
        'sideBar.background': '#1d2021',

        'sideBar.border': '#3c3836',
        'sideBar.foreground': '#ebdbb2',
        'sideBarSectionHeader.background': '#282828',

        'sideBarSectionHeader.foreground': '#a89984',
        'statusBar.background': '#1d2021',
        'statusBar.foreground': '#a89984',
        'tab.activeBackground': '#282828',

        'tab.activeForeground': '#ebdbb2',
        'tab.border': '#3c3836',
        'tab.inactiveBackground': '#1d2021',
        'tab.inactiveForeground': '#a89984',

        'terminal.ansiBlue': '#83a598',
        'terminal.ansiCyan': '#8ec07c',
        'terminal.ansiGreen': '#b8bb26',
        'terminal.ansiMagenta': '#d3869b',
        'terminal.ansiRed': '#fb4934',
        'terminal.ansiYellow': '#fabd2f',
    },
    name: 'Gruvbox Dark',
    tokenColors: [
        {
            scope: ['comment'],
            settings: { foreground: '#928374', fontStyle: 'italic' },
        },
        {
            scope: ['keyword', 'storage.type', 'storage.modifier'],
            settings: { foreground: '#fb4934' },
        },
        {
            scope: ['string', 'string.quoted'],
            settings: { foreground: '#b8bb26' },
        },
        { scope: ['constant.numeric'], settings: { foreground: '#d3869b' } },
        { scope: ['constant.language'], settings: { foreground: '#d3869b' } },
        {
            scope: ['entity.name.function', 'support.function'],
            settings: { foreground: '#b8bb26' },
        },
        {
            scope: ['variable', 'variable.other'],
            settings: { foreground: '#ebdbb2' },
        },
        {
            scope: ['entity.name.type', 'support.type'],
            settings: { foreground: '#fabd2f' },
        },
        { scope: ['punctuation'], settings: { foreground: '#ebdbb2' } },
        {
            scope: ['constant.other', 'variable.other.constant'],
            settings: { foreground: '#d3869b' },
        },
    ],
    type: 'dark',
};
