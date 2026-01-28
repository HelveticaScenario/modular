//! Two operator FM synthesis based on Mutable Instruments Plaits.

crate::mi_engine_module! {
    name: "mi.fm",
    doc: "Two operator FM - Two sine-wave oscillators modulating each other's phase",
    struct_name: FmOscillator,
    engine_type: FmEngine,
    engine_path: mi_plaits_dsp::engine::fm_engine::FmEngine,
    constructor: new(),
    output_range: (-1.0, 1.0),
    output_doc: "FM synthesis signal output",
    aux_range: (-1.0, 1.0),
    aux_doc: "sub-oscillator",
    params: {
        freq: "frequency in v/oct",
        timbre: "modulation index",
        morph: "feedback - past 12 o'clock: operator 2 modulates its own phase (rough!); before 12 o'clock: operator 1's phase (chaotic!)",
        harmonics: "frequency ratio between operators",
        sync: "sync/trigger input (expects >0V to trigger)",
    }
}
