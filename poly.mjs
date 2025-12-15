const seq (notes) => {

}

const t = track()

t.addKeyframe(0, note('c4'))
t.addKeyframe(0.25, note('g4'))
t.addKeyframe(0.25, note('g4'))

t.addKeyframe(0.5, note('e4'))
t.interpolation('step')
t.playhead(saw().freq(hz(1)))

// Simple 440 Hz sine wave
const osc = sine('osc1')
  .freq(t)
  .scale(
    scope(ad()
      .gate(pulse().freq(hz(0.2)))
      .attack(1)
      .decay(1)),
  )

scope(osc)
out.source(osc)