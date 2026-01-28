use napi::Result;
use schemars::JsonSchema;
use serde::Deserialize;

use crate::{
    dsp::utils::voct_to_midi,
    types::{Clickless, Signal},
};
use mi_plaits_dsp::{
    engine::{Engine, EngineParameters, TriggerState},
    engine2::string_machine_engine::StringMachineEngine,
};

const BLOCK_SIZE: usize = 1;

#[derive(Deserialize, Default, JsonSchema, Connect, ChannelCount)]
#[serde(default)]
struct StringMachineOscillatorParams {
    freq: Signal,
    timbre: Signal,
    morph: Signal,
    harmonics: Signal,
    sync: Signal,
    range: (Signal, Signal),
}

#[derive(Outputs, JsonSchema)]
struct StringMachineOscillatorOutputs {
    #[output("output", "signal output")]
    sample: f32,
    #[output("aux", "auxiliary output")]
    aux: f32,
}

#[derive(Default, Module)]
#[module("mi.stringmachine", "String machine emulation")]
#[args(freq)]
pub struct StringMachineOscillator {
    outputs: StringMachineOscillatorOutputs,
    engine: Option<StringMachineEngine>,
    buffer_out: Vec<f32>,
    buffer_aux: Vec<f32>,
    buffer_pos: usize,
    last_sync: f32,
    sample_rate: f32,
    freq: Clickless,
    timbre: Clickless,
    morph: Clickless,
    harmonics: Clickless,
    params: StringMachineOscillatorParams,
}

impl StringMachineOscillator {
    fn update(&mut self, sample_rate: f32) {
        if self.engine.is_none() || (self.sample_rate - sample_rate).abs() > 0.1 {
            let mut engine = StringMachineEngine::new();
            engine.init(sample_rate);
            self.engine = Some(engine);
            self.sample_rate = sample_rate;
            self.buffer_out = vec![0.0; BLOCK_SIZE];
            self.buffer_aux = vec![0.0; BLOCK_SIZE];
            self.buffer_pos = BLOCK_SIZE;
        }
        if self.buffer_pos >= BLOCK_SIZE {
            self.render_block(sample_rate);
            self.buffer_pos = 0;
        }
        let min = self.params.range.0.get_value_or(-5.0);
        let max = self.params.range.1.get_value_or(5.0);
        self.outputs.sample =
            crate::dsp::utils::map_range(self.buffer_out[self.buffer_pos], -1.0, 1.0, min, max);
        self.outputs.aux =
            crate::dsp::utils::map_range(self.buffer_aux[self.buffer_pos], -1.0, 1.0, min, max);
        self.buffer_pos += 1;
    }

    fn render_block(&mut self, sample_rate: f32) {
        if let Some(ref mut engine) = self.engine {
            // Update smooth parameters
            self.freq
                .update(self.params.freq.get_value_or(4.0).clamp(-10.0, 10.0));
            self.timbre
                .update(self.params.timbre.get_value_or(2.5).clamp(0.0, 5.0));
            self.morph
                .update(self.params.morph.get_value_or(2.5).clamp(0.0, 5.0));
            self.harmonics
                .update(self.params.harmonics.get_value_or(2.5).clamp(0.0, 5.0));

            // Convert V/oct to MIDI note (A4 = 4V/oct = 81 MIDI)
            let midi_note = voct_to_midi(*self.freq);

            // Convert signals (0V to +5V) to normalized (0.0 to 1.0)
            let timbre_norm = (*self.timbre) / 5.0;
            let morph_norm = (*self.morph) / 5.0;
            let harmonics_norm = (*self.harmonics) / 5.0;
            let trigger_state = if self.params.sync == Signal::Disconnected {
                TriggerState::Unpatched
            } else {
                let sync_val = self.params.sync.get_value_or(0.0);
                if sync_val > 0.0 && self.last_sync <= 0.0 {
                    self.last_sync = sync_val;
                    TriggerState::RisingEdge
                } else if sync_val > 0.0 {
                    TriggerState::High
                } else {
                    self.last_sync = sync_val;
                    TriggerState::Low
                }
            };
            let engine_params = EngineParameters {
                trigger: trigger_state,
                note: midi_note,
                timbre: timbre_norm,
                morph: morph_norm,
                harmonics: harmonics_norm,
                accent: 1.0,
                a0_normalized: 55.0 / sample_rate,
            };
            let mut already_enveloped = false;
            engine.render(
                &engine_params,
                &mut self.buffer_out,
                &mut self.buffer_aux,
                &mut already_enveloped,
            );
        }
    }
}

message_handlers!(impl StringMachineOscillator {});
