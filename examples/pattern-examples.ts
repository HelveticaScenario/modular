import { parsePattern } from '../dsl/parser';

// Example 1: Simple pattern with notes and rests
const pattern1 = parsePattern('melody', 'c4 d4 e4 ~ f4 g4 a4 ~');
console.log('Pattern 1:', JSON.stringify(pattern1, null, 2));

// Example 2: Pattern with Hz values
const pattern2 = parsePattern('frequencies', '440hz 880hz 1.76khz');
console.log('Pattern 2:', JSON.stringify(pattern2, null, 2));

// Example 3: Pattern with fast subsequences (arpeggios)
const pattern3 = parsePattern('arp', '[c4 e4 g4] [d4 f4 a4]');
console.log('Pattern 3:', JSON.stringify(pattern3, null, 2));

// Example 4: Pattern with slow subsequences (chord progression)
const pattern4 = parsePattern('chords', '<c4 f4 g4 c4>');
console.log('Pattern 4:', JSON.stringify(pattern4, null, 2));

// Example 5: Pattern with random choices
const pattern5 = parsePattern('random', 'c4 | e4 | g4');
console.log('Pattern 5:', JSON.stringify(pattern5, null, 2));

// Example 6: Complex nested pattern
const pattern6 = parsePattern('complex', '<[c4 e4] [d4 f4] ~ [g4 b4]>');
console.log('Pattern 6:', JSON.stringify(pattern6, null, 2));

// Example 7: Pattern with MIDI note numbers
const pattern7 = parsePattern('midi', 'm60 m62 m64 m65');
console.log('Pattern 7:', JSON.stringify(pattern7, null, 2));

// Example 8: Random choices with rests
const pattern8 = parsePattern('sparse', 'c4 | ~ | e4 | ~');
console.log('Pattern 8:', JSON.stringify(pattern8, null, 2));

// Example usage in a DSL script:
/*
// In a .mjs patch file:

const melody = parsePattern('melody', 'c4 d4 e4 f4 g4 a4 b4 c5');
const rhythm = parsePattern('kick', '[1.0 ~ 0.8 ~] [1.0 ~ 0.9 ~]');
const bassline = parsePattern('bass', '<c2 f2 g2 c2>');

// Add patterns to the graph
ctx.addPatterns([melody, rhythm, bassline]);

// Use patterns with a sequencer module (when implemented)
// const seq = sequencer().pattern(melody);
// scope(seq);
*/
