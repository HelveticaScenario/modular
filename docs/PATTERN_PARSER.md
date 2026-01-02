# Musical DSL Pattern Parser

This module provides an Ohm-based parser for the Musical DSL, allowing you to create `PatternProgram` objects that can be added to a `PatchGraph`.

## Features

- **Note Names**: Standard musical notation (e.g., `c4`, `c#4`, `db5`)
- **Hz Values**: Frequency values with `hz` or `khz` suffix (e.g., `440hz`, `1.76khz`)
- **MIDI Note Numbers**: MIDI notation with `m` prefix (e.g., `m60`, `m69`)
- **Rests**: Use `~` to indicate silence
- **Numeric Literals**: Direct voltage values (e.g., `1.5`, `-2.0`)
- **Fast Subsequences**: Square brackets for fast cycling (e.g., `[c4 e4 g4]`)
- **Slow Subsequences**: Angle brackets for slow progression (e.g., `<c4 f4 g4>`)
- **Random Choices**: Pipe operator for random selection (e.g., `c4 | e4 | g4`)

## Grammar

```ohm
MusicalDSL {
  Program = Element*

  Element = FastSubsequence
          | SlowSubsequence
          | RandomChoice
          | Value

  FastSubsequence = "[" Element* "]"
  
  SlowSubsequence = "<" Element* ">"

  RandomChoice = Element ("|" Element)+

  Value = Rest
        | HzValue
        | NumericLiteral
        | NoteName
        | MidiValue

  Rest = "~"

  HzValue = "-"? digit+ ("." digit+)? ("hz" | "khz")

  NumericLiteral = "-"? digit+ ("." digit+)?

  NoteName = letter accidental? digit+

  MidiValue = "m" digit+

  accidental = "#" | "b"

  letter = "a".."g" | "A".."G"
}
```

## Usage

### Basic Patterns

```typescript
import { parsePattern } from './dsl/parser';

// Simple melody
const melody = parsePattern('melody', 'c4 d4 e4 f4 g4');

// With rests
const rhythmic = parsePattern('rhythm', 'c4 ~ e4 ~ g4 ~');

// Using Hz values
const frequencies = parsePattern('freqs', '440hz 880hz 1.76khz');

// Using MIDI note numbers
const midiNotes = parsePattern('midi', 'm60 m62 m64 m65 m67');
```

### Subsequences

Fast subsequences cycle through elements quickly:
```typescript
const arpeggio = parsePattern('arp', '[c4 e4 g4]');
// Each loop: c4, e4, g4, c4, e4, g4, ...
```

Slow subsequences progress one element per loop:
```typescript
const chords = parsePattern('chords', '<c4 f4 g4 c4>');
// Loop 1: c4, Loop 2: f4, Loop 3: g4, Loop 4: c4, Loop 5: c4, ...
```

Nested subsequences:
```typescript
const complex = parsePattern('complex', '<[c4 e4] [d4 f4] [e4 g4]>');
```

### Random Choices

```typescript
// Random note selection
const random = parsePattern('rand', 'c4 | e4 | g4');

// Random with rests for sparse patterns
const sparse = parsePattern('sparse', 'c4 | ~ | e4 | ~');

// Random in subsequences
const randomArp = parsePattern('randArp', '<c4 | d4 e4 | f4>');
```

### Integration with PatchGraph

```javascript
// In a .mjs patch file:

// Parse patterns
const melody = parsePattern('melody', 'c4 d4 e4 f4 g4 a4 b4 c5');
const bass = parsePattern('bass', '<c2 f2 g2 c2>');

// Add to context
ctx.addPatterns([melody, bass]);

// The patterns will be included in the PatchGraph when toPatch() is called
```

## Value Conversions

All values are automatically converted to voltage (V/oct) for use with the audio engine:

- **Note Names**: Converted using A4 = 440Hz reference
  - `c4` → voltage corresponding to 261.63 Hz
  - `a4` → voltage corresponding to 440 Hz
  
- **Hz Values**: Converted using `V/oct = log2(Hz / 27.5)`
  - `440hz` → ~4.0 V/oct
  - `1khz` → ~5.17 V/oct
  
- **MIDI Notes**: Converted using `V/oct = (MIDI - 69) / 12`
  - `m60` → -0.75 V/oct (Middle C)
  - `m69` → 0.0 V/oct (A4)

- **Rests**: Represented as the `Rest` variant in the AST

- **Numeric Literals**: Passed through as-is (assumed to be in V/oct)

## Pattern Execution

Patterns are executed by the Rust audio engine using the `PatternProgram` runner. The runner:

1. Compiles the AST for efficient evaluation
2. Uses deterministic seeded RNG for random choices
3. Tracks loop indices for slow subsequences
4. Provides stateless evaluation at any time point

See `crates/modular_core/src/pattern.rs` for the execution engine implementation.

## Examples

See `examples/pattern-examples.ts` for more comprehensive examples.

## Testing

```bash
yarn test src/dsl/__tests__/parser.spec.ts
```

## Type Safety

The parser generates objects that match the `PatternProgram` and `ASTNode` types defined in Rust and exposed via NAPI. TypeScript types are automatically generated from the Rust definitions during build.
