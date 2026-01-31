//! Analog snare drum model based on Mutable Instruments Plaits.

crate::mi_engine_module! {
    name: "mi.snare",
    doc: "Analog snare drum model - Behavioral simulation of classic snare drum circuits",
    struct_name: SnareDrumOscillator,
    engine_type: SnareDrumEngine,
    engine_path: mi_plaits_dsp::engine::snare_drum_engine::SnareDrumEngine,
    constructor: new(),
    output_doc: "snare drum signal output",
    aux_doc: "auxiliary snare output (noise component)",
    params: {
        freq: "frequency in v/oct - tunes the snare body",
        timbre: "snare wire / noise amount",
        morph: "decay time",
        harmonics: "snare tone / body resonance",
        sync: "trigger input (expects >0V to trigger)",
    }
}
