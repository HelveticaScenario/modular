use deserr::Deserr;
use schemars::JsonSchema;

use crate::{poly::PolyOutput, Buffer, PolySignal};

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

fn delay_read_derive_channel_count(params: &DelayReadParams) -> usize {
    params.buffer.channel_count().max(params.time.channels())
}

#[derive(Clone, Deserr, JsonSchema, Connect, ChannelCount, SignalParams)]
#[serde(rename_all = "camelCase")]
#[deserr(rename_all = camelCase, deny_unknown_fields)]
struct DelayReadParams {
    buffer: Buffer,
    /// Delay time in seconds (e.g. 0.5 for 500ms)
    #[signal(default = 0.1, range = (0.0, 5.0))]
    time: PolySignal,
}

#[derive(Outputs, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct DelayReadOutputs {
    #[output("output", "delayed signal", default)]
    sample: PolyOutput,
}

/// Reads a signal from a buffer at a specified delay time relative to the write position.
#[module(name = "$delayRead", channels_derive = delay_read_derive_channel_count, args(buffer, time))]
pub struct DelayRead {
    outputs: DelayReadOutputs,
    params: DelayReadParams,
}

impl DelayRead {
    fn update(&mut self, sample_rate: f32) {
        // Ensure the $buffer module has processed this frame first.
        // This triggers $buffer.update() which writes the sample and increments write_index.
        self.params.buffer.ensure_source_updated();

        let write_index = self.params.buffer.read_write_index() as f64;
        let frame_count = self.params.buffer.frame_count();
        let channels = self.channel_count();

        for channel in 0..channels {
            let delay_time_secs = (self.params.time.get_value(channel) as f64).max(0.0);
            let delay_frames = delay_time_secs * (sample_rate as f64);
            let read_frame = write_index - delay_frames;

            let wrapped_frame = if frame_count > 0 {
                read_frame.rem_euclid(frame_count as f64) as f32
            } else {
                0.0
            };

            let buf_channels = self.params.buffer.channel_count().max(1);
            let buf_channel = channel % buf_channels;
            let value = read_interpolated_wrapped(&self.params.buffer, buf_channel, wrapped_frame);
            self.outputs.sample.set(channel, value);
        }
    }
}

message_handlers!(impl DelayRead {});
