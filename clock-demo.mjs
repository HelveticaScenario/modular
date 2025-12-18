// Comprehensive clock module demo
// Shows all inputs and outputs

// The default clock runs at 120 BPM
// You can change its tempo
clock.freq = bpm(128);

// Create a kick drum sound triggered on each bar
const kick = sine();
kick.freq(note("c1"));

const kickEnv = ad();
kickEnv.gate(clock.barTrigger);
kickEnv.attack(0.001);
kickEnv.decay(0.3);

const kickVca = kick.scale(kickEnv);


// Create a hi-hat sound triggered at 48 PPQ (fast pulses)
const hat = noise();

const hatEnv = ad();
hatEnv.gate(clock.ppqTrigger);
hatEnv.attack(0.001);
hatEnv.decay(0.05);

const hatVca = hat.scale(hatEnv);


// Mix kick and hat
const mixer = mix();
mixer.in1(kickVca);
mixer.in2(hatVca);

out.source(mixer);

// Scope the clock signals
scope(clock.barTrigger);
scope(clock.ppqTrigger);
scope(clock.ramp);
