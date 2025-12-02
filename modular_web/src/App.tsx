import { useCallback, useState, useEffect } from 'react'
import { useWebSocket, type OutputMessage } from './hooks/useWebSocket'
import { PatchEditor } from './components/PatchEditor'
import { Oscilloscope } from './components/Oscilloscope'
import { AudioControls } from './components/AudioControls'
import { ErrorDisplay } from './components/ErrorDisplay'
import './App.css'

const DEFAULT_PATCH = `# Modular Synthesizer Patch
# Press Ctrl+Enter to update, Ctrl+. to stop

graph:
  osc1:
    module: sine
    params:
      frequency: 440
      
  vca:
    module: vca
    params:
      gain: 0.5
    inputs:
      input: [osc1, output]
      
  output:
    module: output
    inputs:
      left: [vca, output]
      right: [vca, output]
`

function App() {
  const [patchYaml, setPatchYaml] = useState(DEFAULT_PATCH)
  const [isPlaying, setIsPlaying] = useState(false)
  const [isRecording, setIsRecording] = useState(false)
  const [oscilloscopeData, setOscilloscopeData] = useState<number[] | null>(null)
  const [error, setError] = useState<string | null>(null)

  const handleMessage = useCallback((msg: OutputMessage) => {
    switch (msg.type) {
      case 'state':
        // Handle full state update
        if (msg.data && typeof msg.data === 'object') {
          const state = msg.data as { patch?: string; audio_playing?: boolean; recording?: boolean }
          if (state.patch) setPatchYaml(state.patch)
          if (typeof state.audio_playing === 'boolean') setIsPlaying(state.audio_playing)
          if (typeof state.recording === 'boolean') setIsRecording(state.recording)
        }
        break
      case 'error':
        setError(msg.data as string)
        break
      case 'audio_started':
        setIsPlaying(true)
        break
      case 'audio_stopped':
        setIsPlaying(false)
        setIsRecording(false)
        break
      case 'recording_started':
        setIsRecording(true)
        break
      case 'recording_stopped':
        setIsRecording(false)
        break
      case 'oscilloscope_data':
        setOscilloscopeData(msg.data as number[])
        break
    }
  }, [])

  const {
    connectionState,
    updatePatch,
    startAudio,
    stopAudio,
    startRecording,
    stopRecording,
    getState,
  } = useWebSocket({ onMessage: handleMessage })

  // Request initial state when connected
  useEffect(() => {
    if (connectionState === 'connected') {
      getState()
    }
  }, [connectionState, getState])

  const handleSubmit = useCallback(() => {
    updatePatch(patchYaml)
  }, [updatePatch, patchYaml])

  const handleStop = useCallback(() => {
    stopAudio()
  }, [stopAudio])

  const dismissError = useCallback(() => {
    setError(null)
  }, [])

  // Global keyboard shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Ctrl+R for recording (when not in an input)
      if ((e.ctrlKey || e.metaKey) && e.key === 'r') {
        if (document.activeElement?.tagName !== 'INPUT') {
          e.preventDefault()
          if (isRecording) {
            stopRecording()
          } else if (isPlaying) {
            startRecording()
          }
        }
      }
    }

    window.addEventListener('keydown', handleKeyDown)
    return () => window.removeEventListener('keydown', handleKeyDown)
  }, [isPlaying, isRecording, startRecording, stopRecording])

  return (
    <div className="app">
      <header className="app-header">
        <h1>Modular Synthesizer</h1>
        <AudioControls
          connectionState={connectionState}
          isPlaying={isPlaying}
          isRecording={isRecording}
          onStartAudio={startAudio}
          onStopAudio={stopAudio}
          onStartRecording={startRecording}
          onStopRecording={stopRecording}
          onUpdatePatch={handleSubmit}
        />
      </header>

      <ErrorDisplay error={error} onDismiss={dismissError} />

      <main className="app-main">
        <div className="editor-panel">
          <PatchEditor
            value={patchYaml}
            onChange={setPatchYaml}
            onSubmit={handleSubmit}
            onStop={handleStop}
            disabled={connectionState !== 'connected'}
          />
        </div>

        <div className="visualization-panel">
          <Oscilloscope data={oscilloscopeData} />
          <div className="keyboard-shortcuts">
            <h3>Keyboard Shortcuts</h3>
            <ul>
              <li><kbd>Ctrl</kbd>+<kbd>Enter</kbd> Update Patch</li>
              <li><kbd>Ctrl</kbd>+<kbd>.</kbd> Stop Audio</li>
              <li><kbd>Ctrl</kbd>+<kbd>R</kbd> Toggle Recording</li>
            </ul>
          </div>
        </div>
      </main>
    </div>
  )
}

export default App
