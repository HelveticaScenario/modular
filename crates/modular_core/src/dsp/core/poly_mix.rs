//! PolyMix module - mixes a polyphonic signal down to mono.
//!
//! Supports different mixing modes: sum, average, or max.

use napi::Result;
use schemars::JsonSchema;
use serde::Deserialize;

use crate::poly::PolySignal;
use crate::types::Signal;

/// Mixing mode for combining channels
#[derive(Clone, Copy, Debug, Default, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MixMode {
    /// Sum all channels
    #[default]
    Sum,
    /// Average all channels
    Average,
    /// Take the maximum absolute value
    Max,
    /// Take the minimum absolute value
    Min,
}

impl crate::types::Connect for MixMode {
    fn connect(&mut self, _patch: &crate::Patch) {}
}

#[derive(Deserialize, Default, JsonSchema, Connect, ChannelCount)]
#[serde(default)]
struct PolyMixParams {
    /// Polyphonic input to mix down
    input: PolySignal,
    /// Mixing mode
    mode: MixMode,
    /// Output gain/attenuation (mono)
    gain: Signal,
}

#[derive(Outputs, JsonSchema)]
struct PolyMixOutputs {
    /// Mixed mono output
    #[output("output", "mono mixed output")]
    sample: f32,
    /// Number of channels that were mixed
    #[output("channels", "number of input channels")]
    channels: f32,
}

#[derive(Default, Module)]
#[module("poly_mix", "Mix polyphonic signal to mono")]
#[args(input)]
pub struct PolyMix {
    outputs: PolyMixOutputs,
    params: PolyMixParams,
}

impl PolyMix {
    fn update(&mut self, _sample_rate: f32) {
        let poly = &self.params.input;
        let channels = poly.channels();
        let gain = self.params.gain.get_value_or(1.0);

        if channels == 0 {
            self.outputs.sample = 0.0;
            self.outputs.channels = 0.0;
            return;
        }

        // Collect all channel values
        let voltages: Vec<f32> = (0..channels as usize)
            .map(|i| poly.get_value(i))
            .collect();

        let result = match self.params.mode {
            MixMode::Sum => voltages.iter().sum::<f32>(),
            MixMode::Average => voltages.iter().sum::<f32>() / channels as f32,
            MixMode::Max => voltages
                .iter()
                .max_by(|a: &&f32, b: &&f32| a.abs().partial_cmp(&b.abs()).unwrap())
                .copied()
                .unwrap_or(0.0),
            MixMode::Min => voltages
                .iter()
                .min_by(|a: &&f32, b: &&f32| a.abs().partial_cmp(&b.abs()).unwrap())
                .copied()
                .unwrap_or(0.0),
        };

        self.outputs.sample = result * gain;
        self.outputs.channels = channels as f32;
    }
}

message_handlers!(impl PolyMix {});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::poly::PolySignal;

    #[test]
    fn test_poly_mix_sum() {
        let mut mixer = PolyMix {
            params: PolyMixParams {
                input: PolySignal::poly(&[Signal::Volts(1.0), Signal::Volts(2.0), Signal::Volts(3.0)]),
                mode: MixMode::Sum,
                gain: Signal::Volts(1.0),
            },
            ..Default::default()
        };
        mixer.update(48000.0);
        assert_eq!(mixer.outputs.sample, 6.0);
        assert_eq!(mixer.outputs.channels, 3.0);
    }

    #[test]
    fn test_poly_mix_average() {
        let mut mixer = PolyMix {
            params: PolyMixParams {
                input: PolySignal::poly(&[Signal::Volts(1.0), Signal::Volts(2.0), Signal::Volts(3.0)]),
                mode: MixMode::Average,
                gain: Signal::Volts(1.0),
            },
            ..Default::default()
        };
        mixer.update(48000.0);
        assert_eq!(mixer.outputs.sample, 2.0);
    }

    #[test]
    fn test_poly_mix_max() {
        let mut mixer = PolyMix {
            params: PolyMixParams {
                input: PolySignal::poly(&[Signal::Volts(1.0), Signal::Volts(-5.0), Signal::Volts(3.0)]),
                mode: MixMode::Max,
                gain: Signal::Volts(1.0),
            },
            ..Default::default()
        };
        mixer.update(48000.0);
        assert_eq!(mixer.outputs.sample, -5.0); // -5 has max abs value
    }

    #[test]
    fn test_poly_mix_gain() {
        let mut mixer = PolyMix {
            params: PolyMixParams {
                input: PolySignal::poly(&[Signal::Volts(1.0), Signal::Volts(2.0)]),
                mode: MixMode::Sum,
                gain: Signal::Volts(0.5),
            },
            ..Default::default()
        };
        mixer.update(48000.0);
        assert_eq!(mixer.outputs.sample, 1.5); // (1+2) * 0.5
    }

    #[test]
    fn test_poly_mix_disconnected() {
        let mut mixer = PolyMix::default();
        mixer.update(48000.0);
        assert_eq!(mixer.outputs.sample, 0.0);
        assert_eq!(mixer.outputs.channels, 0.0);
    }
}
