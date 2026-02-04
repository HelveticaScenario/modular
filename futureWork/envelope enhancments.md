Plan: Improve ADSR & Percussion Envelopes
The external repos (Mutable Instruments eurorack, VCV Fundamental, 4ms metamodule) contain sophisticated envelope implementations with features your current envelopes lack. Your percussion_envelope.rs has only fixed exponential decay, and your ADSR uses linear curves only with no retrigger or looping.

Steps
Add curve shaping to ADSR — Implement Stages-style warp_phase() function in adsr.rs for smooth expo/linear/log morphing per-stage (attack_curve, decay_curve, release_curve params).

Add retrigger input with delay — Add dedicated retrigger input to ADSR with optional 32-sample delay for natural articulation "dip" (Stages technique), and hard_reset flag.

Add looping/cycling mode — Add loop boolean to percussion envelope and optionally ADSR for AD-style cycling behavior (Peaks/ENVVCA pattern).

Add Hold stage → AHDSR — Insert configurable hold time between Attack and Decay stages in ADSR for better percussion/pluck sounds.

Add end-of-stage gate outputs — Emit eoa, eod, eor triggers at stage transitions for envelope chaining (PEG-style).

Further Considerations
Curve implementation approach? Stages-style math ((1+a)*t/(1+a*t)) is CPU-cheap vs. Peaks-style lookup tables which are faster but require more memory.

Add velocity input to percussion envelope? Would allow MIDI-style dynamic control over peak level — straightforward addition.

Clock sync for envelope duration? PEG-style tap-tempo sync is powerful but significantly more complex — consider as separate future enhancement.

Updated Findings: Befaco ADEnvelope
The Befaco ADEnvelope (used by Percall and Kickall) is very similar to your percussion envelope but has key improvements:

Befaco ADEnvelope vs Your Percussion Envelope
Feature	Your Perc Envelope	Befaco ADEnvelope
Decay curve	Fixed exponential (e^(-t/τ))	Variable via shape param (t^shape)
Attack phase	❌ None	✅ Configurable attack time & shape
Retrigger	Resets to peak	Smooth - continues from current value
Shape control	❌ Fixed	✅ 0.5 (log) → 1.0 (linear) → 3.0 (expo)
Key Ideas to Adapt
Add shape parameter — Use pow(t, shape) instead of fixed exponential. With shape > 1 it approximates exponential; shape < 1 gives snappier attack-style curves.

Smooth retrigger — Track linear position separately:

Optional attack phase — Transform percussion envelope into a full AD with independent attack/decay times and shapes. Percall uses attack_shape = 0.5 (log) and decay_shape = 3.0 (expo) for punchy percussion.

Choke groups — Percall's choke feature silences one channel when another triggers — useful for hi-hat open/closed behavior.

Further Considerations
Polynomial vs true exponential? Befaco's pow(t, shape) is cheaper than exp(-t/τ) and offers more flexibility — worth switching?

Add optional attack to percussion envelope? Would make it a proper AD envelope like Percall, more versatile for plucks and hits.