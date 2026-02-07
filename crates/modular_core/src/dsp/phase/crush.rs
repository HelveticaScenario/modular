//! Bit-crushing / XOR phase-distortion module.
//!
//! Adapted from the 4ms Ensemble Oscillator twist mode.
//! Copyright 4ms Company. Used under GPL v3.

use schemars::JsonSchema;
use serde::Deserialize;

use crate::poly::{PORT_MAX_CHANNELS, PolyOutput, PolySignal};
use crate::types::Clickless;

#[derive(Deserialize, Default, JsonSchema, Connect, ChannelCount)]
#[serde(default, rename_all = "camelCase")]
struct CrushParams {
    /// input phase (0 to 1)
    input: PolySignal,
    /// crush amount (0-5, where 0 = clean, 5 = maximum XOR distortion)
    amount: PolySignal,
}

#[derive(Outputs, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct CrushOutputs {
    #[output("output", "crushed phase output", default, range = (0.0, 1.0))]
    sample: PolyOutput,
}

#[derive(Default, Clone, Copy)]
struct ChannelState {
    amount: Clickless,
}

/// Bit-crushing XOR phase-distortion effect adapted from 4ms Ensemble Oscillator.
///
/// Takes a phase (0-1) and applies XOR-based bit manipulation,
/// creating digital/crushed phase patterns. Unlike traditional bitcrushing,
/// this uses XOR operations between the phase and scaled versions of itself.
///
/// No anti-aliasing is applied—the aliasing artifacts are intentional
/// and part of the character.
#[module(
    name = "phase.crush",
    description = "XOR bit-crush phase-distortion adapted from 4ms Ensemble Oscillator",
    args(input, amount?)
)]
#[derive(Default)]
pub struct Crush {
    outputs: CrushOutputs,
    channels: [ChannelState; PORT_MAX_CHANNELS],
    params: CrushParams,
}

impl Crush {
    fn update(&mut self, _sample_rate: f32) {
        let num_channels = self.channel_count();

        for ch in 0..num_channels {
            let state = &mut self.channels[ch];

            let input = self.params.input.get_value(ch);
            let amount_raw = self.params.amount.get_value_or(ch, 0.0);

            // Smooth amount parameter
            state.amount.update(amount_raw);
            let amount = *state.amount;

            // Normalize amount from [0, 5] to [0, 1]
            let amount_norm = (amount / 5.0).clamp(0.0, 1.0);

            // Apply reference scaling curve: 0.5 * x^2
            // Quadratic onset with cap at 0.5 for gentler XOR distortion
            let amount_norm = amount_norm * amount_norm * 0.5;
            // No AA for crush — aliasing is intentional

            // Input is already a phase [0, 1]
            let input_phase = input.clamp(0.0, 1.0);

            // Convert to 32-bit fixed point matching reference u0_32/u0_16 types
            let phase_u32 = (input_phase * 4_294_967_295.0) as u32; // u0_32
            let phase_u16 = (phase_u32 >> 16) as u16; // u0_16::narrow(phase)
            let am_u16 = (amount_norm * 65535.0) as u16; // u0_16(amount)

            // Reference: Distortion::twist<CRUSH>
            //   x ^= Bitfield<32>(u0_16::narrow(phase) * am)
            //   x ^= Bitfield<32>(0.25_u0_16 * am)
            let mut x = phase_u32;
            x ^= phase_u16 as u32 * am_u16 as u32;
            x ^= 16384u32 * am_u16 as u32; // 0.25 in u0_16 = 16384

            // Convert crushed phase back to [0, 1]
            let crushed_phase = x as f32 / 4_294_967_295.0;

            // Output the distorted phase
            self.outputs.sample.set(ch, crushed_phase);
        }
    }
}

message_handlers!(impl Crush {});
