"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.executePatchScript = executePatchScript;
exports.validateDSLSyntax = validateDSLSyntax;
const factories_1 = require("./factories");
const GraphBuilder_1 = require("./GraphBuilder");
const sourceSpanAnalyzer_1 = require("./sourceSpanAnalyzer");
const spanTypes_1 = require("../../shared/dsl/spanTypes");
/**
 * Execute a DSL script and return the resulting PatchGraph with source locations.
 */
function executePatchScript(source, schemas) {
    // Create DSL context
    // console.log('Executing DSL script with schemas:', schemas);
    const context = new factories_1.DSLContext(schemas);
    const clock = context.namespaceTree['clock'];
    if (typeof clock !== 'function') {
        throw new Error('DSL execution error: "clock" module not found in schemas');
    }
    const signal = context.namespaceTree['signal'];
    if (typeof signal !== 'function') {
        throw new Error('DSL execution error: "signal" module not found in schemas');
    }
    // Create default clock module that runs at 120 BPM
    const rootClock = clock((0, factories_1.bpm)(120), {
        id: 'ROOT_CLOCK',
    });
    // console.log('Created clock module:', rootClock);
    const rootInput = signal(Array.from({ length: 16 }, (_, i) => ({
        type: 'cable',
        module: 'HIDDEN_AUDIO_IN',
        port: 'input',
        channel: i,
    })), { id: 'ROOT_INPUT' });
    // Create functions to set global tempo and output gain
    const builder = context.getBuilder();
    const setTempo = (tempo) => {
        builder.setTempo(tempo);
    };
    const setOutputGain = (gain) => {
        builder.setOutputGain(gain);
    };
    /**
     * Create a DeferredCollection with placeholder signals that can be assigned later.
     * Useful for feedback loops and forward references.
     * @param channels - Number of deferred outputs (1-16, default 1)
     */
    const deferred = (channels = 1) => {
        if (channels < 1 || channels > 16) {
            throw new Error(`deferred() channels must be between 1 and 16, got ${channels}`);
        }
        const items = [];
        for (let i = 0; i < channels; i++) {
            items.push(new GraphBuilder_1.DeferredModuleOutput(builder));
        }
        return new GraphBuilder_1.DeferredCollection(...items);
    };
    // Slider collector — populated by slider() calls during execution
    const sliders = [];
    /**
     * Create a slider control: a signal module with a UI slider bound to it.
     * @param label - Display label (must be a string literal)
     * @param value - Initial value (must be a numeric literal)
     * @param min - Minimum value
     * @param max - Maximum value
     * @returns The signal module's output
     */
    const slider = (label, value, min, max) => {
        if (typeof label !== 'string') {
            throw new Error('slider() label must be a string literal');
        }
        if (typeof value !== 'number' || !isFinite(value)) {
            throw new Error('slider() value must be a finite number literal');
        }
        if (typeof min !== 'number' || !isFinite(min)) {
            throw new Error('slider() min must be a finite number');
        }
        if (typeof max !== 'number' || !isFinite(max)) {
            throw new Error('slider() max must be a finite number');
        }
        if (min >= max) {
            throw new Error(`slider() min (${min}) must be less than max (${max})`);
        }
        const moduleId = `__slider_${label.replace(/[^a-zA-Z0-9_]/g, '_')}`;
        // Create backing signal module via the existing signal factory
        const result = signal(value, { id: moduleId });
        sliders.push({ moduleId, label, value, min, max });
        return result;
    };
    console.log(context.namespaceTree);
    // Create the execution environment with all DSL functions
    const dslGlobals = {
        $: { ...context.namespaceTree },
        // Helper functions
        hz: factories_1.hz,
        note: factories_1.note,
        bpm: factories_1.bpm,
        // Collection helpers
        $c: GraphBuilder_1.$c,
        $r: GraphBuilder_1.$r,
        // Deferred signal helper
        deferred,
        // Slider control
        slider,
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
    (0, factories_1.setDSLWrapperLineOffset)(wrapperLineCount);
    // The function body template indents the first line of source with 4 spaces
    // This affects the column reported by V8 for the first line only
    const firstLineColumnOffset = 4;
    // Analyze source code to extract argument spans before execution
    // The registry maps call-site keys (line:column) to argument span info
    const { registry: spanRegistry, interpolationResolutions } = (0, sourceSpanAnalyzer_1.analyzeSourceSpans)(source, schemas, wrapperLineCount, firstLineColumnOffset);
    (0, factories_1.setActiveSpanRegistry)(spanRegistry);
    (0, spanTypes_1.setActiveInterpolationResolutions)(interpolationResolutions);
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
        return { patch, sourceLocationMap, interpolationResolutions, sliders };
    }
    catch (error) {
        if (error instanceof Error) {
            throw new Error(`DSL execution error: ${error.message}`);
        }
        throw error;
    }
    finally {
        // Clear the span registry after execution — spans are already baked into
        // module state via ARGUMENT_SPANS_KEY so the registry isn't needed anymore.
        (0, factories_1.setActiveSpanRegistry)(null);
        // NOTE: Do NOT clear interpolation resolutions here. They are read
        // asynchronously by moduleStateTracking during decoration polling and
        // must persist until the next execution replaces them.
    }
}
/**
 * Validate DSL script syntax without executing
 */
function validateDSLSyntax(source) {
    try {
        new Function(source);
        return { valid: true };
    }
    catch (error) {
        if (error instanceof Error) {
            return { valid: false, error: error.message };
        }
        return { valid: false, error: 'Unknown syntax error' };
    }
}
//# sourceMappingURL=executor.js.map