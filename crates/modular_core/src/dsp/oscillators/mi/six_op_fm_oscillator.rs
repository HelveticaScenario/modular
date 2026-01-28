//! Six operator FM synthesis based on Mutable Instruments Plaits.

crate::mi_engine_module! {
    name: "mi.sixopfm",
    doc: "6-operator FM synth with 32 presets - DX7-style FM synthesis",
    struct_name: SixOpFmOscillator,
    engine_type: SixOpEngine<'a>,
    engine_path: mi_plaits_dsp::engine2::six_op_engine::SixOpEngine,
    constructor: new(BLOCK_SIZE),
    output_range: (-1.0, 1.0),
    output_doc: "6-operator FM signal output",
    aux_range: (-1.0, 1.0),
    aux_doc: "auxiliary FM output",
    params: {
        freq: "frequency in v/oct",
        timbre: "modulator(s) level",
        morph: "envelope and modulation stretching / time-travel through the sound",
        harmonics: "preset selection (32 classic DX7-style patches)",
        sync: "sync/trigger input (expects >0V to trigger)",
    }
}
