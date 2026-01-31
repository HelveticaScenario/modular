//! Wavetable oscillator based on Mutable Instruments Plaits.

crate::mi_engine_module! {
    name: "mi.wavetable",
    doc: "Wavetable oscillator - Four banks of 8x8 waveforms: A) harmonically poor (sine harmonics, drawbar organ), B) harmonically rich (formant synthesis, waveshaping), C) Shruthi-1/Ambika wavetables, D) semi-random permutation",
    struct_name: WavetableOscillator,
    engine_type: WavetableEngine<'static>,
    engine_path: mi_plaits_dsp::engine::wavetable_engine::WavetableEngine,
    constructor: new(),
    output_doc: "wavetable signal output",
    aux_doc: "low-fi (5-bit) output",
    params: {
        freq: "frequency in v/oct",
        timbre: "row index - within a row, waves sorted by spectral brightness (except bank D which is a mess!)",
        morph: "column index",
        harmonics: "bank selection - 4 interpolated banks followed by same 4 banks (reverse order) without interpolation",
        sync: "sync/trigger input (expects >0V to trigger)",
    }
}
