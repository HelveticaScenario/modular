import { describe, expect, test } from 'vitest';
import { reconcilePatchBySimilarity } from '../patchSimilarityRemap';

function graph({ modules, scopes = [] }: any): any {
    return {
        modules: modules.map((m: any) => ({ ...m })),
        scopes,
    };
}

describe('reconcilePatchBySimilarity', () => {
    test('no currentGraph => no remaps', () => {
        const desired = graph({
            modules: [
                { id: 'sine-1', moduleType: 'sine', params: { freq: 440 } },
            ],
        });

        const { moduleIdRemap, appliedPatch } = reconcilePatchBySimilarity(
            desired,
            null,
        );

        expect(moduleIdRemap).toEqual({});
        expect(appliedPatch).toBe(desired);
    });

    test('matches by params when ids differ', () => {
        const current = graph({
            modules: [
                {
                    id: 'a',
                    moduleType: 'sine',
                    params: { freq: 440, phase: 0 },
                },
            ],
        });

        const desired = graph({
            modules: [
                {
                    id: 'b',
                    moduleType: 'sine',
                    params: { freq: 440, phase: 0 },
                },
            ],
        });

        const { moduleIdRemap, appliedPatch } = reconcilePatchBySimilarity(
            desired,
            current,
            {
                ambiguityMargin: 0.05,
                matchThreshold: 0.9,
            },
        );

        expect(moduleIdRemap).toEqual({ a: 'b' });
        expect(appliedPatch).toBe(desired);
    });

    test('does not match across module types', () => {
        const current = graph({
            modules: [{ id: 'a', moduleType: 'sine', params: { freq: 440 } }],
        });

        const desired = graph({
            modules: [{ id: 'b', moduleType: 'noise', params: { freq: 440 } }],
        });

        const { moduleIdRemap } = reconcilePatchBySimilarity(desired, current, {
            matchThreshold: 0.1,
        });

        expect(moduleIdRemap).toEqual({});
    });

    test('ambiguity guard rejects ties', () => {
        const current = graph({
            modules: [
                { id: 'a', moduleType: 'sine', params: { freq: 440 } },
                { id: 'b', moduleType: 'sine', params: { freq: 440 } },
            ],
        });

        const desired = graph({
            modules: [{ id: 'x', moduleType: 'sine', params: { freq: 440 } }],
        });

        const { moduleIdRemap } = reconcilePatchBySimilarity(desired, current, {
            ambiguityMargin: 0.01,
            matchThreshold: 0.5,
        });

        // Both candidates tie, so we expect rejection.
        expect(moduleIdRemap).toEqual({});
    });

    test('returns multiple remaps when confident', () => {
        const current = graph({
            modules: [
                { id: 'a', moduleType: 'sine', params: { freq: 110 } },
                { id: 'b', moduleType: 'sine', params: { freq: 220 } },
            ],
        });

        const desired = graph({
            modules: [
                { id: 'x', moduleType: 'sine', params: { freq: 110 } },
                { id: 'y', moduleType: 'sine', params: { freq: 220 } },
            ],
        });

        const { moduleIdRemap } = reconcilePatchBySimilarity(desired, current, {
            ambiguityMargin: 0.05,
            matchThreshold: 0.9,
        });

        expect(moduleIdRemap).toEqual({ a: 'x', b: 'y' });
    });

    test('downstream usage disambiguates identical params', () => {
        const current = graph({
            modules: [
                { id: 'a', moduleType: 'sine', params: { freq: 440 } },
                { id: 'b', moduleType: 'sine', params: { freq: 440 } },
                {
                    id: 'filter-1',
                    moduleType: 'filter',
                    params: {
                        input: {
                            module: 'a',
                            port: 'output',
                            type: 'cable',
                        },
                    },
                },
            ],
        });

        const desired = graph({
            modules: [
                { id: 'x', moduleType: 'sine', params: { freq: 440 } },
                { id: 'y', moduleType: 'sine', params: { freq: 440 } },
                {
                    id: 'filter-1',
                    moduleType: 'filter',
                    params: {
                        input: {
                            module: 'x',
                            port: 'output',
                            type: 'cable',
                        },
                    },
                },
            ],
        });

        const { moduleIdRemap } = reconcilePatchBySimilarity(desired, current, {
            ambiguityMargin: 0.05,
            matchThreshold: 0.8,
        });

        expect(moduleIdRemap).toEqual({ a: 'x', b: 'y' });
    });

    test('stress test large graph', () => {
        const N = 150;
        const perm = (i: number) => (i * 73) % N;

        const desiredModules: any[] = [];
        for (let i = 0; i < N; i++) {
            desiredModules.push({
                id: `s${i}`,
                moduleType: 'sine',
                params: { freq: (i + 1) * 1000, tag: `sine-${i}` },
            });
        }
        for (let i = 0; i < N; i++) {
            desiredModules.push({
                id: `g${i}`,
                moduleType: 'gain',
                params: {
                    gain: i,
                    input: {
                        module: `s${i}`,
                        port: 'output',
                        type: 'cable',
                    },
                    tag: `gain-${i}`,
                },
            });
        }

        const currentModules: any[] = [];
        for (let i = 0; i < N; i++) {
            const p = perm(i);
            currentModules.push({
                id: `a${i}`,
                moduleType: 'sine',
                params: { freq: (p + 1) * 1000, tag: `sine-${p}` },
            });
        }
        for (let i = 0; i < N; i++) {
            const p = perm(i);
            currentModules.push({
                id: `ga${i}`,
                moduleType: 'gain',
                params: {
                    gain: p,
                    input: {
                        module: `a${i}`,
                        port: 'output',
                        type: 'cable',
                    },
                    tag: `gain-${p}`,
                },
            });
        }

        const desired = graph({ modules: desiredModules });
        const current = graph({ modules: currentModules });

        const { moduleIdRemap } = reconcilePatchBySimilarity(desired, current, {
            ambiguityMargin: 0.1,
            matchThreshold: 0.95,
        });

        expect(Object.keys(moduleIdRemap).length).toBe(2 * N);

        expect(moduleIdRemap['a0']).toBe('s0');
        expect(moduleIdRemap['ga0']).toBe('g0');
        expect(moduleIdRemap['a1']).toBe(`s${perm(1)}`);
        expect(moduleIdRemap['ga1']).toBe(`g${perm(1)}`);
        expect(moduleIdRemap[`a${N - 1}`]).toBe(`s${perm(N - 1)}`);
        expect(moduleIdRemap[`ga${N - 1}`]).toBe(`g${perm(N - 1)}`);
    });

    describe('buffer_ref support', () => {
        test('matches modules connected via buffer_ref when ids differ', () => {
            const current = graph({
                modules: [
                    {
                        id: 'buf-1',
                        moduleType: '$buffer',
                        params: { input: 0.0, length: 0.5 },
                    },
                    {
                        id: 'delay-1',
                        moduleType: '$delayRead',
                        params: {
                            buffer: {
                                type: 'buffer_ref',
                                module: 'buf-1',
                                port: 'buffer',
                                channels: 1,
                                frameCount: 24000,
                            },
                            time: 0.1,
                        },
                    },
                ],
            });
            const desired = graph({
                modules: [
                    {
                        id: 'buf-2',
                        moduleType: '$buffer',
                        params: { input: 0.0, length: 0.5 },
                    },
                    {
                        id: 'delay-2',
                        moduleType: '$delayRead',
                        params: {
                            buffer: {
                                type: 'buffer_ref',
                                module: 'buf-2',
                                port: 'buffer',
                                channels: 1,
                                frameCount: 24000,
                            },
                            time: 0.1,
                        },
                    },
                ],
            });

            const { moduleIdRemap } = reconcilePatchBySimilarity(
                desired,
                current,
                {
                    matchThreshold: 0.8,
                    ambiguityMargin: 0.05,
                },
            );

            expect(moduleIdRemap['buf-1']).toBe('buf-2');
            expect(moduleIdRemap['delay-1']).toBe('delay-2');
        });

        test('buffer_ref downstream usage disambiguates identical $buffer modules', () => {
            // Two $buffer modules with identical params, but different downstream consumers
            const current = graph({
                modules: [
                    {
                        id: 'buf-a',
                        moduleType: '$buffer',
                        params: { input: 0.0, length: 0.5 },
                    },
                    {
                        id: 'buf-b',
                        moduleType: '$buffer',
                        params: { input: 0.0, length: 0.5 },
                    },
                    {
                        id: 'delay-1',
                        moduleType: '$delayRead',
                        params: {
                            buffer: {
                                type: 'buffer_ref',
                                module: 'buf-a',
                                port: 'buffer',
                                channels: 1,
                                frameCount: 24000,
                            },
                            time: 0.1,
                        },
                    },
                ],
            });
            const desired = graph({
                modules: [
                    {
                        id: 'buf-x',
                        moduleType: '$buffer',
                        params: { input: 0.0, length: 0.5 },
                    },
                    {
                        id: 'buf-y',
                        moduleType: '$buffer',
                        params: { input: 0.0, length: 0.5 },
                    },
                    {
                        id: 'delay-2',
                        moduleType: '$delayRead',
                        params: {
                            buffer: {
                                type: 'buffer_ref',
                                module: 'buf-x',
                                port: 'buffer',
                                channels: 1,
                                frameCount: 24000,
                            },
                            time: 0.1,
                        },
                    },
                ],
            });

            const { moduleIdRemap } = reconcilePatchBySimilarity(
                desired,
                current,
                {
                    matchThreshold: 0.8,
                    ambiguityMargin: 0.05,
                },
            );

            // buf-a should map to buf-x (both have downstream delayRead consumer)
            expect(moduleIdRemap['buf-a']).toBe('buf-x');
            // buf-b should map to buf-y (both have no downstream consumer)
            expect(moduleIdRemap['buf-b']).toBe('buf-y');
        });

        test('does not match buffer_ref modules across different types', () => {
            const current = graph({
                modules: [
                    {
                        id: 'buf-1',
                        moduleType: '$buffer',
                        params: { input: 0.0, length: 0.5 },
                    },
                ],
            });
            const desired = graph({
                modules: [
                    {
                        id: 'other-1',
                        moduleType: '$sine',
                        params: { freq: 440 },
                    },
                ],
            });

            const { moduleIdRemap } = reconcilePatchBySimilarity(
                desired,
                current,
                {
                    matchThreshold: 0.1,
                },
            );

            expect(moduleIdRemap).toEqual({});
        });

        test('buffer_ref with different frameCount produces different features', () => {
            // Two $delayRead modules with different buffer specs should not be ambiguous
            const current = graph({
                modules: [
                    {
                        id: 'buf-short',
                        moduleType: '$buffer',
                        params: { input: 0.0, length: 0.1 },
                    },
                    {
                        id: 'buf-long',
                        moduleType: '$buffer',
                        params: { input: 0.0, length: 2.0 },
                    },
                    {
                        id: 'dr-1',
                        moduleType: '$delayRead',
                        params: {
                            buffer: {
                                type: 'buffer_ref',
                                module: 'buf-short',
                                port: 'buffer',
                                channels: 1,
                                frameCount: 4800,
                            },
                            time: 0.05,
                        },
                    },
                    {
                        id: 'dr-2',
                        moduleType: '$delayRead',
                        params: {
                            buffer: {
                                type: 'buffer_ref',
                                module: 'buf-long',
                                port: 'buffer',
                                channels: 1,
                                frameCount: 96000,
                            },
                            time: 1.0,
                        },
                    },
                ],
            });
            const desired = graph({
                modules: [
                    {
                        id: 'buf-s',
                        moduleType: '$buffer',
                        params: { input: 0.0, length: 0.1 },
                    },
                    {
                        id: 'buf-l',
                        moduleType: '$buffer',
                        params: { input: 0.0, length: 2.0 },
                    },
                    {
                        id: 'dr-a',
                        moduleType: '$delayRead',
                        params: {
                            buffer: {
                                type: 'buffer_ref',
                                module: 'buf-s',
                                port: 'buffer',
                                channels: 1,
                                frameCount: 4800,
                            },
                            time: 0.05,
                        },
                    },
                    {
                        id: 'dr-b',
                        moduleType: '$delayRead',
                        params: {
                            buffer: {
                                type: 'buffer_ref',
                                module: 'buf-l',
                                port: 'buffer',
                                channels: 1,
                                frameCount: 96000,
                            },
                            time: 1.0,
                        },
                    },
                ],
            });

            const { moduleIdRemap } = reconcilePatchBySimilarity(
                desired,
                current,
                {
                    matchThreshold: 0.8,
                    ambiguityMargin: 0.05,
                },
            );

            // Short buffer matched to short, long to long
            expect(moduleIdRemap['buf-short']).toBe('buf-s');
            expect(moduleIdRemap['buf-long']).toBe('buf-l');
            expect(moduleIdRemap['dr-1']).toBe('dr-a');
            expect(moduleIdRemap['dr-2']).toBe('dr-b');
        });
    });
});
