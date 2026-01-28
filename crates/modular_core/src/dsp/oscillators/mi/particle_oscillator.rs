//! Particle noise synthesis based on Mutable Instruments Plaits.

crate::mi_engine_module! {
    name: "mi.particle",
    doc: "Particle noise synthesis - Filtered random impulses and dust noise",
    struct_name: ParticleOscillator,
    engine_type: ParticleEngine,
    engine_path: mi_plaits_dsp::engine::particle_engine::ParticleEngine,
    constructor: new(BLOCK_SIZE),
    output_range: (-1.0, 1.0),
    output_doc: "particle noise signal output",
    aux_range: (-1.0, 1.0),
    aux_doc: "auxiliary particle output",
    params: {
        freq: "frequency in v/oct - controls particle rate",
        timbre: "particle density / filter cutoff",
        morph: "particle decay / filter resonance",
        harmonics: "particle spread / noise color",
        sync: "trigger input (expects >0V to trigger)",
    }
}
