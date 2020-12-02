use std::collections::HashMap;
use std::sync::Mutex;

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::types::{ModuleState, Param, PatchMap, Sampleable, SampleableConstructor};

const NAME: &str = "mix";

#[derive(Serialize, Deserialize, Debug)]
struct MixParams {
    inputs: Option<Vec<Param>>,
}

#[derive(Debug)]
struct MixModule {
    sample: f32,
    params: MixParams,
}

impl MixModule {
    fn update(&mut self, patch_map: &PatchMap) -> () {
        self.sample = if let Some(ref inputs) = self.params.inputs {
            inputs
                .iter()
                .fold(0.0, |acc, x| acc + x.get_value(patch_map))
                / inputs.len() as f32
        } else {
            0.0
        }
    }
}

#[derive(Debug)]
struct Mix {
    id: String,
    sample: Mutex<f32>,
    module: Mutex<MixModule>,
}

impl Sampleable for Mix {
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
        let mut params = HashMap::new();

        params.insert(
            "inputs".to_owned(),
            if let Some(ref inputs) = self.module.lock().unwrap().params.inputs {
                Some(inputs.iter().map(|input| input.clone()).collect())
            } else {
                None
            },
        );
        ModuleState {
            module_type: NAME.to_owned(),
            id: self.id.clone(),
            params,
        }
    }
}

fn constructor(id: &String, params: Value) -> Result<Box<dyn Sampleable>> {
    let params = serde_json::from_value(params)?;
    Ok(Box::new(Mix {
        id: id.clone(),
        sample: Mutex::new(0.0),
        module: Mutex::new(MixModule {
            params: params,
            sample: 0.0,
        }),
    }))
}

pub fn install_constructor(map: &mut HashMap<String, SampleableConstructor>) {
    map.insert(NAME.into(), Box::new(constructor));
}
