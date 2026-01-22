import type React from 'react';
import type { editor } from 'monaco-editor';
import type { Monaco } from '../../hooks/useCustomMonaco';

/**
 * State returned by seq modules in the new pattern system.
 * Contains source_spans which are character offsets into the pattern string.
 */
interface SeqModuleState {
    active_hap?: {
        begin: number;
        end: number;
        is_rest: boolean;
    };
    source_spans?: [number, number][];
    /** The evaluated pattern string (what was passed to Rust parser) */
    pattern_source?: string;
}

/**
 * Represents an interpolation region in a template string.
 * sourceStart/sourceEnd are positions in the source code (including ${...})
 * evaluatedStart/evaluatedEnd are positions in the evaluated string (the result)
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
 * Returns null if not a template literal or has no interpolations.
 * 
 * @param sourcePattern - The pattern as it appears in source (may have ${...})
 * @param evaluatedPattern - The pattern after JS evaluation
 */
function extractInterpolationRegions(
    sourcePattern: string,
    evaluatedPattern: string,
): InterpolationRegion[] | null {
    // Find all ${...} in the source pattern
    // We need to handle nested braces, so we track brace depth
    const interpolationRegex = /\$\{/g;
    const regions: InterpolationRegion[] = [];
    let match;
    
    while ((match = interpolationRegex.exec(sourcePattern)) !== null) {
        const startIdx = match.index;
        let depth = 1;
        let endIdx = startIdx + 2; // Past the ${
        
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
                evaluatedStart: 0, // Will be computed
                evaluatedLen: 0, // Will be computed
            });
        }
    }
    
    if (regions.length === 0) return null;
    
    // Now we need to figure out what each interpolation evaluated to.
    // Strategy: use the literal text between interpolations as anchors.
    // Split source by interpolations to get literal pieces
    
    // Build an array of literal pieces from the source
    const literalPieces: { text: string; sourceStart: number; sourceEnd: number }[] = [];
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
    
    // Final literal piece after last interpolation
    if (pos < sourcePattern.length) {
        literalPieces.push({
            text: sourcePattern.slice(pos),
            sourceStart: pos,
            sourceEnd: sourcePattern.length,
        });
    }
    
    // Now match these literal pieces in the evaluated string to infer
    // where each interpolation result sits
    let evalPos = 0;
    let regionIdx = 0;
    
    for (let i = 0; i < literalPieces.length; i++) {
        const piece = literalPieces[i];
        const pieceIdx = evaluatedPattern.indexOf(piece.text, evalPos);
        
        if (pieceIdx === -1) {
            // Ambiguous/broken - bail out
            console.warn('Failed to match literal piece in evaluated pattern:', piece.text);
            return null;
        }
        
        // Check if there should be an interpolation before this literal piece
        // This is true if the literal's sourceStart is after an interpolation's sourceEnd
        const interpolationBeforeThisPiece = regionIdx < regions.length && 
            (i === 0 ? regions[0].sourceStart < piece.sourceStart : true);
        
        if (interpolationBeforeThisPiece) {
            // The interpolation result spans from evalPos to pieceIdx
            regions[regionIdx].evaluatedStart = evalPos;
            regions[regionIdx].evaluatedLen = pieceIdx - evalPos;
            regionIdx++;
        }
        
        evalPos = pieceIdx + piece.text.length;
    }
    
    // Handle interpolation at the very end (after all literal pieces)
    if (regionIdx < regions.length) {
        regions[regionIdx].evaluatedStart = evalPos;
        regions[regionIdx].evaluatedLen = evaluatedPattern.length - evalPos;
    }
    
    return regions;
}

/**
 * Build a position mapping function from evaluated string positions to source positions.
 * 
 * @param regions - Interpolation regions from extractInterpolationRegions
 * @param sourceLen - Length of the source pattern
 */
function buildPositionMapper(
    regions: InterpolationRegion[],
): (evalPos: number) => number | null {
    return (evalPos: number): number | null => {
        // Walk through regions to find which segment this position falls in
        let sourceOffset = 0;
        let evalOffset = 0;
        
        for (const region of regions) {
            // Calculate where this region starts in evaluated space
            const evalRegionStart = region.evaluatedStart;
            
            // If evalPos is before this interpolation, it's in a literal region
            if (evalPos < evalRegionStart) {
                // In a literal region - direct 1:1 mapping with current offset
                return evalPos + (sourceOffset - evalOffset);
            }
            
            // If evalPos is within this interpolation's result
            if (evalPos < evalRegionStart + region.evaluatedLen) {
                // Position is inside an interpolation result - no valid source mapping
                return null;
            }
            
            // Update offsets for next iteration
            // The difference grows by (sourceLen - evaluatedLen) for this region
            sourceOffset = region.sourceEnd;
            evalOffset = evalRegionStart + region.evaluatedLen;
        }
        
        // After all interpolations - in trailing literal region
        return evalPos + (sourceOffset - evalOffset);
    };
}

/**
 * Maps from seq module ID to a map of span ID -> decoration ID.
 * Span IDs are formatted as "start:end" (e.g., "0:2").
 */
type SpanDecorationMap = Map<string, Map<string, string>>;

/**
 * Info about a seq() call extracted from source code.
 */
interface SeqCallInfo {
    seqId: string;
    /** The pattern string as it appears in source (may contain ${...}) */
    sourcePattern: string;
    /** Offset from start of document to start of pattern content (after opening quote) */
    patternOffset: number;
    /** The quote character used - backtick means template literal */
    quoteChar: string;
    /** True if this is a template literal with interpolations */
    hasInterpolations: boolean;
}

/**
 * Cached info about seq calls for use during polling.
 */
interface SeqCallCache {
    /** Source pattern from code (may have ${...}) */
    sourcePattern: string;
    /** Offset to pattern content in document */
    patternOffset: number;
    /** True if this has interpolations */
    hasInterpolations: boolean;
    /** Position mapper for this pattern (created on first poll) */
    positionMapper?: ((evalPos: number) => number | null);
    /** The evaluated pattern this mapper was built for */
    evaluatedPatternForMapper?: string;
}

type BuildTrackingParams = {
    editor: editor.IStandaloneCodeEditor;
    monaco: Monaco;
    lastSubmittedCode: string;
    currentFile?: string;
    runningBufferId?: string | null;
    existingCollection?: editor.IEditorDecorationsCollection | null;
    getMiniLeafSpans: (pattern: string) => Promise<number[][]>;
};

type BuildTrackingResult = {
    spanDecorations: SpanDecorationMap;
    collection: editor.IEditorDecorationsCollection;
    /** Cached seq call info for patterns with interpolations */
    seqCallCache: Map<string, SeqCallCache>;
};

/**
 * Find all seq() calls in the code and return their pattern info.
 */
function findSeqCalls(code: string): SeqCallInfo[] {
    const regex = /seq\s*\(\s*(['"`])((?:(?!\1)[\s\S])*)\1/g;
    const results: SeqCallInfo[] = [];
    let match;
    let seqIndex = 1;

    while ((match = regex.exec(code)) !== null) {
        const quoteChar = match[1];
        const sourcePattern = match[2];
        const patternOffset = match.index + match[0].indexOf(quoteChar) + 1;
        const hasInterpolations = quoteChar === '`' && sourcePattern.includes('${');
        
        results.push({
            seqId: `seq-${seqIndex}`,
            sourcePattern,
            patternOffset,
            quoteChar,
            hasInterpolations,
        });
        seqIndex++;
    }

    return results;
}

/**
 * Build tracked decorations for all seq patterns at eval time.
 * 
 * This creates invisible decorations for every leaf in every seq pattern.
 * Monaco tracks these decorations as the user edits, so their positions
 * stay correct even when the code changes.
 * 
 * For patterns WITH interpolations (template literals with ${...}):
 * - We skip decoration creation here since we can't parse the source pattern
 * - Decorations will be created dynamically during polling when we get the evaluated pattern
 * 
 * For patterns WITHOUT interpolations:
 * - We parse and create tracked decorations as usual
 * - Span IDs (e.g., "0:2") let us match with hap source_spans during polling
 */
export async function buildSequenceTracking({
    editor,
    monaco,
    lastSubmittedCode,
    currentFile,
    runningBufferId,
    existingCollection,
    getMiniLeafSpans,
}: BuildTrackingParams): Promise<BuildTrackingResult | null> {
    if (!lastSubmittedCode) return null;
    if (currentFile !== runningBufferId) return null;

    const seqCalls = findSeqCalls(lastSubmittedCode);
    const model = editor.getModel();
    if (!model) return null;

    const spanDecorations: SpanDecorationMap = new Map();
    const decorationsToCreate: editor.IModelDeltaDecoration[] = [];
    const decorationMetadata: { seqId: string; spanId: string }[] = [];
    const seqCallCache = new Map<string, SeqCallCache>();

    for (const { seqId, sourcePattern, patternOffset, hasInterpolations } of seqCalls) {
        // Always cache the seq call info (needed for polling)
        seqCallCache.set(seqId, {
            sourcePattern,
            patternOffset,
            hasInterpolations,
        });

        // For patterns with interpolations, skip decoration creation
        // They'll be handled dynamically during polling
        if (hasInterpolations) {
            spanDecorations.set(seqId, new Map());
            continue;
        }

        try {
            const leafSpans = await getMiniLeafSpans(sourcePattern);
            const seqSpanMap = new Map<string, string>();
            spanDecorations.set(seqId, seqSpanMap);

            for (const [spanStart, spanEnd] of leafSpans) {
                const spanId = `${spanStart}:${spanEnd}`;
                
                // Convert pattern-relative offsets to document offsets
                const startOffset = patternOffset + spanStart;
                const endOffset = patternOffset + spanEnd;

                const startPos = model.getPositionAt(startOffset);
                const endPos = model.getPositionAt(endOffset);

                decorationsToCreate.push({
                    range: new monaco.Range(
                        startPos.lineNumber,
                        startPos.column,
                        endPos.lineNumber,
                        endPos.column,
                    ),
                    options: {
                        // Use AlwaysGrowsWhenTypingAtEdges so decorations track with text
                        stickiness:
                            monaco.editor.TrackedRangeStickiness
                                .AlwaysGrowsWhenTypingAtEdges,
                        // No visual style - we'll apply active-seq-step during polling
                    },
                });
                decorationMetadata.push({ seqId, spanId });
            }
        } catch (e) {
            console.error('Failed to parse pattern for', seqId, e);
        }
    }

    // Clear existing decorations
    if (existingCollection) {
        existingCollection.clear();
    }

    // Create new decorations
    const collection = editor.createDecorationsCollection();
    const ids = collection.set(decorationsToCreate);

    // Build the span -> decoration ID map
    for (let i = 0; i < ids.length; i++) {
        const { seqId, spanId } = decorationMetadata[i];
        spanDecorations.get(seqId)!.set(spanId, ids[i]);
    }

    return { spanDecorations, collection, seqCallCache };
}

type PollingParams = {
    editor: editor.IStandaloneCodeEditor;
    monaco: Monaco;
    currentFile?: string;
    runningBufferId?: string | null;
    spanDecorations: SpanDecorationMap;
    seqCallCache: Map<string, SeqCallCache>;
    activeStepCollectionRef: React.MutableRefObject<editor.IEditorDecorationsCollection | null>;
    getModuleStates: () => Promise<Record<string, unknown>>;
};

/**
 * Start polling for active seq steps.
 * 
 * This matches hap source_spans to tracked decorations by their span IDs,
 * then applies the active-seq-step class to the matching decorations.
 * 
 * For patterns WITHOUT interpolations:
 * - Directly match span IDs to decoration IDs
 * - Decorations track with text edits via Monaco
 * 
 * For patterns WITH interpolations:
 * - Build position mapping on first poll when we get the evaluated pattern
 * - Map evaluated span positions to source positions
 * - Create decorations dynamically (no edit tracking)
 */
export function startActiveStepPolling({
    editor,
    monaco,
    currentFile,
    runningBufferId,
    spanDecorations,
    seqCallCache,
    activeStepCollectionRef,
    getModuleStates,
}: PollingParams) {
    // Only track if we're viewing the running buffer
    if (currentFile !== runningBufferId) {
        if (activeStepCollectionRef.current) {
            activeStepCollectionRef.current.clear();
        }
        return () => {};
    }

    const interval = setInterval(async () => {
        try {
            const states = await getModuleStates();
            const newDecorations: editor.IModelDeltaDecoration[] = [];
            const model = editor.getModel();
            if (!model) return;

            for (const [id, state] of Object.entries(states)) {
                if (!id.startsWith('seq-')) continue;

                const seqState = state as SeqModuleState;
                const sourceSpans = seqState?.source_spans;
                const seqSpanMap = spanDecorations.get(id);
                const cache = seqCallCache.get(id);

                if (!sourceSpans || !cache) continue;

                // Skip rests - no highlighting
                if (seqState.active_hap?.is_rest) continue;

                // Handle patterns with interpolations using position mapping
                if (cache.hasInterpolations) {
                    const evaluatedPattern = seqState.pattern_source;
                    if (!evaluatedPattern) continue;

                    // Build position mapper on first poll (or if pattern changed)
                    if (cache.evaluatedPatternForMapper !== evaluatedPattern) {
                        const regions = extractInterpolationRegions(
                            cache.sourcePattern,
                            evaluatedPattern,
                        );
                        if (regions) {
                            cache.positionMapper = buildPositionMapper(regions);
                            cache.evaluatedPatternForMapper = evaluatedPattern;
                        } else {
                            // Failed to build mapper - skip this seq
                            cache.positionMapper = undefined;
                            cache.evaluatedPatternForMapper = evaluatedPattern;
                            continue;
                        }
                    }

                    if (!cache.positionMapper) continue;

                    // Map each span from evaluated space to source space
                    for (const [evalStart, evalEnd] of sourceSpans) {
                        const sourceStart = cache.positionMapper(evalStart);
                        const sourceEnd = cache.positionMapper(evalEnd);

                        // Skip spans that fall inside interpolation results
                        if (sourceStart === null || sourceEnd === null) continue;

                        // Convert to document positions
                        const startOffset = cache.patternOffset + sourceStart;
                        const endOffset = cache.patternOffset + sourceEnd;

                        const startPos = model.getPositionAt(startOffset);
                        const endPos = model.getPositionAt(endOffset);

                        newDecorations.push({
                            range: new monaco.Range(
                                startPos.lineNumber,
                                startPos.column,
                                endPos.lineNumber,
                                endPos.column,
                            ),
                            options: {
                                className: 'active-seq-step',
                                isWholeLine: false,
                            },
                        });
                    }
                } else {
                    // Standard case: use pre-built tracked decorations
                    if (!seqSpanMap) continue;

                    for (const [spanStart, spanEnd] of sourceSpans) {
                        const spanId = `${spanStart}:${spanEnd}`;
                        const decoId = seqSpanMap.get(spanId);

                        if (!decoId) continue;

                        // Get the current (tracked) range of this decoration
                        const range = model.getDecorationRange(decoId);
                        if (!range || range.isEmpty()) continue;

                        newDecorations.push({
                            range,
                            options: {
                                className: 'active-seq-step',
                                isWholeLine: false,
                            },
                        });
                    }
                }
            }

            if (activeStepCollectionRef.current) {
                activeStepCollectionRef.current.set(newDecorations);
            } else {
                activeStepCollectionRef.current =
                    editor.createDecorationsCollection(newDecorations);
            }
        } catch (e) {
            // ignore polling errors
        }
    }, 50);

    return () => clearInterval(interval);
}