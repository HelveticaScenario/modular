import type { ConnectionState } from '../hooks/useWebSocket'

interface AudioControlsProps {
  connectionState: ConnectionState
  isPlaying: boolean
  isRecording: boolean
  onStartAudio: () => void
  onStopAudio: () => void
  onStartRecording: () => void
  onStopRecording: () => void
  onUpdatePatch: () => void
}

export function AudioControls({
  connectionState,
  isPlaying,
  isRecording,
  onStartAudio,
  onStopAudio,
  onStartRecording,
  onStopRecording,
  onUpdatePatch,
}: AudioControlsProps) {
  const isConnected = connectionState === 'connected'

  return (
    <div className="audio-controls">
      <div className="connection-status">
        <span className={`status-indicator ${connectionState}`} />
        <span className="status-text">
          {connectionState === 'connected' && 'Connected'}
          {connectionState === 'connecting' && 'Connecting...'}
          {connectionState === 'disconnected' && 'Disconnected'}
          {connectionState === 'error' && 'Connection Error'}
        </span>
      </div>

      <div className="control-buttons">
        <button
          onClick={onUpdatePatch}
          disabled={!isConnected}
          className="btn btn-primary"
          title="Ctrl+Enter / Cmd+Enter"
        >
          ▶ Update Patch
        </button>

        {isPlaying ? (
          <button
            onClick={onStopAudio}
            disabled={!isConnected}
            className="btn btn-danger"
            title="Ctrl+. / Cmd+."
          >
            ⏹ Stop Audio
          </button>
        ) : (
          <button
            onClick={onStartAudio}
            disabled={!isConnected}
            className="btn btn-success"
          >
            🔊 Start Audio
          </button>
        )}

        {isRecording ? (
          <button
            onClick={onStopRecording}
            disabled={!isConnected}
            className="btn btn-danger recording"
          >
            ⏺ Stop Recording
          </button>
        ) : (
          <button
            onClick={onStartRecording}
            disabled={!isConnected || !isPlaying}
            className="btn btn-secondary"
            title="Ctrl+R / Cmd+R"
          >
            ⏺ Record
          </button>
        )}
      </div>
    </div>
  )
}
