//! Split module - extracts individual channels from a polyphonic signal.
//!
//! Takes a polyphonic input and outputs each channel as a separate mono signal.

use napi::Result;
use schemars::JsonSchema;
use serde::Deserialize;

use crate::types::Signal;

#[derive(Deserialize, Default, JsonSchema, Connect)]
#[serde(default)]
struct SplitParams {
    /// Polyphonic input to split
    input: Signal,
}

#[derive(Outputs, JsonSchema)]
struct SplitOutputs {
    /// Channel 0 output
    #[output("ch0", "channel 0 output", default)]
    ch0: f32,
    /// Channel 1 output
    #[output("ch1", "channel 1 output")]
    ch1: f32,
    /// Channel 2 output
    #[output("ch2", "channel 2 output")]
    ch2: f32,
    /// Channel 3 output
    #[output("ch3", "channel 3 output")]
    ch3: f32,
    /// Channel 4 output
    #[output("ch4", "channel 4 output")]
    ch4: f32,
    /// Channel 5 output
    #[output("ch5", "channel 5 output")]
    ch5: f32,
    /// Channel 6 output
    #[output("ch6", "channel 6 output")]
    ch6: f32,
    /// Channel 7 output
    #[output("ch7", "channel 7 output")]
    ch7: f32,
    /// Channel 8 output
    #[output("ch8", "channel 8 output")]
    ch8: f32,
    /// Channel 9 output
    #[output("ch9", "channel 9 output")]
    ch9: f32,
    /// Channel 10 output
    #[output("ch10", "channel 10 output")]
    ch10: f32,
    /// Channel 11 output
    #[output("ch11", "channel 11 output")]
    ch11: f32,
    /// Channel 12 output
    #[output("ch12", "channel 12 output")]
    ch12: f32,
    /// Channel 13 output
    #[output("ch13", "channel 13 output")]
    ch13: f32,
    /// Channel 14 output
    #[output("ch14", "channel 14 output")]
    ch14: f32,
    /// Channel 15 output
    #[output("ch15", "channel 15 output")]
    ch15: f32,
    /// Number of active channels in the input
    #[output("channels", "number of active input channels")]
    channels: f32,
}

#[derive(Default, Module)]
#[module("split", "Split a polyphonic signal into individual channels")]
#[args(input)]
pub struct Split {
    outputs: SplitOutputs,
    params: SplitParams,
}

impl Split {
    fn update(&mut self, _sample_rate: f32) {
        let poly = self.params.input.get_poly_signal();
        let channels = poly.channels() as usize;

        // Output each channel (0.0 for channels beyond input count)
        self.outputs.ch0 = if channels > 0 { poly.get(0) as f32 } else { 0.0 };
        self.outputs.ch1 = if channels > 1 { poly.get(1) as f32 } else { 0.0 };
        self.outputs.ch2 = if channels > 2 { poly.get(2) as f32 } else { 0.0 };
        self.outputs.ch3 = if channels > 3 { poly.get(3) as f32 } else { 0.0 };
        self.outputs.ch4 = if channels > 4 { poly.get(4) as f32 } else { 0.0 };
        self.outputs.ch5 = if channels > 5 { poly.get(5) as f32 } else { 0.0 };
        self.outputs.ch6 = if channels > 6 { poly.get(6) as f32 } else { 0.0 };
        self.outputs.ch7 = if channels > 7 { poly.get(7) as f32 } else { 0.0 };
        self.outputs.ch8 = if channels > 8 { poly.get(8) as f32 } else { 0.0 };
        self.outputs.ch9 = if channels > 9 { poly.get(9) as f32 } else { 0.0 };
        self.outputs.ch10 = if channels > 10 { poly.get(10) as f32 } else { 0.0 };
        self.outputs.ch11 = if channels > 11 { poly.get(11) as f32 } else { 0.0 };
        self.outputs.ch12 = if channels > 12 { poly.get(12) as f32 } else { 0.0 };
        self.outputs.ch13 = if channels > 13 { poly.get(13) as f32 } else { 0.0 };
        self.outputs.ch14 = if channels > 14 { poly.get(14) as f32 } else { 0.0 };
        self.outputs.ch15 = if channels > 15 { poly.get(15) as f32 } else { 0.0 };

        self.outputs.channels = channels as f32;
    }
}

message_handlers!(impl Split {});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::poly::PolySignal;

    #[test]
    fn test_split_mono() {
        let mut split = Split {
            params: SplitParams {
                input: Signal::Volts(PolySignal::mono(2.5)),
            },
            ..Default::default()
        };
        split.update(48000.0);
        assert_eq!(split.outputs.ch0, 2.5);
        assert_eq!(split.outputs.ch1, 0.0);
        assert_eq!(split.outputs.channels, 1.0);
    }

    #[test]
    fn test_split_poly() {
        let mut split = Split {
            params: SplitParams {
                input: Signal::Volts(PolySignal::poly(&[1.0, 2.0, 3.0, 4.0])),
            },
            ..Default::default()
        };
        split.update(48000.0);
        assert_eq!(split.outputs.ch0, 1.0);
        assert_eq!(split.outputs.ch1, 2.0);
        assert_eq!(split.outputs.ch2, 3.0);
        assert_eq!(split.outputs.ch3, 4.0);
        assert_eq!(split.outputs.ch4, 0.0);
        assert_eq!(split.outputs.channels, 4.0);
    }

    #[test]
    fn test_split_disconnected() {
        let mut split = Split::default();
        split.update(48000.0);
        assert_eq!(split.outputs.ch0, 0.0);
        assert_eq!(split.outputs.channels, 0.0);
    }
}
