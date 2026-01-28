//! String machine emulation based on Mutable Instruments Plaits.

crate::mi_engine_module! {
    name: "mi.stringmachine",
    doc: "String machine emulation with filter and chorus - Emulates vintage string synthesizers",
    struct_name: StringMachineOscillator,
    engine_type: StringMachineEngine,
    engine_path: mi_plaits_dsp::engine2::string_machine_engine::StringMachineEngine,
    constructor: new(),
    output_range: (-1.0, 1.0),
    output_doc: "voices 1 & 3 predominantly",
    aux_range: (-1.0, 1.0),
    aux_doc: "voices 2 & 4 predominantly",
    params: {
        freq: "frequency in v/oct",
        timbre: "chorus/filter amount",
        morph: "waveform selection",
        harmonics: "chord selection",
        sync: "sync/trigger input (expects >0V to trigger)",
    }
}
