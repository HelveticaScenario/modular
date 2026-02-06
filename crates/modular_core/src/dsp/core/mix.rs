//! Mix module - merges multiple polyphonic signals into a single polyphonic output.
//!
//! Mixes corresponding channels from each input together (channel 0 from each input
//! is mixed to output channel 0, etc.). Inputs with fewer channels contribute 0.0
//! for missing channels (no cycling). The gain parameter can extend the output
//! channel count beyond the max input channels, cycling the pre-gain mixed values.

use schemars::JsonSchema;
use serde::Deserialize;

use crate::{
    poly::{PORT_MAX_CHANNELS, PolyOutput, PolySignal},
    types::Clickless,
};

/// Mixing mode for combining channels
#[derive(Clone, Copy, Debug, Default, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MixMode {
    /// Sum all inputs at each channel
    Sum,
    /// Average all inputs at each channel
    #[default]
    Average,
    /// Take the maximum absolute value at each channel
    Max,
    /// Take the minimum absolute value at each channel
    Min,
}

impl crate::types::Connect for MixMode {
    fn connect(&mut self, _patch: &crate::Patch) {}
}

#[derive(Deserialize, Default, JsonSchema, Connect, ChannelCount)]
#[serde(default, rename_all = "camelCase")]
pub struct MixParams {
    /// Polyphonic inputs to mix together
    pub inputs: Vec<PolySignal>,
    /// Mixing mode (applied per-channel across all inputs)
    mode: MixMode,
    /// Output gain/attenuation (polyphonic - can extend output channels)
    pub gain: PolySignal,
}

#[derive(Outputs, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct MixOutputs {
    /// Mixed polyphonic output
    #[output("output", "mixed polyphonic output", default)]
    sample: PolyOutput,
}

/// Custom channel count derivation for Mix.
///
/// Mix output channels = max(max_input_channels, gain_channels), at least 1.
/// This matches the runtime behavior in update().
pub fn mix_derive_channel_count(params: &MixParams) -> usize {
    // Get max channel count from inputs
    let input_refs: Vec<&PolySignal> = params.inputs.iter().collect();

    let max_input_channels = if params.inputs.is_empty() {
        0usize
    } else {
        PolySignal::max_channels(&input_refs) as usize
    };

    // Get gain channel count
    let gain_channels = params.gain.channels() as usize;

    // Output channels = max(max_input_channels, gain_channels), at least 1 if inputs empty
    if params.inputs.is_empty() {
        gain_channels.max(1)
    } else {
        max_input_channels.max(gain_channels)
    }
    .min(PORT_MAX_CHANNELS)
}

#[module(name = "mix", description = "Mix multiple polyphonic signals together", channels_derive = mix_derive_channel_count, args(inputs))]
#[derive(Default)]
pub struct Mix {
    outputs: MixOutputs,
    params: MixParams,
    gain_buffer: [Clickless; PORT_MAX_CHANNELS],
}

message_handlers!(impl Mix {});

impl Mix {
    fn update(&mut self, _sample_rate: f32) {
        let inputs = &self.params.inputs;
        let gain = &self.params.gain;

        let input_refs: Vec<&PolySignal> = self.params.inputs.iter().collect();

        let max_input_channels = if self.params.inputs.is_empty() {
            0usize
        } else {
            PolySignal::max_channels(&input_refs) as usize
        };

        let output_channels = self.channel_count();

        // Handle empty inputs case - output silence
        if inputs.is_empty() {
            for i in 0..output_channels {
                self.outputs.sample.set(i, 0.0);
            }
            return;
        }

        // Pre-compute mixed values for each input channel (no cycling on inputs)
        let mut pre_gain_values = [0.0f32; PORT_MAX_CHANNELS];
        for channel in 0..max_input_channels {
            // Collect values from each input at this channel index
            // Inputs with fewer channels contribute 0.0 (no cycling)
            let values: Vec<f32> = inputs
                .iter()
                .map(|input| {
                    if channel < input.channels() as usize {
                        input.get(channel).get_value()
                    } else {
                        0.0
                    }
                })
                .collect();

            // Count non-zero contributors for averaging
            let contributor_count = inputs
                .iter()
                .filter(|input| channel < input.channels() as usize)
                .count();

            pre_gain_values[channel] = match self.params.mode {
                MixMode::Sum => values.iter().sum::<f32>(),
                MixMode::Average => {
                    if contributor_count > 0 {
                        values.iter().sum::<f32>() / contributor_count as f32
                    } else {
                        0.0
                    }
                }
                MixMode::Max => values
                    .iter()
                    .max_by(|a, b| a.abs().partial_cmp(&b.abs()).unwrap())
                    .copied()
                    .unwrap_or(0.0),
                MixMode::Min => values
                    .iter()
                    .filter(|&&v| v != 0.0) // Exclude zero-contributors for min
                    .min_by(|a, b| a.abs().partial_cmp(&b.abs()).unwrap())
                    .copied()
                    .unwrap_or(0.0),
            };
        }

        // Apply gain with cycling on pre_gain_values
        for i in 0..output_channels {
            let pre_gain_index = i % max_input_channels;
            let pre_gain_value = pre_gain_values[pre_gain_index];
            self.gain_buffer[i].update(gain.get_value_or(i, 5.0) / 5.0);
            let gain_value = *self.gain_buffer[i];
            self.outputs.sample.set(i, pre_gain_value * gain_value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::poly::PolySignal;
    use crate::types::Signal;

    #[test]
    fn test_mix_single_poly_sum() {
        let mut mixer = Mix {
            params: MixParams {
                inputs: vec![PolySignal::poly(&[
                    Signal::Volts(1.0),
                    Signal::Volts(2.0),
                    Signal::Volts(3.0),
                ])],
                mode: MixMode::Sum,
                gain: PolySignal::default(),
            },
            ..Default::default()
        };
        mixer.update(48000.0);
        assert_eq!(mixer.outputs.sample.channels(), 3);
        assert_eq!(mixer.outputs.sample.get(0), 1.0);
        assert_eq!(mixer.outputs.sample.get(1), 2.0);
        assert_eq!(mixer.outputs.sample.get(2), 3.0);
    }

    #[test]
    fn test_mix_two_poly_sum() {
        // A: 2 channels [1, 2], B: 3 channels [10, 20, 30]
        let mut mixer = Mix {
            params: MixParams {
                inputs: vec![
                    PolySignal::poly(&[Signal::Volts(1.0), Signal::Volts(2.0)]),
                    PolySignal::poly(&[
                        Signal::Volts(10.0),
                        Signal::Volts(20.0),
                        Signal::Volts(30.0),
                    ]),
                ],
                mode: MixMode::Sum,
                gain: PolySignal::default(),
            },
            ..Default::default()
        };
        mixer.update(48000.0);
        // Output should be 3 channels
        assert_eq!(mixer.outputs.sample.channels(), 3);
        // Channel 0: 1 + 10 = 11
        assert_eq!(mixer.outputs.sample.get(0), 11.0);
        // Channel 1: 2 + 20 = 22
        assert_eq!(mixer.outputs.sample.get(1), 22.0);
        // Channel 2: 0 + 30 = 30 (A has no channel 2, contributes 0)
        assert_eq!(mixer.outputs.sample.get(2), 30.0);
    }

    #[test]
    fn test_mix_average_mode() {
        // A: 2 channels [2, 4], B: 2 channels [6, 8]
        let mut mixer = Mix {
            params: MixParams {
                inputs: vec![
                    PolySignal::poly(&[Signal::Volts(2.0), Signal::Volts(4.0)]),
                    PolySignal::poly(&[Signal::Volts(6.0), Signal::Volts(8.0)]),
                ],
                mode: MixMode::Average,
                gain: PolySignal::default(),
            },
            ..Default::default()
        };
        mixer.update(48000.0);
        assert_eq!(mixer.outputs.sample.channels(), 2);
        // Channel 0: (2 + 6) / 2 = 4
        assert_eq!(mixer.outputs.sample.get(0), 4.0);
        // Channel 1: (4 + 8) / 2 = 6
        assert_eq!(mixer.outputs.sample.get(1), 6.0);
    }

    #[test]
    fn test_mix_gain_extends_channels() {
        // A: 1 channel [5], B: 2 channels [10, 20], gain: 3 channels [1, 2, 0.5]
        let mut mixer = Mix {
            params: MixParams {
                inputs: vec![
                    PolySignal::mono(Signal::Volts(5.0)),
                    PolySignal::poly(&[Signal::Volts(10.0), Signal::Volts(20.0)]),
                ],
                mode: MixMode::Sum,
                gain: PolySignal::poly(&[
                    Signal::Volts(5.0),
                    Signal::Volts(10.0),
                    Signal::Volts(2.5),
                ]),
            },
            ..Default::default()
        };
        mixer.update(48000.0);
        // Output channels = max(2 input channels, 3 gain channels) = 3
        assert_eq!(mixer.outputs.sample.channels(), 3);
        // Channel 0: (5 + 10) * 1 (normalized from 5) = 15
        assert_eq!(mixer.outputs.sample.get(0), 15.0);
        // Channel 1: (0 + 20) * 2 (normalized from 10) = 40
        assert_eq!(mixer.outputs.sample.get(1), 40.0);
        // Channel 2: pre_gain cycles from channel 0 (15 pre-gain), gain[2] = 0.5 (normalized from 2.5) -> 15 * 0.5 = 7.5
        assert_eq!(mixer.outputs.sample.get(2), 7.5);
    }

    #[test]
    fn test_mix_empty_inputs() {
        let mut mixer = Mix {
            params: MixParams {
                inputs: vec![],
                mode: MixMode::Sum,
                gain: PolySignal::poly(&[
                    Signal::Volts(1.0),
                    Signal::Volts(2.0),
                    Signal::Volts(3.0),
                ]),
            },
            ..Default::default()
        };
        mixer.update(48000.0);
        // Empty inputs with 3-channel gain -> 3 channels of silence
        assert_eq!(mixer.outputs.sample.channels(), 3);
        assert_eq!(mixer.outputs.sample.get(0), 0.0);
        assert_eq!(mixer.outputs.sample.get(1), 0.0);
        assert_eq!(mixer.outputs.sample.get(2), 0.0);
    }

    #[test]
    fn test_mix_empty_inputs_no_gain() {
        let mut mixer = Mix::default();
        mixer.update(48000.0);
        // Empty inputs with no gain -> 1 channel of silence
        assert_eq!(mixer.outputs.sample.channels(), 1);
        assert_eq!(mixer.outputs.sample.get(0), 0.0);
    }

    #[test]
    fn test_mix_max_mode() {
        let mut mixer = Mix {
            params: MixParams {
                inputs: vec![
                    PolySignal::poly(&[Signal::Volts(1.0), Signal::Volts(-5.0)]),
                    PolySignal::poly(&[Signal::Volts(-3.0), Signal::Volts(2.0)]),
                ],
                mode: MixMode::Max,
                gain: PolySignal::default(),
            },
            ..Default::default()
        };
        mixer.update(48000.0);
        assert_eq!(mixer.outputs.sample.channels(), 2);
        // Channel 0: max by abs(1, -3) = -3
        assert_eq!(mixer.outputs.sample.get(0), -3.0);
        // Channel 1: max by abs(-5, 2) = -5
        assert_eq!(mixer.outputs.sample.get(1), -5.0);
    }
}
