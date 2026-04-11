use deserr::Deserr;
use schemars::JsonSchema;

use crate::{
    poly::{PolyOutput, PolySignal},
    Buffer, MonoSignal,
};

fn read_interpolated_wrapped(buffer: &Buffer, channel: usize, frame: f32) -> f32 {
    if !frame.is_finite() {
        return 0.0;
    }

    let frame_count = buffer.frame_count();
    if frame_count == 0 {
        return 0.0;
    }

    let wrapped_frame = frame.rem_euclid(frame_count as f32);

    let left = wrapped_frame.floor() as usize;
    let frac = wrapped_frame - left as f32;
    if frac <= f32::EPSILON {
        return buffer.read(channel, left);
    }

    // 4-point cubic (Hermite) interpolation with wrapping
    let i0 = (left + frame_count - 1) % frame_count;
    let i1 = left;
    let i2 = (left + 1) % frame_count;
    let i3 = (left + 2) % frame_count;

    let y0 = buffer.read(channel, i0);
    let y1 = buffer.read(channel, i1);
    let y2 = buffer.read(channel, i2);
    let y3 = buffer.read(channel, i3);

    let c0 = y1;
    let c1 = 0.5 * (y2 - y0);
    let c2 = y0 - 2.5 * y1 + 2.0 * y2 - 0.5 * y3;
    let c3 = 0.5 * (y3 - y0) + 1.5 * (y1 - y2);

    ((c3 * frac + c2) * frac + c1) * frac + c0
}

#[derive(Clone, Deserr, JsonSchema, Connect, ChannelCount, SignalParams)]
#[serde(rename_all = "camelCase")]
#[deserr(rename_all = camelCase, deny_unknown_fields)]
struct DelayWriteParams {
    buffer: Buffer,
    input: PolySignal,
}

#[derive(Outputs, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct DelayWriteOutputs {
    #[output("output", "input signal after writing to the buffer", default)]
    sample: PolyOutput,
    #[output("sync", "current write frame index for syncing with delayRead")]
    sync: PolyOutput,
}

#[derive(Default)]
struct DelayWriteState {
    write_frame: usize,
}

/// Writes a signal continuously into a buffer to act as a delay line.
#[module(name = "$delayWrite", args(buffer, input))]
pub struct DelayWrite {
    outputs: DelayWriteOutputs,
    state: DelayWriteState,
    params: DelayWriteParams,
}

impl DelayWrite {
    fn update(&mut self, _sample_rate: f32) {
        let channels = self.channel_count();
        let buffer_channels = self.params.buffer.channel_count();
        let frame_count = self.params.buffer.frame_count();

        if frame_count == 0 {
            for channel in 0..channels {
                self.outputs
                    .sample
                    .set(channel, self.params.input.get_value(channel));
            }
            self.outputs.sync.set(0, 0.0);
            return;
        }

        let write_frame = self.state.write_frame % frame_count;

        for channel in 0..channels {
            let input_val = self.params.input.get_value(channel);
            if channel < buffer_channels {
                self.params.buffer.write(channel, write_frame, input_val);
            }
            self.outputs.sample.set(channel, input_val);
        }

        self.outputs.sync.set(0, write_frame as f32);
        self.state.write_frame = self.state.write_frame.wrapping_add(1);
    }
}

message_handlers!(impl DelayWrite {});

fn delay_read_derive_channel_count(params: &DelayReadParams) -> usize {
    params.buffer.channel_count()
}

#[derive(Clone, Deserr, JsonSchema, Connect, ChannelCount, SignalParams)]
#[serde(rename_all = "camelCase")]
#[deserr(rename_all = camelCase, deny_unknown_fields)]
struct DelayReadParams {
    buffer: Buffer,
    /// Delay time in seconds (e.g. 0.5 for 500ms)
    #[signal(default = 0.1, range = (0.0, 5.0))]
    time: MonoSignal,
    /// Sync signal from a delayWrite module
    #[signal(default = 0.0, range = (0.0, 5.0))]
    sync: MonoSignal,
}

#[derive(Outputs, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct DelayReadOutputs {
    #[output("output", "delayed signal", default)]
    sample: PolyOutput,
}

/// Reads a signal from a buffer at a specified delay time relative to a sync position.
#[module(name = "$delayRead", channels_derive = delay_read_derive_channel_count, args(buffer, time, sync))]
pub struct DelayRead {
    outputs: DelayReadOutputs,
    params: DelayReadParams,
}

impl DelayRead {
    fn update(&mut self, sample_rate: f32) {
        let sync_frame = self.params.sync.get_value_f64();
        let delay_time_secs = self.params.time.get_value_f64().max(0.0);
        let delay_frames = delay_time_secs * (sample_rate as f64);
        let read_frame = sync_frame - delay_frames;

        let frame_count = self.params.buffer.frame_count();
        let wrapped_frame = if frame_count > 0 {
            read_frame.rem_euclid(frame_count as f64) as f32
        } else {
            0.0
        };

        let channels = self.channel_count();
        for channel in 0..channels {
            let value = read_interpolated_wrapped(&self.params.buffer, channel, wrapped_frame);
            self.outputs.sample.set(channel, value);
        }
    }
}

message_handlers!(impl DelayRead {});
