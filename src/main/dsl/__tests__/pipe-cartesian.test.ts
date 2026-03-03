/**
 * Tests for the Cartesian product overload on ModuleOutput.pipe and BaseCollection.pipe,
 * and for the cartesian() utility function.
 */

import { describe, test, expect, beforeAll } from 'vitest';
import { getSchemas, type ModuleSchema } from '@modular/core';
import {
    GraphBuilder,
    BaseCollection,
    Collection,
    ModuleOutput,
    $c,
    cartesian,
} from '../GraphBuilder';

let schemas: ModuleSchema[];
let builder: GraphBuilder;

beforeAll(() => {
    schemas = getSchemas();
    builder = new GraphBuilder(schemas);
});

// ─── cartesian() utility ──────────────────────────────────────────────────────

describe('cartesian()', () => {
    test('with no arrays returns [[]]', () => {
        expect(cartesian()).toEqual([[]]);
    });

    test('with one array wraps each element', () => {
        expect(cartesian([1, 2, 3])).toEqual([[1], [2], [3]]);
    });

    test('with two arrays returns full cartesian product', () => {
        const result = cartesian([1, 2], ['a', 'b', 'c']);
        expect(result).toEqual([
            [1, 'a'],
            [1, 'b'],
            [1, 'c'],
            [2, 'a'],
            [2, 'b'],
            [2, 'c'],
        ]);
    });
});

// ─── Helpers ──────────────────────────────────────────────────────────────────

function makeOutput(id: string = 'test-1'): ModuleOutput {
    return new ModuleOutput(builder, id, 'out', 0);
}

// ─── BaseCollection.pipe ─────────────────────────────────────────────────────

describe('BaseCollection.pipe', () => {
    test('with no arrays returns non-Collection value (old behavior)', () => {
        const col = new BaseCollection<ModuleOutput>(makeOutput());
        const result = col.pipe(() => 42);
        expect(result).toBe(42);
    });

    test('with one array returns a Collection', () => {
        const col = new BaseCollection<ModuleOutput>(makeOutput());
        const result = col.pipe((_self, val) => makeOutput(), [10, 20]);
        expect(result).toBeInstanceOf(Collection);
    });

    test('with one array returns Collection with item count equal to array length', () => {
        const col = new BaseCollection<ModuleOutput>(makeOutput());
        const result = col.pipe((_self, _val) => makeOutput(), [10, 20, 30]);
        expect(result).toBeInstanceOf(Collection);
        expect(result.items.length).toBe(3);
    });

    test('with two arrays returns Collection with item count equal to product length', () => {
        const col = new BaseCollection<ModuleOutput>(makeOutput());
        const result = col.pipe(
            (_self, _a, _b) => makeOutput(),
            [1, 2],
            ['x', 'y', 'z'],
        );
        expect(result).toBeInstanceOf(Collection);
        // 2 * 3 = 6 combinations
        expect(result.items.length).toBe(6);
    });

    test('passes self as first argument to pipelineFunc', () => {
        const col = new BaseCollection<ModuleOutput>(makeOutput());
        let captured: unknown;
        col.pipe(
            (s) => {
                captured = s;
                return makeOutput();
            },
            [1],
        );
        expect(captured).toBe(col);
    });
});

// ─── ModuleOutput.pipe ───────────────────────────────────────────────────────

describe('ModuleOutput.pipe', () => {
    test('with no arrays returns non-Collection value (old behavior)', () => {
        const output = makeOutput();
        const result = output.pipe(() => 99);
        expect(result).toBe(99);
    });

    test('with one array returns a Collection', () => {
        const output = makeOutput();
        const result = output.pipe((_self, _val) => makeOutput(), [10, 20]);
        expect(result).toBeInstanceOf(Collection);
        expect(result.items.length).toBe(2);
    });

    test('with two arrays returns Collection with correct size', () => {
        const output = makeOutput();
        const result = output.pipe(
            (_self, _a, _b) => makeOutput(),
            [1, 2, 3],
            ['x', 'y'],
        );
        expect(result).toBeInstanceOf(Collection);
        // 3 * 2 = 6 combinations
        expect(result.items.length).toBe(6);
    });
});
