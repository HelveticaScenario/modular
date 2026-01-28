//! Clocked noise synthesis based on Mutable Instruments Plaits.

crate::mi_engine_module! {
    name: "mi.clocknoise",
    doc: "Clocked noise synthesis - Digital noise with controllable clock rate and filtering",
    struct_name: ClockedNoiseOscillator,
    engine_type: NoiseEngine,
    engine_path: mi_plaits_dsp::engine::noise_engine::NoiseEngine,
    constructor: new(BLOCK_SIZE),
    output_range: (-1.0, 1.0),
    output_doc: "clocked noise signal output",
    aux_range: (-1.0, 1.0),
    aux_doc: "auxiliary noise output",
    params: {
        freq: "frequency in v/oct - controls the noise clock rate",
        timbre: "noise color / filter cutoff",
        morph: "noise character / filter resonance",
        harmonics: "noise density / bit depth",
        sync: "sync/trigger input (expects >0V to trigger)",
    }
}
