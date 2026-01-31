//! Virtual analog oscillator with VCF based on Mutable Instruments Plaits.

crate::mi_engine_module! {
    name: "mi.vavcf",
    doc: "Virtual analog oscillator with VCF - Classic subtractive synthesis with resonant filter",
    struct_name: VcfOscillator,
    engine_type: VirtualAnalogVcfEngine,
    engine_path: mi_plaits_dsp::engine2::virtual_analog_vcf_engine::VirtualAnalogVcfEngine,
    constructor: new(),
    output_doc: "low-pass filter output",
    aux_doc: "12dB/octave high-pass filter output",
    params: {
        freq: "frequency in v/oct",
        timbre: "filter cutoff frequency",
        morph: "waveform and sub-oscillator level",
        harmonics: "resonance and filter character - gentle 24dB/octave at 0.0, harsh 12dB/octave at 1.0",
        sync: "sync/trigger input (expects >0V to trigger)",
    }
}
