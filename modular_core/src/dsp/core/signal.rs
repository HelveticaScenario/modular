use crate::types::{ChannelBuffer, InternalParam};
use anyhow::{anyhow, Result};

#[derive(Default, Params)]
struct SignalParams {
    #[param("source", "signal input")]
    source: InternalParam,
}

#[derive(Default, Module)]
#[module("signal", "a signal")]
pub struct Signal {
    #[output("output", "signal output", default)]
    sample: ChannelBuffer,
    params: SignalParams,
}

impl Signal {
    fn update(&mut self, _sample_rate: f32) -> () {
        self.params.source.get_value(&mut self.sample);
    }
}
