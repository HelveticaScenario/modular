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
440hz           // 440 Hz (A4)
880hz           // 880 Hz (A5)
27.5hz          // A0 = 0V
55hz            // A1 = 1V
```

### MIDI Note Numbers (`Xm`)
MIDI note numbers with `m` suffix:
```
60m             // MIDI 60 = C4
69m             // MIDI 69 = A4
21m             // MIDI 21 = A0 = 0V
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
- **A0 = 0V = 27.5Hz = MIDI 21**
- Each volt = one octave
- Each 1/12 volt = one semitone

| Note | MIDI | V/Oct | Hz     |
|------|------|-------|--------|
| A0   | 21   | 0.000 | 27.5   |
| A1   | 33   | 1.000 | 55     |
| A2   | 45   | 2.000 | 110    |
| C3   | 48   | 2.250 | 130.81 |
| A3   | 57   | 3.000 | 220    |
| C4   | 60   | 3.250 | 261.63 |
| A4   | 69   | 4.000 | 440    |
| A5   | 81   | 5.000 | 880    |

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
let pitch = Signal::from_str("440hz").unwrap();   // A4 via frequency
let pitch = Signal::from_str("60m").unwrap();     // C4 via MIDI
let pitch = Signal::from_str("1s(C4:Major)").unwrap(); // C4 via scale
```
