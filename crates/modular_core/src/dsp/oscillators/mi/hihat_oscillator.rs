//! Analog hi-hat model based on Mutable Instruments Plaits.

crate::mi_engine_module! {
    name: "mi.hihat",
    doc: "Analog hi-hat model - A bunch of square oscillators generating a harsh, metallic tone",
    struct_name: HihatOscillator,
    engine_type: HihatEngine,
    engine_path: mi_plaits_dsp::engine::hihat_engine::HihatEngine,
    constructor: new(BLOCK_SIZE),
    output_doc: "6 square oscillators and a dirty transistor VCA",
    aux_doc: "three pairs of square oscillators ring-modulating each other with a clean, linear VCA",
    params: {
        freq: "frequency in v/oct",
        timbre: "high-pass filter cutoff",
        morph: "decay time",
        harmonics: "balance of metallic and filtered noise",
        sync: "trigger input (expects >0V to trigger)",
    }
}
