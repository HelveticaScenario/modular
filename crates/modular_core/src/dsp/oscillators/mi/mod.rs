use std::collections::HashMap;

use crate::types::{ChannelCountDeriver, Module, ModuleSchema, ParamsValidator, SampleableConstructor};

// Macro for generating MI engine wrappers
#[macro_use]
pub mod macros;

// mi-plaits-dsp-rs engine wrappers
pub mod additive_oscillator;
pub mod bass_drum_oscillator;
pub mod chiptune_oscillator;
pub mod chord_oscillator;
pub mod clocked_noise_oscillator;
pub mod fm_oscillator;
pub mod grain_oscillator;
pub mod hihat_oscillator;
pub mod modal_oscillator;
pub mod particle_oscillator;
pub mod phase_distortion_oscillator;
pub mod six_op_fm_oscillator;
pub mod snare_drum_oscillator;
pub mod speech_oscillator;
pub mod string_machine_oscillator;
pub mod string_oscillator;
pub mod swarm_oscillator;
pub mod vcf_oscillator;
pub mod virtual_analog_oscillator;
pub mod wave_terrain_oscillator;
pub mod waveshaping_oscillator;
pub mod wavetable_oscillator;

pub fn install_constructors(map: &mut HashMap<String, SampleableConstructor>) {
    additive_oscillator::AdditiveOscillator::install_constructor(map);
    bass_drum_oscillator::BassDrumOscillator::install_constructor(map);
    chiptune_oscillator::ChiptuneOscillator::install_constructor(map);
    chord_oscillator::ChordOscillator::install_constructor(map);
    clocked_noise_oscillator::ClockedNoiseOscillator::install_constructor(map);
    fm_oscillator::FmOscillator::install_constructor(map);
    grain_oscillator::GrainOscillator::install_constructor(map);
    hihat_oscillator::HihatOscillator::install_constructor(map);
    modal_oscillator::ModalOscillator::install_constructor(map);
    particle_oscillator::ParticleOscillator::install_constructor(map);
    phase_distortion_oscillator::PhaseDistortionOscillator::install_constructor(map);
    six_op_fm_oscillator::SixOpFmOscillator::install_constructor(map);
    snare_drum_oscillator::SnareDrumOscillator::install_constructor(map);
    speech_oscillator::SpeechOscillator::install_constructor(map);
    string_machine_oscillator::StringMachineOscillator::install_constructor(map);
    string_oscillator::StringOscillator::install_constructor(map);
    swarm_oscillator::SwarmOscillator::install_constructor(map);
    vcf_oscillator::VcfOscillator::install_constructor(map);
    virtual_analog_oscillator::VirtualAnalogOscillator::install_constructor(map);
    wave_terrain_oscillator::WaveTerrainOscillator::install_constructor(map);
    waveshaping_oscillator::WaveshapingOscillator::install_constructor(map);
    wavetable_oscillator::WavetableOscillator::install_constructor(map);
}

pub fn install_param_validators(map: &mut HashMap<String, ParamsValidator>) {
    additive_oscillator::AdditiveOscillator::install_params_validator(map);
    bass_drum_oscillator::BassDrumOscillator::install_params_validator(map);
    chiptune_oscillator::ChiptuneOscillator::install_params_validator(map);
    chord_oscillator::ChordOscillator::install_params_validator(map);
    clocked_noise_oscillator::ClockedNoiseOscillator::install_params_validator(map);
    fm_oscillator::FmOscillator::install_params_validator(map);
    grain_oscillator::GrainOscillator::install_params_validator(map);
    hihat_oscillator::HihatOscillator::install_params_validator(map);
    modal_oscillator::ModalOscillator::install_params_validator(map);
    particle_oscillator::ParticleOscillator::install_params_validator(map);
    phase_distortion_oscillator::PhaseDistortionOscillator::install_params_validator(map);
    six_op_fm_oscillator::SixOpFmOscillator::install_params_validator(map);
    snare_drum_oscillator::SnareDrumOscillator::install_params_validator(map);
    speech_oscillator::SpeechOscillator::install_params_validator(map);
    string_machine_oscillator::StringMachineOscillator::install_params_validator(map);
    string_oscillator::StringOscillator::install_params_validator(map);
    swarm_oscillator::SwarmOscillator::install_params_validator(map);
    vcf_oscillator::VcfOscillator::install_params_validator(map);
    virtual_analog_oscillator::VirtualAnalogOscillator::install_params_validator(map);
    wave_terrain_oscillator::WaveTerrainOscillator::install_params_validator(map);
    waveshaping_oscillator::WaveshapingOscillator::install_params_validator(map);
    wavetable_oscillator::WavetableOscillator::install_params_validator(map);
}

pub fn install_channel_count_derivers(map: &mut HashMap<String, ChannelCountDeriver>) {
    additive_oscillator::AdditiveOscillator::install_channel_count_deriver(map);
    bass_drum_oscillator::BassDrumOscillator::install_channel_count_deriver(map);
    chiptune_oscillator::ChiptuneOscillator::install_channel_count_deriver(map);
    chord_oscillator::ChordOscillator::install_channel_count_deriver(map);
    clocked_noise_oscillator::ClockedNoiseOscillator::install_channel_count_deriver(map);
    fm_oscillator::FmOscillator::install_channel_count_deriver(map);
    grain_oscillator::GrainOscillator::install_channel_count_deriver(map);
    hihat_oscillator::HihatOscillator::install_channel_count_deriver(map);
    modal_oscillator::ModalOscillator::install_channel_count_deriver(map);
    particle_oscillator::ParticleOscillator::install_channel_count_deriver(map);
    phase_distortion_oscillator::PhaseDistortionOscillator::install_channel_count_deriver(map);
    six_op_fm_oscillator::SixOpFmOscillator::install_channel_count_deriver(map);
    snare_drum_oscillator::SnareDrumOscillator::install_channel_count_deriver(map);
    speech_oscillator::SpeechOscillator::install_channel_count_deriver(map);
    string_machine_oscillator::StringMachineOscillator::install_channel_count_deriver(map);
    string_oscillator::StringOscillator::install_channel_count_deriver(map);
    swarm_oscillator::SwarmOscillator::install_channel_count_deriver(map);
    vcf_oscillator::VcfOscillator::install_channel_count_deriver(map);
    virtual_analog_oscillator::VirtualAnalogOscillator::install_channel_count_deriver(map);
    wave_terrain_oscillator::WaveTerrainOscillator::install_channel_count_deriver(map);
    waveshaping_oscillator::WaveshapingOscillator::install_channel_count_deriver(map);
    wavetable_oscillator::WavetableOscillator::install_channel_count_deriver(map);
}

pub fn schemas() -> Vec<ModuleSchema> {
    vec![
        additive_oscillator::AdditiveOscillator::get_schema(),
        bass_drum_oscillator::BassDrumOscillator::get_schema(),
        chiptune_oscillator::ChiptuneOscillator::get_schema(),
        chord_oscillator::ChordOscillator::get_schema(),
        clocked_noise_oscillator::ClockedNoiseOscillator::get_schema(),
        fm_oscillator::FmOscillator::get_schema(),
        grain_oscillator::GrainOscillator::get_schema(),
        hihat_oscillator::HihatOscillator::get_schema(),
        modal_oscillator::ModalOscillator::get_schema(),
        particle_oscillator::ParticleOscillator::get_schema(),
        phase_distortion_oscillator::PhaseDistortionOscillator::get_schema(),
        six_op_fm_oscillator::SixOpFmOscillator::get_schema(),
        snare_drum_oscillator::SnareDrumOscillator::get_schema(),
        speech_oscillator::SpeechOscillator::get_schema(),
        string_machine_oscillator::StringMachineOscillator::get_schema(),
        string_oscillator::StringOscillator::get_schema(),
        swarm_oscillator::SwarmOscillator::get_schema(),
        vcf_oscillator::VcfOscillator::get_schema(),
        virtual_analog_oscillator::VirtualAnalogOscillator::get_schema(),
        wave_terrain_oscillator::WaveTerrainOscillator::get_schema(),
        waveshaping_oscillator::WaveshapingOscillator::get_schema(),
        wavetable_oscillator::WavetableOscillator::get_schema(),
    ]
}
