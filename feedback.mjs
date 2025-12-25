const mod = sine()

const osc = saw().freq(
  mod.shift(note('c2')),
)
console.countReset

mod.freq(
  osc
    .shift(note('c2'))
    .scale(
      sine().freq(hz(0.01)),
    ),
)

out.source(
  lpf()
    .input(osc)
    .cutoff(hz(880))
    .res(2).scale(2),
)

scope(out)