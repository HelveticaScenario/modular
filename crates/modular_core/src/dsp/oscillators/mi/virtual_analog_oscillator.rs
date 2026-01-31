//! Virtual analog oscillator based on Mutable Instruments Plaits.

crate::mi_engine_module! {
    name: "mi.va",
    doc: "Pair of classic waveforms - Virtual-analog synthesis using two variable shape oscillators with sync and crossfading",
    struct_name: VirtualAnalogOscillator,
    engine_type: VirtualAnalogEngine,
    engine_path: mi_plaits_dsp::engine::virtual_analog_engine::VirtualAnalogEngine,
    constructor: new(BLOCK_SIZE),
    output_doc: "crossfaded variable square and saw waveforms",
    aux_doc: "sum of two hardsync'd waveforms, shape controlled by morph, detuning by harmonics",
    params: {
        freq: "frequency in v/oct",
        timbre: "variable square - from narrow pulse to full square to hardsync formants",
        morph: "variable saw - from triangle to saw with an increasingly wide notch (Braids' CSAW)",
        harmonics: "detuning between the two waves",
        sync: "sync/trigger input (expects >0V to trigger)",
    }
}
