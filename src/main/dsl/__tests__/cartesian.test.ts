/**
 * Tests for the $cartesian() utility function.
 */

import { describe, test, expect } from 'vitest';
import { $cartesian } from '../GraphBuilder';

describe('$cartesian()', () => {
    test('with no arrays returns [[]]', () => {
        expect($cartesian()).toEqual([[]]);
    });

    test('with one array wraps each element', () => {
        expect($cartesian([1, 2, 3])).toEqual([[1], [2], [3]]);
    });

    test('with two arrays returns full cartesian product', () => {
        const result = $cartesian([1, 2], ['a', 'b', 'c']);
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
