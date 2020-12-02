use std::collections::HashMap;
use std::sync::Mutex;

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::types::{ModuleState, Param, PatchMap, Sampleable, SampleableConstructor};

const NAME: &str = "scale-and-shift";

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
    fn update(&mut self, patch_map: &PatchMap) -> () {
        let input = self.params.input.get_value(patch_map);
        let scale = if let Some(ref scale) = self.params.scale {
            scale.get_value(patch_map)
        } else {
            5.0
        };
        let shift = if let Some(ref shift) = self.params.shift {
            shift.get_value(patch_map)
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
        let mut params_map = HashMap::new();
        let ref params = self.module.lock().unwrap().params;
        params_map.insert("input".to_owned(), Some(vec![params.input.clone()]));
        params_map.insert(
            "scale".to_owned(),
            if let Some(ref scale) = params.scale {
                Some(vec![scale.clone()])
            } else {
                None
            },
        );
        params_map.insert(
            "shift".to_owned(),
            if let Some(ref shift) = params.shift {
                Some(vec![shift.clone()])
            } else {
                None
            },
        );
        ModuleState {
            module_type: NAME.to_owned(),
            id: self.id.clone(),
            params: params_map,
        }
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
