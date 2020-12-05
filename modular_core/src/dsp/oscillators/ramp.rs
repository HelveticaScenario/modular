use std::collections::HashMap;
use std::sync::Mutex;

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    dsp::utils::clamp,
    dsp::utils::wrap,
    types::PatchMap,
    types::{
        ModuleSchema, ModuleState, PortSchema, Param, Sampleable,
        SampleableConstructor,
    },
};

const NAME: &str = "ramp-oscillator";
const OUTPUT: &str = "output";
const FREQ: &str = "freq";
const PHASE: &str = "phase";

#[derive(Serialize, Deserialize, Debug)]
struct RampOscillatorParams {
    freq: Param,
    phase: Param,
}

#[derive(Debug)]
struct RampOscillatorModule {
    sample: f32,
    phase: f32,
    params: RampOscillatorParams,
}

impl RampOscillatorModule {
    fn update(&mut self, patch_map: &PatchMap, sample_rate: f32) -> () {
        if self.params.phase != Param::Disconnected {
            self.sample = wrap(0.0..1.0, self.params.phase.get_value(patch_map));
        } else {
            let voltage = clamp(
                self.params.freq.get_value_or(patch_map, 4.0),
                12.0,
                0.0,
            );
            let frequency = 27.5f32 * 2.0f32.powf(voltage) / sample_rate;
            // let frequency = semitones_to_ratio(voltage * 12.0) * 220.0 / SAMPLE_RATE * 100.0;
            self.phase += frequency;
            if self.phase >= 1.0 {
                self.phase -= 1.0;
            }
            self.sample = 5.0 * self.phase;
        }
    }
}

#[derive(Debug)]
struct RampOscillator {
    id: String,
    sample: Mutex<f32>,
    module: Mutex<RampOscillatorModule>,
}

impl Sampleable for RampOscillator {
    fn tick(&self) -> () {
        *self.sample.try_lock().unwrap() = self.module.try_lock().unwrap().sample;
    }

    fn update(&self, patch_map: &PatchMap, sample_rate: f32) -> () {
        self.module
            .try_lock()
            .unwrap()
            .update(patch_map, sample_rate);
    }

    fn get_sample(&self, port: &String) -> Result<f32> {
        if port == OUTPUT {
            return Ok(*self.sample.try_lock().unwrap());
        }
        Err(anyhow!(
            "{} with id {} does not have port {}",
            NAME,
            self.id,
            port
        ))
    }

    fn get_state(&self) -> crate::types::ModuleState {
        let mut param_map = HashMap::new();
        let ref params = self.module.lock().unwrap().params;
        param_map.insert(FREQ.to_owned(), params.freq.clone());
        param_map.insert(PHASE.to_owned(), params.phase.clone());
        ModuleState {
            module_type: NAME.to_owned(),
            id: self.id.clone(),
            params: param_map,
        }
    }
}

pub const SCHEMA: ModuleSchema = ModuleSchema {
    name: NAME,
    description: "A ramp oscillator",
    params: &[
        PortSchema {
            name: FREQ,
            description: "frequency in v/oct",
        },
        PortSchema {
            name: PHASE,
            description: "the phase of the oscillator, overrides freq if present",
        },
    ],
    outputs: &[PortSchema {
        name: OUTPUT,
        description: "signal output",
    }],
};

fn constructor(id: &String, params: Value) -> Result<Box<dyn Sampleable>> {
    let params = serde_json::from_value(params)?;
    Ok(Box::new(RampOscillator {
        id: id.clone(),
        sample: Mutex::new(0.0),
        module: Mutex::new(RampOscillatorModule {
            params,
            sample: 0.0,
            phase: 0.0,
        }),
    }))
}

pub fn install_constructor(map: &mut HashMap<String, SampleableConstructor>) {
    map.insert(NAME.into(), Box::new(constructor));
}
