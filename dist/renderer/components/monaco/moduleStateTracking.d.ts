/**
 * Generic Module State Tracking
 *
 * A unified system for tracking module state and creating Monaco decorations
 * based on argument spans and internal source spans. Works for any module
 * with `#[args]` and optional `param_spans` in its state.
 *
 * Key concepts:
 * - `argument_spans`: Document offsets for each positional argument (from ts-morph analysis)
 * - `param_spans`: Map of param name -> { spans, source } for internal highlighting
 * - Combining them: document_offset = argument_spans[paramName].start + param_spans[paramName].spans[i]
 *
 * For template literals with interpolations, the system maps evaluated positions
 * back to source positions so highlighting works correctly.
 *
 * IMPORTANT: This system uses Monaco's tracked decorations with stickiness so that
 * decorations automatically move when the user types. We create tracked decorations
 * for each span when we first see a module's argument_spans, then during polling
 * we use model.getDecorationRange() to get the current (tracked) positions.
 * This applies to both interpolated and non-interpolated spans.
 */
import type React from 'react';
import type { editor } from 'monaco-editor';
import type { Monaco } from '../../hooks/useCustomMonaco';
import type { SourceSpan } from '../../../shared/dsl/spanTypes';
/**
 * Argument spans as they come from module state (document offsets)
 */
export interface ArgumentSpans {
    [argName: string]: SourceSpan;
}
/**
 * Internal span info for a single parameter
 */
export interface ParamSpanInfo {
    /** Currently active spans within this argument (offsets relative to argument content) */
    spans: [number, number][];
    /** The evaluated source string (for interpolation mapping) */
    source: string;
    /** All leaf spans in the pattern (for creating tracked decorations at patch time).
     * This is computed once when the pattern is parsed and doesn't change during playback.
     */
    all_spans?: [number, number][];
}
/**
 * Map of parameter name to its span info
 */
export interface ParamSpans {
    [paramName: string]: ParamSpanInfo;
}
/**
 * Generic module state structure
 */
export interface ModuleStateWithSpans {
    /** Spans for positional arguments (document offsets) */
    argument_spans?: ArgumentSpans;
    /** Map of param name -> { spans, source, all_spans } for internal highlighting */
    param_spans?: ParamSpans;
    /** Any other state fields */
    [key: string]: unknown;
}
/**
 * Parameters for starting module state polling
 */
export interface ModuleStatePollingParams {
    editor: editor.IStandaloneCodeEditor;
    monaco: Monaco;
    currentFile?: string;
    runningBufferId?: string | null;
    activeDecorationRef: React.MutableRefObject<editor.IEditorDecorationsCollection | null>;
    getModuleStates: () => Promise<Record<string, unknown>>;
    /** CSS class for active spans (default: 'active-seq-step') */
    activeClassName?: string;
    /** Polling interval in ms (default: 50) */
    pollInterval?: number;
}
/**
 * Start polling for module states and create decorations.
 *
 * This is a fully generic system that works with any module that has:
 * - `argument_spans`: Document offsets for positional arguments
 * - `param_spans`: Map of param name -> { spans, source }
 *
 * For each param with spans, it finds the corresponding argument_span,
 * handles interpolation mapping if needed, and creates Monaco decorations.
 *
 * IMPORTANT: For non-interpolated spans, we use Monaco's tracked decorations
 * with stickiness so they automatically move when the user types. We only
 * create these decorations once (when we first see the argument_spans),
 * then during polling we use model.getDecorationRange() to get current positions.
 */
export declare function startModuleStatePolling({ editor, monaco, currentFile, runningBufferId, activeDecorationRef, getModuleStates, activeClassName, pollInterval, }: ModuleStatePollingParams): () => void;
