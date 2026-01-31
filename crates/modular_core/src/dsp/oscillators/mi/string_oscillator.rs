//! Inharmonic string modeling based on Mutable Instruments Plaits.

crate::mi_engine_module! {
    name: "mi.string",
    doc: "Inharmonic string modeling - Physical modeling synthesis of plucked and bowed strings",
    struct_name: StringOscillator,
    engine_type: StringEngine,
    engine_path: mi_plaits_dsp::engine::string_engine::StringEngine,
    constructor: new(BLOCK_SIZE),
    output_doc: "string resonator signal output",
    aux_doc: "raw exciter signal",
    params: {
        freq: "frequency in v/oct",
        timbre: "excitation brightness and dust density",
        morph: "decay time (energy absorption)",
        harmonics: "amount of inharmonicity, or material selection",
        sync: "trigger input - without trigger: string excited by dust (particle) noise; with trigger: excited by short burst of filtered white noise or low-pass filtered click",
    }
}
