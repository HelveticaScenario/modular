use std::collections::HashMap;

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    dsp::{
        consts::{LUT_SINE, LUT_SINE_SIZE, SAMPLE_RATE},
        utils::{clamp, interpolate, semitones_to_ratio},
    },
    types::{Param, Sampleable, SampleableConstructor},
};

const NAME: &str = "SineOscillator";

#[derive(Serialize, Deserialize, Debug)]
struct SineOscillatorParams {
    freq: Param,
}

struct SineOscillator {
    id: String,
    current_sample: f32,
    next_sample: f32,
    phase: f32,
    params: SineOscillatorParams,
}

impl Sampleable for SineOscillator {
    fn tick(&mut self) -> () {
        self.current_sample = self.next_sample;
    }

    fn update(&mut self, patch: &HashMap<String, Box<dyn Sampleable>>) -> () {
        let voltage = clamp(self.params.freq.get_value(patch), -5.0, 5.0);
        let frequency = semitones_to_ratio(voltage * 12.0) * 220.0 / SAMPLE_RATE;
        self.phase += frequency;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }
        self.next_sample = 5.0 * interpolate(LUT_SINE, self.phase, LUT_SINE_SIZE);
    }

    fn get_sample(&self, port: &String) -> Result<f32> {
        if port == "output" {
            return Ok(self.current_sample);
        }
        Err(anyhow!("{} with id {} does not have port {}", NAME, self.id, port))
    }
}

fn sine_oscillator_constructor(id: &String, params: Value) -> Result<Box<dyn Sampleable>> {
    let sine_params = serde_json::from_value(params)?;
    Ok(Box::new(SineOscillator {
        id: id.clone(),
        params: sine_params,
        current_sample: 0.0,
        next_sample: 0.0,
        phase: 0.0,
    }))
}

pub fn install_constructor(map: &mut HashMap<String, SampleableConstructor>) {
    map.insert(NAME.into(), Box::new(sine_oscillator_constructor));
}
