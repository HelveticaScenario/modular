import type { SourceMapConsumer } from 'source-map-js';

/**
 * Active source map consumer for mapping V8 stack positions back to
 * original DSL source positions. Set by the executor before running
 * user code, cleared afterwards.
 */
let activeSourceMapConsumer: SourceMapConsumer | null = null;

export function setActiveSourceMapConsumer(
    consumer: SourceMapConsumer | null,
): void {
    activeSourceMapConsumer = consumer;
}

export function clearActiveSourceMapConsumer(): void {
    activeSourceMapConsumer = null;
}

/**
 * Capture source location from the current stack trace.
 * Looks for the `<anonymous>` frame which corresponds to DSL code executed
 * inside `new Function(...)` by `executePatchScript`.
 *
 * If an active source map consumer is set, maps the raw V8 position back
 * to the original DSL source position. Otherwise returns the raw position.
 *
 * Returns `undefined` if the source location cannot be determined.
 */
export function captureSourceLocation():
    | { line: number; column: number }
    | undefined {
    const stackHolder: { stack?: string } = {};
    Error.captureStackTrace(stackHolder, captureSourceLocation);

    if (!stackHolder.stack) {
        return undefined;
    }

    // Stack frames from evaluated code look like:
    // "    at eval (eval at executePatchScript ..., <anonymous>:5:12)"
    // or in some V8 versions:
    // "    at <anonymous>:5:12"
    const lines = stackHolder.stack.split('\n');

    for (const line of lines) {
        const anonymousMatch = line.match(/<anonymous>:(\d+):(\d+)/);
        if (anonymousMatch) {
            const rawLine = parseInt(anonymousMatch[1], 10);
            const rawCol = parseInt(anonymousMatch[2], 10);

            if (activeSourceMapConsumer) {
                // source-map-js uses 0-based columns; V8 uses 1-based
                const originalPosition =
                    activeSourceMapConsumer.originalPositionFor({
                        line: rawLine,
                        column: rawCol - 1,
                    });

                if (
                    originalPosition.line != null &&
                    originalPosition.column != null
                ) {
                    // Convert back to 1-based columns for our system
                    return {
                        line: originalPosition.line,
                        column: originalPosition.column + 1,
                    };
                }
            }

            // No source map or mapping failed — return raw position
            return { line: rawLine, column: rawCol };
        }
    }

    return undefined;
}
