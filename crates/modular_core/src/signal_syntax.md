# Signal Syntax Documentation

This document describes the signal string syntax used for module parameters that accept pitch/frequency values.

## Value Types

### Note Names

Standard note names with optional octave (defaults to octave 3):

```
c               // C3 (octave omitted = 3)
c4              // C4
a#3             // A#3
db5             // Db5
c#              // C#3 (accidentals work without octave)
f#              // F#3
c-1             // C-1 (negative octaves supported)
```

### Hz Values (`Xhz`)

Frequency values with Hz suffix, converted to V/Oct:

```
440hz           // 440 Hz (A3)
880hz           // 880 Hz (A4)
55hz            // A0 = 0V
110hz           // A1 = 1V
```

### MIDI Note Numbers (`Xm`)

MIDI note numbers with `m` suffix:

```
72m             // MIDI 72 = C4
69m             // MIDI 69 = A3
33m             // MIDI 33 = A0 = 0V
```

### Scale Intervals (`Xs(Root:Scale)`)

Scale-relative intervals with root and scale name:

```
1s(C4:Major)    // 1st degree of C Major = C4
3s(C4:Major)    // 3rd degree of C Major = E4
5s(A3:Minor)    // 5th degree of A Minor = E4
```

Decimal values add cents:

```
1.5s(C4:Major)  // Root + 50 cents
```

## V/Oct Reference

The V/Oct (Volts per Octave) standard used:

- **A0 = 0V = 55Hz = MIDI 33**
- Each volt = one octave
- Each 1/12 volt = one semitone

| Note | MIDI | V/Oct | Hz     |
| ---- | ---- | ----- | ------ |
| A0   | 33   | 0.000 | 55     |
| A1   | 45   | 1.000 | 110    |
| A2   | 57   | 2.000 | 220    |
| C3   | 60   | 2.250 | 261.63 |
| A3   | 69   | 3.000 | 440    |
| C4   | 72   | 3.250 | 523.25 |
| A4   | 81   | 4.000 | 880    |
| A5   | 93   | 5.000 | 1760   |

## Supported Scale Names

Common scales (case-insensitive):

- `Major`, `Minor`
- `Dorian`, `Phrygian`, `Lydian`, `Mixolydian`, `Aeolian`, `Locrian`
- `HarmonicMinor`, `MelodicMinor`
- `MajorPentatonic`, `MinorPentatonic`
- `Blues`
- `Chromatic`
- `WholeTone`

## Examples

```rust
// In Rust module parameters
let pitch = Signal::from_str("c4").unwrap();      // C4
let pitch = Signal::from_str("a").unwrap();       // A3 (default octave)
let pitch = Signal::from_str("440hz").unwrap();   // A3 via frequency
let pitch = Signal::from_str("72m").unwrap();     // C4 via MIDI
let pitch = Signal::from_str("1s(C4:Major)").unwrap(); // C4 via scale
```
