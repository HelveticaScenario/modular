import type React from 'react';
import type { editor } from 'monaco-editor';
import type { Monaco } from '../../hooks/useCustomMonaco';

type TrackingMaps = {
    seqTrackingIds: Map<string, Map<number, string>>;
    scaleTrackingIds: Map<string, Map<number, string>>;
    addTrackingIds: Map<string, Map<number, string>>;
};

type BuildTrackingParams = {
    editor: editor.IStandaloneCodeEditor;
    monaco: Monaco;
    lastSubmittedCode: string;
    currentFile?: string;
    runningBufferId?: string | null;
    existingCollection?: editor.IEditorDecorationsCollection | null;
    parsePattern: (pattern: string) => Promise<any>;
};

export async function buildSequenceTracking({
    editor,
    monaco,
    lastSubmittedCode,
    currentFile,
    runningBufferId,
    existingCollection,
    parsePattern,
}: BuildTrackingParams): Promise<
    (TrackingMaps & { collection: editor.IEditorDecorationsCollection }) | null
> {
    if (!lastSubmittedCode) return null;
    if (currentFile !== runningBufferId) return null;

    const regex = /seq\s*\(\s*(['"`])((?:(?!\1)[\s\S])*)\1/g;
    const matches: {
        fullMatch: string;
        quote: string;
        pattern: string;
        index: number;
    }[] = [];
    let match;
    while ((match = regex.exec(lastSubmittedCode)) !== null) {
        matches.push({
            fullMatch: match[0],
            quote: match[1],
            pattern: match[2],
            index: match.index,
        });
    }

    const currentCode = editor.getValue();
    const currentMatches: typeof matches = [];
    let cm;
    regex.lastIndex = 0;
    while ((cm = regex.exec(currentCode)) !== null) {
        currentMatches.push({
            fullMatch: cm[0],
            quote: cm[1],
            pattern: cm[2],
            index: cm.index,
        });
    }

    const newTrackingIds = new Map<string, Map<number, string>>();
    const newScaleTrackingIds = new Map<string, Map<number, string>>();
    const newAddTrackingIds = new Map<string, Map<number, string>>();
    const decorationsToCreate: editor.IModelDeltaDecoration[] = [];
    const decorationMetadata: {
        seqId: string;
        stepIdx: number;
        type: 'main' | 'scale' | 'add';
    }[] = [];
    const model = editor.getModel();
    if (!model) return null;

    for (let i = 0; i < matches.length; i++) {
        if (i >= currentMatches.length) break;

        const submittedMatch = matches[i];
        const currentMatch = currentMatches[i];

        if (submittedMatch.pattern !== currentMatch.pattern) continue;

        try {
            let patternToParse = submittedMatch.pattern;
            if (submittedMatch.quote === '`') {
                patternToParse = patternToParse.replace(
                    /\$\{[\s\S]*?\}/g,
                    (m) => '\x00'.repeat(m.length),
                );
            }

            const program = await parsePattern(patternToParse);
            const seqId = `seq-${i + 1}`;

            const patternStartOffset =
                currentMatch.index +
                currentMatch.fullMatch.indexOf(currentMatch.quote) +
                1;

            const appendDecorations = (
                nodes: any[],
                type: 'main' | 'scale' | 'add',
            ) => {
                for (const node of nodes) {
                    if (node.Leaf) {
                        const { idx, span } = node.Leaf;
                        const startOffset = patternStartOffset + span[0];
                        const endOffset = patternStartOffset + span[1];

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
                                stickiness:
                                    monaco.editor.TrackedRangeStickiness
                                        .NeverGrowsWhenTypingAtEdges,
                            },
                        });
                        decorationMetadata.push({
                            seqId,
                            stepIdx: idx,
                            type,
                        });
                    }
                    if (node.FastSubsequence)
                        appendDecorations(node.FastSubsequence.elements, type);
                    if (node.SlowSubsequence)
                        appendDecorations(node.SlowSubsequence.elements, type);
                    if (node.RandomChoice)
                        appendDecorations(node.RandomChoice.choices, type);
                }
            };

            if (program.elements) {
                appendDecorations(program.elements, 'main');
            }

            if (program.scale_pattern?.elements) {
                appendDecorations(program.scale_pattern.elements, 'scale');
            }

            if (program.add_pattern?.elements) {
                appendDecorations(program.add_pattern.elements, 'add');
            }
        } catch (e) {
            console.error('Failed to parse pattern', e);
        }
    }

    if (existingCollection) {
        existingCollection.clear();
    }
    const collection = editor.createDecorationsCollection();
    const ids = collection.set(decorationsToCreate);

    for (let k = 0; k < ids.length; k++) {
        const { seqId, stepIdx, type } = decorationMetadata[k];
        if (type === 'main') {
            if (!newTrackingIds.has(seqId)) {
                newTrackingIds.set(seqId, new Map());
            }
            newTrackingIds.get(seqId)!.set(stepIdx, ids[k]);
        } else if (type === 'scale') {
            if (!newScaleTrackingIds.has(seqId)) {
                newScaleTrackingIds.set(seqId, new Map());
            }
            newScaleTrackingIds.get(seqId)!.set(stepIdx, ids[k]);
        } else if (type === 'add') {
            if (!newAddTrackingIds.has(seqId)) {
                newAddTrackingIds.set(seqId, new Map());
            }
            newAddTrackingIds.get(seqId)!.set(stepIdx, ids[k]);
        }
    }

    return {
        seqTrackingIds: newTrackingIds,
        scaleTrackingIds: newScaleTrackingIds,
        addTrackingIds: newAddTrackingIds,
        collection,
    };
}

type PollingParams = {
    editor: editor.IStandaloneCodeEditor;
    monaco: Monaco;
    currentFile?: string;
    runningBufferId?: string | null;
    seqTrackingIds: Map<string, Map<number, string>>;
    scaleTrackingIds: Map<string, Map<number, string>>;
    addTrackingIds: Map<string, Map<number, string>>;
    activeStepCollectionRef: React.MutableRefObject<editor.IEditorDecorationsCollection | null>;
    getModuleStates: () => Promise<Record<string, unknown>>;
};

export function startActiveStepPolling({
    editor,
    monaco,
    currentFile,
    runningBufferId,
    seqTrackingIds,
    scaleTrackingIds,
    addTrackingIds,
    activeStepCollectionRef,
    getModuleStates,
}: PollingParams) {
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
                if (
                    id.startsWith('seq-') &&
                    state &&
                    typeof state === 'object' &&
                    'active_step' in state
                ) {
                    const typedState = state as {
                        active_step: number;
                        active_scale_step?: number | null;
                        active_add_step?: number | null;
                    };

                    const activeStep = typedState.active_step;
                    const stepMap = seqTrackingIds.get(id);
                    if (stepMap && stepMap.has(activeStep)) {
                        const decoId = stepMap.get(activeStep)!;
                        const range = model.getDecorationRange(decoId);

                        if (range && !range.isEmpty()) {
                            newDecorations.push({
                                range: range,
                                options: {
                                    className: 'active-seq-step',
                                    isWholeLine: false,
                                },
                            });
                        }
                    }

                    const activeScaleStep = typedState.active_scale_step;
                    if (activeScaleStep != null) {
                        const scaleMap = scaleTrackingIds.get(id);
                        if (scaleMap && scaleMap.has(activeScaleStep)) {
                            const decoId = scaleMap.get(activeScaleStep)!;
                            const range = model.getDecorationRange(decoId);

                            if (range && !range.isEmpty()) {
                                newDecorations.push({
                                    range: range,
                                    options: {
                                        className: 'active-seq-step',
                                        isWholeLine: false,
                                    },
                                });
                            }
                        }
                    }

                    const activeAddStep = typedState.active_add_step;
                    if (activeAddStep != null) {
                        const addMap = addTrackingIds.get(id);
                        if (addMap && addMap.has(activeAddStep)) {
                            const decoId = addMap.get(activeAddStep)!;
                            const range = model.getDecorationRange(decoId);

                            if (range && !range.isEmpty()) {
                                newDecorations.push({
                                    range: range,
                                    options: {
                                        className: 'active-seq-step',
                                        isWholeLine: false,
                                    },
                                });
                            }
                        }
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
            // ignore
        }
    }, 50);

    return () => clearInterval(interval);
}