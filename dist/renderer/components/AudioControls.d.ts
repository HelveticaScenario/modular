interface AudioControlsProps {
    isRunning: boolean;
    isRecording: boolean;
    onStop: () => void;
    onStartRecording: () => void;
    onStopRecording: () => void;
    onUpdatePatch: () => void;
}
export declare function AudioControls({ isRunning, isRecording, onStop, onStartRecording, onStopRecording, onUpdatePatch, }: AudioControlsProps): import("react/jsx-runtime").JSX.Element;
export {};
