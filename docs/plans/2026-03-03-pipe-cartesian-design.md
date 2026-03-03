# Design: `pipe` Cartesian Product Overload

**Date:** 2026-03-03

## Summary

Extend `ModuleOutput.pipe` and `BaseCollection.pipe` with an overload that accepts any number of arrays after the pipeline function, computes their Cartesian product, calls the function once per combination (with `self` as the first argument), and collects all results into a `$c` Collection.

`Array.prototype.pipe` is left unchanged.

## Shared Type

```ts
type ElementsOf<T extends unknown[][]> = {
    [K in keyof T]: T[K] extends (infer E)[] ? E : never;
};
```

`ElementsOf<[string[], number[]]>` → `[string, number]`

## Shared Utility

```ts
function cartesian<A extends unknown[][]>(...arrays: A): ElementsOf<A>[] {
    return arrays.reduce<unknown[][]>(
        (acc, arr) => acc.flatMap((combo) => arr.map((val) => [...combo, val])),
        [[]],
    ) as ElementsOf<A>[];
}
```

## `pipe` Overloads (on `ModuleOutput` and `BaseCollection`)

```ts
// Existing behavior preserved
pipe<T>(pipelineFunc: (self: this) => T): T;

// New Cartesian overload
pipe<T extends ModuleOutput | Iterable<ModuleOutput>, A extends unknown[][]>(
    pipelineFunc: (self: this, ...args: ElementsOf<A>) => T,
    ...arrays: A
): Collection;

// Implementation
pipe<T>(pipelineFunc: (self: this, ...args: unknown[]) => T, ...arrays: unknown[][]): T | Collection {
    if (arrays.length === 0) return pipelineFunc(this);
    return $c(...cartesian(...arrays).map((combo) => pipelineFunc(this, ...combo)));
}
```

## Example Usage

```ts
osc.out().pipe(
    (self, freq, detune) => self.shift(freq).gain(detune),
    [100, 200, 400],
    [0.5, 1.0],
);
// → Collection of 6 ModuleOutputs
```

## Files Changed

- `src/main/dsl/GraphBuilder.ts` — add `ElementsOf`, `cartesian()`, update `ModuleOutput.pipe` and `BaseCollection.pipe`

## Testing

- Unit tests in `src/main/dsl/__tests__/` (Vitest)
- Test `cartesian()` standalone
- Test `pipe` with no arrays (old behavior unchanged)
- Test `pipe` with one array
- Test `pipe` with two arrays (Cartesian product)
- Test that result is a `Collection` instance when arrays are passed
