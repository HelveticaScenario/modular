interface AudioControlsProps {
    isRunning: boolean;
    isRecording: boolean;
    onStop: () => void;
    onStartRecording: () => void;
    onStopRecording: () => void;
    onUpdatePatch: () => void;
}

export function AudioControls({
    isRunning,
    isRecording,
    onStop,
    onStartRecording,
    onStopRecording,
    onUpdatePatch,
}: AudioControlsProps) {
    return (
        <div className="audio-controls">
            <div className="control-buttons">
                <button
                    onClick={onUpdatePatch}
                    className="btn btn-primary"
                    title="Ctrl+Enter / Cmd+Enter"
                >
                    ▶ Update Patch
                </button>

                <button
                    onClick={onStop}
                    disabled={!isRunning}
                    className="btn btn-danger"
                    title="Ctrl+. / Cmd+."
                >
                    ⏹ Stop
                </button>

                {isRecording ? (
                    <button
                        onClick={onStopRecording}
                        className="btn btn-danger recording"
                    >
                        ⏺ Stop Recording
                    </button>
                ) : (
                    <button
                        onClick={onStartRecording}
                        className="btn btn-secondary"
                        title="Ctrl+R / Cmd+R"
                    >
                        ⏺ Record
                    </button>
                )}
            </div>
        </div>
    );
}
