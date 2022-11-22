use anyhow::{anyhow, Result};

use crate::types::InternalParam;

#[derive(Default, Params)]
struct PanParams {
    #[param("input-1", "a signal input")]
    input1: InternalParam,
    #[param("input-2", "a signal input")]
    input2: InternalParam,
    #[param(
        "pan",
        "degree of pan, 0 to 5, where 0 is 100% input-1 and 5 is 100% input-2"
    )]
    pan: InternalParam,
}

#[derive(Default, Module)]
#[module("mix", "A 4 channel mixer")]
pub struct Mix {
    #[output("output", "signal output")]
    sample: f32,
    params: MixParams,
}

impl Mix {
    fn update(&mut self, _sample_rate: f32) -> () {
        self.sample = match (self.params.input1, self.params.input2) {
            (input1, InternalParam::Disconnected) => input1.get_value(),
            (InternalParam::Disconnected, input2) => input2.get_value(),
            (InternalParam::Disconnected, InternalParam::Disconnected) => 0.0,
            (input1, input2) => {
                let pan = self.params.pan.get_value_or(2.5) / 5.0;
                (input1.get_value() * pan) + (input2.get_value() * (1.0 - pan))
            }
        }
    }
}
