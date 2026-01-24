//! Combine module - packs mono signals into a polyphonic output.
//!
//! Takes an array of signals and combines them into channels of a single
//! polyphonic output. Each input signal becomes one channel.

use napi::Result;
use schemars::JsonSchema;
use serde::Deserialize;

use crate::poly::{PolySignal, PORT_MAX_CHANNELS};
use crate::types::Signal;

#[derive(Deserialize, Default, JsonSchema, Connect)]
#[serde(default)]
struct CombineParams {
    /// Array of signals to combine into poly channels.
    /// Each signal becomes one channel of the output.
    signals: Vec<Signal>,
}

#[derive(Outputs, JsonSchema)]
struct CombineOutputs {
    /// Polyphonic combined output
    #[output("output", "polyphonic combined output", default)]
    sample: PolySignal,
}

#[derive(Default, Module)]
#[module("combine", "Combine mono signals into a polyphonic signal")]
#[args(signals)]
pub struct Combine {
    outputs: CombineOutputs,
    params: CombineParams,
}

impl Combine {
    fn update(&mut self, _sample_rate: f32) {
        let signals = &self.params.signals;

        // Filter to connected signals and take up to PORT_MAX_CHANNELS
        let connected: Vec<_> = signals
            .iter()
            .filter(|s| !s.is_disconnected())
            .take(PORT_MAX_CHANNELS)
            .collect();

        if connected.is_empty() {
            self.outputs.sample = PolySignal::default();
            return;
        }

        let channels = connected.len();
        let mut output = PolySignal::default();
        output.set_channels(channels as u8);

        for (i, sig) in connected.iter().enumerate() {
            // Take channel 0 of each input (flattens poly inputs to mono)
            output.set(i, sig.get_poly_signal().get(0));
        }

        self.outputs.sample = output;
    }
}

message_handlers!(impl Combine {});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::poly::PolySignal;

    #[test]
    fn test_combine_empty() {
        let mut combine = Combine::default();
        combine.update(48000.0);
        assert!(combine.outputs.sample.is_disconnected());
    }

    #[test]
    fn test_combine_mono() {
        let mut combine = Combine {
            params: CombineParams {
                signals: vec![Signal::Volts(PolySignal::mono(1.5))],
            },
            ..Default::default()
        };
        combine.update(48000.0);
        assert_eq!(combine.outputs.sample.channels(), 1);
        assert_eq!(combine.outputs.sample.get(0), 1.5);
    }

    #[test]
    fn test_combine_poly() {
        let mut combine = Combine {
            params: CombineParams {
                signals: vec![
                    Signal::Volts(PolySignal::mono(1.0)),
                    Signal::Volts(PolySignal::mono(2.0)),
                    Signal::Volts(PolySignal::mono(3.0)),
                ],
            },
            ..Default::default()
        };
        combine.update(48000.0);
        assert_eq!(combine.outputs.sample.channels(), 3);
        assert_eq!(combine.outputs.sample.get(0), 1.0);
        assert_eq!(combine.outputs.sample.get(1), 2.0);
        assert_eq!(combine.outputs.sample.get(2), 3.0);
    }

    #[test]
    fn test_combine_filters_disconnected() {
        let mut combine = Combine {
            params: CombineParams {
                signals: vec![
                    Signal::Volts(PolySignal::mono(1.0)),
                    Signal::Disconnected,
                    Signal::Volts(PolySignal::mono(3.0)),
                ],
            },
            ..Default::default()
        };
        combine.update(48000.0);
        assert_eq!(combine.outputs.sample.channels(), 2);
        assert_eq!(combine.outputs.sample.get(0), 1.0);
        assert_eq!(combine.outputs.sample.get(1), 3.0);
    }
}
