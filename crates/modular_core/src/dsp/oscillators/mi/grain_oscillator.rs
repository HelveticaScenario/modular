//! Granular formant oscillator based on Mutable Instruments Plaits.

crate::mi_engine_module! {
    name: "mi.grain",
    doc: "Granular formant oscillator - Simulation of formants and filtered waveforms through multiplication, addition, and synchronization of sine wave segments",
    struct_name: GrainOscillator,
    engine_type: GrainEngine,
    engine_path: mi_plaits_dsp::engine::grain_engine::GrainEngine,
    constructor: new(),
    output_range: (-2.5, 2.5),
    output_doc: "granular formant signal output",
    aux_range: (-1.0, 1.0),
    aux_doc: "simulation of filtered waveforms by windowed sine waves (Braids' Z*** models) - harmonics controls filter type (peaking, LP, BP, HP)",
    params: {
        freq: "frequency in v/oct",
        timbre: "formant frequency",
        morph: "formant width and shape - controls window shape multiplying synchronized sine oscillators",
        harmonics: "frequency ratio between formant 1 and 2",
        sync: "sync/trigger input (expects >0V to trigger)",
    }
}
