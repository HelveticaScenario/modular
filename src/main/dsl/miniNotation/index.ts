/**
 * Public entry point for the TypeScript mini-notation implementation.
 *
 * `$p(source)` parses a mini-notation string into a serializable
 * `ParsedPattern` object that can be passed as an argument to `$cycle` or
 * `$iCycle`. The object is consumed by the Rust side during patch-graph
 * deserialization: its `ast`, `source`, and `all_spans` fields map
 * directly onto the Rust `SeqPatternParam` / `IntervalPatternParam`
 * deserialization payload.
 */

import type { MiniAST, ParsedPattern } from './ast';
import { collectLeafSpans } from './collectLeafSpans';
import { MiniParseError, parseMini } from './parser';
import { captureSourceLocation } from '../captureSourceLocation';
import { lookupArgumentSpan } from '../factories';

export type { MiniAST, ParsedPattern } from './ast';
export { MiniParseError } from './parser';

/**
 * Parse a mini-notation string into a `ParsedPattern`.
 *
 * The returned object is JSON-serializable and structurally compatible
 * with the Rust `{ ast, source, all_spans }` shape expected by
 * `SeqPatternParam` / `IntervalPatternParam` during patch-graph
 * deserialization.
 */
export function $p(source: string): ParsedPattern {
    if (typeof source !== 'string') {
        throw new MiniParseError(
            `$p() expects a string argument, got ${typeof source}`,
        );
    }
    const ast: MiniAST = parseMini(source);
    const all_spans = collectLeafSpans(ast);
    const sourceLocation = captureSourceLocation();
    const argument_span = lookupArgumentSpan(sourceLocation, 'source');
    const pattern: ParsedPattern = {
        __kind: 'ParsedPattern',
        ast,
        source,
        all_spans,
    };
    if (argument_span) {
        pattern.argument_span = argument_span;
    }
    return pattern;
}

/** Type guard for runtime `ParsedPattern` checks. */
export function isParsedPattern(value: unknown): value is ParsedPattern {
    return (
        typeof value === 'object' &&
        value !== null &&
        (value as { __kind?: unknown }).__kind === 'ParsedPattern'
    );
}
