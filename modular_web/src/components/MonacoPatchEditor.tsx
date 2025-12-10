import { useEffect, useMemo, useRef, useState } from 'react';
import Editor, { type OnMount, useMonaco } from '@monaco-editor/react';
import { editor, type IDisposable } from 'monaco-editor';
import prettier from 'prettier/standalone';
import prettierBabel from 'prettier/plugins/babel';
import prettierEstree from 'prettier/plugins/estree';
import { useSchemas } from '../SchemaContext';
import { buildLibSource } from '../dsl/typescriptLibGen';
import type { ModuleSchema } from '../types/generated/ModuleSchema';
import { findScopeCallEndLines } from '../utils/findScopeCallEndLines';

type Monaco = ReturnType<typeof useMonaco>;

declare global {
    interface Window {
        __MONACO_DSL_SCHEMAS__?: ModuleSchema[];
        __MONACO_DSL_LIB__?: string;
    }
}

export type ScopeView = {
    key: string;
    lineNumber: number;
};

export interface PatchEditorProps {
    value: string;
    onChange: (value: string) => void;
    onSubmit: () => void;
    onStop: () => void;
    onSave?: () => void;
    editorRef: React.RefObject<editor.IStandaloneCodeEditor | null>;
    scopeViews?: ScopeView[];
    onRegisterScopeCanvas?: (key: string, canvas: HTMLCanvasElement) => void;
    onUnregisterScopeCanvas?: (key: string) => void;
    // Optional explicit schemas prop; if omitted, we fall back to context.
    schemas?: ModuleSchema[];
}

// Apply the generated DSL .d.ts library to Monaco and expose some
// debug handles on window so we can inspect schemas and lib source
// from the browser console.
function applyDslLibToMonaco(
    monaco: Monaco,
    schemas: ModuleSchema[],
    extraLibDisposeRef: { current: IDisposable | null }
) {
    if (!monaco) return;

    if (extraLibDisposeRef.current) {
        extraLibDisposeRef.current.dispose();
        extraLibDisposeRef.current = null;
    }

    const libSource = buildLibSource(schemas);

    if (typeof window !== 'undefined') {
        window.__MONACO_DSL_SCHEMAS__ = schemas;
        window.__MONACO_DSL_LIB__ = libSource;
    }

    const ts = monaco.typescript;
    const jsDefaults = ts.javascriptDefaults;
    extraLibDisposeRef.current = jsDefaults.addExtraLib(
        libSource,
        'file:///modular/dsl-lib.d.ts'
    );
}

/**
 * Find all slider() calls in the code
 */
function findSliderCalls(code: string) {
    const regex =
        /slider\s*\(\s*(-?\d+(?:\.\d+)?)\s*,\s*(-?\d+(?:\.\d+)?)\s*,\s*(-?\d+(?:\.\d+)?)\s*\)/g;
    const matches = [];
    let match;

    while ((match = regex.exec(code)) !== null) {
        const startIndex = match.index;
        const endIndex = startIndex + match[0].length;
        const openParenIndex = code.indexOf('(', startIndex);
        const firstArgMatch = match[1];
        const firstArgStart = match[0].indexOf(firstArgMatch);

        matches.push({
            fullMatch: match[0],
            value: parseFloat(match[1]),
            min: parseFloat(match[2]),
            max: parseFloat(match[3]),
            startIndex,
            endIndex,
            openParenIndex,
            valueStartIndex: startIndex + firstArgStart,
            valueEndIndex: startIndex + firstArgStart + firstArgMatch.length,
        });
    }

    return matches;
}

export function MonacoPatchEditor({
    value,
    onChange,
    onSubmit,
    onStop,
    onSave,
    editorRef,
    scopeViews = [],
    onRegisterScopeCanvas,
    onUnregisterScopeCanvas,
}: PatchEditorProps) {
    const schemas = useSchemas();
    const extraLibDisposeRef = useRef<IDisposable | null>(null);
    const inlayHintDisposeRef = useRef<IDisposable | null>(null);
    const formattingProviderRef = useRef<IDisposable | null>(null);
    const viewZoneIdsRef = useRef<string[]>([]);
    const scopeCanvasMapRef = useRef<Map<string, HTMLCanvasElement>>(new Map());
    const layoutListenerRef = useRef<IDisposable | null>(null);
    const monaco = useMonaco();
    const [editor, setEditor] = useState<editor.IStandaloneCodeEditor | null>(
        null
    );

    const isMac = useMemo(() => {
        if (typeof navigator === 'undefined') return false;
        const platform = navigator.platform || navigator.userAgent;
        return /Mac|iP(hone|ad|od)/.test(platform);
    }, []);

    const handleMount: OnMount = (ed, monaco) => {
        setEditor(ed);
        editorRef.current = ed;

        const model = ed.getModel();
        if (model) {
            model.updateOptions({ tabSize: 2, insertSpaces: true });
        }

        if (isMac) {
            ed.addCommand(monaco.KeyMod.Alt | monaco.KeyCode.Enter, () => {
                onSubmit();
            });
            ed.addCommand(monaco.KeyMod.Alt | monaco.KeyCode.Period, () => {
                onStop();
            });
        } else {
            ed.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.Enter, () => {
                onSubmit();
            });
            ed.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.Period, () => {
                onStop();
            });
        }

        ed.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyS, () => {
            if (onSave) {
                onSave();
            }
        });
    };

    useEffect(() => {
        if (!monaco) return;
        const ts = monaco.typescript;
        const jsDefaults = ts.javascriptDefaults;

        jsDefaults.setCompilerOptions({
            allowJs: true,
            checkJs: true,
            allowNonTsExtensions: true,
            target: ts.ScriptTarget.ES2020,
            module: ts.ModuleKind.ESNext,
            noEmit: true,
        });

        jsDefaults.setDiagnosticsOptions({
            noSemanticValidation: false,
            noSyntaxValidation: false,
        });

        jsDefaults.setEagerModelSync(true);

        // Ensure the DSL library is registered as soon as the editor mounts,
        // using whatever schemas we currently have.
        applyDslLibToMonaco(monaco, schemas, extraLibDisposeRef);

        inlayHintDisposeRef.current =
            monaco.languages.registerInlayHintsProvider('javascript', {
                provideInlayHints(model, range) {
                    const code = model.getValueInRange(range);
                    const sliderCalls = findSliderCalls(code);
                    console.log(
                        'Providing inlay hints for slider calls:',
                        sliderCalls
                    );
                    return {
                        hints: sliderCalls.map((call) => {
                            const position = model.getPositionAt(
                                call.openParenIndex + 1
                            );
                            return {
                                position,
                                label: ' '.repeat(10), // 10 spaces for slider width
                            };
                        }),
                        dispose() {},
                    };
                },
            });
        return () => {
            if (extraLibDisposeRef.current) {
                extraLibDisposeRef.current.dispose();
                extraLibDisposeRef.current = null;
            }

            if (inlayHintDisposeRef.current) {
                inlayHintDisposeRef.current.dispose();
                inlayHintDisposeRef.current = null;
            }
        };
    }, [monaco, schemas]);

    useEffect(() => {
        if (!monaco) return;

        if (formattingProviderRef.current) {
            formattingProviderRef.current.dispose();
            formattingProviderRef.current = null;
        }

        // Use Prettier for Monaco's format action so users get consistent DSL formatting.
        formattingProviderRef.current =
            monaco.languages.registerDocumentFormattingEditProvider(
                'javascript',
                {
                    async provideDocumentFormattingEdits(model) {
                        const formatted = await prettier.format(
                            model.getValue(),
                            {
                                parser: 'babel',
                                plugins: [prettierBabel, prettierEstree],
                                singleQuote: true,
                                trailingComma: 'all',
                                semi: false,
                                tabWidth: 2,
                            }
                        );

                        return [
                            {
                                range: model.getFullModelRange(),
                                text: formatted.trimEnd(),
                            },
                        ];
                    },
                }
            );

        return () => {
            if (formattingProviderRef.current) {
                formattingProviderRef.current.dispose();
                formattingProviderRef.current = null;
            }
        };
    }, [monaco]);

    useEffect(() => {
        if (!monaco || !editor) return;
        const model = editor.getModel();
        if (!model) return;

        const code = editor.getValue();
        const calls = findScopeCallEndLines(code);
        console.log(
            'Found scope() calls:',
            calls.map((c) => c.endLine)
        );

        const sliderWidgets: editor.IContentWidget[] = createSliderWidgets(
            editor,
            model,
            monaco,
            code
        );
        return () => {
            for (const widget of sliderWidgets) {
                editor.removeContentWidget(widget);
            }
        };
    }, [monaco, editor, value]);

    useEffect(() => {
        if (!editor || !monaco) return;

        const disposeViewZones = () => {
            if (viewZoneIdsRef.current.length > 0) {
                editor.changeViewZones((accessor) => {
                    for (const id of viewZoneIdsRef.current) {
                        accessor.removeZone(id);
                    }
                });
                viewZoneIdsRef.current = [];
            }

            scopeCanvasMapRef.current.forEach((_canvas, key) => {
                onUnregisterScopeCanvas?.(key);
            });
            scopeCanvasMapRef.current.clear();

            if (layoutListenerRef.current) {
                layoutListenerRef.current.dispose();
                layoutListenerRef.current = null;
            }
        };

        disposeViewZones();

        if (scopeViews.length === 0) {
            return;
        }

        const dpr =
            typeof window !== 'undefined' ? window.devicePixelRatio || 1 : 1;
        const layoutInfo = editor.getLayoutInfo();

        const zones = scopeViews.map((view) => {
            const container = document.createElement('div');
            container.className = 'scope-view-zone';
            container.style.height = '120px';
            container.style.width = '100%';
            container.style.display = 'flex';

            const canvas = document.createElement('canvas');
            canvas.style.width = '100%';
            canvas.style.height = '120px';
            canvas.dataset.scopeKey = view.key;

            const pixelWidth = Math.max(
                1,
                Math.floor(layoutInfo.contentWidth * dpr)
            );
            const pixelHeight = Math.floor(120 * dpr);
            canvas.width = pixelWidth;
            canvas.height = pixelHeight;

            container.appendChild(canvas);

            scopeCanvasMapRef.current.set(view.key, canvas);
            onRegisterScopeCanvas?.(view.key, canvas);

            return { view, container };
        });

        editor.changeViewZones((accessor) => {
            viewZoneIdsRef.current = zones.map(({ view, container }) => {
                return accessor.addZone({
                    afterLineNumber: Math.max(1, view.lineNumber),
                    heightInPx: 120,
                    domNode: container,
                    marginDomNode: undefined,
                });
            });
        });

        const resizeCanvases = () => {
            const info = editor.getLayoutInfo();
            const nextDpr =
                typeof window !== 'undefined'
                    ? window.devicePixelRatio || 1
                    : 1;
            scopeCanvasMapRef.current.forEach((canvas) => {
                canvas.width = Math.max(
                    1,
                    Math.floor(info.contentWidth * nextDpr)
                );
                canvas.height = Math.floor(120 * nextDpr);
            });
        };

        layoutListenerRef.current = editor.onDidLayoutChange(resizeCanvases);

        return () => {
            disposeViewZones();
        };
    }, [
        editor,
        monaco,
        scopeViews,
        onRegisterScopeCanvas,
        onUnregisterScopeCanvas,
    ]);

    return (
        <div className="patch-editor" style={{ height: '100%' }}>
            <Editor
                height="100%"
                defaultLanguage="javascript"
                path="file:///modular/dsl.js"
                theme="vs-dark"
                value={value}
                onChange={(val) => {
                    onChange(val ?? '');
                }}
                onMount={handleMount}
                options={{
                    minimap: { enabled: false },
                    lineNumbers: 'on',
                    folding: true,
                    matchBrackets: 'always',
                    automaticLayout: true,
                }}
            />
        </div>
    );
}
function createSliderWidgets(
    editor: editor.IStandaloneCodeEditor,
    model: editor.ITextModel,
    monaco: Monaco,
    code: string
) {
    if (!monaco) return [];
    let sliderCalls = findSliderCalls(code);
    const sliderWidgets: editor.IContentWidget[] = [];
    for (const [index, call] of sliderCalls.entries()) {
        console.log('Slider call:', call);
        const position = model.getPositionAt(call.openParenIndex + 1);

        // Create slider widget DOM
        const widgetId = `slider-widget-${index}-${Date.now()}`;

        const slider = document.createElement('input');
        // slider.className = 'slider-widget';
        slider.style.width = `${
            editor.getOption(monaco.editor.EditorOption.fontInfo)
                .typicalHalfwidthCharacterWidth * 10
        }px`;
        slider.style.height = `${editor.getOption(
            monaco.editor.EditorOption.lineHeight
        )}px`;
        slider.style.pointerEvents = 'auto';

        // Map call.value between call.min and call.max
        const mappedValue = (call.value - call.min) / (call.max - call.min);

        slider.type = 'range';
        slider.min = '0';
        slider.max = '1';
        slider.value = mappedValue.toString(10);
        // slider.tabIndex = -1
        // Set appropriate step size
        slider.step = '0.01';

        // Update code when slider changes
        slider.addEventListener('input', (e: Event) => {
            console.log('Slider changed:', e);
            sliderCalls = findSliderCalls(editor.getValue());
            const call = sliderCalls[index];
            if (!call) return;
            const target = e.target as HTMLInputElement | null;
            const newValue = parseFloat(target?.value ?? '0');

            const valuePos = model.getPositionAt(call.valueStartIndex);
            const valueEndPos = model.getPositionAt(call.valueEndIndex);

            // // Format the number appropriately
            const formattedValue = newValue.toFixed(2);

            editor.executeEdits('slider-update', [
                {
                    range: new monaco.Range(
                        valuePos.lineNumber,
                        valuePos.column,
                        valueEndPos.lineNumber,
                        valueEndPos.column
                    ),
                    text: formattedValue,
                },
            ]);
        });

        const domNode = document.createElement('div');
        domNode.className = 'slider-widget';
        domNode.appendChild(slider);

        // Create and add content widget
        const contentWidget: editor.IContentWidget = {
            getId: () => widgetId,
            getDomNode: () => domNode,
            getPosition: () => ({
                position: {
                    lineNumber: position.lineNumber,
                    column: position.column,
                },
                preference: [
                    monaco.editor.ContentWidgetPositionPreference.EXACT,
                ],
            }),
        };

        editor.addContentWidget(contentWidget);
        sliderWidgets.push(contentWidget);
    }
    return sliderWidgets;
}
