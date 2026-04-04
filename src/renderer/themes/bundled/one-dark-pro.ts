import type { VSCodeTheme } from '../types';

export const oneDarkPro: VSCodeTheme = {
    colors: {
        'activityBar.background': '#282c34',
        'activityBar.foreground': '#d7dae0',
        'button.background': '#404754',
        'button.foreground': '#ffffff',
        'button.hoverBackground': '#4d5566',
        descriptionForeground: '#5c6370',
        disabledForeground: '#5c6370',
        'editor.background': '#282c34',
        'editor.foreground': '#abb2bf',
        'editor.inactiveSelectionBackground': '#3a3f4b',
        'editor.lineHighlightBackground': '#2c313a',
        'editor.selectionBackground': '#3e4451',
        'editorBracketMatch.background': '#3e4451',
        'editorBracketMatch.border': '#528bff',
        'editorCursor.foreground': '#528bff',
        'editorGutter.background': '#282c34',
        'editorHoverWidget.background': '#21252b',
        'editorHoverWidget.border': '#181a1f',
        'editorIndentGuide.activeBackground': '#4b5263',
        'editorIndentGuide.background': '#3b4048',
        'editorLineNumber.activeForeground': '#abb2bf',
        'editorLineNumber.foreground': '#495162',

        'editorSuggestWidget.background': '#21252b',
        'editorSuggestWidget.border': '#181a1f',
        'editorSuggestWidget.foreground': '#abb2bf',
        'editorSuggestWidget.selectedBackground': '#2c313a',
        'editorWhitespace.foreground': '#3b4048',
        'editorWidget.background': '#21252b',
        'editorWidget.border': '#181a1f',

        focusBorder: '#528bff',
        foreground: '#abb2bf',

        'input.background': '#1d1f23',
        'input.border': '#181a1f',
        'input.foreground': '#abb2bf',
        'input.placeholderForeground': '#5c6370',
        'list.activeSelectionBackground': '#2c313a',

        'list.activeSelectionForeground': '#d7dae0',
        'list.focusBackground': '#2c313a',
        'list.hoverBackground': '#2c313a',
        'scrollbar.shadow': '#00000000',

        'scrollbarSlider.activeBackground': '#747d9180',
        'scrollbarSlider.background': '#4e566680',
        'scrollbarSlider.hoverBackground': '#5a637580',
        'sideBar.background': '#21252b',

        'sideBar.border': '#181a1f',
        'sideBar.foreground': '#abb2bf',
        'sideBarSectionHeader.background': '#282c34',

        'sideBarSectionHeader.foreground': '#abb2bf',
        'statusBar.background': '#21252b',
        'statusBar.foreground': '#9da5b4',
        'tab.activeBackground': '#282c34',

        'tab.activeForeground': '#d7dae0',
        'tab.border': '#181a1f',
        'tab.inactiveBackground': '#21252b',
        'tab.inactiveForeground': '#9da5b4',

        'terminal.ansiBlue': '#61afef',
        'terminal.ansiCyan': '#56b6c2',
        'terminal.ansiGreen': '#98c379',
        'terminal.ansiMagenta': '#c678dd',
        'terminal.ansiRed': '#e06c75',
        'terminal.ansiYellow': '#e5c07b',
    },
    name: 'One Dark Pro',
    tokenColors: [
        {
            scope: ['comment'],
            settings: { foreground: '#5c6370', fontStyle: 'italic' },
        },
        {
            scope: ['keyword', 'storage.type', 'storage.modifier'],
            settings: { foreground: '#c678dd' },
        },
        {
            scope: ['string', 'string.quoted'],
            settings: { foreground: '#98c379' },
        },
        { scope: ['constant.numeric'], settings: { foreground: '#d19a66' } },
        { scope: ['constant.language'], settings: { foreground: '#d19a66' } },
        {
            scope: ['entity.name.function', 'support.function'],
            settings: { foreground: '#61afef' },
        },
        {
            scope: ['variable', 'variable.other'],
            settings: { foreground: '#e06c75' },
        },
        {
            scope: ['entity.name.type', 'support.type'],
            settings: { foreground: '#e5c07b' },
        },
        { scope: ['punctuation'], settings: { foreground: '#abb2bf' } },
        {
            scope: ['constant.other', 'variable.other.constant'],
            settings: { foreground: '#d19a66' },
        },
    ],
    type: 'dark',
};
