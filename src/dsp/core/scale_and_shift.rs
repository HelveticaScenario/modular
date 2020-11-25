use std::collections::HashMap;
use std::sync::Mutex;

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::types::{Param, Sampleable, SampleableConstructor};

const NAME: &str = "ScaleAndShift";

#[derive(Serialize, Deserialize, Debug)]
struct ScaleAndShiftParams {
    input: Param,
    scale: Option<Param>,
    shift: Option<Param>,
}

#[derive(Debug)]
struct ScaleAndShiftModule {
    sample: f32,
    params: ScaleAndShiftParams,
}

impl ScaleAndShiftModule {
    fn update(&mut self, patch: &HashMap<String, Box<dyn Sampleable>>) -> () {
        let input = self.params.input.get_value(patch);
        let scale = if let Some(ref scale) = self.params.scale {
            scale.get_value(patch)
        } else {
            5.0
        };
        let shift =  if let Some(ref shift) = self.params.shift {
            shift.get_value(patch)
        } else {
            0.0
        };
        self.sample = input * (scale / 5.0) + shift
    }
}

#[derive(Debug)]
struct ScaleAndShift {
    id: String,
    sample: Mutex<f32>,
    module: Mutex<ScaleAndShiftModule>,
}

impl Sampleable for ScaleAndShift {
    fn tick(&self) -> () {
        *self.sample.try_lock().unwrap() = self.module.try_lock().unwrap().sample;
    }

    fn update(&self, patch: &HashMap<String, Box<dyn Sampleable>>, _sample_rate: f32) -> () {
        self.module.try_lock().unwrap().update(patch);
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
    let sine_params = serde_json::from_value(params)?;
    Ok(Box::new(ScaleAndShift {
        id: id.clone(),
        sample: Mutex::new(0.0),
        module: Mutex::new(ScaleAndShiftModule {
            params: sine_params,
            sample: 0.0,
        }),
    }))
}

pub fn install_constructor(map: &mut HashMap<String, SampleableConstructor>) {
    map.insert(NAME.into(), Box::new(constructor));
}
