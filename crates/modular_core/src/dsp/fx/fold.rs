//! Wavefolding effect module.
//!
//! Adapted from the 4ms Ensemble Oscillator warp mode.
//! Copyright 4ms Company. Used under GPL v3.

use schemars::JsonSchema;
use serde::Deserialize;

use crate::dsp::fx::enosc_tables::{aa_fold, lookup_fold};
use crate::dsp::utils::voct_to_hz;
use crate::poly::{PORT_MAX_CHANNELS, PolyOutput, PolySignal};
use crate::types::Clickless;

#[derive(Deserialize, Default, JsonSchema, Connect, ChannelCount)]
#[serde(default, rename_all = "camelCase")]
struct FoldParams {
    /// input signal to fold (bipolar, typically -5 to 5)
    input: PolySignal,
    /// fold amount (0-5, where 0 = bypass, 5 = maximum folding)
    amount: PolySignal,
    /// frequency in v/oct (optional, enables anti-aliasing when connected)
    freq: PolySignal,
}

#[derive(Outputs, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct FoldOutputs {
    #[output("output", "folded signal output", default, range = (-5.0, 5.0))]
    sample: PolyOutput,
}

#[derive(Default, Clone, Copy)]
struct ChannelState {
    amount: Clickless,
}

/// Wavefolding effect adapted from 4ms Ensemble Oscillator.
///
/// Folds the input signal back on itself when it exceeds thresholds,
/// creating harmonic-rich tones. Uses a 6x overfolding lookup table
/// for smooth, musical results.
#[module(
    name = "fx.fold",
    description = "Wavefolder effect adapted from 4ms Ensemble Oscillator",
    args(input, amount?)
)]
#[derive(Default)]
pub struct Fold {
    outputs: FoldOutputs,
    channels: [ChannelState; PORT_MAX_CHANNELS],
    params: FoldParams,
}

impl Fold {
    fn update(&mut self, sample_rate: f32) {
        let num_channels = self.channel_count();
        let freq_connected = !self.params.freq.is_disconnected();

        for ch in 0..num_channels {
            let state = &mut self.channels[ch];

            let input = self.params.input.get_value(ch);
            let amount_raw = self.params.amount.get_value_or(ch, 1.0);

            // Smooth amount parameter to avoid clicks
            state.amount.update(amount_raw);
            let amount = *state.amount;

            // Normalize amount from [0, 5] to [0, 1] for table lookup
            let amount_norm = (amount / 5.0).clamp(0.0, 1.0);

            // Apply reference scaling curve: 0.9 * x^2 + 0.004
            // Gives quadratic onset and a small offset so fold is never fully off
            let amount_norm = amount_norm * amount_norm * 0.9 + 0.004;

            // Apply anti-aliasing when freq is connected
            let amount_norm = if freq_connected {
                let freq_hz = voct_to_hz(self.params.freq.get_value(ch));
                aa_fold(freq_hz / sample_rate, amount_norm)
            } else {
                amount_norm
            };

            // Normalize input from typical [-5, 5] range to [-1, 1]
            let input_norm = (input / 5.0).clamp(-1.0, 1.0);

            // Apply wavefold
            let folded = lookup_fold(input_norm, amount_norm);

            // Scale back to output range
            self.outputs.sample.set(ch, folded * 5.0);
        }
    }
}

message_handlers!(impl Fold {});
