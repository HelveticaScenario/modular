use crate::types::InternalParam;
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
    sample: f32,
    params: SignalParams,
}

impl Signal {
    fn update(&mut self, _sample_rate: f32) -> () {
        self.sample = self.params.source.get_value();
    }
}
