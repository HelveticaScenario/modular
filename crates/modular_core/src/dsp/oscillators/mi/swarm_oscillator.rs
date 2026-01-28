//! Granular swarm synthesis based on Mutable Instruments Plaits.

crate::mi_engine_module! {
    name: "mi.swarm",
    doc: "Granular cloud - A swarm of 8 enveloped sawtooth waves",
    struct_name: SwarmOscillator,
    engine_type: SwarmEngine,
    engine_path: mi_plaits_dsp::engine::swarm_engine::SwarmEngine,
    constructor: new(),
    output_range: (-1.0, 1.0),
    output_doc: "swarm of sawtooth waves output",
    aux_range: (-1.0, 1.0),
    aux_doc: "variant with sine wave oscillators",
    params: {
        freq: "frequency in v/oct",
        timbre: "grain density",
        morph: "grain duration and overlap - at maximum, grains merge into each other: a stack of eight randomly frequency-modulated waveforms",
        harmonics: "amount of pitch randomization",
        sync: "sync/trigger input (expects >0V to trigger)",
    }
}
