"use strict";
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.MonacoPatchEditor = MonacoPatchEditor;
const jsx_runtime_1 = require("react/jsx-runtime");
const react_1 = require("react");
const react_2 = __importDefault(require("@monaco-editor/react"));
const ThemeContext_1 = require("../themes/ThemeContext");
const useCustomMonaco_1 = require("../hooks/useCustomMonaco");
const configSchema_1 = require("../configSchema");
const monacoHelpers_1 = require("./monaco/monacoHelpers");
const monacoLanguage_1 = require("./monaco/monacoLanguage");
const definitionProvider_1 = require("./monaco/definitionProvider");
const formattingProvider_1 = require("./monaco/formattingProvider");
const theme_1 = require("./monaco/theme");
const jsonSchema_1 = require("./monaco/jsonSchema");
const scopeViewZones_1 = require("./monaco/scopeViewZones");
const moduleStateTracking_1 = require("./monaco/moduleStateTracking");
const midiCompletionProvider_1 = require("./monaco/midiCompletionProvider");
const electronAPI_1 = __importDefault(require("../electronAPI"));
function MonacoPatchEditor({ value, currentFile, onChange, editorRef, scopeViews = [], onRegisterScopeCanvas, onUnregisterScopeCanvas, runningBufferId, }) {
    // Fetch DSL lib source once at mount for Monaco autocomplete
    const [libSource, setLibSource] = (0, react_1.useState)(null);
    const [schemas, setSchemas] = (0, react_1.useState)([]);
    (0, react_1.useEffect)(() => {
        electronAPI_1.default.getDslLibSource().then(setLibSource).catch(console.error);
        electronAPI_1.default.getSchemas().then(setSchemas).catch(console.error);
    }, []);
    const monaco = (0, useCustomMonaco_1.useCustomMonaco)();
    const [editor, setEditor] = (0, react_1.useState)(null);
    // Decoration collection for active module state highlighting (seq steps, etc.)
    const activeDecorationRef = (0, react_1.useRef)(null);
    // Poll module states for active step highlighting using the generic system
    // This uses argument_spans from Rust to know where arguments are in the document,
    // combined with source_spans for internal highlighting (like mini-notation spans)
    (0, react_1.useEffect)(() => {
        if (!editor || !monaco)
            return;
        return (0, moduleStateTracking_1.startModuleStatePolling)({
            editor,
            monaco,
            currentFile,
            runningBufferId,
            activeDecorationRef,
            getModuleStates: () => window.electronAPI.synthesizer.getModuleStates(),
        });
    }, [editor, monaco, currentFile, runningBufferId]);
    const activeScopeViews = (0, react_1.useMemo)(() => scopeViews.filter((view) => view.file === currentFile), [scopeViews, currentFile]);
    const handleMount = (ed, monaco) => {
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
    };
    (0, react_1.useEffect)(() => {
        if (!monaco || !libSource)
            return;
        return (0, monacoLanguage_1.setupMonacoJavascript)(monaco, libSource, {
            schemas,
        });
    }, [monaco, libSource, schemas]);
    const { theme: appTheme, cursorStyle, font, fontLigatures, fontSize, prettierConfig } = (0, ThemeContext_1.useTheme)();
    const monacoThemeId = `theme-${appTheme.id}`;
    // Open help for DSL symbols on Cmd+Click (not Cmd+Hover)
    (0, react_1.useEffect)(() => {
        if (!editor || !monaco || schemas.length === 0)
            return;
        const { moduleNames, namespaceNames } = (0, definitionProvider_1.buildSymbolSets)(schemas);
        const disposable = editor.onMouseDown((e) => {
            // Check for Cmd (Mac) / Ctrl (Win/Linux) + primary button click
            if (!e.event.metaKey && !e.event.ctrlKey)
                return;
            if (e.target.position == null)
                return;
            const model = editor.getModel();
            if (!model)
                return;
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
    (0, react_1.useEffect)(() => {
        if (!monaco)
            return;
        const disposable = (0, formattingProvider_1.registerDslFormattingProvider)(monaco, prettierConfig);
        return () => disposable.dispose();
    }, [monaco, prettierConfig]);
    // Register MIDI device autocomplete provider
    (0, react_1.useEffect)(() => {
        if (!monaco)
            return;
        const midiProvider = (0, midiCompletionProvider_1.registerMidiCompletionProvider)(monaco, () => electronAPI_1.default.midi.listInputs());
        return () => midiProvider.dispose();
    }, [monaco]);
    (0, react_1.useEffect)(() => {
        if (!editor || !monaco)
            return;
        return (0, scopeViewZones_1.createScopeViewZones)({
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
    (0, react_1.useEffect)(() => {
        if (!monaco)
            return;
        (0, theme_1.applyMonacoTheme)(monaco, appTheme, monacoThemeId);
    }, [monaco, appTheme, monacoThemeId]);
    // Configure JSON schema for config files
    (0, react_1.useEffect)(() => {
        if (!monaco)
            return;
        (0, jsonSchema_1.registerConfigSchema)(monaco, configSchema_1.configSchema);
    }, [monaco]);
    // Also configure schema when editing config file specifically
    (0, react_1.useEffect)(() => {
        if (!monaco || !currentFile?.endsWith('config.json'))
            return;
        (0, jsonSchema_1.registerConfigSchemaForFile)(monaco, configSchema_1.configSchema, currentFile);
    }, [monaco, currentFile]);
    // Determine language based on file extension
    const editorLanguage = (0, react_1.useMemo)(() => {
        if (!currentFile)
            return 'javascript';
        if (currentFile.endsWith('.json'))
            return 'json';
        return 'javascript';
    }, [currentFile]);
    return ((0, jsx_runtime_1.jsx)("div", { className: "patch-editor", style: { height: '100%' }, children: currentFile && ((0, jsx_runtime_1.jsx)(react_2.default, { height: "100%", path: (0, monacoHelpers_1.formatPath)(currentFile), language: editorLanguage, theme: monacoThemeId, value: value, onChange: (val) => {
                onChange(val ?? '');
            }, onMount: handleMount, options: {
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
            } })) }));
}
//# sourceMappingURL=MonacoPatchEditor.js.map