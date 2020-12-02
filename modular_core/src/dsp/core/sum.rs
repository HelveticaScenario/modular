use std::collections::HashMap;
use std::sync::Mutex;

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::types::{Param, PatchMap, Sampleable, SampleableConstructor};

const NAME: &str = "sum";

#[derive(Serialize, Deserialize, Debug)]
struct SumParams {
    inputs: Option<Vec<Param>>,
}

#[derive(Debug)]
struct SumModule {
    sample: f32,
    params: SumParams,
}

impl SumModule {
    fn update(&mut self, patch_map: &PatchMap) -> () {
        self.sample = if let Some(ref inputs) = self.params.inputs {
            inputs.iter().fold(0.0, |acc, x| acc + x.get_value(patch_map))
        } else {
            0.0
        }
    }
}

#[derive(Debug)]
struct Sum {
    id: String,
    sample: Mutex<f32>,
    module: Mutex<SumModule>,
}

impl Sampleable for Sum {
    fn tick(&self) -> () {
        *self.sample.try_lock().unwrap() = self.module.try_lock().unwrap().sample;
    }

    fn update(&self, patch_map: &PatchMap, _sample_rate: f32) -> () {
        self.module.try_lock().unwrap().update(patch_map);
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

    fn get_state(&self) -> crate::types::ModuleState {
        todo!()
    }
}

fn constructor(id: &String, params: Value) -> Result<Box<dyn Sampleable>> {
    let params = serde_json::from_value(params)?;
    Ok(Box::new(Sum {
        id: id.clone(),
        sample: Mutex::new(0.0),
        module: Mutex::new(SumModule {
            params,
            sample: 0.0,
        }),
    }))
}

pub fn install_constructor(map: &mut HashMap<String, SampleableConstructor>) {
    map.insert(NAME.into(), Box::new(constructor));
}
