// Test the clock module
// The default clock is available as a global called "rootClock"
// It runs at 120 BPM by default

// Create an oscillator that follows the clock's ramp
// Connect to output
out.source(sine().freq(rootClock.ramp))

// Scope the clock outputs to see them in action
scope(rootClock.barTrigger);
scope(rootClock.ramp);
scope(rootClock.ppqTrigger);