const seq = (notes, ph, cb) => {
  // console.log(cb)
  const [s1, s2, s3, s4] = notes.map(([n, s, d], i) => {
    // console.log(n, s, d)
    const tr = track(`t-${i}`)
    if (s !== 0) {
      tr.addKeyframe(0, 0)
    }
    tr.addKeyframe(s, 5)
    tr.addKeyframe(Math.min(s + d, 1), 0)
    tr.interpolation('step')
    tr.playhead(ph)
    console.log(tr)
    return cb(n, tr, i)
  })
  const sm = mix()
  if (s1) {
    sm.in1(s1)
  }
  if (s2) {
    sm.in2(s2)
  }
  if (s3) {
    sm.in3(s3)
  }
  if (s4) {
    sm.in4(s4)
  }
  return sm
}

// const t = track()

// t.addKeyframe(0, note('c4'))
// t.addKeyframe(0.25, note('g4'))
// t.addKeyframe(0.25, note('g4'))

// t.addKeyframe(0.5, note('e4'))
// t.interpolation('step')
// t.playhead(saw().freq(hz(1)))

// Simple 440 Hz sine wave
// const osc = sine('osc1')
//   .freq(t)
//   .scale(
//     scope(
//       ad()
//         .gate(pulse().freq(hz(0.2)))
//         .attack(1)
//         .decay(1),
//     ),
//   )

const sm = seq(
  [
    ['c4', 0, 0.5],
    ['e4', 0.1, 0.5],
    ['g4', 0.2, 0.5],
    ['a4', 0.3, 0.5],
  ],
  saw().freq(hz(.1)),
  (pitch, gate, i) => {
    console.log(pitch, gate)
    return sine(`seq-sine-${i}`).freq(note(pitch)).scale(gate)
  },
)

// scope(osc)
out.source(scope(sm))
// scope(sm)