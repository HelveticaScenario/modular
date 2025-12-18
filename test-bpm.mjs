// Test BPM conversion function
// The bpm() function converts BPM to V/Oct frequency

// Create a custom clock at 140 BPM
const fastClock = clock();
fastClock.freq(bpm(140));

// Create another clock at 90 BPM
const slowClock = clock();
slowClock.freq(bpm(90));

// Modulate amplitude with fast clock ramp
out.source(sine().freq(note("a4")).scale(fastClock.ramp));

// Scope the clocks to compare timing
scope(fastClock.barTrigger);
scope(slowClock.barTrigger);
