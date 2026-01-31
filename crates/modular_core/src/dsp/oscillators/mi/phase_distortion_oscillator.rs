//! Phase distortion synthesis based on Mutable Instruments Plaits.

crate::mi_engine_module! {
    name: "mi.phasedist",
    doc: "Phase distortion and phase modulation with an asymmetric triangle as the modulator",
    struct_name: PhaseDistortionOscillator,
    engine_type: PhaseDistortionEngine,
    engine_path: mi_plaits_dsp::engine2::phase_distortion_engine::PhaseDistortionEngine,
    constructor: new(BLOCK_SIZE),
    output_doc: "carrier is sync'd (phase distortion)",
    aux_doc: "carrier is free-running (phase modulation)",
    params: {
        freq: "frequency in v/oct",
        timbre: "distortion amount",
        morph: "distortion asymmetry",
        harmonics: "distortion frequency",
        sync: "sync/trigger input (expects >0V to trigger)",
    }
}
