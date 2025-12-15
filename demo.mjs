

const phead = saw('track-phead').freq(hz(.5))



scope(phead)

const t = track('t')
  .addKeyframe(0, sine('k1').freq(hz(.4)))
  .addKeyframe(1, 0)
  .interpolation("linear")
  .playhead(phead)

scope(t)


const makeSeq = (id, notes) => {
  const tr = track(id).interpolation('exponential')
  if (notes.length === 0) {
    return tr
  }
  const inc = 1 / notes.length
  for (const [i, n] of notes.entries()) {
    tr.addKeyframe(i * inc, note(n))
  }
  return tr
}

const seq = makeSeq('seq', ["c4", "eb4", "f4", "g4", "bb4"]).playhead(phead)

// Simple 440 Hz sine wave
const osc = saw('osc1')
  .freq(
    seq
  ).scale(t);

scope(osc)



out.source(osc);


