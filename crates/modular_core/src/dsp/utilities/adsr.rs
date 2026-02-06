use crate::dsp::utils::SchmittTrigger;
use crate::poly::{PORT_MAX_CHANNELS, PolyOutput, PolySignal};
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Deserialize, Default, JsonSchema, Connect, ChannelCount)]
#[serde(default, rename_all = "camelCase")]
struct AdsrParams {
    /// gate input (expects >0V for on)
    gate: PolySignal,
    /// attack time in seconds
    attack: PolySignal,
    /// decay time in seconds
    decay: PolySignal,
    /// sustain level in volts (0-5)
    sustain: PolySignal,
    /// release time in seconds
    release: PolySignal,
}

#[derive(Clone, Copy, PartialEq, Default)]
enum EnvelopeStage {
    #[default]
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

/// Per-channel envelope state
#[derive(Clone, Copy)]
struct ChannelState {
    stage: EnvelopeStage,
    current_level: f32,
    gate_schmitt: SchmittTrigger,
    attack: f32,
    decay: f32,
    release: f32,
    sustain: f32,
}

impl Default for ChannelState {
    fn default() -> Self {
        Self {
            stage: EnvelopeStage::Idle,
            current_level: 0.0,
            gate_schmitt: SchmittTrigger::default(),
            attack: 0.01,
            decay: 0.1,
            release: 0.1,
            sustain: 3.5,
        }
    }
}

#[derive(Module)]
#[module("env.adsr", "ADSR envelope generator")]
#[args(gate)]
pub struct Adsr {
    outputs: AdsrOutputs,
    channels: [ChannelState; PORT_MAX_CHANNELS],
    params: AdsrParams,
}

#[derive(Outputs, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct AdsrOutputs {
    #[output("output", "envelope output", default, range = (0.0, 5.0))]
    sample: PolyOutput,
}

impl Default for Adsr {
    fn default() -> Self {
        Self {
            outputs: AdsrOutputs::default(),
            channels: std::array::from_fn(|_| ChannelState::default()),
            params: AdsrParams::default(),
        }
    }
}

impl Adsr {
    fn update(&mut self, sample_rate: f32) {
        let num_channels = self.channel_count();

        self.outputs.sample.set_channels(num_channels);

        for ch in 0..num_channels {
            let state = &mut self.channels[ch];

            // Smooth parameter targets to avoid clicks when values change (times in seconds)
            state.attack = self.params.attack.get_value_or(ch, 0.01).max(0.001);
            state.decay = self.params.decay.get_value_or(ch, 0.1).max(0.001);
            state.release = self.params.release.get_value_or(ch, 0.1).max(0.001);
            state.sustain = self.params.sustain.get_value_or(ch, 5.).max(0.0);

            let attack = state.attack;
            let decay = state.decay;
            let release_var = state.release;

            let gate_val = self.params.gate.get_value(ch);
            let (gate_on, edge) = state.gate_schmitt.process_with_edge(gate_val);

            if edge.is_rising() {
                state.stage = EnvelopeStage::Attack;
            } else if edge.is_falling() {
                state.stage = EnvelopeStage::Release;
            }

            let sustain_level = (state.sustain / 5.0).clamp(0.0, 1.0);

            match state.stage {
                EnvelopeStage::Idle => {
                    state.current_level = 0.0;
                }
                EnvelopeStage::Attack => {
                    if attack < 0.0001 {
                        state.current_level = 1.0;
                        state.stage = EnvelopeStage::Decay;
                    } else {
                        let step = 1.0 / (attack * sample_rate);
                        state.current_level += step;
                        if state.current_level >= 1.0 {
                            state.current_level = 1.0;
                            state.stage = EnvelopeStage::Decay;
                        }
                    }
                }
                EnvelopeStage::Decay => {
                    if decay <= 0.0001 || state.current_level <= sustain_level {
                        state.current_level = sustain_level;
                        state.stage = EnvelopeStage::Sustain;
                    } else {
                        let step = (1.0 - sustain_level) / (decay * sample_rate);
                        state.current_level = (state.current_level - step).max(sustain_level);
                        if state.current_level <= sustain_level {
                            state.current_level = sustain_level;
                            state.stage = EnvelopeStage::Sustain;
                        }
                    }
                }
                EnvelopeStage::Sustain => {
                    state.current_level = sustain_level;
                    if !gate_on {
                        state.stage = EnvelopeStage::Release;
                    }
                }
                EnvelopeStage::Release => {
                    if release_var <= 0.0001 {
                        state.current_level = 0.0;
                        state.stage = EnvelopeStage::Idle;
                    } else {
                        let step = state.current_level / (release_var * sample_rate);
                        state.current_level = (state.current_level - step).max(0.0);
                        if state.current_level <= 0.00001 {
                            state.current_level = 0.0;
                            state.stage = if gate_on {
                                EnvelopeStage::Attack
                            } else {
                                EnvelopeStage::Idle
                            };
                        }
                    }
                }
            }

            self.outputs.sample.set(ch, state.current_level * 5.0);
        }
    }
}

message_handlers!(impl Adsr {});
