const tr = track()
tr.addKeyframe(0, saw().freq(hz(1)))

tr.addKeyframe(0.5, sine().freq(hz(1)))
tr.playhead(saw().freq(hz(2)).scale(2.5).shift(5))
scope(tr)

// Simple 440 Hz sine wave
const osc = sine('osc1').freq(hz(440)).scale(tr)
scope(osc)
out.source(osc)