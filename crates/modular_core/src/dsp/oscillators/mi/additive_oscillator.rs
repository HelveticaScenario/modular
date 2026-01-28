//! Harmonic additive oscillator based on Mutable Instruments Plaits.

crate::mi_engine_module! {
    name: "mi.additive",
    doc: "Harmonic oscillator - An additive mixture of harmonically-related sine waves",
    struct_name: AdditiveOscillator,
    engine_type: AdditiveEngine,
    engine_path: mi_plaits_dsp::engine::additive_engine::AdditiveEngine,
    constructor: new(),
    output_range: (-1.0, 1.0),
    output_doc: "additive harmonic signal output",
    aux_range: (-1.0, 1.0),
    aux_doc: "Hammond organ drawbar harmonics variant (frequency ratios 1, 2, 3, 4, 6, 8, 10, 12)",
    params: {
        freq: "frequency in v/oct",
        timbre: "index of the most prominent harmonic (similar to cutoff frequency of a band-pass filter)",
        morph: "bump shape - from flat and wide to peaked and narrow (similar to resonance of a band-pass filter)",
        harmonics: "number of bumps in the spectrum (starts with one big bump, progressively adds ripples)",
        sync: "sync/trigger input (expects >0V to trigger)",
    }
}
