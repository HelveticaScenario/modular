import React from 'react';
export interface AudioSettingsHandle {
    apply: () => Promise<void>;
    isDirty: () => boolean;
}
interface AudioSettingsTabProps {
    isActive: boolean;
}
export declare const AudioSettingsTab: React.ForwardRefExoticComponent<AudioSettingsTabProps & React.RefAttributes<AudioSettingsHandle>>;
export {};
