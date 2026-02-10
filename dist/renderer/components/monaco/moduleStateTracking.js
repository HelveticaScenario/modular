"use strict";
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
Object.defineProperty(exports, "__esModule", { value: true });
exports.startModuleStatePolling = startModuleStatePolling;
const spanTypes_1 = require("../../../shared/dsl/spanTypes");
/**
 * Extract interpolation regions from a template literal.
 * Maps ${...} regions in source to their evaluated result positions.
 */
function extractInterpolationRegions(sourceContent, evaluatedContent) {
    const interpolationRegex = /\$\{/g;
    const regions = [];
    let match;
    while ((match = interpolationRegex.exec(sourceContent)) !== null) {
        const startIdx = match.index;
        let depth = 1;
        let endIdx = startIdx + 2;
        while (endIdx < sourceContent.length && depth > 0) {
            if (sourceContent[endIdx] === '{')
                depth++;
            else if (sourceContent[endIdx] === '}')
                depth--;
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
    if (regions.length === 0)
        return null;
    // Build literal pieces for mapping
    const literalPieces = [];
    let pos = 0;
    for (const region of regions) {
        if (pos < region.sourceStart) {
            literalPieces.push({
                text: sourceContent.slice(pos, region.sourceStart),
                sourceStart: pos,
                sourceEnd: region.sourceStart,
            });
        }
        pos = region.sourceEnd;
    }
    if (pos < sourceContent.length) {
        literalPieces.push({
            text: sourceContent.slice(pos),
            sourceStart: pos,
            sourceEnd: sourceContent.length,
        });
    }
    // Match literal pieces in evaluated string
    let evalPos = 0;
    let regionIdx = 0;
    for (let i = 0; i < literalPieces.length; i++) {
        const piece = literalPieces[i];
        const pieceIdx = evaluatedContent.indexOf(piece.text, evalPos);
        if (pieceIdx === -1) {
            return null;
        }
        const interpolationBeforeThisPiece = regionIdx < regions.length &&
            (i === 0
                ? regions[0].sourceStart < piece.sourceStart
                : true);
        if (interpolationBeforeThisPiece) {
            regions[regionIdx].evaluatedStart = evalPos;
            regions[regionIdx].evaluatedLen = pieceIdx - evalPos;
            regionIdx++;
        }
        evalPos = pieceIdx + piece.text.length;
    }
    if (regionIdx < regions.length) {
        regions[regionIdx].evaluatedStart = evalPos;
        regions[regionIdx].evaluatedLen = evaluatedContent.length - evalPos;
    }
    return regions;
}
/**
 * Build interpolation regions using accurate data from the interpolation
 * resolution map (computed by sourceSpanAnalyzer via ts-morph).
 *
 * This avoids the fragile indexOf-based text matching in extractInterpolationRegions,
 * which can fail when the interpolated content contains substrings matching the
 * template's literal text (e.g., `${interpolated} 2 3` where interpolated = '0 2 3 ...').
 */
function buildInterpolationRegionsFromResolutions(sourceContent, resolutions) {
    // Find ${...} in source to get source-side positions
    const interpolationRegex = /\$\{/g;
    const sourceRegions = [];
    let match;
    while ((match = interpolationRegex.exec(sourceContent)) !== null) {
        const startIdx = match.index;
        let depth = 1;
        let endIdx = startIdx + 2;
        while (endIdx < sourceContent.length && depth > 0) {
            if (sourceContent[endIdx] === '{')
                depth++;
            else if (sourceContent[endIdx] === '}')
                depth--;
            endIdx++;
        }
        if (depth === 0) {
            sourceRegions.push({ sourceStart: startIdx, sourceEnd: endIdx });
        }
    }
    if (sourceRegions.length === 0 || sourceRegions.length !== resolutions.length)
        return null;
    return sourceRegions.map((sr, i) => ({
        sourceStart: sr.sourceStart,
        sourceEnd: sr.sourceEnd,
        sourceLen: sr.sourceEnd - sr.sourceStart,
        evaluatedStart: resolutions[i].evaluatedStart,
        evaluatedLen: resolutions[i].evaluatedLength,
    }));
}
/**
 * Build a position mapper from evaluated to source positions
 */
function buildPositionMapper(regions) {
    return (evalPos) => {
        let sourceOffset = 0;
        let evalOffset = 0;
        for (const region of regions) {
            const evalRegionStart = region.evaluatedStart;
            if (evalPos < evalRegionStart) {
                return evalPos + (sourceOffset - evalOffset);
            }
            if (evalPos < evalRegionStart + region.evaluatedLen) {
                return null; // Inside interpolation result
            }
            sourceOffset = region.sourceEnd;
            evalOffset = evalRegionStart + region.evaluatedLen;
        }
        return evalPos + (sourceOffset - evalOffset);
    };
}
/**
 * Resolve an evaluated position that falls inside an interpolation result
 * to a document offset by looking up the interpolation resolution map.
 *
 * When a template literal contains `${someConst}` and the position mapper
 * returns null (position is inside the interpolation result), this function
 * redirects the highlight to the original const literal's location in the document.
 *
 * Handles recursive resolution: if the const is itself a template with
 * interpolations, recurses into nested resolutions.
 *
 * @param evalPos - Position in evaluated string that fell inside an interpolation
 * @param resolutions - Resolved interpolations for this argument span
 * @returns Document offset to highlight, or null if no resolution found
 */
function resolveInterpolatedPosition(evalPos, resolutions) {
    for (const r of resolutions) {
        const rEnd = r.evaluatedStart + r.evaluatedLength;
        // Use <= for the end check because span ends are exclusive:
        // a Rust span [0, 2] means characters 0-1, and position 2 is the
        // exclusive end that should map to the exclusive end of the const literal.
        if (evalPos >= r.evaluatedStart && evalPos <= rEnd) {
            const offsetInResult = evalPos - r.evaluatedStart;
            // If the const has nested resolutions (it's a template with interpolations),
            // check if this offset falls inside one of the nested interpolations
            if (r.nestedResolutions && r.nestedResolutions.length > 0) {
                const nestedResult = resolveInterpolatedPosition(offsetInResult, r.nestedResolutions);
                if (nestedResult !== null)
                    return nestedResult;
            }
            // Simple case or fallback: map directly into the const literal
            // +1 to skip the opening quote character
            return r.constLiteralSpan.start + 1 + offsetInResult;
        }
    }
    return null;
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
function startModuleStatePolling({ editor, monaco, currentFile, runningBufferId, activeDecorationRef, getModuleStates, activeClassName = 'active-seq-step', pollInterval = 50, }) {
    // Only track if viewing the running buffer
    if (currentFile !== runningBufferId) {
        if (activeDecorationRef.current) {
            activeDecorationRef.current.clear();
        }
        return () => { };
    }
    // Global cache for all modules and their params
    const globalCache = new Map();
    const interval = setInterval(async () => {
        try {
            const states = await getModuleStates();
            const newDecorations = [];
            const model = editor.getModel();
            if (!model)
                return;
            // Clean up cache entries for modules that no longer exist in the patch.
            // Without this, tracked decorations from removed modules would linger.
            for (const [cachedModuleId, moduleCache] of globalCache) {
                if (!(cachedModuleId in states)) {
                    for (const paramCache of moduleCache.values()) {
                        if (paramCache.decorationCollection) {
                            paramCache.decorationCollection.clear();
                        }
                    }
                    globalCache.delete(cachedModuleId);
                }
            }
            for (const [moduleId, state] of Object.entries(states)) {
                const typedState = state;
                // Need both argument_spans and param_spans
                const argumentSpans = typedState.argument_spans;
                const paramSpans = typedState.param_spans;
                if (!argumentSpans || !paramSpans)
                    continue;
                // Get or create module cache
                let moduleCache = globalCache.get(moduleId);
                if (!moduleCache) {
                    moduleCache = new Map();
                    globalCache.set(moduleId, moduleCache);
                }
                // Process each param that has spans
                for (const [paramName, paramInfo] of Object.entries(paramSpans)) {
                    const { spans, source: evaluatedSource } = paramInfo;
                    // Skip if no spans to highlight
                    if (!spans || spans.length === 0)
                        continue;
                    // Get the document position for this argument
                    const argSpan = argumentSpans[paramName];
                    if (!argSpan)
                        continue;
                    // Get or create param cache
                    let paramCache = moduleCache.get(paramName);
                    if (!paramCache) {
                        paramCache = { hasInterpolations: false };
                        moduleCache.set(paramName, paramCache);
                    }
                    // Check if argument span changed (new patch was submitted)
                    const argSpanChanged = !paramCache.argumentSpan ||
                        paramCache.argumentSpan.start !== argSpan.start ||
                        paramCache.argumentSpan.end !== argSpan.end;
                    if (argSpanChanged) {
                        // Clear old tracked decorations if any
                        if (paramCache.decorationCollection) {
                            paramCache.decorationCollection.clear();
                        }
                        paramCache.trackedDecorationIds = undefined;
                        paramCache.decorationCollection = undefined;
                        paramCache.trackedDecorationsCreated = false;
                        paramCache.argumentSpan = argSpan;
                        paramCache.positionMapper = undefined;
                        paramCache.evaluatedContentForMapper = undefined;
                        // Extract source content from document
                        const startPos = model.getPositionAt(argSpan.start);
                        const endPos = model.getPositionAt(argSpan.end);
                        const sourceText = model.getValueInRange({
                            startLineNumber: startPos.lineNumber,
                            startColumn: startPos.column,
                            endLineNumber: endPos.lineNumber,
                            endColumn: endPos.column,
                        });
                        // Check if it's a template literal with interpolations
                        paramCache.hasInterpolations = sourceText.includes('${');
                        paramCache.sourceContent = sourceText;
                    }
                    // Process spans with or without interpolation mapping
                    if (paramCache.hasInterpolations && evaluatedSource) {
                        // Look up interpolation resolutions once for this param
                        const interpolationResolutions = (0, spanTypes_1.getActiveInterpolationResolutions)();
                        const spanKey = `${argSpan.start}:${argSpan.end}`;
                        const resolutions = interpolationResolutions?.get(spanKey);
                        // Build mapper if needed (evaluated source changed)
                        if (paramCache.evaluatedContentForMapper !== evaluatedSource) {
                            // Strip quotes from source content for mapping
                            let sourceWithoutQuotes = paramCache.sourceContent || '';
                            if (sourceWithoutQuotes.startsWith('`') ||
                                sourceWithoutQuotes.startsWith('"') ||
                                sourceWithoutQuotes.startsWith("'")) {
                                sourceWithoutQuotes = sourceWithoutQuotes.slice(1, -1);
                            }
                            // Prefer building regions from resolution data (accurate)
                            // over indexOf-based text matching (can fail when
                            // interpolated content contains literal piece substrings)
                            let regions = null;
                            if (resolutions) {
                                regions = buildInterpolationRegionsFromResolutions(sourceWithoutQuotes, resolutions);
                            }
                            if (!regions) {
                                regions = extractInterpolationRegions(sourceWithoutQuotes, evaluatedSource);
                            }
                            if (regions) {
                                paramCache.positionMapper = buildPositionMapper(regions);
                            }
                            else {
                                paramCache.positionMapper = undefined;
                            }
                            paramCache.evaluatedContentForMapper = evaluatedSource;
                            // Mapper changed — tracked decorations need recreating
                            paramCache.trackedDecorationsCreated = false;
                            if (paramCache.decorationCollection) {
                                paramCache.decorationCollection.clear();
                            }
                            paramCache.trackedDecorationIds = undefined;
                        }
                        if (!paramCache.positionMapper)
                            continue;
                        // Create tracked decorations once for all interpolated spans,
                        // mapping each evaluated position to its document position
                        // (either in the template literal source or a const literal).
                        const allSpans = paramInfo.all_spans;
                        if (!paramCache.trackedDecorationsCreated && allSpans && allSpans.length > 0) {
                            const decorationsToCreate = [];
                            const spanIds = [];
                            for (const [evalStart, evalEnd] of allSpans) {
                                const sourceStart = paramCache.positionMapper(evalStart);
                                const sourceEnd = paramCache.positionMapper(evalEnd);
                                let startOffset = null;
                                let endOffset = null;
                                if (sourceStart !== null && sourceEnd !== null) {
                                    // Positions map to source text within the template literal
                                    startOffset = argSpan.start + 1 + sourceStart;
                                    endOffset = argSpan.start + 1 + sourceEnd;
                                }
                                else if (resolutions) {
                                    // Positions inside interpolation result — redirect to const literal
                                    const resolvedStart = resolveInterpolatedPosition(evalStart, resolutions);
                                    const resolvedEnd = resolveInterpolatedPosition(evalEnd, resolutions);
                                    if (resolvedStart !== null && resolvedEnd !== null) {
                                        startOffset = resolvedStart;
                                        endOffset = resolvedEnd;
                                    }
                                }
                                if (startOffset !== null && endOffset !== null) {
                                    const spanId = `${evalStart}:${evalEnd}`;
                                    spanIds.push(spanId);
                                    const startPos = model.getPositionAt(startOffset);
                                    const endPos = model.getPositionAt(endOffset);
                                    decorationsToCreate.push({
                                        range: new monaco.Range(startPos.lineNumber, startPos.column, endPos.lineNumber, endPos.column),
                                        options: {
                                            stickiness: monaco.editor.TrackedRangeStickiness
                                                .NeverGrowsWhenTypingAtEdges,
                                        },
                                    });
                                }
                            }
                            if (decorationsToCreate.length > 0) {
                                paramCache.decorationCollection = editor.createDecorationsCollection();
                                const ids = paramCache.decorationCollection.set(decorationsToCreate);
                                paramCache.trackedDecorationIds = new Map();
                                for (let i = 0; i < spanIds.length; i++) {
                                    paramCache.trackedDecorationIds.set(spanIds[i], ids[i]);
                                }
                            }
                            paramCache.trackedDecorationsCreated = true;
                        }
                        // Use tracked decorations for active spans
                        if (paramCache.trackedDecorationIds) {
                            for (const [spanStart, spanEnd] of spans) {
                                const spanId = `${spanStart}:${spanEnd}`;
                                const decoId = paramCache.trackedDecorationIds.get(spanId);
                                if (!decoId)
                                    continue;
                                const range = model.getDecorationRange(decoId);
                                if (!range || range.isEmpty())
                                    continue;
                                newDecorations.push({
                                    range,
                                    options: {
                                        className: activeClassName,
                                        isWholeLine: false,
                                    },
                                });
                            }
                        }
                    }
                    else {
                        // No interpolations - use tracked decorations with all_spans
                        // Create tracked decorations for ALL spans once (when we first see this param)
                        // Then during polling, just look up which ones are currently active
                        const allSpans = paramInfo.all_spans;
                        // Create tracked decorations if we haven't yet and we have all_spans
                        if (!paramCache.trackedDecorationsCreated && allSpans && allSpans.length > 0) {
                            const decorationsToCreate = [];
                            const spanIds = [];
                            for (const [spanStart, spanEnd] of allSpans) {
                                const spanId = `${spanStart}:${spanEnd}`;
                                spanIds.push(spanId);
                                // +1 to skip opening quote in document
                                const startOffset = argSpan.start + 1 + spanStart;
                                const endOffset = argSpan.start + 1 + spanEnd;
                                const startPos = model.getPositionAt(startOffset);
                                const endPos = model.getPositionAt(endOffset);
                                decorationsToCreate.push({
                                    range: new monaco.Range(startPos.lineNumber, startPos.column, endPos.lineNumber, endPos.column),
                                    options: {
                                        // Use stickiness so decorations track with text edits
                                        stickiness: monaco.editor.TrackedRangeStickiness
                                            .NeverGrowsWhenTypingAtEdges,
                                        // No visual style - these are invisible tracking decorations
                                    },
                                });
                            }
                            // Create the decoration collection and get IDs
                            paramCache.decorationCollection = editor.createDecorationsCollection();
                            const ids = paramCache.decorationCollection.set(decorationsToCreate);
                            // Build span ID -> decoration ID map
                            paramCache.trackedDecorationIds = new Map();
                            for (let i = 0; i < spanIds.length; i++) {
                                paramCache.trackedDecorationIds.set(spanIds[i], ids[i]);
                            }
                            paramCache.trackedDecorationsCreated = true;
                        }
                        // If we have tracked decorations, use them to get current positions for active spans
                        if (paramCache.trackedDecorationIds) {
                            for (const [spanStart, spanEnd] of spans) {
                                const spanId = `${spanStart}:${spanEnd}`;
                                const decoId = paramCache.trackedDecorationIds.get(spanId);
                                if (!decoId) {
                                    // This span wasn't in all_spans - shouldn't happen but skip
                                    continue;
                                }
                                // Get the current (tracked) range of this decoration
                                const range = model.getDecorationRange(decoId);
                                if (!range || range.isEmpty())
                                    continue;
                                newDecorations.push({
                                    range,
                                    options: {
                                        className: activeClassName,
                                        isWholeLine: false,
                                    },
                                });
                            }
                        }
                    }
                }
            }
            // Update active decorations (the visual highlighting)
            if (activeDecorationRef.current) {
                activeDecorationRef.current.set(newDecorations);
            }
            else {
                activeDecorationRef.current =
                    editor.createDecorationsCollection(newDecorations);
            }
        }
        catch (e) {
            // Ignore polling errors
        }
    }, pollInterval);
    // Cleanup: clear all tracked decoration collections
    return () => {
        clearInterval(interval);
        for (const moduleCache of globalCache.values()) {
            for (const paramCache of moduleCache.values()) {
                if (paramCache.decorationCollection) {
                    paramCache.decorationCollection.clear();
                }
            }
        }
    };
}
//# sourceMappingURL=moduleStateTracking.js.map