use napi::Result;
use schemars::JsonSchema;
use serde::Deserialize;

use crate::{
    dsp::utils::voct_to_midi,
    types::{Clickless, Signal},
};
use mi_plaits_dsp::engine::{
    Engine, EngineParameters, TriggerState, wavetable_engine::WavetableEngine,
};

const BLOCK_SIZE: usize = 1;

#[derive(Deserialize, Default, JsonSchema, Connect, ChannelCount)]
#[serde(default)]
struct WavetableOscillatorParams {
    /// frequency in v/oct
    freq: Signal,
    /// timbre parameter (0-1): row index, waves sorted by spectral brightness
    timbre: Signal,
    /// morph parameter (0-1): column index
    morph: Signal,
    /// harmonics parameter (0-1): bank selection (4 interpolated banks, then same 4 reversed without interpolation)
    harmonics: Signal,
    /// sync input (expects >0V to trigger)
    sync: Signal,
}

#[derive(Outputs, JsonSchema)]
struct WavetableOscillatorOutputs {
    #[output("output", "signal output", range = (-1.0, 1.0))]
    sample: f32,
    #[output("aux", "low-fi (5-bit) output", range = (-1.0, 1.0))]
    aux: f32,
}

#[derive(Default, Module)]
#[module("mi.wavetable", "Wavetable oscillator with 4 banks of 8x8 waveforms")]
#[args(freq)]
pub struct WavetableOscillator {
    outputs: WavetableOscillatorOutputs,
    engine: Option<WavetableEngine<'static>>,

    buffer_out: Vec<f32>,
    buffer_aux: Vec<f32>,
    buffer_pos: usize,

    last_sync: f32,
    sample_rate: f32,

    freq: Clickless,
    timbre: Clickless,
    morph: Clickless,
    harmonics: Clickless,

    params: WavetableOscillatorParams,
}

impl WavetableOscillator {
    fn update(&mut self, sample_rate: f32) {
        // Initialize engine if needed
        if self.engine.is_none() || (self.sample_rate - sample_rate).abs() > 0.1 {
            let mut engine = WavetableEngine::new();
            engine.init(sample_rate);
            self.engine = Some(engine);
            self.sample_rate = sample_rate;
            self.buffer_out = vec![0.0; BLOCK_SIZE];
            self.buffer_aux = vec![0.0; BLOCK_SIZE];
            self.buffer_pos = BLOCK_SIZE; // Force initial render
        }

        // Check if we need to render a new block
        if self.buffer_pos >= BLOCK_SIZE {
            self.render_block(sample_rate);
            self.buffer_pos = 0;
        }

        // Output current sample from buffer
        self.outputs.sample = self.buffer_out[self.buffer_pos];
        self.outputs.aux = self.buffer_aux[self.buffer_pos];

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

            // Detect trigger state
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

            // Build engine parameters
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

message_handlers!(impl WavetableOscillator {});
