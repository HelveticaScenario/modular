//! Modal resonator based on Mutable Instruments Plaits.

crate::mi_engine_module! {
    name: "mi.modal",
    doc: "Modal resonator - Physical modeling synthesis based on resonant filter banks",
    struct_name: ModalOscillator,
    engine_type: ModalEngine,
    engine_path: mi_plaits_dsp::engine::modal_engine::ModalEngine,
    constructor: new(BLOCK_SIZE),
    output_doc: "modal resonator signal output",
    aux_doc: "raw exciter signal",
    params: {
        freq: "frequency in v/oct",
        timbre: "excitation brightness and dust density",
        morph: "decay time (energy absorption)",
        harmonics: "amount of inharmonicity, or material selection",
        sync: "trigger input - without trigger: resonator excited by dust (particle) noise; with trigger: excited by short burst of filtered white noise or low-pass filtered click",
    }
}
