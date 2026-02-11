/**
 * Tests for template interpolation position mapping in sequence tracking.
 */

import test from 'ava';

/**
 * Represents an interpolation region in a template string.
 */
interface InterpolationRegion {
    sourceStart: number;
    sourceEnd: number;
    sourceLen: number;
    evaluatedStart: number;
    evaluatedLen: number;
}

/**
 * Extract interpolation regions from a template literal pattern.
 * Duplicated from moduleStateTracking.ts for unit testing.
 */
function extractInterpolationRegions(
    sourcePattern: string,
    evaluatedPattern: string,
): InterpolationRegion[] | null {
    const interpolationRegex = /\$\{/g;
    const regions: InterpolationRegion[] = [];
    let match;

    while ((match = interpolationRegex.exec(sourcePattern)) !== null) {
        const startIdx = match.index;
        let depth = 1;
        let endIdx = startIdx + 2;

        while (endIdx < sourcePattern.length && depth > 0) {
            if (sourcePattern[endIdx] === '{') depth++;
            else if (sourcePattern[endIdx] === '}') depth--;
            endIdx++;
        }

        if (depth === 0) {
            regions.push({
                sourceStart: startIdx,
                sourceEnd: endIdx,
                sourceLen: endIdx - startIdx,
                evaluatedStart: 0,
                evaluatedLen: 0,
            });
        }
    }

    if (regions.length === 0) return null;

    const literalPieces: {
        text: string;
        sourceStart: number;
        sourceEnd: number;
    }[] = [];
    let pos = 0;

    for (const region of regions) {
        if (pos < region.sourceStart) {
            literalPieces.push({
                text: sourcePattern.slice(pos, region.sourceStart),
                sourceStart: pos,
                sourceEnd: region.sourceStart,
            });
        }
        pos = region.sourceEnd;
    }

    if (pos < sourcePattern.length) {
        literalPieces.push({
            text: sourcePattern.slice(pos),
            sourceStart: pos,
            sourceEnd: sourcePattern.length,
        });
    }

    let evalPos = 0;
    let regionIdx = 0;

    for (let i = 0; i < literalPieces.length; i++) {
        const piece = literalPieces[i];
        const pieceIdx = evaluatedPattern.indexOf(piece.text, evalPos);

        if (pieceIdx === -1) {
            return null;
        }

        // Check if there should be an interpolation before this literal piece
        // This is true if the literal's sourceStart is after an interpolation's sourceEnd
        const interpolationBeforeThisPiece =
            regionIdx < regions.length &&
            (i === 0 ? regions[0].sourceStart < piece.sourceStart : true);

        if (interpolationBeforeThisPiece) {
            // The interpolation result spans from evalPos to pieceIdx
            regions[regionIdx].evaluatedStart = evalPos;
            regions[regionIdx].evaluatedLen = pieceIdx - evalPos;
            regionIdx++;
        }

        evalPos = pieceIdx + piece.text.length;
    }

    if (regionIdx < regions.length) {
        regions[regionIdx].evaluatedStart = evalPos;
        regions[regionIdx].evaluatedLen = evaluatedPattern.length - evalPos;
    }

    return regions;
}

/**
 * Build a position mapping function from evaluated string positions to source positions.
 */
function buildPositionMapper(
    regions: InterpolationRegion[],
): (evalPos: number) => number | null {
    return (evalPos: number): number | null => {
        let sourceOffset = 0;
        let evalOffset = 0;

        for (const region of regions) {
            const evalRegionStart = region.evaluatedStart;

            if (evalPos < evalRegionStart) {
                return evalPos + (sourceOffset - evalOffset);
            }

            if (evalPos < evalRegionStart + region.evaluatedLen) {
                return null;
            }

            sourceOffset = region.sourceEnd;
            evalOffset = evalRegionStart + region.evaluatedLen;
        }

        return evalPos + (sourceOffset - evalOffset);
    };
}

// interpolation region extraction tests

test('extractInterpolationRegions: handles single interpolation at start', (t: any) => {
    // ${note} c e where note='g'
    const regions = extractInterpolationRegions('${note} c e', 'g c e');
    t.deepEqual(regions, [
        {
            sourceStart: 0,
            sourceEnd: 7,
            sourceLen: 7,
            evaluatedStart: 0,
            evaluatedLen: 1,
        },
    ]);
});

test('extractInterpolationRegions: handles single interpolation in middle', (t: any) => {
    // a ${x} b where x='XX'
    const regions = extractInterpolationRegions('a ${x} b', 'a XX b');
    t.deepEqual(regions, [
        {
            sourceStart: 2,
            sourceEnd: 6,
            sourceLen: 4,
            evaluatedStart: 2,
            evaluatedLen: 2,
        },
    ]);
});

test('extractInterpolationRegions: handles single interpolation at end', (t: any) => {
    // a b ${x} where x='c'
    const regions = extractInterpolationRegions('a b ${x}', 'a b c');
    t.deepEqual(regions, [
        {
            sourceStart: 4,
            sourceEnd: 8,
            sourceLen: 4,
            evaluatedStart: 4,
            evaluatedLen: 1,
        },
    ]);
});

test('extractInterpolationRegions: handles multiple interpolations', (t: any) => {
    // a ${x} b ${y} c where x='XX', y='YYY'
    const regions = extractInterpolationRegions(
        'a ${x} b ${y} c',
        'a XX b YYY c',
    );
    t.deepEqual(regions, [
        {
            sourceStart: 2,
            sourceEnd: 6,
            sourceLen: 4,
            evaluatedStart: 2,
            evaluatedLen: 2,
        },
        {
            sourceStart: 9,
            sourceEnd: 13,
            sourceLen: 4,
            evaluatedStart: 7,
            evaluatedLen: 3,
        },
    ]);
});

test('extractInterpolationRegions: returns null for patterns without interpolations', (t: any) => {
    const regions = extractInterpolationRegions('a b c', 'a b c');
    t.is(regions, null);
});

test('extractInterpolationRegions: handles empty interpolation result', (t: any) => {
    // ${x} b where x=''
    const regions = extractInterpolationRegions('${x} b', ' b');
    t.deepEqual(regions, [
        {
            sourceStart: 0,
            sourceEnd: 4,
            sourceLen: 4,
            evaluatedStart: 0,
            evaluatedLen: 0,
        },
    ]);
});

// position mapping tests

test('buildPositionMapper: maps positions for single interpolation at start', (t: any) => {
    // ${note} c e where note='g'
    const regions = extractInterpolationRegions('${note} c e', 'g c e')!;
    const map = buildPositionMapper(regions);

    // Position 0 is inside interpolation result
    t.is(map(0), null);

    // Position 1 maps to source position 7 (the space after ${note})
    t.is(map(1), 7);

    // Position 2 maps to source position 8 (the 'c')
    t.is(map(2), 8);

    // Position 4 maps to source position 10 (the 'e')
    t.is(map(4), 10);
});

test('buildPositionMapper: maps positions for interpolation in middle', (t: any) => {
    // a ${x} b where x='XX'
    const regions = extractInterpolationRegions('a ${x} b', 'a XX b')!;
    const map = buildPositionMapper(regions);

    // Positions 0-1 are before interpolation
    t.is(map(0), 0); // 'a'
    t.is(map(1), 1); // ' ' before ${x}

    // Positions 2-3 are inside interpolation result
    t.is(map(2), null);
    t.is(map(3), null);

    // Position 4 maps to source position 6 (the space after ${x})
    t.is(map(4), 6);

    // Position 5 maps to source position 7 (the 'b')
    t.is(map(5), 7);
});

test('buildPositionMapper: maps positions for multiple interpolations', (t: any) => {
    // a ${x} b ${y} c where x='XX', y='YYY'
    const regions = extractInterpolationRegions(
        'a ${x} b ${y} c',
        'a XX b YYY c',
    )!;
    const map = buildPositionMapper(regions);

    // Before first interpolation
    t.is(map(0), 0); // 'a'
    t.is(map(1), 1); // ' '

    // Inside first interpolation
    t.is(map(2), null);
    t.is(map(3), null);

    // Between interpolations
    t.is(map(4), 6); // ' ' after ${x}
    t.is(map(5), 7); // 'b'
    t.is(map(6), 8); // ' ' before ${y}

    // Inside second interpolation
    t.is(map(7), null);
    t.is(map(8), null);
    t.is(map(9), null);

    // After second interpolation
    t.is(map(10), 13); // ' ' after ${y}
    t.is(map(11), 14); // 'c'
});
