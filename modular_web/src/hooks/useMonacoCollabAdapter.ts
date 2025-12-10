import { useCallback, useEffect, useRef } from 'react'
import { Position, Range, type editor } from 'monaco-editor'
import * as yjs from 'yjs'
import * as awarenessProtocol from 'y-protocols/awareness'
import type { ConnectionState } from './useWebSocket'
import type { InputMessage } from '../types/generated/InputMessage'
import type { OutputMessage } from '../types/generated/OutputMessage'

type AwarenessChange = { added: number[]; updated: number[]; removed: number[] }

type YrsContext = {
    doc: yjs.Doc
    awareness: awarenessProtocol.Awareness
    text: yjs.Text
    isApplyingRemote: boolean
}

function toUint8Array(value: Uint8Array | number[]): Uint8Array {
    return value instanceof Uint8Array ? value : new Uint8Array(value)
}

function runTransaction(doc: yjs.Doc, fn: () => void) {
    if (typeof doc.transact === 'function') {
        doc.transact(fn)
    } else {
        fn()
    }
}

export interface UseYrsMonacoCollabOptions {
    editorRef: React.RefObject<editor.IStandaloneCodeEditor | null>
    docId: string
    clientId: string
    sendMessage: (msg: InputMessage) => void
    connectionState: ConnectionState
    enabled?: boolean
    userDisplayName?: string
    userColor?: string
    initialText: string
    onTextUpdated?: (value: string) => void
    presenceThrottleMs?: number
}

export function useYrsMonacoCollab(options: UseYrsMonacoCollabOptions) {
    const {
        editorRef,
        docId,
        clientId,
        sendMessage,
        connectionState,
        enabled = false,
        userDisplayName,
        userColor,
        initialText,
        onTextUpdated,
        presenceThrottleMs = 150,
    } = options

    const ctxRef = useRef<YrsContext | null>(null)
    const joinedRef = useRef(false)
    const presenceStyleRef = useRef<Map<number, HTMLStyleElement>>(new Map())
    const decorationsRef = useRef<string[]>([])
    const lastPresenceSendRef = useRef(0)

    // Load yrs client and set up doc/awareness/text for current docId
    useEffect(() => {
        let cancelled = false

        async function setup() {
            if (cancelled) return

            const doc = new yjs.Doc()
            const awareness = new awarenessProtocol.Awareness(doc)
            const text = doc.getText('patch')

            if (text.length === 0 && initialText) {
                text.insert(0, initialText)
            }

            awareness.setLocalState({
                user: {
                    name: userDisplayName || clientId,
                    color: userColor,
                },
                cursor: null,
            })

            ctxRef.current = {
                doc,
                awareness,
                text,
                isApplyingRemote: false,
            }
            joinedRef.current = false
        }

        setup()

        return () => {
            cancelled = true
            ctxRef.current = null
        }
    }, [clientId, docId, initialText, userColor, userDisplayName])

    // Hook Monaco up to Y.Text
    useEffect(() => {
        if (!enabled) return
        const ctx = ctxRef.current
        const editorInstance = editorRef.current
        const model = editorInstance?.getModel()
        if (!ctx || !editorInstance || !model) return

        // Keep Monaco in sync when Y.Text changes
        const textObserver = (event: yjs.YTextEvent) => {
            if (!ctxRef.current) return
            const current = ctxRef.current
            const monacoModel = editorRef.current?.getModel()
            if (!monacoModel) return

            current.isApplyingRemote = true
            let index = 0
            const edits: { range: Range; text: string }[] = []
            for (const delta of event.delta || []) {
                if (delta.retain) {
                    index += delta.retain
                }
                if (delta.delete) {
                    const start = monacoModel.getPositionAt(index)
                    const end = monacoModel.getPositionAt(index + delta.delete)
                    edits.push({
                        range: new Range(
                            start.lineNumber,
                            start.column,
                            end.lineNumber,
                            end.column
                        ),
                        text: '',
                    })
                }
                if (delta.insert) {
                    const pos = monacoModel.getPositionAt(index)
                    if (typeof delta.insert !== 'string') {
                        console.error('Non-string insert not supported:', delta.insert)
                        continue
                    }
                    edits.push({
                        range: new Range(
                            pos.lineNumber,
                            pos.column,
                            pos.lineNumber,
                            pos.column
                        ),
                        text: delta.insert,
                    })
                    index += delta.insert.length
                }
            }

            if (edits.length > 0) {
                monacoModel.pushEditOperations([], edits, () => null)
            }
            onTextUpdated?.(current.text.toString())
            current.isApplyingRemote = false
        }

        ctx.text.observe(textObserver)
        // Seed Monaco with current text state
        model.setValue(ctx.text.toString())

        const onContent = editorInstance.onDidChangeModelContent((event) => {
            if (!ctxRef.current || ctxRef.current.isApplyingRemote) return
            const current = ctxRef.current
            const currentModel = editorRef.current?.getModel()
            if (!currentModel) return

            // Apply edits in reverse order to maintain offsets
            const sorted = [...event.changes].sort(
                (a, b) => b.rangeOffset - a.rangeOffset
            )
            runTransaction(current.doc, () => {
                for (const change of sorted) {
                    const startOffset = currentModel.getOffsetAt(
                        new Position(
                            change.range.startLineNumber,
                            change.range.startColumn
                        )
                    )
                    if (change.rangeLength > 0) {
                        current.text.delete(startOffset, change.rangeLength)
                    }
                    if (change.text.length > 0) {
                        current.text.insert(startOffset, change.text)
                    }
                }
            })
            onTextUpdated?.(current.text.toString())
        })

        const updatePresence = () => {
            if (!ctxRef.current) return
            const current = ctxRef.current
            const selection = editorRef.current?.getSelection()
            const currentModel = editorRef.current?.getModel()
            if (!selection || !currentModel) return

            const now = Date.now()
            if (now - lastPresenceSendRef.current < presenceThrottleMs) return
            lastPresenceSendRef.current = now

            const anchor = currentModel.getOffsetAt(selection.getStartPosition())
            const head = currentModel.getOffsetAt(selection.getEndPosition())
            current.awareness.setLocalState({
                user: {
                    name: userDisplayName || clientId,
                    color: userColor,
                },
                cursor: { anchor, head },
            })
        }

        const disposeCursor = editorInstance.onDidChangeCursorPosition(updatePresence)
        const disposeSelection = editorInstance.onDidChangeCursorSelection(updatePresence)

        return () => {
            ctx.text.unobserve(textObserver)
            onContent.dispose()
            disposeCursor.dispose()
            disposeSelection.dispose()
            decorationsRef.current = []
        }
    }, [clientId, docId, editorRef, enabled, onTextUpdated, presenceThrottleMs, userColor, userDisplayName])

    // Send local Y updates to the server
    useEffect(() => {
        const ctx = ctxRef.current
        if (!ctx) return

        const docListener = (update: Uint8Array) => {
            if (ctxRef.current?.isApplyingRemote) return
            const msg: InputMessage = { type: 'collabYrsUpdate', docId, update: Array.from(update) }
            sendMessage(msg)
        }

        const awarenessListener = (
            update: AwarenessChange,
            origin: unknown
        ) => {
            if (!ctxRef.current) return
            if (origin === 'remote') return
            const payload = awarenessProtocol.encodeAwarenessUpdate(
                ctx.awareness,
                [...update.added, ...update.updated, ...update.removed]
            )
            const msg: InputMessage = {
                type: 'collabYrsAwareness',
                docId,
                update: Array.from(payload),
            }
            sendMessage(msg)
        }

        ctx.doc.on('update', docListener)
        ctx.awareness.on('update', awarenessListener)

        return () => {
            ctx.doc.off('update', docListener)
            ctx.awareness.off('update', awarenessListener)
        }
    }, [docId, sendMessage])

    // Join when the socket is ready
    useEffect(() => {
        const ctx = ctxRef.current
        if (!enabled || !ctx) return
        if (connectionState !== 'connected') return
        if (joinedRef.current) return

        const awarenessUpdate = awarenessProtocol.encodeAwarenessUpdate(ctx.awareness, [ctx.doc.clientID])
        const msg: InputMessage = {
            type: 'collabYrsJoin',
            docId,
            clientId,
            awarenessClientId: ctx.doc.clientID,
            awarenessUpdate: Array.from(awarenessUpdate),
        }
        sendMessage(msg)
        joinedRef.current = true
    }, [clientId, connectionState, docId, enabled, sendMessage])

    const ensurePresenceStyles = useCallback((clientNum: number, color?: string) => {
        if (!color) return
        if (presenceStyleRef.current.has(clientNum)) return
        const style = document.createElement('style')
        style.textContent = `
            .remote-cursor-${clientNum} { border-left: 2px solid ${color}; }
            .remote-selection-${clientNum} { background-color: ${color}33; }
        `
        document.head.appendChild(style)
        presenceStyleRef.current.set(clientNum, style)
    }, [])

    const updateRemotePresenceDecorations = useCallback(() => {
        const ctx = ctxRef.current
        const editorInstance = editorRef.current
        const model = editorInstance?.getModel()
        if (!ctx || !editorInstance || !model) return

        const states = ctx.awareness.getStates()
        const decorations: editor.IModelDeltaDecoration[] = []
        states.forEach((state, clientNum) => {
            if (clientNum === ctx.doc.clientID) return
            const presence = state
            console.log('Processing presence for clientNum', clientNum, presence)
            const cursor = presence?.cursor
            if (!cursor) return
            ensurePresenceStyles(clientNum, presence?.user?.color)

            const anchor = model.getPositionAt(cursor.anchor)
            const head = model.getPositionAt(cursor.head)
            const start = cursor.anchor <= cursor.head ? anchor : head
            const end = cursor.anchor <= cursor.head ? head : anchor

            decorations.push({
                range: new Range(start.lineNumber, start.column, start.lineNumber, start.column),
                options: {
                    className: `remote-cursor-${clientNum}`,
                    stickiness: 1,
                },
            })

            if (cursor.anchor !== cursor.head) {
                decorations.push({
                    range: new Range(start.lineNumber, start.column, end.lineNumber, end.column),
                    options: {
                        className: `remote-selection-${clientNum}`,
                        isWholeLine: false,
                    },
                })
            }
        })

        decorationsRef.current = editorInstance.deltaDecorations(
            decorationsRef.current,
            decorations
        )
    }, [editorRef, ensurePresenceStyles])

    const handleOutputMessage = useCallback(
        (msg: OutputMessage): boolean => {
            console.log('Collab adapter received message:', msg)
            const ctx = ctxRef.current
            if (!ctx) return false
            const collabMsg = msg
            switch (collabMsg.type) {
                case 'collabYrsInit': {
                    const update = toUint8Array(collabMsg.init.update)
                    const awarenessUpdate = toUint8Array(collabMsg.init.awareness)
                    ctx.isApplyingRemote = true
                    yjs.applyUpdate(ctx.doc, update)
                    awarenessProtocol.applyAwarenessUpdate(
                        ctx.awareness,
                        awarenessUpdate,
                        'remote'
                    )
                    ctx.isApplyingRemote = false
                    console.log('Applied collabYrsInit update:', ctx.text.toJSON())
                    updateRemotePresenceDecorations()
                    onTextUpdated?.(ctx.text.toString())
                    return true
                }
                case 'collabYrsUpdate': {
                    ctx.isApplyingRemote = true
                    yjs.applyUpdate(ctx.doc, toUint8Array(collabMsg.update))
                    ctx.isApplyingRemote = false
                    console.log('Applied collabYrsUpdate:', ctx.text.toString())
                    onTextUpdated?.(ctx.text.toString())
                    return true
                }
                case 'collabYrsAwareness': {
                    awarenessProtocol.applyAwarenessUpdate(
                        ctx.awareness,
                        toUint8Array(collabMsg.update),
                        'remote'
                    )
                    console.log('Applied collabYrsAwareness:', ctx.awareness.getStates())
                    updateRemotePresenceDecorations()
                    return true
                }
                default:
                    return false
            }
        },
        [onTextUpdated, updateRemotePresenceDecorations]
    )

    return handleOutputMessage
}

// Backward compatibility export name
export { useYrsMonacoCollab as useMonacoCollabAdapter }
