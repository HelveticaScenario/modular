//! Analog bass drum model based on Mutable Instruments Plaits.

crate::mi_engine_module! {
    name: "mi.bd",
    doc: "Analog bass drum model - Behavioral simulation of circuits from classic drum machines",
    struct_name: BassDrumOscillator,
    engine_type: BassDrumEngine,
    engine_path: mi_plaits_dsp::engine::bass_drum_engine::BassDrumEngine,
    constructor: new(),
    output_range: (-1.0, 1.0),
    output_doc: "bridged T-network excited by a nicely shaped pulse",
    aux_range: (-1.0, 1.0),
    aux_doc: "frequency-modulated triangle VCO, turned into a sine with diodes, shaped by a dirty VCA",
    params: {
        freq: "frequency in v/oct",
        timbre: "brightness",
        morph: "decay time",
        harmonics: "attack sharpness and amount of overdrive",
        sync: "trigger input - without trigger patched, produces a continuous tone (expects >0V to trigger)",
    }
}
