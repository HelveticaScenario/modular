import { useEffect, useMemo, useRef } from 'react';
import Editor, { type OnMount, useMonaco } from '@monaco-editor/react';
import type { IDisposable } from 'monaco-editor';
import { useSchemas } from '../SchemaContext';
import type { ModuleSchema } from '../types';
import { buildLibSource } from '../dsl/typescriptLibGen';

export interface PatchEditorProps {
    value: string;
    onChange: (value: string) => void;
    onSubmit: () => void;
    onStop: () => void;
    onSave?: () => void;
    // Optional explicit schemas prop; if omitted, we fall back to context.
    schemas?: ModuleSchema[];
}

// Apply the generated DSL .d.ts library to Monaco and expose some
// debug handles on window so we can inspect schemas and lib source
// from the browser console.
function applyDslLibToMonaco(
    monaco: ReturnType<typeof useMonaco>,
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
        (window as any).__MONACO_DSL_SCHEMAS__ = schemas;
        (window as any).__MONACO_DSL_LIB__ = libSource;
    }

    const ts = monaco.typescript;
    const jsDefaults = ts.javascriptDefaults;
    extraLibDisposeRef.current = jsDefaults.addExtraLib(
        libSource,
        'file:///modular/dsl-lib.d.ts'
    );
}

export function MonacoPatchEditor({
    value,
    onChange,
    onSubmit,
    onStop,
    onSave,
}: PatchEditorProps) {
    const schemas = useSchemas();
    const extraLibDisposeRef = useRef<IDisposable | null>(null);
    const monaco = useMonaco();

    const isMac = useMemo(() => {
        if (typeof navigator === 'undefined') return false;
        const platform = navigator.platform || navigator.userAgent;
        return /Mac|iP(hone|ad|od)/.test(platform);
    }, []);

    const handleMount: OnMount = (editor, monaco) => {
        const model = editor.getModel();
        if (model) {
            model.updateOptions({ tabSize: 2, insertSpaces: true });
        }

        if (isMac) {
            editor.addCommand(monaco.KeyMod.Alt | monaco.KeyCode.Enter, () => {
                onSubmit();
            });
            editor.addCommand(monaco.KeyMod.Alt | monaco.KeyCode.Period, () => {
                onStop();
            });
        } else {
            editor.addCommand(
                monaco.KeyMod.CtrlCmd | monaco.KeyCode.Enter,
                () => {
                    onSubmit();
                }
            );
            editor.addCommand(
                monaco.KeyMod.CtrlCmd | monaco.KeyCode.Period,
                () => {
                    onStop();
                }
            );
        }

        editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyS, () => {
            if (onSave) {
                onSave();
            }
        });
    };

    useEffect(() => {
        console.log('MonacoPatchEditor mount');
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
        return () => {
            console.log(
                'MonacoPatchEditor unmount, disposing extra lib',
                extraLibDisposeRef.current
            );
            if (extraLibDisposeRef.current) {
                extraLibDisposeRef.current.dispose();
                extraLibDisposeRef.current = null;
            }
        };
    }, [monaco, schemas]);

    return (
        <div className="patch-editor" style={{ height: '100%' }}>
            <Editor
                height="100%"
                defaultLanguage="javascript"
                path="file:///modular/dsl.js"
                theme="vs-dark"
                value={value}
                onChange={(val) => onChange(val ?? '')}
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
