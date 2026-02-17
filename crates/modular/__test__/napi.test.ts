/**
 * Integration tests for the N-API boundary (@modular/core).
 *
 * These run against the compiled native module — no Electron or audio device needed.
 */

import { describe, test, expect } from 'vitest';
import {
    getSchemas,
    validatePatchGraph,
    deriveChannelCount,
    getMiniLeafSpans,
    getPatternPolyphony,
    type ModuleSchema,
    type PatchGraph,
    type ValidationError,
} from '@modular/core';

// ─── getSchemas ──────────────────────────────────────────────────────────────

describe('getSchemas', () => {
    test('returns a non-empty array of schemas', () => {
        const schemas = getSchemas();
        expect(Array.isArray(schemas)).toBe(true);
        expect(schemas.length).toBeGreaterThan(0);
    });

    test('each schema has required fields', () => {
        const schemas = getSchemas();
        for (const s of schemas) {
            expect(s).toHaveProperty('name');
            expect(s).toHaveProperty('documentation');
            expect(s).toHaveProperty('paramsSchema');
            expect(s).toHaveProperty('outputs');
            expect(typeof s.name).toBe('string');
            expect(typeof s.documentation).toBe('string');
        }
    });

    test('schemas include $sine with expected outputs', () => {
        const schemas = getSchemas();
        const sine = schemas.find((s) => s.name === '$sine');
        expect(sine).toBeDefined();
        expect(sine!.outputs.length).toBeGreaterThan(0);
    });

    test('schemas include $clock with expected positionalArgs', () => {
        const schemas = getSchemas();
        const clock = schemas.find((s) => s.name === '$clock');
        expect(clock).toBeDefined();
        expect(clock!.positionalArgs).toBeDefined();
        expect(clock!.positionalArgs!.length).toBeGreaterThan(0);
    });

    test('schemas include polyphonic module with channels', () => {
        const schemas = getSchemas();
        // Find a module that declares channelsParam (polyphonic)
        const withChannels = schemas.filter(
            (s) => s.channelsParam !== undefined && s.channelsParam !== null,
        );
        expect(withChannels.length).toBeGreaterThan(0);
    });

    test('schemas are stable across calls', () => {
        const a = getSchemas();
        const b = getSchemas();
        expect(a.length).toBe(b.length);
        expect(a.map((s) => s.name).sort()).toEqual(
            b.map((s) => s.name).sort(),
        );
    });
});

// ─── validatePatchGraph ──────────────────────────────────────────────────────

describe('validatePatchGraph', () => {
    test('empty patch is valid', () => {
        const patch: PatchGraph = { modules: [], scopes: [] };
        const errors = validatePatchGraph(patch);
        expect(errors).toEqual([]);
    });

    test('valid single-module patch with correct param name passes', () => {
        const patch: PatchGraph = {
            modules: [
                {
                    id: 'sine-1',
                    moduleType: '$sine',
                    idIsExplicit: false,
                    params: { freq: '440hz' },
                },
            ],
            scopes: [],
        };
        const errors = validatePatchGraph(patch);
        expect(errors).toEqual([]);
    });

    test('wrong param name "frequency" is rejected', () => {
        const patch: PatchGraph = {
            modules: [
                {
                    id: 'sine-1',
                    moduleType: '$sine',
                    idIsExplicit: false,
                    params: { frequency: '440hz' },
                },
            ],
            scopes: [],
        };
        const errors = validatePatchGraph(patch);
        expect(errors.length).toBeGreaterThan(0);
    });

    test('invalid module type produces errors', () => {
        const patch: PatchGraph = {
            modules: [
                {
                    id: 'bad-1',
                    moduleType: '$nonExistentFooBar',
                    idIsExplicit: false,
                    params: {},
                },
            ],
            scopes: [],
        };
        const errors = validatePatchGraph(patch);
        expect(errors.length).toBeGreaterThan(0);
        expect(errors[0].message).toMatch(/unknown|not found|exist/i);
    });

    test('invalid param name produces errors', () => {
        const patch: PatchGraph = {
            modules: [
                {
                    id: 'sine-1',
                    moduleType: '$sine',
                    idIsExplicit: false,
                    params: { freq: '440hz', bogusParam: 42 },
                },
            ],
            scopes: [],
        };
        const errors = validatePatchGraph(patch);
        expect(errors.length).toBeGreaterThan(0);
    });

    test('scope referencing non-existent module produces errors', () => {
        const patch: PatchGraph = {
            modules: [],
            scopes: [
                {
                    item: {
                        type: 'ModuleOutput',
                        moduleId: 'ghost-module',
                        portName: 'output',
                    },
                    msPerFrame: 50,
                    range: [-5, 5] as [number, number],
                },
            ],
        };
        const errors = validatePatchGraph(patch);
        expect(errors.length).toBeGreaterThan(0);
    });

    test('cable referencing non-existent module produces errors', () => {
        const patch: PatchGraph = {
            modules: [
                {
                    id: 'lpf-1',
                    moduleType: '$lpf',
                    idIsExplicit: false,
                    params: {
                        input: {
                            type: 'cable',
                            module: 'does-not-exist',
                            port: 'output',
                            channel: 0,
                        },
                        cutoff: '1000hz',
                    },
                },
            ],
            scopes: [],
        };
        const errors = validatePatchGraph(patch);
        expect(errors.length).toBeGreaterThan(0);
    });

    test('error objects have field and message', () => {
        const patch: PatchGraph = {
            modules: [
                {
                    id: 'bad-1',
                    moduleType: '$nonExistentModule',
                    idIsExplicit: false,
                    params: {},
                },
            ],
            scopes: [],
        };
        const errors = validatePatchGraph(patch);
        expect(errors.length).toBeGreaterThan(0);
        expect(errors[0]).toHaveProperty('field');
        expect(errors[0]).toHaveProperty('message');
        expect(typeof errors[0].field).toBe('string');
        expect(typeof errors[0].message).toBe('string');
    });
});

// ─── deriveChannelCount ──────────────────────────────────────────────────────

describe('deriveChannelCount', () => {
    test('single note returns 1', () => {
        const count = deriveChannelCount('$sine', { freq: 'C4' });
        expect(count).toBe(1);
    });

    test('array of notes returns correct count', () => {
        const count = deriveChannelCount('$sine', {
            freq: ['C4', 'E4', 'G4'],
        });
        expect(count).toBe(3);
    });

    test('unknown module type returns null', () => {
        const count = deriveChannelCount('$unknownFoo', { x: 1 });
        expect(count).toBeNull();
    });

    test('Hz string returns 1', () => {
        const count = deriveChannelCount('$sine', { freq: '440hz' });
        expect(count).toBe(1);
    });
});

// ─── getMiniLeafSpans ────────────────────────────────────────────────────────

describe('getMiniLeafSpans', () => {
    test('simple mini-notation returns spans', () => {
        // getMiniLeafSpans parses music mini-notation, not JavaScript
        const spans = getMiniLeafSpans('C4 E4 G4');
        expect(Array.isArray(spans)).toBe(true);
        expect(spans.length).toBe(3);
        // Each span is [start, end]
        for (const span of spans) {
            expect(Array.isArray(span)).toBe(true);
            expect(span.length).toBe(2);
            expect(typeof span[0]).toBe('number');
            expect(typeof span[1]).toBe('number');
        }
    });

    test('pattern with groups', () => {
        const spans = getMiniLeafSpans('C4 [E4 G4]');
        expect(spans.length).toBeGreaterThan(0);
    });

    test('single note', () => {
        const spans = getMiniLeafSpans('C4');
        expect(spans).toEqual([[0, 2]]);
    });
});

// ─── getPatternPolyphony ─────────────────────────────────────────────────────

describe('getPatternPolyphony', () => {
    test('single note is 1', () => {
        const p = getPatternPolyphony('C4');
        expect(p).toBe(1);
    });

    test('chord is polyphonic', () => {
        const p = getPatternPolyphony('[C4,E4,G4]');
        expect(p).toBe(3);
    });

    test('sequential pattern is monophonic', () => {
        const p = getPatternPolyphony('C4 E4 G4');
        expect(p).toBe(1);
    });
});
