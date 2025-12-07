use anyhow::{anyhow, Result};

use crate::types::InternalParam;

#[derive(Default, Params)]
struct SumParams {
    #[param("in1", "a signal input")]
    input1: InternalParam,
    #[param("in2", "a signal input")]
    input2: InternalParam,
    #[param("in3", "a signal input")]
    input3: InternalParam,
    #[param("in4", "a signal input")]
    input4: InternalParam,
}

#[derive(Default, Module)]
#[module("sum", "A 4 channel signal adder")]
pub struct Sum {
    #[output("output", "signal output", default)]
    sample: f32,
    params: SumParams,
}

impl Sum {
    fn update(&mut self, _sample_rate: f32) -> () {
        let inputs = [
            &self.params.input1,
            &self.params.input2,
            &self.params.input3,
            &self.params.input4,
        ];

        self.sample = inputs.iter().fold(0.0, |acc, x| acc + x.get_value())
    }
}
