import type { AppConfig } from '../../shared/ipcTypes';
import type { AppTheme } from '../themes/types';
interface EditorSettingsTabProps {
    config: AppConfig;
    themes: AppTheme[];
    onConfigChange: (partial: Partial<AppConfig>) => void;
}
export declare function EditorSettingsTab({ config, themes, onConfigChange }: EditorSettingsTabProps): import("react/jsx-runtime").JSX.Element;
export {};
