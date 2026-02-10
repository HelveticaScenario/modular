/**
 * Utility for finding slider value literal positions in DSL source code.
 *
 * Uses lightweight string parsing (no ts-morph) to locate `slider(label, value, ...)`
 * calls by matching the label string literal. Returns character offsets of the value
 * argument so the UI can replace it via Monaco edits.
 *
 * This runs in the renderer process, so it must not depend on Node.js-only modules.
 */

import type { SliderDefinition } from '../../shared/dsl/sliderTypes';

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
export function findSliderValueSpan(source: string, label: string): SourceSpanResult | null {
    // Build regex to find slider( with the exact label string.
    // The label is a validated string literal, so we escape it for regex safety.
    const escapedLabel = label.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
    // Match: slider( optional-whitespace, "label" or 'label', optional-whitespace, comma
    const pattern = new RegExp(
        `\\bslider\\s*\\(\\s*(?:"${escapedLabel}"|'${escapedLabel}')\\s*,`,
        'g',
    );

    let match: RegExpExecArray | null;
    while ((match = pattern.exec(source)) !== null) {
        // match[0] ends right after the comma following the label
        const afterComma = match.index + match[0].length;

        // Skip whitespace after the comma
        let start = afterComma;
        while (start < source.length && /\s/.test(source[start])) start++;

        if (start >= source.length) continue;

        // Parse the numeric literal: optional minus, digits, optional decimal + digits
        const numMatch = source.slice(start).match(/^-?\d+(\.\d+)?([eE][+-]?\d+)?/);
        if (!numMatch) continue;

        return {
            start,
            end: start + numMatch[0].length,
        };
    }

    return null;
}

/**
 * Parse all slider definitions from DSL source code without executing it.
 * 
 * Uses regex to find slider(label, value, min, max) calls and extract their parameters.
 * This enables real-time label updates as the user types, without needing to execute the patch.
 * 
 * @param source - The full DSL source code
 * @returns Array of slider definitions found in the source
 */
export function parseSliderDefinitions(source: string): SliderDefinition[] {
    const sliders: SliderDefinition[] = [];
    
    // Pattern to match: slider( "label" or 'label', value, min, max )
    // This captures:
    // - Group 1: the quote character (" or ')
    // - Group 2: the label (at least one character required)
    // 
    // Note: The label capture uses `.+?` (non-greedy match) which captures the label
    // content literally without interpreting regex special characters. This works
    // because the label is inside quotes in the source code, so regex metacharacters
    // like $, *, +, ? are treated as literal characters in the label string.
    // Empty labels are rejected as they would be invalid/confusing UI.
    const sliderPattern = /\bslider\s*\(\s*(['"])(.+?)\1\s*,/g;
    
    let match: RegExpExecArray | null;
    // Track seen moduleIds to handle duplicate moduleId issue
    const seenModuleIds = new Map<string, number>();
    
    while ((match = sliderPattern.exec(source)) !== null) {
        const label = match[2]; // The captured label without quotes
        const afterLabel = match.index + match[0].length;
        
        // Parse the remaining arguments: value, min, max
        const remainingSource = source.slice(afterLabel);
        
        // Extract up to 3 numeric arguments
        const numericArgs = parseNumericArguments(remainingSource, 3);
        
        if (numericArgs.length >= 3) {
            const [value, min, max] = numericArgs;
            
            // Validate that min < max
            if (min < max && isFinite(value) && isFinite(min) && isFinite(max)) {
                // Generate unique moduleId, handling potential collisions
                const baseModuleId = `__slider_${label.replace(/[^a-zA-Z0-9_]/g, '_')}`;
                const occurrenceCount = (seenModuleIds.get(baseModuleId) || 0);
                seenModuleIds.set(baseModuleId, occurrenceCount + 1);
                
                const moduleId = occurrenceCount > 0 
                    ? `${baseModuleId}_${occurrenceCount}`
                    : baseModuleId;
                
                sliders.push({
                    moduleId,
                    label,
                    value,
                    min,
                    max,
                });
            }
        }
    }
    
    return sliders;
}

/**
 * Parse numeric arguments from a function call.
 * Extracts numeric literals (including scientific notation) separated by commas.
 * Supports formats like: 123, -45, 67.89, .5, 1e3, 2.5e-2
 * 
 * @param source - Source code starting after the opening parenthesis or comma
 * @param count - Maximum number of arguments to extract
 * @returns Array of parsed numbers
 */
function parseNumericArguments(source: string, count: number): number[] {
    const args: number[] = [];
    let pos = 0;
    
    for (let i = 0; i < count; i++) {
        // Skip whitespace
        while (pos < source.length && /\s/.test(source[pos])) {
            pos++;
        }
        
        if (pos >= source.length) break;
        
        // Match a numeric literal
        // Supports: -123, 45.67, .5, 1e3, 2.5e-2
        const numMatch = source.slice(pos).match(/^-?(\d+(\.\d*)?|\.\d+)([eE][+-]?\d+)?/);
        if (!numMatch) break;
        
        const numStr = numMatch[0];
        const num = parseFloat(numStr);
        args.push(num);
        pos += numStr.length;
        
        // Skip whitespace after the number
        while (pos < source.length && /\s/.test(source[pos])) {
            pos++;
        }
        
        // Expect a comma (unless this is the last argument we're looking for)
        if (i < count - 1) {
            if (pos < source.length && source[pos] === ',') {
                pos++; // Skip the comma
            } else {
                break; // No comma means we're done
            }
        }
    }
    
    return args;
}
