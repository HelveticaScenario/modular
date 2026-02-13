import React, { useEffect, useMemo, useRef, useState } from 'react';
import Editor, { type OnMount } from '@monaco-editor/react';
import { editor } from 'monaco-editor';
import type { ModuleSchema } from '@modular/core';
import { useTheme } from '../themes/ThemeContext';
import { useCustomMonaco } from '../hooks/useCustomMonaco';
import { configSchema } from '../configSchema';
import { formatPath } from './monaco/monacoHelpers';
import type { ScopeView } from '../types/editor';
import { setupMonacoJavascript } from './monaco/monacoLanguage';
import {
    buildSymbolSets,
    resolveDslSymbolAtPosition,
} from './monaco/definitionProvider';
import { registerDslFormattingProvider } from './monaco/formattingProvider';
import { applyMonacoTheme } from './monaco/theme';
import {
    registerConfigSchema,
    registerConfigSchemaForFile,
} from './monaco/jsonSchema';
import { createScopeViewZones } from './monaco/scopeViewZones';
import { startModuleStatePolling } from './monaco/moduleStateTracking';
import { registerMidiCompletionProvider } from './monaco/midiCompletionProvider';
import electronAPI from '../electronAPI';

export interface PatchEditorProps {
    value: string;
    currentFile?: string;
    onChange: (value: string) => void;
    editorRef: React.RefObject<editor.IStandaloneCodeEditor | null>;
    scopeViews?: ScopeView[];
    onRegisterScopeCanvas?: (key: string, canvas: HTMLCanvasElement) => void;
    onUnregisterScopeCanvas?: (key: string) => void;
    runningBufferId?: string | null;
}

export function MonacoPatchEditor({
    value,
    currentFile,
    onChange,
    editorRef,
    scopeViews = [],
    onRegisterScopeCanvas,
    onUnregisterScopeCanvas,
    runningBufferId,
}: PatchEditorProps) {
    // Fetch DSL lib source once at mount for Monaco autocomplete
    const [libSource, setLibSource] = useState<string | null>(null);
    const [schemas, setSchemas] = useState<ModuleSchema[]>([]);

    useEffect(() => {
        electronAPI.getDslLibSource().then(setLibSource).catch(console.error);
        electronAPI.getSchemas().then(setSchemas).catch(console.error);
    }, []);

    const monaco = useCustomMonaco();
    const [editor, setEditor] = useState<editor.IStandaloneCodeEditor | null>(
        null,
    );

    // Decoration collection for active module state highlighting (seq steps, etc.)
    const activeDecorationRef =
        useRef<editor.IEditorDecorationsCollection | null>(null);

    // Poll module states for active step highlighting using the generic system
    // This uses argument_spans from Rust to know where arguments are in the document,
    // combined with source_spans for internal highlighting (like mini-notation spans)
    useEffect(() => {
        if (!editor || !monaco) return;
        return startModuleStatePolling({
            editor,
            monaco,
            currentFile,
            runningBufferId,
            activeDecorationRef,
            getModuleStates: () =>
                window.electronAPI.synthesizer.getModuleStates(),
        });
    }, [editor, monaco, currentFile, runningBufferId]);

    const activeScopeViews = useMemo(
        () => scopeViews.filter((view) => view.file === currentFile),
        [scopeViews, currentFile],
    );

    const handleMount: OnMount = (ed, monaco) => {
        setEditor(ed);
        editorRef.current = ed;

        const model = ed.getModel();
        if (model) {
            model.updateOptions({ tabSize: 2, insertSpaces: true });
        }

        // On Windows, Monaco swallows global accelerators, so we need to
        // register them as Monaco keybindings that trigger the Electron menu actions.
        // Ctrl+Enter -> Update Patch
        ed.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.Enter, () => {
            // Trigger the same IPC that the Electron menu sends
            window.electronAPI.triggerMenuAction('UPDATE_PATCH');
        });

        // Ctrl+. -> Stop Sound
        ed.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.Period, () => {
            window.electronAPI.triggerMenuAction('STOP');
        });

        // Ctrl+N -> New File
        ed.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyN, () => {
            window.electronAPI.triggerMenuAction('NEW_FILE');
        });

        // Ctrl+W -> Close Buffer
        ed.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyW, () => {
            window.electronAPI.triggerMenuAction('CLOSE_BUFFER');
        });
    };

    useEffect(() => {
        if (!monaco || !libSource) return;
        return setupMonacoJavascript(monaco, libSource, {
            schemas,
        });
    }, [monaco, libSource, schemas]);

    const {
        theme: appTheme,
        cursorStyle,
        font,
        fontLigatures,
        fontSize,
        prettierConfig,
    } = useTheme();
    const monacoThemeId = `theme-${appTheme.id}`;

    // Open help for DSL symbols on Cmd+Click (not Cmd+Hover)
    useEffect(() => {
        if (!editor || !monaco || schemas.length === 0) return;
        const { moduleNames, namespaceNames } = buildSymbolSets(schemas);
        const disposable = editor.onMouseDown((e) => {
            // Check for Cmd (Mac) / Ctrl (Win/Linux) + primary button click
            if (!e.event.metaKey && !e.event.ctrlKey) return;
            if (e.target.position == null) return;

            const model = editor.getModel();
            if (!model) return;

            editor.focus();
            editor.trigger('api', 'editor.action.peekDefinition', {});

            // console.log({ model, e });

            // const match = resolveDslSymbolAtPosition(
            //     model,
            //     e.target.position,
            //     moduleNames,
            //     namespaceNames,
            // );
            // if (match) {
            //     electronAPI.openHelpForSymbol(match.symbolType, match.symbolName);
            // }
        });
        return () => disposable.dispose();
    }, [editor, monaco, schemas]);

    useEffect(() => {
        if (!monaco) return;
        const disposable = registerDslFormattingProvider(
            monaco,
            prettierConfig,
        );
        return () => disposable.dispose();
    }, [monaco, prettierConfig]);

    // Register MIDI device autocomplete provider
    useEffect(() => {
        if (!monaco) return;
        const midiProvider = registerMidiCompletionProvider(monaco, () =>
            electronAPI.midi.listInputs(),
        );
        return () => midiProvider.dispose();
    }, [monaco]);

    useEffect(() => {
        if (!editor || !monaco) return;
        return createScopeViewZones({
            editor,
            monaco,
            views: activeScopeViews,
            onRegisterScopeCanvas,
            onUnregisterScopeCanvas,
        });
    }, [
        editor,
        monaco,
        activeScopeViews,
        onRegisterScopeCanvas,
        onUnregisterScopeCanvas,
    ]);

    // Define Monaco theme from the current app theme
    useEffect(() => {
        if (!monaco) return;
        applyMonacoTheme(monaco, appTheme, monacoThemeId);
    }, [monaco, appTheme, monacoThemeId]);

    // Configure JSON schema for config files
    useEffect(() => {
        if (!monaco) return;
        registerConfigSchema(monaco, configSchema);
    }, [monaco]);

    // Also configure schema when editing config file specifically
    useEffect(() => {
        if (!monaco || !currentFile?.endsWith('config.json')) return;
        registerConfigSchemaForFile(monaco, configSchema, currentFile);
    }, [monaco, currentFile]);

    // Determine language based on file extension
    const editorLanguage = useMemo(() => {
        if (!currentFile) return 'javascript';
        if (currentFile.endsWith('.json')) return 'json';
        return 'javascript';
    }, [currentFile]);

    return (
        <div className="patch-editor" style={{ height: '100%' }}>
            {currentFile && (
                <Editor
                    height="100%"
                    path={formatPath(currentFile)}
                    language={editorLanguage}
                    theme={monacoThemeId}
                    value={value}
                    onChange={(val) => {
                        onChange(val ?? '');
                    }}
                    onMount={handleMount}
                    options={{
                        minimap: { enabled: false },
                        lineNumbers: 'on',
                        folding: false,
                        matchBrackets: 'always',
                        automaticLayout: true,
                        fontFamily: `${font}, monospace`,
                        fontLigatures: fontLigatures,
                        fontSize: fontSize,
                        // lineHeight: 1.6,
                        padding: { top: 8, bottom: 8 },
                        renderLineHighlight: 'line',
                        cursorBlinking: 'solid',
                        cursorStyle: cursorStyle,
                        scrollbar: {
                            vertical: 'auto',
                            horizontal: 'auto',
                            verticalScrollbarSize: 8,
                            horizontalScrollbarSize: 8,
                        },
                        overviewRulerBorder: false,
                        hideCursorInOverviewRuler: true,
                        renderLineHighlightOnlyWhenFocus: false,
                        guides: {
                            indentation: true,
                            bracketPairs: false,
                        },
                    }}
                />
            )}
        </div>
    );
}
