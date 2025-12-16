const phead = saw('phead').freq(hz(0.5))

const stepWise = (id, beginning, end, steps) => {
  const diff = end - beginning
  const s = diff / steps
  const tr = track(id)
  tr.interpolation('step')
  const timeStep = 1 / steps
  for (let i = 0; i < steps; i++) {
    tr.addKeyframe(i * timeStep, beginning + i * s)
  }
  return tr
}

const steps = stepWise('steps', note('c5'), note('c6'), 12).playhead(phead)

const t = track('t')
  .addKeyframe(0, note('c4'))
  .addKeyframe(0.25, note('d4'))
  .addKeyframe(0.5, note('e5'))
  .addKeyframe(0.75, note('e4'))
  .interpolation('step')
  .playhead(phead)

scope(t)

// Simple 440 Hz saw wave
const osc = lpf()
  .input(
    saw('osc')
      .freq(steps.shift(note('c0')))
      .shape(sine().freq(hz(0.1)).scale(2.5).shift(2.5))
      .scale(scope(ad().gate(pulse().freq(hz(.4))).attack(2).scale(2.4))),
  )
  .cutoff(hz(2200))

scope(osc)

out.source(osc)