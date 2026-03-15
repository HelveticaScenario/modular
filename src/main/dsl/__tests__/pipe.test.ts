/**
 * Tests for the array overload on ModuleOutput.pipe and BaseCollection.pipe.
 */

import { describe, test, expect, beforeAll } from 'vitest';
import schemas from '@modular/core/schemas.json';
import {
    GraphBuilder,
    BaseCollection,
    Collection,
    ModuleOutput,
} from '../GraphBuilder';

let builder: GraphBuilder;

beforeAll(() => {
    builder = new GraphBuilder(schemas);
});

function makeOutput(id: string = 'test-1'): ModuleOutput {
    return new ModuleOutput(builder, id, 'out', 0);
}

// ─── BaseCollection.pipe ─────────────────────────────────────────────────────

describe('BaseCollection.pipe', () => {
    test('with no arrays returns non-Collection value (simple overload)', () => {
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

    test('passes each array element as second argument to pipelineFunc', () => {
        const col = new BaseCollection<ModuleOutput>(makeOutput());
        const captured: unknown[] = [];
        col.pipe(
            (_s, item) => {
                captured.push(item);
                return makeOutput();
            },
            ['a', 'b', 'c'],
        );
        expect(captured).toEqual(['a', 'b', 'c']);
    });
});

// ─── ModuleOutput.pipe ───────────────────────────────────────────────────────

describe('ModuleOutput.pipe', () => {
    test('with no arrays returns non-Collection value (simple overload)', () => {
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

    test('passes each array element as second argument to pipelineFunc', () => {
        const output = makeOutput();
        const captured: unknown[] = [];
        output.pipe(
            (_s, item) => {
                captured.push(item);
                return makeOutput();
            },
            [1, 2, 3],
        );
        expect(captured).toEqual([1, 2, 3]);
    });
});
