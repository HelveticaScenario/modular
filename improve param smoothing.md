Plan: Adapt Parameter Smoothing to Match VCVRack
TL;DR: The current Clickless implementation uses a fixed coefficient (0.99) that isn't sample-rate aware, resulting in ~5ms smoothing only at 48kHz. VCVRack uses a time-domain exponential decay formula (out += (in - out) * lambda * deltaTime) that's sample-rate independent and includes snap detection. We need to refactor Clickless to accept a configurable lambda/tau, use sample_rate, and add float precision snapping.

Steps
Refactor Clickless struct in crates/modular_core/src/clickless.rs to store lambda and compute smoothing as out + (in - out) * lambda * delta_time, adding a snap-to-target check when out == y.

Add constructor variants for Clickless: new_with_lambda(lambda: f32) (VCVRack-style, 30-60 for parameter smoothing) and new_with_tau(tau_seconds: f32) (time constant style), defaulting to λ=60 (16.7ms like Rack's engine smoothing).

Update Clickless::update() signature to accept sample_rate: f32 (or delta_time: f32) so the smoothing formula is sample-rate independent: alpha = lambda * (1.0 / sample_rate).

Update all DSP module usages in dsp (oscillators, filters, mixers, MI modules) to pass sample_rate to Clickless::update() calls.

Optional: Add ExponentialSlewLimiter for asymmetric rise/fall smoothing (useful for future envelope followers, VU meters, light brightness).

Further Considerations
Default lambda value? VCVRack uses λ=60 for params (16.7ms) and λ=30 for MIDI (33ms). Recommend λ=60 as default for general parameter smoothing.

Breaking API change? Adding sample_rate to update() breaks the current signature—consider update_sr(input, sample_rate) as a new method and deprecating the old one, or pass sample_rate at construction time via a builder pattern.

Per-module vs global smoothing? VCVRack's engine only smooths one param at a time globally. Our per-module approach is more flexible—keep it, but ensure time constants match Rack's feel.