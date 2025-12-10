# Eurorack Module Research
## Potential Modules to Add to the Project

This document summarizes research from Eurorack module catalogs (Perfect Circuit, Ctrl-Mod, ModularGrid, and other vendors) to identify potential DSP modules to implement in our software modular synthesizer project.

---

## Oscillators (VCO)

### Mutable Instruments Plaits
- **Function**: Digital oscillator with multiple synthesis models
- **Features**: 16 synthesis engines including VA, FM, physical modeling, noise, percussion
- **Use Case**: Versatile sound source for experimental patches, compact all-in-one oscillator
- **Implementation Notes**: Could implement multiple oscillator models in one module with model selection parameter

### Klavis Twin Waves MkII
- **Function**: Dual voltage-controlled oscillator
- **Features**: Multiple waveform outputs, sync, FM, waveshaping
- **Use Case**: Complex timbres through dual oscillator interaction
- **Implementation Notes**: Dual oscillator with cross-modulation capabilities

### Frap Tools BRENSO
- **Function**: Complex analog oscillator
- **Features**: Triple VCO core, wavefolding, amplitude modulation, ring modulation
- **Use Case**: West Coast synthesis, complex harmonic content
- **Implementation Notes**: Advanced wavefolding and multiple modulation types

### NANO Modules ONA
- **Function**: Compact analog VCO
- **Features**: Multiple simultaneous waveform outputs, FM input
- **Use Case**: Space-efficient oscillator for portable systems
- **Implementation Notes**: Basic oscillator with standard waveforms (saw, square, triangle, sine)

---

## Filters (VCF)

### XAOC Devices Belgrad
- **Function**: Multi-mode analog filter
- **Features**: LP, HP, BP, notch modes; voltage-controlled resonance and drive
- **Use Case**: Musical filtering with character, stereo processing
- **Implementation Notes**: Multi-mode state variable filter with drive/saturation

### Make Noise QPAS
- **Function**: Stereo filter with unique topology
- **Features**: Parallel/series routing, frequency shifter-like effects, radiate control
- **Use Case**: Spatial filtering, stereo field manipulation
- **Implementation Notes**: Dual filter with unique routing and modulation options

### Mutable Instruments Ripples/Blades
- **Function**: Clean analog cascading filter
- **Features**: 2-pole/4-pole modes, clean self-oscillation
- **Use Case**: Classic subtractive synthesis, clean resonant sweeps
- **Implementation Notes**: Clean ladder-style filter with variable pole count

### Erica Synths Fusion VCF
- **Function**: Multi-character filter
- **Features**: Multiple filter types, overdrive, voltage-controlled everything
- **Use Case**: Gritty, aggressive filtering for techno and industrial sounds
- **Implementation Notes**: Filter with saturation/distortion stages

---

## VCAs (Voltage Controlled Amplifiers)

### Intellijel Quad VCA
- **Function**: Four-channel VCA with mixer
- **Features**: Linear/exponential response, CV control, summed output
- **Use Case**: Essential dynamics control, both audio and CV processing
- **Implementation Notes**: Basic but essential - linear and exponential curves

### Cosmotronic Delta-V
- **Function**: Combined envelope and VCA
- **Features**: Built-in envelope generator, reduces patching complexity
- **Use Case**: Quick voice control without separate envelope module
- **Implementation Notes**: Integrated ADSR + VCA for efficiency

### Happy Nerding 3x MIA
- **Function**: Utility mixer/attenuator/VCA
- **Features**: Three channels, offset control, mixing
- **Use Case**: Flexible signal processing and mixing
- **Implementation Notes**: Multi-purpose utility module

---

## Envelopes & LFOs

### Make Noise Maths
- **Function**: Dual function generator (envelope/LFO/slew/mixer)
- **Features**: Can be envelope, LFO, oscillator, slew limiter, logic processor
- **Use Case**: "Swiss Army knife" of modulation, extremely flexible
- **Implementation Notes**: Complex multi-function module with cycle mode, attenuverters, mixing

### Frap Tools FALISTRI
- **Function**: Dual function generator with complex behavior
- **Features**: Independent envelopes/LFOs, cascading, complex modulation
- **Use Case**: Advanced modulation with one module
- **Implementation Notes**: Dual function generator with various loop and trigger modes

### Intellijel Quadrax
- **Function**: Quad envelope/function generator
- **Features**: Four independent channels, burst mode, chaining
- **Use Case**: Multiple envelopes for complex patches
- **Implementation Notes**: Flexible ADSR/AD/AR with various trigger modes

### Doepfer A-140-2 Dual Mini ADSR
- **Function**: Classic dual ADSR envelope
- **Features**: Standard ADSR parameters, compact
- **Use Case**: Traditional envelope generation
- **Implementation Notes**: Basic ADSR - attack, decay, sustain, release parameters

### Erica Synths Black EG2
- **Function**: Advanced envelope generator
- **Features**: Exponential curves, looping, re-triggering
- **Use Case**: Complex modulation shapes
- **Implementation Notes**: ADSR with curve control and loop modes

---

## Sequencers

### Westlicht PER|FORMER
- **Function**: Complex pattern-based sequencer
- **Features**: Multiple tracks, euclidean patterns, polyrhythms
- **Use Case**: Generative and complex rhythmic sequencing
- **Implementation Notes**: Step sequencer with probability, ratcheting, swing

### Intellijel Metropolix
- **Function**: Gate and pitch sequencer
- **Features**: Real-time playability, scale quantization, modulation
- **Use Case**: Musical sequence creation with performance features
- **Implementation Notes**: Multi-track sequencer with per-step modulation

### vpme.de Euclidean Circles v2
- **Function**: Euclidean rhythm generator
- **Features**: Multiple euclidean patterns, rotation, pattern morphing
- **Use Case**: Polyrhythmic and evolving drum patterns
- **Implementation Notes**: Euclidean algorithm implementation with CV control

---

## Random & Noise Generators

### Mutable Instruments Marbles
- **Function**: Random sampler and gate generator
- **Features**: Quantized random voltages, random gates/triggers, probability control
- **Use Case**: Generative melodies and rhythms
- **Implementation Notes**: Random voltage with quantization, distribution control

### Steady State Fate Ultra-Random Redux
- **Function**: Comprehensive random source
- **Features**: Multiple sample & hold circuits, random pulses, flux generator, slew
- **Use Case**: Deep modulation with multiple random sources
- **Implementation Notes**: Multi-output random generator with various types (stepped, smooth, gated)

### Frap Tools BAGÀI
- **Function**: Digital noise and random source
- **Features**: Stepped voltages, clock bursts, noise, sample & hold, bit crushing
- **Use Case**: Crunchy textures and random modulation
- **Implementation Notes**: Digital noise with sample rate reduction and S&H

### Intellijel Flurry
- **Function**: Noise and random module
- **Features**: Analog/digital noise, multiple S&H circuits, slew, envelope follower
- **Use Case**: Sound design and modulation
- **Implementation Notes**: Comprehensive noise module with filtering and processing

### Zlob Modular Diode Chaos
- **Function**: Triple chaos LFO
- **Features**: Chaotic oscillators based on academic research
- **Use Case**: Unpredictable cyclic modulation for experimental patches
- **Implementation Notes**: Chaos oscillator (e.g., Lorenz attractor, double scroll)

### Doepfer A-149 Series
- **Function**: Random voltage generators
- **Features**: Quantized/stored/fluctuating random voltages
- **Use Case**: Buchla-style uncertainty and randomness
- **Implementation Notes**: Various random voltage generation algorithms

### Erica Synths Hexinverter VCNO
- **Function**: Digital noise oscillator
- **Features**: LFSR-based noise, CV-controllable, unusual noise colors
- **Use Case**: Evolving textural noise
- **Implementation Notes**: Linear feedback shift register noise generator

### Tiptop Audio Buchla 266t
- **Function**: Source of Uncertainty (Buchla clone)
- **Features**: Multiple noise types, random voltages, sample & hold, probability
- **Use Case**: Classic Buchla-style random generation
- **Implementation Notes**: Comprehensive Buchla-inspired random module

---

## Effects

### Delay

#### Qu-Bit Nautilus
- **Function**: Stereo delay with advanced routing
- **Features**: Eight routable delay lines, clock sync, freeze, filtering, saturation
- **Use Case**: Complex rhythmic delays and pitch effects
- **Implementation Notes**: Multi-tap delay with feedback matrix and modulation

#### Strymon Magneto
- **Function**: Tape-style delay/looper
- **Features**: Tape saturation, wow/flutter, phrase sampling
- **Use Case**: Ambient music, compositional looping
- **Implementation Notes**: Delay with tape emulation (saturation, wow, flutter)

### Reverb

#### Qu-Bit Aurora
- **Function**: Spectral reverb
- **Features**: FFT-based resynthesis, pitch/time separation, shimmer
- **Use Case**: Unique atmospheric effects, spectral processing
- **Implementation Notes**: FFT-based reverb with frequency domain processing

#### Erica Synths Black Hole DSP2
- **Function**: Multi-effect processor
- **Features**: Reverb, delay, chorus, flanger, phaser, bit crushing
- **Use Case**: All-in-one effects for space and time-based processing
- **Implementation Notes**: DSP platform with multiple effect algorithms

#### Strymon StarLab
- **Function**: High-end reverb processor
- **Features**: Shimmer, infinite decay, modulation
- **Use Case**: Lush ambient spaces
- **Implementation Notes**: High-quality reverb algorithms with modulation

### Modulation Effects

#### SoundForce uChorus 6
- **Function**: Stereo BBD chorus
- **Features**: Juno-inspired, manual/CV control
- **Use Case**: Lush ensemble effects, string-like sounds
- **Implementation Notes**: Bucket-brigade delay chorus emulation

#### Doepfer A-101-8
- **Function**: Classic analog phaser
- **Features**: Multiple stages, resonance control
- **Use Case**: Vintage '70s phasing effects
- **Implementation Notes**: All-pass filter phaser with LFO

#### Erica Synths Black K-Phaser
- **Function**: Advanced analog phaser
- **Features**: Extended modulation, multiple modes
- **Use Case**: Deep psychedelic phasing
- **Implementation Notes**: Multi-stage phaser with feedback control

#### Patching Panda Moon Phase
- **Function**: Stereo filter/imager
- **Features**: Spatial modulation, multi-mode filtering
- **Use Case**: Stereo field manipulation
- **Implementation Notes**: Stereo processing with phase shifting

### Distortion & Saturation

#### Behringer Space FX
- **Function**: Multi-effect processor
- **Features**: 32 algorithms including distortion, reverb, delay, modulation
- **Use Case**: Budget-friendly all-in-one effects
- **Implementation Notes**: Digital multi-effect with preset selection

---

## Utilities

### DivKid ochd
- **Function**: Eight-channel LFO
- **Features**: Organic, interrelated modulation sources
- **Use Case**: Multiple slow modulations for evolving patches
- **Implementation Notes**: Multi-channel LFO with phase relationships

### Joranalogue Audio Design Series
- **Function**: Various logic and signal processing utilities
- **Features**: Contour 1 (function generator), Compare 2 (comparator), Select 2 (switch)
- **Use Case**: Precision signal routing and logic
- **Implementation Notes**: Logic modules (comparators, switches, Boolean operations)

### Intellijel Buff Mult
- **Function**: Signal multiples
- **Features**: Passive/active buffered multiples
- **Use Case**: Splitting signals to multiple destinations
- **Implementation Notes**: Basic utility - copy signal to multiple outputs

---

## Mixers

*Note: See VCAs section for Intellijel Quad VCA and Happy Nerding 3x MIA, which also function as mixers*

### Make Noise ModDemix
- **Function**: Dual ring modulator/mixer
- **Features**: Balanced modulation, mixing, routing flexibility
- **Use Case**: Complex modulation and signal blending
- **Implementation Notes**: Ring modulator with carrier/modulator mixing

### Boredbrain Music Xcelon
- **Function**: Stereo mixer
- **Features**: Multiple inputs, sends, stereo field control
- **Use Case**: Live performance mixing
- **Implementation Notes**: Multi-channel stereo mixer with panning

---

## Controllers & Interfaces

### ALM Busy Circuits Pamela's PRO Workout
- **Function**: Master clock and modulator
- **Features**: Multiple clock outputs, euclidean patterns, modulation
- **Use Case**: System clock and modulation hub
- **Implementation Notes**: Clock generator with divisions, multiplication, and swing

### Nervous Squirrel String Thing
- **Function**: Physical gesture controller
- **Features**: Joystick + string tension = three-axis CV
- **Use Case**: Expressive manual control
- **Implementation Notes**: Not directly applicable to software synth (physical controller)

### Der Mann mit der Maschine DROID
- **Function**: Universal CV processor
- **Features**: Programmable patching, sequencing, logic
- **Use Case**: Complex control and routing
- **Implementation Notes**: Algorithm-based CV processing

---

## Unique & Uncommon Modules Worth Considering

### Nervous Squirrel Zeno's Paradox
- **Function**: Extreme clock divider
- **Features**: Divisions so slow they occur once per decades
- **Use Case**: Extremely long-form generative music
- **Implementation Notes**: Clock divider with very high division ratios

### Clank Chaos
- **Function**: Six-channel aleatoric sequencer
- **Features**: Semi-controlled chaos in sequencing
- **Use Case**: Structured randomness in patterns
- **Implementation Notes**: Random sequencer with controllable probability

### Blukac Instruments Endless Processor
- **Function**: Infinite sound sustainer
- **Features**: Layering and resynthesizing sounds
- **Use Case**: Beyond traditional looping, continuous sound evolution
- **Implementation Notes**: Granular/spectral sustain processor

### AJH Synth MiniMod Ring SM
- **Function**: Ring modulator with extras
- **Features**: Sub-oscillator generation, mixing utilities
- **Use Case**: Classic ring mod with added functionality
- **Implementation Notes**: Ring modulator (amplitude modulation of two signals)

---

## Implementation Priority Recommendations

Based on the research and current module inventory, here are suggested priorities:

### High Priority (Core Functionality)
1. **VCA Module** - Currently missing, essential for dynamics
2. **ADSR Envelope** - Basic envelope generator
3. **Multi-function Envelope/LFO** - Inspired by Maths
4. **Ring Modulator** - Classic modulation technique
5. **Sample & Hold** - Basic random/stepped voltage generator
6. **Noise Generator** - White, pink, and colored noise
7. **Basic Delay** - Time-based effect
8. **Basic Reverb** - Spatial effect

### Medium Priority (Enhanced Functionality)
1. **Wavefolder** - West Coast synthesis
2. **Chorus** - Modulation effect
3. **Phaser/Flanger** - Time-based modulation
4. **Distortion/Saturation** - Harmonic enhancement
5. **Random Voltage Generator** - Multiple modes (smooth, stepped, quantized)
6. **Clock Generator/Divider** - Timing utilities
7. **Comparator** - Logic/threshold utility
8. **Slew Limiter** - Smoothing utility

### Lower Priority (Advanced Features)
1. **Complex Oscillator** - Multiple waveforms with cross-mod
2. **Spectral Effects** - FFT-based processing
3. **Multi-tap Delay** - Rhythmic complex delays
4. **Chaos Generator** - Unpredictable modulation
5. **Granular Processor** - Texture generation
6. **Multi-effect Processor** - Algorithm switching

---

## Notes on Software Implementation

### Considerations for Digital Implementation
- **V/Oct Standard**: Maintain 1V/octave pitch standard (each volt change represents one musical octave; e.g., 0V = A0 at 27.5Hz, 1V = A1 at 55Hz, 4V = A4 at 440Hz)
- **Modulation Range**: Typically -5V to +5V for audio signals, -10V to +10V for control voltage (CV)
- **Sample Rate**: Consider oversampling for nonlinear effects (distortion, waveshaping) to reduce aliasing
- **Smoothing**: Parameter changes should be smoothed to avoid clicks and zipper noise
- **Efficiency**: Some hardware modules use DSP chips; ensure CPU-efficient algorithms for real-time performance

### Modules Best Suited for Software
- Multi-algorithm modules (like Plaits) are ideal for software
- Complex digital effects (reverb, delay, spectral processing)
- Precise modulation sources (LFOs, envelopes)
- Unlimited voices/copies (unlike hardware)

### Modules Requiring Special Attention
- Analog circuit emulation (filters, distortion) - need careful modeling
- Nonlinear effects - may need oversampling
- Physical modeling - computationally intensive
- Reverbs - require good algorithms to avoid metallic artifacts

---

## References

This research was compiled from:
- Perfect Circuit (https://www.perfectcircuit.com/eurorack-modular-synths.html)
- ModularGrid Top 100 (https://modulargrid.net/)
- Ctrl-Mod Collections (https://www.ctrl-mod.com/collections/modules)
- Schneidersladen Eurorack Catalog
- Sweetwater Eurorack Section
- Music Radar Eurorack Reviews
- Various manufacturer documentation

Research conducted: December 2024
