//! Four-note chord oscillator based on Mutable Instruments Plaits.

crate::mi_engine_module! {
    name: "mi.chord",
    doc: "Four-note chords played by virtual analogue or wavetable oscillators - Emulates vintage string & organ machines",
    struct_name: ChordOscillator,
    engine_type: ChordEngine<'static>,
    engine_path: mi_plaits_dsp::engine::chord_engine::ChordEngine,
    constructor: new(),
    output_doc: "four-voice chord signal output",
    aux_doc: "root note of the chord",
    params: {
        freq: "frequency in v/oct",
        timbre: "chord inversion and transposition",
        morph: "waveform - first half: string-machine raw waveforms (organ/string drawbars); second half: scans a 16-waveform wavetable",
        harmonics: "chord type selection",
        sync: "sync/trigger input (expects >0V to trigger)",
    }
}
