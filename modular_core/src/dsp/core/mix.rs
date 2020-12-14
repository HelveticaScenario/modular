use anyhow::{anyhow, Result};

use crate::types::InternalParam;

#[derive(Default, Params)]
struct MixParams {
    #[param("input-1", "a signal input")]
    input1: InternalParam,
    #[param("input-2", "a signal input")]
    input2: InternalParam,
    #[param("input-3", "a signal input")]
    input3: InternalParam,
    #[param("input-4", "a signal input")]
    input4: InternalParam,
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
        let inputs = [
            &self.params.input1,
            &self.params.input2,
            &self.params.input3,
            &self.params.input4,
        ];
        let count = inputs
            .iter()
            .filter(|input| ***input != InternalParam::Disconnected)
            .count();

        self.sample = if count > 0 {
            inputs.iter().fold(0.0, |acc, x| acc + x.get_value()) / count as f32
        } else {
            0.0
        }
    }
}
