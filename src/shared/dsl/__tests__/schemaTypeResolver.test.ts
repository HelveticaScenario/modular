import { describe, test, expect } from 'vitest';
import { getEnumVariants } from '../schemaTypeResolver';

describe('getEnumVariants', () => {
    test('extracts variants from oneOf with const + description (documented enum)', () => {
        const schema = {
            oneOf: [
                { const: 'sum', description: 'Sum all inputs at each channel' },
                {
                    const: 'average',
                    description: 'Average all inputs at each channel',
                },
                {
                    const: 'max',
                    description:
                        'Take the maximum absolute value at each channel',
                },
                {
                    const: 'min',
                    description:
                        'Take the minimum absolute value at each channel',
                },
            ],
        };
        const result = getEnumVariants(schema, {});
        expect(result).toEqual([
            {
                value: '"sum"',
                rawValue: 'sum',
                description: 'Sum all inputs at each channel',
            },
            {
                value: '"average"',
                rawValue: 'average',
                description: 'Average all inputs at each channel',
            },
            {
                value: '"max"',
                rawValue: 'max',
                description: 'Take the maximum absolute value at each channel',
            },
            {
                value: '"min"',
                rawValue: 'min',
                description: 'Take the minimum absolute value at each channel',
            },
        ]);
    });

    test('extracts variants from bare enum array (undocumented enum)', () => {
        const schema = {
            enum: ['white', 'pink', 'brown'],
        };
        const result = getEnumVariants(schema, {});
        expect(result).toEqual([
            { value: '"white"', rawValue: 'white', description: undefined },
            { value: '"pink"', rawValue: 'pink', description: undefined },
            { value: '"brown"', rawValue: 'brown', description: undefined },
        ]);
    });

    test('returns null for non-enum schemas', () => {
        expect(getEnumVariants({ type: 'number' }, {})).toBeNull();
        expect(getEnumVariants({ type: 'string' }, {})).toBeNull();
        expect(
            getEnumVariants(
                { type: 'object', properties: { x: { type: 'number' } } },
                {},
            ),
        ).toBeNull();
    });

    test('returns null for null/undefined/boolean schemas', () => {
        expect(getEnumVariants(null, {})).toBeNull();
        expect(getEnumVariants(undefined, {})).toBeNull();
        expect(getEnumVariants(true, {})).toBeNull();
        expect(getEnumVariants(false, {})).toBeNull();
    });

    test('follows $ref to resolve enum in $defs', () => {
        const rootSchema = {
            $defs: {
                MixMode: {
                    oneOf: [
                        {
                            const: 'sum',
                            description: 'Sum all inputs',
                        },
                        {
                            const: 'average',
                            description: 'Average all inputs',
                        },
                    ],
                },
            },
        };
        const schema = { $ref: '#/$defs/MixMode' };
        const result = getEnumVariants(schema, rootSchema);
        expect(result).toEqual([
            {
                value: '"sum"',
                rawValue: 'sum',
                description: 'Sum all inputs',
            },
            {
                value: '"average"',
                rawValue: 'average',
                description: 'Average all inputs',
            },
        ]);
    });

    test('returns null for Signal $ref sentinels', () => {
        const rootSchema = {
            $defs: {
                Signal: { title: 'Signal' },
            },
        };
        expect(
            getEnumVariants({ $ref: '#/$defs/Signal' }, rootSchema),
        ).toBeNull();
        expect(
            getEnumVariants({ $ref: '#/$defs/PolySignal' }, rootSchema),
        ).toBeNull();
        expect(
            getEnumVariants({ $ref: '#/$defs/MonoSignal' }, rootSchema),
        ).toBeNull();
    });

    test('returns null for oneOf/anyOf that is not all-const (union types)', () => {
        const schema = {
            oneOf: [{ type: 'string' }, { type: 'number' }],
        };
        expect(getEnumVariants(schema, {})).toBeNull();
    });

    test('handles anyOf the same as oneOf', () => {
        const schema = {
            anyOf: [
                { const: 'a', description: 'First' },
                { const: 'b', description: 'Second' },
            ],
        };
        const result = getEnumVariants(schema, {});
        expect(result).toEqual([
            { value: '"a"', rawValue: 'a', description: 'First' },
            { value: '"b"', rawValue: 'b', description: 'Second' },
        ]);
    });

    test('handles variants where some have descriptions and some do not', () => {
        const schema = {
            oneOf: [
                { const: 'x', description: 'Has description' },
                { const: 'y' },
            ],
        };
        const result = getEnumVariants(schema, {});
        expect(result).toEqual([
            { value: '"x"', rawValue: 'x', description: 'Has description' },
            { value: '"y"', rawValue: 'y', description: undefined },
        ]);
    });
});
