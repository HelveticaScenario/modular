// FM synthesis example
// Modulator oscillator at 2x carrier frequency
const modulator = sine('mod').freq(note('a4'));

// Carrier oscillator with frequency modulation
const carrier = sine('carrier')
  .freq(note('a4'))
  .phase(modulator.output.scale(0.5));

out.source(carrier);

