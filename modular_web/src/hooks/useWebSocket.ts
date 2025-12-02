import { useCallback, useEffect, useRef, useState } from 'react'

export type ConnectionState = 'connecting' | 'connected' | 'disconnected' | 'error'

export interface OutputMessage {
  type: 'state' | 'error' | 'audio_started' | 'audio_stopped' | 'recording_started' | 'recording_stopped' | 'oscilloscope_data'
  data?: unknown
}

export interface UseWebSocketOptions {
  url?: string
  onMessage?: (msg: OutputMessage) => void
  onStateChange?: (state: ConnectionState) => void
}

export function useWebSocket(options: UseWebSocketOptions = {}) {
  const { 
    url = `ws://${window.location.host}/ws`,
    onMessage,
    onStateChange,
  } = options
  
  const wsRef = useRef<WebSocket | null>(null)
  const [connectionState, setConnectionState] = useState<ConnectionState>('disconnected')
  const reconnectTimeoutRef = useRef<number | null>(null)
  
  const updateState = useCallback((state: ConnectionState) => {
    setConnectionState(state)
    onStateChange?.(state)
  }, [onStateChange])
  
  const connect = useCallback(() => {
    if (wsRef.current?.readyState === WebSocket.OPEN) return
    
    updateState('connecting')
    
    const ws = new WebSocket(url)
    wsRef.current = ws
    
    ws.onopen = () => {
      updateState('connected')
    }
    
    ws.onclose = () => {
      updateState('disconnected')
      // Reconnect after 2 seconds
      reconnectTimeoutRef.current = window.setTimeout(() => {
        connect()
      }, 2000)
    }
    
    ws.onerror = () => {
      updateState('error')
    }
    
    ws.onmessage = (event) => {
      try {
        const msg = JSON.parse(event.data) as OutputMessage
        onMessage?.(msg)
      } catch (e) {
        console.error('Failed to parse WebSocket message:', e)
      }
    }
  }, [url, updateState, onMessage])
  
  const disconnect = useCallback(() => {
    if (reconnectTimeoutRef.current) {
      clearTimeout(reconnectTimeoutRef.current)
      reconnectTimeoutRef.current = null
    }
    wsRef.current?.close()
    wsRef.current = null
    updateState('disconnected')
  }, [updateState])
  
  const send = useCallback((message: object) => {
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      wsRef.current.send(JSON.stringify(message))
    }
  }, [])
  
  // Commands
  const updatePatch = useCallback((patch: string) => {
    send({ type: 'update_patch', patch })
  }, [send])
  
  const startAudio = useCallback(() => {
    send({ type: 'start_audio' })
  }, [send])
  
  const stopAudio = useCallback(() => {
    send({ type: 'stop_audio' })
  }, [send])
  
  const startRecording = useCallback(() => {
    send({ type: 'start_recording' })
  }, [send])
  
  const stopRecording = useCallback(() => {
    send({ type: 'stop_recording' })
  }, [send])
  
  const getState = useCallback(() => {
    send({ type: 'get_state' })
  }, [send])
  
  useEffect(() => {
    connect()
    return () => disconnect()
  }, [connect, disconnect])
  
  return {
    connectionState,
    connect,
    disconnect,
    send,
    updatePatch,
    startAudio,
    stopAudio,
    startRecording,
    stopRecording,
    getState,
  }
}
