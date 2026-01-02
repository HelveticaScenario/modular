// Example: Using the pattern parser in a patch
// This demonstrates how to create and use patterns in your modular patches

import { parsePattern } from './src/dsl/parser';

// Create a melody pattern
const melody = parsePattern('melody', 'c4 d4 e4 f4 g4 a4 b4 c5');

// Create a bass pattern that cycles slowly
const bass = parsePattern('bass', '<c2 f2 g2 c2>');

// Create a rhythmic kick pattern with fast subsequences
const kick = parsePattern('kick', '[1.0 ~ 0.8 ~]');

// Create random hi-hat pattern
const hihat = parsePattern('hihat', '0.3 | ~ | 0.5 | ~ | 0.4');

// Create pattern with Hz values
const frequencies = parsePattern('freq', '440hz 880hz 1.76khz 3.52khz');

// Add all patterns to the graph
ctx.addPatterns([melody, bass, kick, hihat, frequencies]);

// TODO: When sequencer modules are implemented, use patterns like:
// const melodySeq = sequencer().pattern('melody');
// const bassSeq = sequencer().pattern('bass');
// const kickSeq = sequencer().pattern('kick');

// For now, patterns are stored in PatchGraph and can be retrieved
// by the Rust audio engine for processing

console.log('Patterns added to graph:', {
  melody: melody.id,
  bass: bass.id,
  kick: kick.id,
  hihat: hihat.id,
  frequencies: frequencies.id
});
