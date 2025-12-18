

const phead = saw('track-phead').freq(hz(.2))



scope(phead)

const t = track('t')
  .addKeyframe(0, sine('k1').freq(hz(4)))
  .addKeyframe(1, saw('k1').freq(hz(3)))
  .interpolation("exponential")
  .playhead(phead)

scope(t)


const makeSeq = (id, notes) => {
  const t = track(id).interpolation('step')
  if (notes.length === 0) {
    return t
  }
  const inc = 1 / notes.length
  for (const [i, n] of notes.entries()) {
    t.addKeyframe(i * inc, note(n))
  }
  return t
}

const seq = makeSeq('seq', ["c4", "eb4", "f4", "g4", "bb4"]).playhead(phead)

// Simple 440 Hz sine wave
const osc = sine('osc1')
  .freq(
    seq
  ).scale(t);

scope(osc)



out.source(osc);


