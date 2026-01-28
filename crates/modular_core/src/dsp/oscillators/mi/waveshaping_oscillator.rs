//! Waveshaping oscillator based on Mutable Instruments Plaits.

crate::mi_engine_module! {
    name: "mi.waveshape",
    doc: "Waveshaping oscillator - Asymmetric triangle processed by a waveshaper and a wavefolder",
    struct_name: WaveshapingOscillator,
    engine_type: WaveshapingEngine,
    engine_path: mi_plaits_dsp::engine::waveshaping_engine::WaveshapingEngine,
    constructor: new(),
    output_range: (-1.0, 1.0),
    output_doc: "waveshaped and folded signal output",
    aux_range: (-1.0, 1.0),
    aux_doc: "variant employing another wavefolder curve (as available in Warps)",
    params: {
        freq: "frequency in v/oct",
        timbre: "wavefolder amount",
        morph: "waveform asymmetry",
        harmonics: "waveshaper waveform selection",
        sync: "sync/trigger input (expects >0V to trigger)",
    }
}
