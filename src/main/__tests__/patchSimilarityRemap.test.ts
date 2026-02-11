import { describe, test, expect } from 'vitest';
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
                matchThreshold: 0.9,
                ambiguityMargin: 0.05,
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
            matchThreshold: 0.5,
            ambiguityMargin: 0.01,
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
            matchThreshold: 0.9,
            ambiguityMargin: 0.05,
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
                            type: 'cable',
                            module: 'a',
                            port: 'output',
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
                            type: 'cable',
                            module: 'x',
                            port: 'output',
                        },
                    },
                },
            ],
        });

        const { moduleIdRemap } = reconcilePatchBySimilarity(desired, current, {
            matchThreshold: 0.8,
            ambiguityMargin: 0.05,
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
                    input: {
                        type: 'cable',
                        module: `s${i}`,
                        port: 'output',
                    },
                    gain: i,
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
                    input: {
                        type: 'cable',
                        module: `a${i}`,
                        port: 'output',
                    },
                    gain: p,
                    tag: `gain-${p}`,
                },
            });
        }

        const desired = graph({ modules: desiredModules });
        const current = graph({ modules: currentModules });

        const { moduleIdRemap } = reconcilePatchBySimilarity(desired, current, {
            matchThreshold: 0.95,
            ambiguityMargin: 0.1,
        });

        expect(Object.keys(moduleIdRemap).length).toBe(2 * N);

        expect(moduleIdRemap['a0']).toBe('s0');
        expect(moduleIdRemap['ga0']).toBe('g0');
        expect(moduleIdRemap['a1']).toBe(`s${perm(1)}`);
        expect(moduleIdRemap['ga1']).toBe(`g${perm(1)}`);
        expect(moduleIdRemap[`a${N - 1}`]).toBe(`s${perm(N - 1)}`);
        expect(moduleIdRemap[`ga${N - 1}`]).toBe(`g${perm(N - 1)}`);
    });
});
