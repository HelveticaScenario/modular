use napi::Result;
use schemars::JsonSchema;
use serde::Deserialize;
use simple_easing;

use crate::types::{InterpolationType, Signal};

#[derive(Deserialize, Default, JsonSchema, Connect)]
#[serde(default, rename_all = "camelCase")]
struct TrackParams {
    playhead: Signal,
    /// Keyframes as (signal, time) tuples. Must be sorted by time.
    keyframes: Vec<(Signal, f32)>,
    interpolation_type: InterpolationType,
}

#[derive(Outputs, JsonSchema)]
struct TrackOutputs {
    #[output("output", "signal output", default)]
    sample: f32,
}

#[derive(Default, Module)]
#[module("track", "A sequencer track")]
#[args(keyframes, playhead, interpolationType?)]
pub struct Track {
    outputs: TrackOutputs,
    params: TrackParams,
}

impl Track {
    fn update(&mut self, _sample_rate: f32) {
        if self.params.playhead.is_disconnected() {
            self.outputs.sample = 0.0;
            return;
        }

        let playhead_value = self.params.playhead.get_value();
        if self.params.keyframes.is_empty() {
            self.outputs.sample = 0.0;
            return;
        }

        let t = playhead_value.fract().abs();

        // Single keyframe: always return its value
        if self.params.keyframes.len() == 1 {
            self.outputs.sample = self.params.keyframes[0].0.get_value_or(0.0);
            return;
        }

        // Clamp to first/last keyframe times
        let first = &self.params.keyframes[0];
        if t <= first.1 {
            self.outputs.sample = first.0.get_value_or(0.0);
            return;
        }
        let last = self.params.keyframes.last().unwrap();
        if t >= last.1 {
            self.outputs.sample = last.0.get_value_or(0.0);
            return;
        }

        // Find the segment [curr, next] such that curr.time <= t <= next.time
        // Use partition_point to find the first keyframe with time > t
        // Then back up one to get the last keyframe with time <= t
        let idx = self.params.keyframes.partition_point(|kf| kf.1 <= t);

        // partition_point returns the index of the first element > t
        // So idx-1 is the last element <= t, which is the start of our interpolation segment
        let idx = if idx > 0 { idx - 1 } else { 0 };

        // Ensure idx is valid for the segment [idx, idx+1]
        let idx = idx.min(self.params.keyframes.len() - 2);

        let curr = &self.params.keyframes[idx];
        let next = &self.params.keyframes[idx + 1];

        let curr_value = curr.0.get_value_or(0.0);
        let next_value = next.0.get_value_or(0.0);

            let time_range = (next.1 - curr.1).max(f32::EPSILON);
            let mut local_t = (t - curr.1) / time_range;
            local_t = local_t.clamp(0.0, 1.0);

            self.outputs.sample = match self.params.interpolation_type {
                InterpolationType::Linear => {
                    curr_value + (next_value - curr_value) * simple_easing::linear(local_t)
                }
                InterpolationType::Step => curr_value,
                InterpolationType::SineIn => {
                    curr_value + (next_value - curr_value) * simple_easing::sine_in(local_t)
                }
                InterpolationType::SineOut => {
                    curr_value + (next_value - curr_value) * simple_easing::sine_out(local_t)
                }
                InterpolationType::SineInOut => {
                    curr_value + (next_value - curr_value) * simple_easing::sine_in_out(local_t)
                }
                InterpolationType::QuadIn => {
                    curr_value + (next_value - curr_value) * simple_easing::quad_in(local_t)
                }
                InterpolationType::QuadOut => {
                    curr_value + (next_value - curr_value) * simple_easing::quad_out(local_t)
                }
                InterpolationType::QuadInOut => {
                    curr_value + (next_value - curr_value) * simple_easing::quad_in_out(local_t)
                }
                InterpolationType::CubicIn => {
                    curr_value + (next_value - curr_value) * simple_easing::cubic_in(local_t)
                }
                InterpolationType::CubicOut => {
                    curr_value + (next_value - curr_value) * simple_easing::cubic_out(local_t)
                }
                InterpolationType::CubicInOut => {
                    curr_value + (next_value - curr_value) * simple_easing::cubic_in_out(local_t)
                }
                InterpolationType::QuartIn => {
                    curr_value + (next_value - curr_value) * simple_easing::quart_in(local_t)
                }
                InterpolationType::QuartOut => {
                    curr_value + (next_value - curr_value) * simple_easing::quart_out(local_t)
                }
                InterpolationType::QuartInOut => {
                    curr_value + (next_value - curr_value) * simple_easing::quart_in_out(local_t)
                }
                InterpolationType::QuintIn => {
                    curr_value + (next_value - curr_value) * simple_easing::quint_in(local_t)
                }
                InterpolationType::QuintOut => {
                    curr_value + (next_value - curr_value) * simple_easing::quint_out(local_t)
                }
                InterpolationType::QuintInOut => {
                    curr_value + (next_value - curr_value) * simple_easing::quint_in_out(local_t)
                }
                InterpolationType::ExpoIn => {
                    curr_value + (next_value - curr_value) * simple_easing::expo_in(local_t)
                }
                InterpolationType::ExpoOut => {
                    curr_value + (next_value - curr_value) * simple_easing::expo_out(local_t)
                }
                InterpolationType::ExpoInOut => {
                    curr_value + (next_value - curr_value) * simple_easing::expo_in_out(local_t)
                }
                InterpolationType::CircIn => {
                    curr_value + (next_value - curr_value) * simple_easing::circ_in(local_t)
                }
                InterpolationType::CircOut => {
                    curr_value + (next_value - curr_value) * simple_easing::circ_out(local_t)
                }
                InterpolationType::CircInOut => {
                    curr_value + (next_value - curr_value) * simple_easing::circ_in_out(local_t)
                }
                InterpolationType::BounceIn => {
                    curr_value + (next_value - curr_value) * simple_easing::bounce_in(local_t)
                }
                InterpolationType::BounceOut => {
                    curr_value + (next_value - curr_value) * simple_easing::bounce_out(local_t)
                }
                InterpolationType::BounceInOut => {
                    curr_value + (next_value - curr_value) * simple_easing::bounce_in_out(local_t)
                }
            };
    }
}

message_handlers!(impl Track {});
