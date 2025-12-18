
const osc = saw('osc1').shape(sine().freq(hz(1))).freq(note('a4')).scale(2.5);
scope(osc)
out.source(osc);


