import { ModuleSchema, PatchGraph } from '@modular/core';
import { DSLContext, hz, note, bpm, setDSLWrapperLineOffset } from './factories';
import { $, $r, Signal, SourceLocation } from './GraphBuilder';

/**
 * Result of executing a DSL script.
 */
export interface DSLExecutionResult {
    /** The generated patch graph */
    patch: PatchGraph;
    /** Map from module ID to source location in DSL code */
    sourceLocationMap: Map<string, SourceLocation>;
}

/**
 * Execute a DSL script and return the resulting PatchGraph with source locations.
 */
export function executePatchScript(
    source: string,
    schemas: ModuleSchema[],
): DSLExecutionResult {
    // Create DSL context
    // console.log('Executing DSL script with schemas:', schemas);
    const context = new DSLContext(schemas);

    const clock = context.namespaceTree['clock'];
    if (typeof clock !== 'function') {
        throw new Error(
            'DSL execution error: "clock" module not found in schemas',
        );
    }

    const signal = context.namespaceTree['signal'];
    if (typeof signal !== 'function') {
        throw new Error(
            'DSL execution error: "signal" module not found in schemas',
        );
    }

    // Create default clock module that runs at 120 BPM
    const rootClock = clock(bpm(120), {
        id: 'ROOT_CLOCK',
    });
    // console.log('Created clock module:', rootClock);

    const rootInput = signal(
        Array.from({ length: 16 }, (_, i) => ({
            type: 'cable',
            module: 'HIDDEN_AUDIO_IN',
            port: 'input',
            channel: i,
        })),
        { id: 'ROOT_INPUT' },
    );

    // Create functions to set global tempo and output gain
    const builder = context.getBuilder();
    const setTempo = (tempo: Signal) => {
        builder.setTempo(tempo);
    };
    const setOutputGain = (gain: Signal) => {
        builder.setOutputGain(gain);
    };

    // Create the execution environment with all DSL functions
    const dslGlobals = {
        ...context.namespaceTree,
        // Helper functions
        hz,
        note,
        bpm,
        // Collection helpers
        $,
        $r,
        // Global settings
        setTempo,
        setOutputGain,
        // Built-in modules
        rootClock,
        input: rootInput,
    };

    // console.log(dslGlobals);

    // Build the function body - count wrapper lines for source mapping
    // When new Function() executes code, line numbers in stack traces are relative
    // to the function body string. The template literal structure plus new Function's
    // own wrapper results in user code starting at line 5 in stack traces.
    const wrapperLineCount = 4;
    setDSLWrapperLineOffset(wrapperLineCount);

    const functionBody = `
    'use strict';
    ${source}
  `;

    // Create parameter names and values
    const paramNames = Object.keys(dslGlobals);
    const paramValues = Object.values(dslGlobals);

    try {
        // Execute the script
        const fn = new Function(...paramNames, functionBody);
        fn(...paramValues);

        // Build and return the patch with source locations
        const builder = context.getBuilder();
        const patch = builder.toPatch();
        const sourceLocationMap = builder.getSourceLocationMap();

        return { patch, sourceLocationMap };
    } catch (error) {
        if (error instanceof Error) {
            throw new Error(`DSL execution error: ${error.message}`);
        }
        throw error;
    }
}

/**
 * Validate DSL script syntax without executing
 */
export function validateDSLSyntax(source: string): {
    valid: boolean;
    error?: string;
} {
    try {
        new Function(source);
        return { valid: true };
    } catch (error) {
        if (error instanceof Error) {
            return { valid: false, error: error.message };
        }
        return { valid: false, error: 'Unknown syntax error' };
    }
}
