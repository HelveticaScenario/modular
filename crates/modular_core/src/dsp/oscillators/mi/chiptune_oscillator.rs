//! Chiptune waveforms with arpeggiator based on Mutable Instruments Plaits.

crate::mi_engine_module! {
    name: "mi.chiptune",
    doc: "Chiptune waveforms with arpeggiator - Emulates classic 8-bit sound chips",
    struct_name: ChiptuneOscillator,
    engine_type: ChiptuneEngine,
    engine_path: mi_plaits_dsp::engine2::chiptune_engine::ChiptuneEngine,
    constructor: new(),
    output_doc: "square wave voices",
    aux_doc: "NES triangle voice",
    params: {
        freq: "frequency in v/oct",
        timbre: "arpeggio type or chord inversion",
        morph: "pulse width / sync amount",
        harmonics: "chord selection",
        sync: "trigger input - clocks the arpeggiator (expects >0V to trigger)",
    }
}
