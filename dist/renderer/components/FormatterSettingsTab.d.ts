import type { AppConfig } from '../../shared/ipcTypes';
interface FormatterSettingsTabProps {
    config: AppConfig;
    onConfigChange: (partial: Partial<AppConfig>) => void;
}
export declare function FormatterSettingsTab({ config, onConfigChange }: FormatterSettingsTabProps): import("react/jsx-runtime").JSX.Element;
export {};
