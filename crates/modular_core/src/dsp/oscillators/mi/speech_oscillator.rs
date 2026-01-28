//! Vowel and speech synthesis based on Mutable Instruments Plaits.

crate::mi_engine_module! {
    name: "mi.speech",
    doc: "Vowel and speech synthesis - A collection of speech synthesis algorithms including formant filtering, SAM, and LPC",
    struct_name: SpeechOscillator,
    engine_type: SpeechEngine<'a>,
    engine_path: mi_plaits_dsp::engine::speech_engine::SpeechEngine,
    constructor: new(BLOCK_SIZE),
    output_range: (-2.0, 2.0),
    output_doc: "speech synthesis signal output",
    aux_range: (-2.0, 2.0),
    aux_doc: "unfiltered vocal cords' signal",
    params: {
        freq: "frequency in v/oct - pitch of the speech",
        timbre: "species selection (Daleks to chipmunks) - shifts formants up/down independently of pitch, or underclocks/overclocks the emulated LPC chip",
        morph: "phoneme or word segment selection - past 11 o'clock: scans through word list; can patch trigger for word utterance, FM attenuverter for intonation, MORPH attenuverter for speed",
        harmonics: "crossfades between formant filtering, SAM, and LPC vowels, then goes through several banks of LPC words",
        sync: "trigger input - triggers word utterance (expects >0V to trigger)",
    }
}
