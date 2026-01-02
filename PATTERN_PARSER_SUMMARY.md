# Pattern Parser Implementation Summary

## Overview

Built a complete Ohm-based parser for the Musical DSL that generates `PatternProgram` objects to be included in `PatchGraph` for the Rust audio engine.

## Files Created/Modified

### Grammar

- **src/dsl/mini.ohm** - Extended with:
    - `Rest` type using `~` character
    - `HzValue` type for frequencies (e.g., `440hz`, `1.76khz`)
    - `RandomChoice` for pipe-separated alternatives
    - Fixed left recursion issue

### Parser Implementation

- **src/dsl/parser.ts** - New file implementing:
    - `parsePattern(id, input)` - Parse DSL string to PatternProgram
    - Automatic conversion of notes/Hz to V/oct voltage
    - Integration with `hz()` and `note()` helper functions

### Integration

- **src/dsl/GraphBuilder.ts** - Extended with:
    - `patterns: PatternProgram[]` field
    - `addPattern(pattern)` method
    - `addPatterns(patterns)` method
    - Include patterns in `toPatch()` output

- **src/dsl/factories.ts** - Extended DSLContext with:
    - `addPattern(pattern)` method
    - `addPatterns(patterns)` method
    - Import PatternProgram type

- **src/dsl/index.ts** - Export parser functions

### Testing & Examples

- **src/dsl/**tests**/parser.spec.ts** - Comprehensive test suite
- **examples/pattern-examples.ts** - Usage examples
- **pattern-demo.mjs** - Demo patch file
- **docs/PATTERN_PARSER.md** - Complete documentation

## Grammar Syntax

```
Value Types:
  - Numeric: 1.5, -2.0
  - Rest: ~
  - Note: c4, c#5, db3
  - Hz: 440hz, 1.76khz
  - MIDI: m60, m69

Structures:
  - Fast Subsequence: [c4 e4 g4]
  - Slow Subsequence: <c4 f4 g4>
  - Random Choice: c4 | e4 | g4

Nesting: <[c4 e4] [d4 f4]>
```

## Voltage Conversions

All values automatically converted to V/oct:

- **Notes**: Using A4 = 440Hz reference
- **Hz**: Using `V/oct = log2(Hz / 27.5)`
- **MIDI**: Using `V/oct = (MIDI - 69) / 12`
- **Rest**: Special AST variant
- **Numeric**: Pass-through (assumed V/oct)

## Integration Flow

```
DSL String → Ohm Parser → PatternProgram → PatchGraph → Rust Audio Engine
```

1. User writes pattern in DSL syntax
2. `parsePattern()` converts to `PatternProgram` AST
3. `ctx.addPattern()` adds to GraphBuilder
4. `ctx.toPatch()` includes in PatchGraph
5. Rust engine evaluates patterns in real-time

## Pattern Execution

Patterns are executed by `crates/modular_core/src/pattern.rs`:

- Compiles AST for efficient evaluation
- Deterministic seeded RNG for random choices
- Loop-index tracking for slow subsequences
- Stateless evaluation at any time point

## Usage Example

```typescript
import { parsePattern } from './dsl/parser';

// Parse patterns
const melody = parsePattern('melody', 'c4 d4 e4 f4 g4');
const bass = parsePattern('bass', '<c2 f2 g2>');
const kick = parsePattern('kick', '[1.0 ~ 0.8 ~]');

// Add to graph
ctx.addPatterns([melody, bass, kick]);

// Patterns are now in PatchGraph.patterns[]
```

## Next Steps

To make patterns functional in patches:

1. Implement sequencer module in Rust that reads PatternProgram
2. Add DSL factory function for sequencer
3. Connect sequencer output to other modules
4. Document pattern → sequencer workflow

## Testing

```bash
yarn test src/dsl/__tests__/parser.spec.ts
```

All tests cover:

- Basic value types (numeric, rest, notes, hz, midi)
- Subsequences (fast, slow, nested)
- Random choices
- Complex patterns
- Error handling
