const test = require('ava');

const { reconcilePatchBySimilarity } = require('../patchSimilarityRemap');

function graph({ modules, tracks = [], scopes = [] }: any): any {
    return {
        modules: modules.map((m: any) => ({ ...m })),
        scopes,
    };
}

test('reconcilePatchBySimilarity: no currentGraph => no remaps', (t: any) => {
    const desired = graph({
        modules: [{ id: 'sine-1', moduleType: 'sine', params: { freq: 440 } }],
    });

    const { moduleIdRemap, appliedPatch } = reconcilePatchBySimilarity(
        desired,
        null,
    );

    t.deepEqual(moduleIdRemap, {});
    t.is(appliedPatch, desired);
});

test('reconcilePatchBySimilarity: matches by params when ids differ', (t: any) => {
    const current = graph({
        modules: [
            { id: 'a', moduleType: 'sine', params: { freq: 440, phase: 0 } },
        ],
    });

    const desired = graph({
        modules: [
            { id: 'b', moduleType: 'sine', params: { freq: 440, phase: 0 } },
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

    t.deepEqual(moduleIdRemap, { a: 'b' });
    t.is(appliedPatch, desired);
});

test('reconcilePatchBySimilarity: does not match across module types', (t: any) => {
    const current = graph({
        modules: [{ id: 'a', moduleType: 'sine', params: { freq: 440 } }],
    });

    const desired = graph({
        modules: [{ id: 'b', moduleType: 'noise', params: { freq: 440 } }],
    });

    const { moduleIdRemap } = reconcilePatchBySimilarity(desired, current, {
        matchThreshold: 0.1,
    });

    t.deepEqual(moduleIdRemap, {});
});

test('reconcilePatchBySimilarity: ambiguity guard rejects ties', (t: any) => {
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
    t.deepEqual(moduleIdRemap, {});
});

test('reconcilePatchBySimilarity: returns multiple remaps when confident', (t: any) => {
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

    t.deepEqual(moduleIdRemap, { a: 'x', b: 'y' });
});

test('reconcilePatchBySimilarity: downstream usage disambiguates identical params', (t: any) => {
    // Two identical sines. Only one is consumed by a filter. The reconciler should
    // match the "used" sine to the "used" sine via downstream usage tokens.
    const current = graph({
        modules: [
            { id: 'a', moduleType: 'sine', params: { freq: 440 } },
            { id: 'b', moduleType: 'sine', params: { freq: 440 } },
            {
                id: 'filter-1',
                moduleType: 'filter',
                params: {
                    input: { type: 'cable', module: 'a', port: 'output' },
                },
            },
        ],
    });

    const desired = graph({
        modules: [
            { id: 'x', moduleType: 'sine', params: { freq: 440 } },
            { id: 'y', moduleType: 'sine', params: { freq: 440 } },
            {
                // Keep the consumer id stable so we only test producer disambiguation.
                id: 'filter-1',
                moduleType: 'filter',
                params: {
                    input: { type: 'cable', module: 'x', port: 'output' },
                },
            },
        ],
    });

    const { moduleIdRemap } = reconcilePatchBySimilarity(desired, current, {
        matchThreshold: 0.8,
        ambiguityMargin: 0.05,
    });

    t.deepEqual(moduleIdRemap, { a: 'x', b: 'y' });
});

test('reconcilePatchBySimilarity: stress test large graph', (t: any) => {
    t.timeout(10_000);

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
                input: { type: 'cable', module: `s${i}`, port: 'output' },
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
                input: { type: 'cable', module: `a${i}`, port: 'output' },
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

    // All ids differ, so we expect a remap for every module.
    t.is(Object.keys(moduleIdRemap).length, 2 * N);

    // Spot-check a few known pairs.
    t.is(moduleIdRemap['a0'], 's0');
    t.is(moduleIdRemap['ga0'], 'g0');
    t.is(moduleIdRemap['a1'], `s${perm(1)}`);
    t.is(moduleIdRemap['ga1'], `g${perm(1)}`);
    t.is(moduleIdRemap[`a${N - 1}`], `s${perm(N - 1)}`);
    t.is(moduleIdRemap[`ga${N - 1}`], `g${perm(N - 1)}`);
});
