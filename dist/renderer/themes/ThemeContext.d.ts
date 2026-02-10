import React from 'react';
import type { AppTheme } from './types';
import type { PrettierConfig } from '../../shared/ipcTypes';
type CursorStyle = 'line' | 'block' | 'underline' | 'line-thin' | 'block-outline' | 'underline-thin';
interface ThemeContextValue {
    theme: AppTheme;
    themes: AppTheme[];
    cursorStyle: CursorStyle;
    font: string;
    fontLigatures: boolean;
    fontSize: number;
    prettierConfig: PrettierConfig;
}
export declare function ThemeProvider({ children }: {
    children: React.ReactNode;
}): import("react/jsx-runtime").JSX.Element;
export declare function useTheme(): ThemeContextValue;
export {};
