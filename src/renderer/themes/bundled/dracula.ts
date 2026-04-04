import type { VSCodeTheme } from '../types';

export const draculaTheme: VSCodeTheme = {
    colors: {
        'activityBar.background': '#21222c',
        'activityBar.foreground': '#f8f8f2',
        'button.background': '#44475a',
        'button.foreground': '#f8f8f2',
        'button.hoverBackground': '#6272a4',
        descriptionForeground: '#6272a4',
        disabledForeground: '#6272a4',
        'editor.background': '#282a36',
        'editor.foreground': '#f8f8f2',
        'editor.inactiveSelectionBackground': '#44475a75',
        'editor.lineHighlightBackground': '#44475a',
        'editor.selectionBackground': '#44475a',
        'editorBracketMatch.background': '#44475a',
        'editorBracketMatch.border': '#ff79c6',
        'editorCursor.foreground': '#f8f8f2',
        'editorGutter.background': '#282a36',
        'editorHoverWidget.background': '#21222c',
        'editorHoverWidget.border': '#191a21',
        'editorIndentGuide.activeBackground': '#6272a4',
        'editorIndentGuide.background': '#44475a',
        'editorLineNumber.activeForeground': '#f8f8f2',
        'editorLineNumber.foreground': '#6272a4',

        'editorSuggestWidget.background': '#21222c',
        'editorSuggestWidget.border': '#191a21',
        'editorSuggestWidget.foreground': '#f8f8f2',
        'editorSuggestWidget.selectedBackground': '#44475a',
        'editorWhitespace.foreground': '#44475a',
        'editorWidget.background': '#21222c',
        'editorWidget.border': '#191a21',

        focusBorder: '#6272a4',
        foreground: '#f8f8f2',

        'input.background': '#21222c',
        'input.border': '#191a21',
        'input.foreground': '#f8f8f2',
        'input.placeholderForeground': '#6272a4',
        'list.activeSelectionBackground': '#44475a',

        'list.activeSelectionForeground': '#f8f8f2',
        'list.focusBackground': '#44475a',
        'list.hoverBackground': '#44475a75',
        'scrollbar.shadow': '#00000000',

        'scrollbarSlider.activeBackground': '#6272a4',
        'scrollbarSlider.background': '#44475a80',
        'scrollbarSlider.hoverBackground': '#44475a',
        'sideBar.background': '#21222c',

        'sideBar.border': '#191a21',
        'sideBar.foreground': '#f8f8f2',
        'sideBarSectionHeader.background': '#282a36',

        'sideBarSectionHeader.foreground': '#f8f8f2',
        'statusBar.background': '#191a21',
        'statusBar.foreground': '#f8f8f2',
        'tab.activeBackground': '#282a36',

        'tab.activeForeground': '#f8f8f2',
        'tab.border': '#191a21',
        'tab.inactiveBackground': '#21222c',
        'tab.inactiveForeground': '#6272a4',

        'terminal.ansiBlue': '#8be9fd',
        'terminal.ansiCyan': '#8be9fd',
        'terminal.ansiGreen': '#50fa7b',
        'terminal.ansiMagenta': '#ff79c6',
        'terminal.ansiRed': '#ff5555',
        'terminal.ansiYellow': '#f1fa8c',
    },
    name: 'Dracula',
    tokenColors: [
        {
            scope: ['comment'],
            settings: { foreground: '#6272a4', fontStyle: 'italic' },
        },
        {
            scope: ['keyword', 'storage.type', 'storage.modifier'],
            settings: { foreground: '#ff79c6' },
        },
        {
            scope: ['string', 'string.quoted'],
            settings: { foreground: '#f1fa8c' },
        },
        { scope: ['constant.numeric'], settings: { foreground: '#bd93f9' } },
        { scope: ['constant.language'], settings: { foreground: '#bd93f9' } },
        {
            scope: ['entity.name.function', 'support.function'],
            settings: { foreground: '#50fa7b' },
        },
        {
            scope: ['variable', 'variable.other'],
            settings: { foreground: '#f8f8f2' },
        },
        {
            scope: ['entity.name.type', 'support.type'],
            settings: { foreground: '#8be9fd', fontStyle: 'italic' },
        },
        { scope: ['punctuation'], settings: { foreground: '#f8f8f2' } },
        {
            scope: ['constant.other', 'variable.other.constant'],
            settings: { foreground: '#bd93f9' },
        },
    ],
    type: 'dark',
};
