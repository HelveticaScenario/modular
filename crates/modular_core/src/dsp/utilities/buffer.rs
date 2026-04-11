use deserr::Deserr;
use schemars::JsonSchema;

use crate::{
    poly::{PolyOutput, PolySignal},
    Buffer, MonoSignal,
};

fn buf_read_derive_channel_count(params: &BufReadParams) -> usize {
    params.buffer.channel_count()
}

fn read_interpolated(buffer: &Buffer, channel: usize, frame: f32) -> f32 {
    if !frame.is_finite() {
        return 0.0;
    }

    let frame_count = buffer.frame_count();
    if frame_count == 0 {
        return 0.0;
    }

    let max_frame = (frame_count - 1) as f32;
    if frame < 0.0 || frame > max_frame {
        return 0.0;
    }

    let left = frame.floor() as usize;
    let frac = frame - left as f32;
    if frac <= f32::EPSILON {
        return buffer.read(channel, left);
    }

    let i0 = left.saturating_sub(1);
    let i1 = left;
    let i2 = (left + 1).min(frame_count - 1);
    let i3 = (left + 2).min(frame_count - 1);

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

fn write_frame_index(frame: f64, frame_count: usize) -> Option<usize> {
    if !frame.is_finite() || frame_count == 0 {
        return None;
    }

    let index = frame.floor();
    if index < 0.0 || index >= frame_count as f64 {
        return None;
    }

    Some(index as usize)
}

#[derive(Clone, Deserr, JsonSchema, Connect, ChannelCount, SignalParams)]
#[serde(rename_all = "camelCase")]
#[deserr(rename_all = camelCase, deny_unknown_fields)]
struct BufReadParams {
    buffer: Buffer,
    /// read position (0 to 5V scales to 0 to buffer length)
    #[signal(default = 0.0, range = (0.0, 5.0))]
    frame: MonoSignal,
}

#[derive(Outputs, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct BufReadOutputs {
    #[output("output", "buffer sample output", default)]
    sample: PolyOutput,
}

/// Reads a sample frame from a buffer and outputs its channels as a poly signal.
#[module(name = "$bufRead", channels_derive = buf_read_derive_channel_count, args(buffer, frame))]
pub struct BufRead {
    outputs: BufReadOutputs,
    params: BufReadParams,
}

impl BufRead {
    fn update(&mut self, _sample_rate: f32) {
        let frame_volts = self.params.frame.get_value(); //.rem_euclid(5.0);
        let frame_count = self.params.buffer.frame_count();
        // let frame = (frame_volts / 5.0) * frame_count as f32;
        let frame = frame_volts;
        let channels = self.channel_count();

        for channel in 0..channels {
            let value = read_interpolated(&self.params.buffer, channel, frame);
            self.outputs.sample.set(channel, value);
        }
    }
}

message_handlers!(impl BufRead {});

#[derive(Clone, Deserr, JsonSchema, Connect, ChannelCount, SignalParams)]
#[serde(rename_all = "camelCase")]
#[deserr(rename_all = camelCase, deny_unknown_fields)]
struct BufWriteParams {
    buffer: Buffer,
    /// write position (0 to 5V scales to 0 to buffer length)
    #[signal(default = 0.0, range = (0.0, 5.0))]
    frame: MonoSignal,
    input: PolySignal,
}

#[derive(Outputs, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct BufWriteOutputs {
    #[output("output", "input signal after writing to the buffer", default)]
    sample: PolyOutput,
}

#[derive(Default)]
struct BufWriteState {
    last_frame: Option<usize>,
}

/// Writes a signal into a buffer at a sample frame position and passes the signal through.
#[module(name = "$bufWrite", args(buffer, frame, input))]
pub struct BufWrite {
    outputs: BufWriteOutputs,
    state: BufWriteState,
    params: BufWriteParams,
}

impl BufWrite {
    fn update(&mut self, _sample_rate: f32) {
        let output_channels = self.channel_count();
        for channel in 0..output_channels {
            self.outputs
                .sample
                .set(channel, self.params.input.get_value(channel));
        }

        let scaled_frame = (self.params.frame.get_value_f64()); //.rem_euclid(5.0);

        let frame_count = self.params.buffer.frame_count();
        // let scaled_frame = (frame_volts / 5.0) * frame_count as f64;

        let Some(frame) = write_frame_index(scaled_frame, frame_count) else {
            self.state.last_frame = None;
            // println!("cant");
            return;
        };

        // println!("{frame_volts} {frame}");

        let buffer_channels = self.params.buffer.channel_count();

        let last = self.state.last_frame.unwrap_or(frame);

        // if (frame < last) {
        //     println!("")
        // }

        // Determine fill range: fill gaps up to 64 frames (handles high frequency writing).
        // For larger jumps (wrap-around), we just write the single frame.
        let (start, end) = if frame > last && frame - last < 64 {
            // println!("big jump 1 {:?}", (last + 1, frame));
            (last + 1, frame)
        } else if last > frame && last - frame < 64 {
            // println!("big jump 2 {:?}", (frame, last - 1));
            (frame, last - 1)
        } else {
            (frame, frame)
        };

        for f in start..=end {
            for channel in 0..buffer_channels {
                self.params
                    .buffer
                    .write(channel, f, self.params.input.get_value(channel));
            }
        }

        self.state.last_frame = Some(frame);
    }
}

message_handlers!(impl BufWrite {});

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use crate::{
        types::{BufferSpec, Connect, OutputStruct, Signal},
        BufferData, Patch,
    };

    fn mono(value: f32) -> MonoSignal {
        MonoSignal::from_poly(PolySignal::mono(Signal::Volts(value)))
    }

    fn make_connected_buffer(samples: Vec<Vec<f32>>) -> (Patch, Buffer) {
        let channels = samples.len();
        let frame_count = samples.first().map_or(0, Vec::len);
        let spec = BufferSpec::new("test.wav".to_string(), channels, frame_count).unwrap();
        let mut patch = Patch::new();
        patch.buffers.insert(
            spec.path.clone(),
            Arc::new(BufferData::from_samples(samples)),
        );

        let mut buffer = Buffer::new(spec);
        buffer.connect(&patch);
        (patch, buffer)
    }

    fn make_buf_read(params: BufReadParams) -> BufRead {
        let channels = buf_read_derive_channel_count(&params);
        let mut outputs = BufReadOutputs::default();
        outputs.set_all_channels(channels);
        BufRead {
            params,
            outputs,
            _channel_count: channels,
        }
    }

    fn make_buf_write(params: BufWriteParams) -> BufWrite {
        let channels = params.input.channels().max(1);
        let mut outputs = BufWriteOutputs::default();
        outputs.set_all_channels(channels);
        BufWrite {
            params,
            outputs,
            state: BufWriteState::default(),
            _channel_count: channels,
        }
    }

    #[test]
    fn buf_read_channel_count_matches_buffer_channels() {
        let params = BufReadParams {
            buffer: Buffer::new(BufferSpec::new("test.wav".to_string(), 2, 8).unwrap()),
            frame: mono(0.0),
        };

        assert_eq!(buf_read_derive_channel_count(&params), 2);
    }

    #[test]
    fn buf_read_outputs_all_buffer_channels() {
        let (_patch, buffer) = make_connected_buffer(vec![vec![1.0, 3.0], vec![2.0, 4.0]]);
        let mut module = make_buf_read(BufReadParams {
            buffer,
            frame: mono(0.5), // 0.5 index
        });

        module.update(48_000.0);

        assert_eq!(module.outputs.sample.channels(), 2);
        assert!((module.outputs.sample.get(0) - 2.0).abs() < 1e-6);
        assert!((module.outputs.sample.get(1) - 3.0).abs() < 1e-6);
    }

    #[test]
    fn buf_read_returns_zero_out_of_range() {
        let (_patch, buffer) = make_connected_buffer(vec![vec![1.0, 3.0], vec![2.0, 4.0]]);
        let mut module = make_buf_read(BufReadParams {
            buffer,
            frame: mono(9.0),
        });

        module.update(48_000.0);

        assert_eq!(module.outputs.sample.get(0), 0.0);
        assert_eq!(module.outputs.sample.get(1), 0.0);
    }

    #[test]
    fn buf_write_writes_and_passes_through_input() {
        let (_patch, buffer) = make_connected_buffer(vec![vec![0.0; 4], vec![0.0; 4]]);
        let mut module = make_buf_write(BufWriteParams {
            buffer,
            frame: mono(1.0), // index 1.0
            input: PolySignal::poly(&[Signal::Volts(1.25), Signal::Volts(-0.5)]),
        });

        module.update(48_000.0);

        assert_eq!(module.outputs.sample.channels(), 2);
        assert!((module.outputs.sample.get(0) - 1.25).abs() < 1e-6);
        assert!((module.outputs.sample.get(1) - (-0.5)).abs() < 1e-6);
        assert!((module.params.buffer.read(0, 1) - 1.25).abs() < 1e-6);
        assert!((module.params.buffer.read(1, 1) - (-0.5)).abs() < 1e-6);
    }

    #[test]
    fn buf_write_cycles_input_across_buffer_channels() {
        let (_patch, buffer) = make_connected_buffer(vec![vec![0.0; 4], vec![0.0; 4]]);
        let mut module = make_buf_write(BufWriteParams {
            buffer,
            frame: mono(2.0), // index 2.0
            input: PolySignal::mono(Signal::Volts(0.75)),
        });

        module.update(48_000.0);

        assert_eq!(module.outputs.sample.channels(), 1);
        assert!((module.params.buffer.read(0, 2) - 0.75).abs() < 1e-6);
        assert!((module.params.buffer.read(1, 2) - 0.75).abs() < 1e-6);
    }
}
