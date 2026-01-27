use crate::poly::{PolyOutput, PolySignal, PORT_MAX_CHANNELS};
use crate::types::Clickless;
use napi::Result;
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Deserialize, Default, JsonSchema, Connect, ChannelCount)]
#[serde(default)]
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

    range: (PolySignal, PolySignal),
}

#[derive(Clone, Copy, PartialEq)]
enum EnvelopeStage {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

impl Default for EnvelopeStage {
    fn default() -> Self {
        EnvelopeStage::Idle
    }
}

/// Per-channel envelope state
#[derive(Clone, Copy)]
struct ChannelState {
    stage: EnvelopeStage,
    current_level: f32,
    gate_was_high: bool,
    attack: Clickless,
    decay: Clickless,
    release: Clickless,
    sustain: Clickless,
}

impl Default for ChannelState {
    fn default() -> Self {
        Self {
            stage: EnvelopeStage::Idle,
            current_level: 0.0,
            gate_was_high: false,
            attack: 0.01.into(),
            decay: 0.1.into(),
            release: 0.1.into(),
            sustain: 3.5.into(),
        }
    }
}

#[derive(Module)]
#[module("adsr", "ADSR envelope generator")]
#[args(gate)]
pub struct Adsr {
    outputs: AdsrOutputs,
    channels: [ChannelState; PORT_MAX_CHANNELS],
    params: AdsrParams,
}

#[derive(Outputs, JsonSchema)]
struct AdsrOutputs {
    #[output("output", "envelope output", default)]
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

        let mut output = PolyOutput::default();
        output.set_channels(num_channels as u8);

        for ch in 0..num_channels {
            let state = &mut self.channels[ch];

            // Smooth parameter targets to avoid clicks when values change (times in seconds)
            state
                .attack
                .update(self.params.attack.get_value_or(ch, 0.01).max(0.001));
            state
                .decay
                .update(self.params.decay.get_value_or(ch, 0.1).max(0.001));
            state
                .release
                .update(self.params.release.get_value_or(ch, 0.1).max(0.001));
            state
                .sustain
                .update(self.params.sustain.get_value_or(ch, 5.).max(0.0));

            let attack = *state.attack;
            let decay = *state.decay;
            let release_var = *state.release;

            let gate_on = self.params.gate.get_value(ch) > 2.5;

            if gate_on && !state.gate_was_high {
                state.stage = EnvelopeStage::Attack;
            } else if !gate_on && state.gate_was_high {
                state.stage = EnvelopeStage::Release;
            }
            state.gate_was_high = gate_on;

            let sustain_level = (*state.sustain / 5.0).clamp(0.0, 1.0);

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

            let min = self.params.range.0.get_value_or(ch, 0.0);
            let max = self.params.range.1.get_value_or(ch, 5.0);
            output.set(
                ch,
                crate::dsp::utils::map_range(state.current_level, 0.0, 1.0, min, max),
            );
        }

        self.outputs.sample = output;
    }
}

message_handlers!(impl Adsr {});
