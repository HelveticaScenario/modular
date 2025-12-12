use anyhow::{anyhow, Result};

use crate::types::{ChannelBuffer, InternalParam, NUM_CHANNELS};

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
    sample: ChannelBuffer,
    params: SumParams,
}

impl Sum {
    fn update(&mut self, _sample_rate: f32) -> () {
        let mut buffers = [ChannelBuffer::default(); 4];
        self.params.input1.get_value(&mut buffers[0]);
        self.params.input2.get_value(&mut buffers[1]);
        self.params.input3.get_value(&mut buffers[2]);
        self.params.input4.get_value(&mut buffers[3]);

        for i in 0..NUM_CHANNELS {
            self.sample[i] = buffers[0][i] + buffers[1][i] + buffers[2][i] + buffers[3][i];
        }
    }
}
