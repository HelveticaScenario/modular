import { useCallback, useEffect, useRef } from 'react'
import useWebSocket, { ReadyState } from 'react-use-websocket'
import type { InputMessage, OutputMessage } from '../types'

export type ConnectionState = 'connecting' | 'connected' | 'disconnected' | 'error'

export type { OutputMessage }

export interface UseModularWebSocketOptions {
    url?: string
    onMessage?: (msg: OutputMessage) => void
    onStateChange?: (state: ConnectionState) => void
}

const readyStateToConnectionState = (readyState: ReadyState): ConnectionState => {
    switch (readyState) {
        case ReadyState.CONNECTING:
            return 'connecting'
        case ReadyState.OPEN:
            return 'connected'
        case ReadyState.CLOSING:
        case ReadyState.CLOSED:
            return 'disconnected'
        case ReadyState.UNINSTANTIATED:
        default:
            return 'disconnected'
    }
}

export function useModularWebSocket(options: UseModularWebSocketOptions = {}) {
    const {
        url = `ws://localhost:7812/ws`,
        onMessage,
        onStateChange,
    } = options

    console.log(`Connecting to WebSocket at ${url}`)

    const onMessageRef = useRef(onMessage)
    const onStateChangeRef = useRef(onStateChange)
    const prevStateRef = useRef<ConnectionState>('disconnected')

    useEffect(() => {
        onMessageRef.current = onMessage
        onStateChangeRef.current = onStateChange
    }, [onMessage, onStateChange])

    const { sendJsonMessage, readyState } = useWebSocket(url, {
        shouldReconnect: () => true,
        reconnectAttempts: Infinity,
        reconnectInterval: 200,
        onMessage: (event) => {
            if (typeof event.data === 'string') {
                console.log('WebSocket message received:', event.data)
                try {
                    const msg = JSON.parse(event.data) as OutputMessage
                    onMessageRef.current?.(msg)
                } catch (e) {
                    console.error('Failed to parse WebSocket message:', e, event.data)
                }
            } else if (event.data instanceof Blob) {
                // event.data is a blob of binary data prepended with a null terminated string message type
                event.data.bytes().then((data) => {

                    let i = 0
                    while (i < data.length && data[i] !== 0) {
                        i++
                    }
                    const firstSection = i++;
                    while (i < data.length && data[i] !== 0) {
                        i++
                    }
                    const secondSection = i;

                    const module_id = new TextDecoder().decode(data.slice(0, firstSection))
                    const port = new TextDecoder().decode(data.slice(firstSection + 1, secondSection))
                    const payload = data.slice(secondSection + 1) // skip null terminator
                    // Handle different binary message types here if needed
                    console.log('Binary message:', { module_id, port, payload })
                }
                ).catch((e) => {
                    console.error('Failed to read binary WebSocket message:', e)
                })
            } else {
                // console.log('WebSocket message received:', event)
            }
        },
    })

    const connectionState = readyStateToConnectionState(readyState)

    // Notify on state changes
    useEffect(() => {
        if (prevStateRef.current !== connectionState) {
            prevStateRef.current = connectionState
            onStateChangeRef.current?.(connectionState)
        }
    }, [connectionState])

    const send = useCallback((message: InputMessage) => {
        console.log('WebSocket sending message:', message)
        sendJsonMessage(message)
    }, [sendJsonMessage])

    // Commands matching server InputMessage types (kebab-case)
    const getPatch = useCallback(() => {
        send({ type: 'getPatch' })
    }, [send])

    const getSchemas = useCallback(() => {
        send({ type: 'getSchemas' })
    }, [send])

    const setPatch = useCallback((yaml: string) => {
        send({ type: 'setPatch', yaml })
    }, [send])
    const mute = useCallback(() => {
        send({ type: 'mute' })
    }, [send])

    const unmute = useCallback(() => {
        send({ type: 'unmute' })
    }, [send])
    const startRecording = useCallback((filename?: string) => {
        send({ type: 'startRecording', filename: filename ?? null })
    }, [send])

    const stopRecording = useCallback(() => {
        send({ type: 'stopRecording' })
    }, [send])
    const subscribeAudio = useCallback((moduleId: string, port: string) => {
        send({ type: 'subscribeAudio', subscription: { moduleId, port } })
    }, [send])

    const unsubscribeAudio = useCallback((moduleId: string, port: string) => {
        send({ type: 'unsubscribeAudio', subscription: { moduleId, port } })
    }, [send])
    return {
        connectionState,
        getPatch,
        getSchemas,
        setPatch,
        mute,
        unmute,
        startRecording,
        stopRecording,
        subscribeAudio,
        unsubscribeAudio,
    }
}
