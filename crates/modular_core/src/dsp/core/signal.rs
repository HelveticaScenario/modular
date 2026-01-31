use napi::Result;
use schemars::JsonSchema;
use serde::Deserialize;

use crate::poly::{PolyOutput, PolySignal};

#[derive(Deserialize, Default, JsonSchema, ChannelCount)]
#[serde(default)]
struct SignalParams {
    /// signal input (polyphonic)
    source: PolySignal,
}

impl crate::types::Connect for SignalParams {
    fn connect(&mut self, patch: &crate::Patch) {
        println!("Connecting SignalParams {:?}", self.source);
        self.source.connect(patch);
    }
}

#[derive(Outputs, JsonSchema)]
struct SignalOutputs {
    #[output("output", "signal output", default)]
    sample: PolyOutput,
}

#[derive(Default, Module)]
#[module("signal", "a polyphonic signal passthrough")]
#[args(source)]
pub struct Signal {
    outputs: SignalOutputs,
    params: SignalParams,
}

impl Signal {
    fn update(&mut self, _sample_rate: f32) -> () {
        let channels = self.channel_count() as u8;
        self.outputs.sample.set_channels(channels);
        for i in 0..channels as usize {
            let val = self.params.source.get_value(i);
            self.outputs.sample.set(i, val);
        }
        
        // Debug: log ROOT_INPUT's first channel value occasionally
        static LOG_COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let count = LOG_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if count % 44100 == 0 && channels > 0 {
            let ch0 = self.outputs.sample.get(0);
            println!("[Signal] update: channels={}, ch0={}", channels, ch0);
        }
    }
}

message_handlers!(impl Signal {});
