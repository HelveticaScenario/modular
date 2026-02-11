// Find all scope(...) calls and return where the call closes (end line + indices)
export type ScopeCallEnd = {
    startIndex: number;
    endIndex: number;
    endLine: number;
    endLineText: string;
};

export function findScopeCallEndLines(code: string): ScopeCallEnd[] {
    // Identify comment spans so we can ignore commented-out scope calls
    const commentRanges: Array<{ start: number; end: number }> = [];
    {
        let i = 0;
        let inLineComment = false;
        let lineCommentStart = 0;
        let inBlockComment = false;
        let blockCommentStart = 0;
        let inString: '"' | "'" | '`' | null = null;

        while (i < code.length) {
            const ch = code[i];
            const next = code[i + 1];

            if (inString) {
                if (ch === '\\') {
                    // Skip escaped character
                    i += 2;
                    continue;
                }
                if (ch === inString) {
                    inString = null;
                }
                i += 1;
                continue;
            }

            if (inLineComment) {
                if (ch === '\n') {
                    commentRanges.push({ start: lineCommentStart, end: i });
                    inLineComment = false;
                }
                i += 1;
                continue;
            }

            if (inBlockComment) {
                if (ch === '*' && next === '/') {
                    commentRanges.push({
                        start: blockCommentStart,
                        end: i + 2,
                    });
                    inBlockComment = false;
                    i += 2;
                    continue;
                }
                i += 1;
                continue;
            }

            if (ch === '"' || ch === "'" || ch === '`') {
                inString = ch;
                i += 1;
                continue;
            }

            if (ch === '/' && next === '/') {
                inLineComment = true;
                lineCommentStart = i;
                i += 2;
                continue;
            }

            if (ch === '/' && next === '*') {
                inBlockComment = true;
                blockCommentStart = i;
                i += 2;
                continue;
            }

            i += 1;
        }

        if (inLineComment) {
            commentRanges.push({ start: lineCommentStart, end: code.length });
        }
        if (inBlockComment) {
            commentRanges.push({ start: blockCommentStart, end: code.length });
        }
    }

    // Precompute line start offsets for fast index-to-line mapping
    const lineStarts = [0];
    for (let i = 0; i < code.length; i++) {
        if (code[i] === '\n') {
            lineStarts.push(i + 1);
        }
    }

    const lines = code.split(/\r?\n/);

    const indexToLine = (idx: number) => {
        // Binary search the greatest lineStart <= idx
        let low = 0;
        let high = lineStarts.length - 1;
        while (low <= high) {
            const mid = Math.floor((low + high) / 2);
            const start = lineStarts[mid];
            const nextStart =
                mid + 1 < lineStarts.length
                    ? lineStarts[mid + 1]
                    : code.length + 1;
            if (idx >= start && idx < nextStart) return mid + 1; // 1-based line number
            if (idx < start) {
                high = mid - 1;
            } else {
                low = mid + 1;
            }
        }
        return lineStarts.length; // Fallback to last line
    };

    const inComment = (idx: number) =>
        commentRanges.some(({ start, end }) => idx >= start && idx < end);

    const results: ScopeCallEnd[] = [];
    const pattern = /scope\s*\(/g;
    let match: RegExpExecArray | null;

    while ((match = pattern.exec(code)) !== null) {
        if (inComment(match.index)) continue;
        const openIdx = code.indexOf('(', match.index);
        if (openIdx === -1) continue;

        let depth = 0;
        let endIdx = -1;
        for (let i = openIdx; i < code.length; i++) {
            const ch = code[i];
            if (ch === '(') depth += 1;
            else if (ch === ')') {
                depth -= 1;
                if (depth === 0) {
                    endIdx = i;
                    break;
                }
            }
        }

        if (endIdx === -1) continue; // Unbalanced call; skip

        const endLine = indexToLine(endIdx);
        results.push({
            startIndex: match.index,
            endIndex: endIdx,
            endLine,
            endLineText: lines[endLine - 1] ?? '',
        });
    }

    return results;
}
