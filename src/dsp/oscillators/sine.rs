use std::collections::HashMap;
use std::sync::Mutex;

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{dsp::{
        consts::{LUT_SINE, LUT_SINE_SIZE},
        utils::{clamp, interpolate},
    }, types::{Param, Sampleable, SampleableConstructor}, dsp::utils::wrap};

const NAME: &str = "SineOscillator";

#[derive(Serialize, Deserialize, Debug)]
struct SineOscillatorParams {
    freq: Option<Param>,
    phase: Option<Param>,
}

#[derive(Debug)]
struct SineOscillatorModule {
    sample: f32,
    phase: f32,
    params: SineOscillatorParams,
}

impl SineOscillatorModule {
    fn update(&mut self, patch: &HashMap<String, Box<dyn Sampleable>>, sample_rate: f32) -> () {
        if let Some(ref phase) = self.params.phase {
            self.sample = wrap(0.0..1.0, phase.get_value(patch));
        } else {
            let voltage = clamp(
                if let Some(ref freq) = self.params.freq {
                    freq.get_value_or(patch, 4.0)
                } else {
                    4.0
                },
                12.0,
                0.0,
            );
            let frequency = 27.5f32 * 2.0f32.powf(voltage) / sample_rate;
            // let frequency = semitones_to_ratio(voltage * 12.0) * 220.0 / SAMPLE_RATE * 100.0;
            self.phase += frequency;
            if self.phase >= 1.0 {
                self.phase -= 1.0;
            }
            self.sample = 5.0 * interpolate(LUT_SINE, self.phase, LUT_SINE_SIZE);
        }
    }
}

#[derive(Debug)]
struct SineOscillator {
    id: String,
    sample: Mutex<f32>,
    module: Mutex<SineOscillatorModule>,
}

impl Sampleable for SineOscillator {
    fn tick(&self) -> () {
        *self.sample.try_lock().unwrap() = self.module.try_lock().unwrap().sample;
    }

    fn update(&self, patch: &HashMap<String, Box<dyn Sampleable>>, sample_rate: f32) -> () {
        self.module.try_lock().unwrap().update(patch, sample_rate);
    }

    fn get_sample(&self, port: &String) -> Result<f32> {
        if port == "output" {
            return Ok(*self.sample.try_lock().unwrap());
        }
        Err(anyhow!(
            "{} with id {} does not have port {}",
            NAME,
            self.id,
            port
        ))
    }
}

fn constructor(id: &String, params: Value) -> Result<Box<dyn Sampleable>> {
    let params = serde_json::from_value(params)?;
    Ok(Box::new(SineOscillator {
        id: id.clone(),
        sample: Mutex::new(0.0),
        module: Mutex::new(SineOscillatorModule {
            params,
            sample: 0.0,
            phase: 0.0,
        }),
    }))
}

pub fn install_constructor(map: &mut HashMap<String, SampleableConstructor>) {
    map.insert(NAME.into(), Box::new(constructor));
}
