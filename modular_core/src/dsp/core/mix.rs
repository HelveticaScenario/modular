use anyhow::{Result, anyhow};

use crate::types::{ChannelBuffer, InternalParam, NUM_CHANNELS};

#[derive(Default, Params)]
struct MixParams {
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
#[module("mix", "A 4 channel mixer")]
pub struct Mix {
    #[output("output", "signal output", default)]
    sample: ChannelBuffer,
    params: MixParams,
}

impl Mix {
    fn update(&mut self, _sample_rate: f32) -> () {
        let mut buffers = [ChannelBuffer::default(); 4];
        let mut count: usize = 0;

        if self.params.input1 != InternalParam::Disconnected {
            self.params.input1.get_value(&mut buffers[count]);
            count += 1;
        }
        if self.params.input2 != InternalParam::Disconnected {
            self.params.input2.get_value(&mut buffers[count]);
            count += 1;
        }
        if self.params.input3 != InternalParam::Disconnected {
            self.params.input3.get_value(&mut buffers[count]);
            count += 1;
        }
        if self.params.input4 != InternalParam::Disconnected {
            self.params.input4.get_value(&mut buffers[count]);
            count += 1;
        }

        if count == 0 {
            self.sample.fill(0.0);
            return;
        }

        let inv = 1.0 / (count as f32);
        for i in 0..NUM_CHANNELS {
            let mut sum = 0.0;
            for b in 0..count {
                sum += buffers[b][i];
            }
            self.sample[i] = sum * inv;
        }
    }
}
