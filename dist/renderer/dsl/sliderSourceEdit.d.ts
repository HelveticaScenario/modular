/**
 * Utility for finding slider value literal positions in DSL source code.
 *
 * Uses lightweight string parsing (no ts-morph) to locate `slider(label, value, ...)`
 * calls by matching the label string literal. Returns character offsets of the value
 * argument so the UI can replace it via Monaco edits.
 *
 * This runs in the renderer process, so it must not depend on Node.js-only modules.
 */
export interface SourceSpanResult {
    /** Inclusive start character offset */
    start: number;
    /** Exclusive end character offset */
    end: number;
}
/**
 * Find the character offset range of the `value` argument in a `slider(label, value, min, max)` call
 * whose label matches the given string.
 *
 * @param source - The full DSL source code
 * @param label  - The label string to match against
 * @returns The start/end offsets of the value argument literal, or null if not found
 */
export declare function findSliderValueSpan(source: string, label: string): SourceSpanResult | null;
