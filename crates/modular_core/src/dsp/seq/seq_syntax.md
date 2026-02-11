# Seq Pattern Syntax Documentation

This document describes the pattern syntax used by the `Seq` module for sequencing musical values.

## Value Types

### Bare Numbers

The meaning of bare numbers depends on whether a scale modifier is present:

**Without scale modifier** → **MIDI note numbers**:

```
60 62 64        // MIDI notes: C3, D3, E3
```

**With scale modifier** → **scale intervals** (0-indexed):

```
0 2 4 $ scale(C4:Major)   // Scale intervals: root, 3rd, 5th → C4, E4, G4
```

Decimal values add **cents** (hundredths of a semitone):

```
60.5            // MIDI 60 + 50 cents (without scale)
0.25            // Root + 25 cents (with scale)
```

### Explicit Volts (`Xv`)

Use the `v` suffix for explicit voltage values:

```
0v 1v 2v        // 0V, 1V, 2V (V/Oct)
-1v 0.5v        // Negative and decimal volts supported
```

### Hz Values (`Xhz`, `Xkhz`)

Frequency values with Hz suffix:

```
440hz           // 440 Hz (A3)
880hz           // 880 Hz (A4)
1khz            // 1000 Hz
55hz            // A0 = 0V
```

### Note Names

Standard note names with optional octave (defaults to octave 3):

```
c d e           // C3, D3, E3 (octave omitted = 3)
c4 d4 e4        // C4, D4, E4
a#3 db5         // A#3, Db5
c# f#           // C#3, F#3 (accidentals work without octave)
c-1 a-2         // Negative octaves supported
```

### Rests (`~`)

Silent steps (no CV output, gate goes low):

```
c4 ~ d4 ~       // Note, rest, note, rest
```

### Module References

Reference another module's output:

```
module(lfo-1:out:0)           // Use LFO output channel 0 as CV.
\`${sine('1hz')}\`            // Syntax in JS (automatically includes channel)
```

## Pattern Structures

### Fast Subsequence `[...]`

Elements play within the parent's time slot (subdivides time):

```
c4 [d4 e4]      // c4 for half, then d4 and e4 split the other half
[c4 d4 e4 f4]   // All four notes in one beat
```

### Slow Subsequence `<...>`

Elements advance once per loop (cycles through over time):

```
<c4 g4>         // Loop 1: c4, Loop 2: g4, Loop 3: c4...
<1 3 5>         // Cycles through scale degrees across loops
```

### Random Choice `|`

Randomly selects one option each time:

```
c4|d4|e4        // Randomly plays c4, d4, or e4
[c4 d4]|[e4 f4] // Randomly plays one subsequence
```

### Nesting

Structures can be nested arbitrarily:

```
<c4 [d4 e4]>              // Slow sequence of a note and a fast pair
[<c4 g4> <d4 a4>]         // Fast sequence of two slow sequences
c4|<d4 e4>                // Random: single note OR slow sequence
```

## Modifiers

### Scale Modifier `$ scale(...)`

#### Simple Scale

```
0 2 4 $ scale(C4:Major)   // C major rooted at C4 (C, E, G)
0 1 2 $ scale(A0:Minor)   // A minor rooted at A0 (A, B, C)
```

#### Patternable Scale (Fast)

Scale alternates within the loop:

```
0 1 2 $ scale([A0:Major C4:Minor])  // Alternates per note
```

#### Patternable Scale (Slow)

Scale alternates between loops:

```
0 1 2 $ scale(<A0:Major A0:Minor>)  // Major on odd loops, minor on even
```

#### Random Scale

```
0 1 2 $ scale(A0:Major|A0:Minor)    // Random scale choice each note
```

### Add Modifier `$ add(...)`

Adds values to the main pattern. **All values in an add pattern must be the same type.**

#### Volts Add

Adds directly to the V/Oct output:

```
c4 $ add([0v 1v])         // C4, then C5 (octave up)
c4 $ add(<0v 0.5v>)       // Alternates: C4, C4+tritone across loops
```

#### Bare Number Add (MIDI)

Adds to MIDI note value, then converts to V/Oct:

```
c4 $ add([0 12])          // C4, then C5 (add 12 semitones)
c4 $ add([0 7])           // C4, then G4 (add perfect 5th)
```

#### Bare Number Add with Scale (Intervals)

When main pattern has scale modifier, adds intervals before scale resolution:

```
0 1 $ scale(A0:Major) $ add([0 2])
// Results: (0+0)=0, (1+2)=3 through scale → A0, D1
```

#### Hz Add

Adds frequencies, then converts to V/Oct:

```
440hz $ add([0hz 440hz])  // 440Hz, then 880Hz
```

#### Type Mixing Error

Mixed types in add pattern cause a parse error:

```
c4 $ add([0v 1hz])        // ERROR: mixed types
```

## Supported Scale Names

Common scales (case-insensitive):

- `Major`, `Minor`
- `Dorian`, `Phrygian`, `Lydian`, `Mixolydian`, `Aeolian`, `Locrian`
- `HarmonicMinor`, `MelodicMinor`
- `MajorPentatonic`, `MinorPentatonic`
- `Blues`
- `Chromatic`
- `WholeTone`

## V/Oct Reference

The V/Oct (Volts per Octave) standard used:

- **A0 = 0V = 55Hz = MIDI 33**
- Each volt = one octave
- Each 1/12 volt = one semitone

| Note | MIDI | V/Oct |
| ---- | ---- | ----- |
| A0   | 33   | 0.000 |
| A1   | 45   | 1.000 |
| C4   | 72   | 3.250 |
| A4   | 81   | 4.000 |
| A5   | 93   | 5.000 |

## Complete Examples

```
// Simple melody
c4 d4 e4 f4 g4

// Arpeggio with octave variation
0 2 4 $ scale(C4:Major) $ add(<0v 1v>)

// Random rhythm pattern
c4 [d4 ~] e4|[f4 g4]

// Evolving chord progression
<0 3 4 3> $ scale(<C4:Major A3:Minor F3:Major G3:Major>)

// Bass with random octave drops
c2 $ add(0v|(-1v))

// Complex polyrhythm
[c4 d4 e4] <[f4 g4] a4>
```
