import { useCallback, useEffect, useRef, useState } from 'react'
import type { InputMessage } from '../types/generated/InputMessage'
import type { OutputMessage } from '../types/generated/OutputMessage'
import type { PatchGraph } from '../types/generated/PatchGraph'
import type { ScopeItem } from '../types/generated/ScopeItem'

export type ConnectionState =
    | 'connecting'
    | 'connected'
    | 'disconnected'
    | 'reconnecting'
    | 'error'


export interface UseModularWebSocketOptions {
    url?: string
    onMessage?: (msg: OutputMessage) => void
    onStateChange?: (state: ConnectionState) => void
}
export function useModularWebSocket(options: UseModularWebSocketOptions = {}) {
    const {
        url = `ws://localhost:7812/ws`,
        onMessage,
        onStateChange,
    } = options

    // console.log(`Connecting to WebSocket at ${url}`)

    const onMessageRef = useRef(onMessage)
    const onStateChangeRef = useRef(onStateChange)
    const [connectionState, setConnectionState] = useState<ConnectionState>('connecting')
    const wsRef = useRef<WebSocket | null>(null)
    const reconnectTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null)
    const reconnectAttemptsRef = useRef(0)
    const manuallyClosedRef = useRef(false)
    const pendingMessagesRef = useRef<InputMessage[]>([])

    useEffect(() => {
        onMessageRef.current = onMessage
        onStateChangeRef.current = onStateChange
    }, [onMessage, onStateChange])

    const updateConnectionState = useCallback((next: ConnectionState) => {
        setConnectionState(prev => {
            if (prev === next) return prev
            onStateChangeRef.current?.(next)
            return next
        })
    }, [])

    useEffect(() => {
        if (typeof window === 'undefined' || typeof WebSocket === 'undefined') {
            // In non-browser environments, do nothing.
            return
        }

        manuallyClosedRef.current = false

        const connect = () => {
            if (manuallyClosedRef.current) {
                return
            }

            // Clear any existing socket before reconnecting
            if (wsRef.current) {
                try {
                    wsRef.current.close()
                } catch {
                    // ignore
                }
            }

            updateConnectionState('connecting')

            const ws = new WebSocket(url)
            wsRef.current = ws

            ws.onopen = () => {
                reconnectAttemptsRef.current = 0
                updateConnectionState('connected')

                // Flush any pending messages queued while disconnected
                const queue = pendingMessagesRef.current
                pendingMessagesRef.current = []
                for (const msg of queue) {
                    try {
                        ws.send(JSON.stringify(msg))
                    } catch (e) {
                        console.error('Failed to send queued WebSocket message:', e, msg)
                    }
                }
            }

            ws.onmessage = (event: MessageEvent) => {
                if (typeof event.data === 'string') {
                    // console.log('WebSocket message received:', event.data)
                    try {
                        const msg = JSON.parse(event.data) as OutputMessage
                        onMessageRef.current?.(msg)
                    } catch (e) {
                        console.error('Failed to parse WebSocket message:', e, event.data)
                    }
                } else if (event.data instanceof Blob) {
                    // event.data is a blob of binary data prepended with a null terminated string message type
                    event.data.arrayBuffer().then((buffer) => {
                        const data = new Uint8Array(buffer)

                        let i = 0
                        while (i < data.length && data[i] !== 0) {
                            i++
                        }
                        const firstSection = i++
                        while (i < data.length && data[i] !== 0) {
                            i++
                        }
                        const secondSection = i

                        const moduleId = new TextDecoder().decode(data.slice(0, firstSection))
                        const port = new TextDecoder().decode(data.slice(firstSection + 1, secondSection))
                        const payload = data.slice(secondSection + 1) // skip null terminator
                        const subscription: ScopeItem =
                            port.length > 0
                                ? { type: 'moduleOutput', moduleId, portName: port }
                                : { type: 'track', trackId: moduleId }
                        // Handle different binary message types here if needed
                        onMessageRef.current?.({ type: 'audioBuffer', subscription, samples: new Float32Array(payload.buffer) })
                    }).catch((e) => {
                        console.error('Failed to read binary WebSocket message:', e)
                    })
                } else {
                    // console.log('WebSocket message received:', event)
                }
            }

            ws.onerror = (event) => {
                console.error('WebSocket error:', event)
                updateConnectionState('error')
            }

            ws.onclose = () => {
                wsRef.current = null
                if (manuallyClosedRef.current) {
                    updateConnectionState('disconnected')
                    return
                }

                // Schedule reconnect with fixed 1s delay (can be adjusted to exponential backoff if desired)
                reconnectAttemptsRef.current += 1
                const delayMs = 1000
                updateConnectionState('reconnecting')

                if (reconnectTimeoutRef.current) {
                    clearTimeout(reconnectTimeoutRef.current)
                }

                reconnectTimeoutRef.current = setTimeout(() => {
                    connect()
                }, delayMs)
            }
        }

        connect()

        return () => {
            manuallyClosedRef.current = true
            if (reconnectTimeoutRef.current) {
                clearTimeout(reconnectTimeoutRef.current)
                reconnectTimeoutRef.current = null
            }
            if (wsRef.current) {
                try {
                    wsRef.current.close()
                } catch {
                    // ignore
                }
                wsRef.current = null
            }
        }
    }, [url, updateConnectionState])

    const send = useCallback((message: InputMessage) => {
        console.log('WebSocket sending message:', message)
        const ws = wsRef.current
        if (ws && ws.readyState === WebSocket.OPEN) {
            ws.send(JSON.stringify(message))
        } else {
            // Queue the message to be sent after we reconnect
            pendingMessagesRef.current.push(message)
        }
    }, [])

    // Commands matching server InputMessage types (kebab-case)
    const getPatch = useCallback(() => {
        send({ type: 'getPatch' })
    }, [send])

    const getSchemas = useCallback(() => {
        send({ type: 'getSchemas' })
    }, [send])

    const setPatch = useCallback((patch: PatchGraph) => {
        send({ type: 'setPatch', patch })
    }, [send])

    const listFiles = useCallback(() => {
        send({ type: 'listFiles' })
    }, [send])

    const readFile = useCallback((path: string) => {
        send({ type: 'readFile', path })
    }, [send])

    const writeFile = useCallback((path: string, content: string) => {
        send({ type: 'writeFile', path, content })
    }, [send])

    const deleteFile = useCallback((path: string) => {
        send({ type: 'deleteFile', path })
    }, [send])

    const renameFile = useCallback((from: string, to: string) => {
        send({ type: 'renameFile', from, to })
    }, [send])

    const start = useCallback(() => {
        send({ type: 'start' })
    }, [send])

    const stop = useCallback(() => {
        send({ type: 'stop' })
    }, [send])
    const startRecording = useCallback((filename?: string) => {
        send({ type: 'startRecording', filename: filename ?? null })
    }, [send])

    const stopRecording = useCallback(() => {
        send({ type: 'stopRecording' })
    }, [send])
    return {
        connectionState,
        sendMessage: send,
        getPatch,
        getSchemas,
        setPatch,
        start,
        stop,
        startRecording,
        stopRecording,
        listFiles,
        readFile,
        writeFile,
        renameFile,
        deleteFile,
    }
}
