use deserr::Deserr;
use schemars::JsonSchema;

use crate::{
    Buffer, MonoSignal,
    poly::{PolyOutput, PolySignal},
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

    let right = (left + 1).min(frame_count - 1);
    let a = buffer.read(channel, left);
    let b = buffer.read(channel, right);
    a + (b - a) * frac
}

fn write_frame_index(frame: f32, frame_count: usize) -> Option<usize> {
    if !frame.is_finite() || frame_count == 0 {
        return None;
    }

    let index = frame.floor();
    if index < 0.0 || index >= frame_count as f32 {
        return None;
    }

    Some(index as usize)
}

#[derive(Clone, Deserr, JsonSchema, Connect, ChannelCount, SignalParams)]
#[serde(rename_all = "camelCase")]
#[deserr(rename_all = camelCase, deny_unknown_fields)]
struct BufReadParams {
    buffer: Buffer,
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
        let frame = self.params.frame.get_value();
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
    frame: MonoSignal,
    input: PolySignal,
}

#[derive(Outputs, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct BufWriteOutputs {
    #[output("output", "input signal after writing to the buffer", default)]
    sample: PolyOutput,
}

/// Writes a signal into a buffer at a sample frame position and passes the signal through.
#[module(name = "$bufWrite", args(buffer, frame, input))]
pub struct BufWrite {
    outputs: BufWriteOutputs,
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

        let Some(frame) = write_frame_index(self.params.frame.get_value(), self.params.buffer.frame_count())
        else {
            return;
        };

        let buffer_channels = self.params.buffer.channel_count();
        for channel in 0..buffer_channels {
            self.params
                .buffer
                .write(channel, frame, self.params.input.get_value(channel));
        }
    }
}

message_handlers!(impl BufWrite {});

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use crate::{
        BufferData, Patch,
        types::{BufferSpec, Connect, OutputStruct, Signal},
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
            frame: mono(0.5),
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
            frame: mono(1.0),
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
            frame: mono(2.0),
            input: PolySignal::mono(Signal::Volts(0.75)),
        });

        module.update(48_000.0);

        assert_eq!(module.outputs.sample.channels(), 1);
        assert!((module.params.buffer.read(0, 2) - 0.75).abs() < 1e-6);
        assert!((module.params.buffer.read(1, 2) - 0.75).abs() < 1e-6);
    }
}
