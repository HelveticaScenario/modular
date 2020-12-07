use std::collections::HashMap;
use std::sync::Mutex;

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::types::{ModuleSchema, ModuleState, Param, PatchMap, PortSchema, Sampleable, SampleableConstructor};

const NAME: &str = "scale-and-shift";
const INPUT: &str = "input";
const SCALE: &str = "scale";
const SHIFT: &str = "shift";
const OUTPUT: &str = "output";

#[derive(Serialize, Deserialize, Debug)]
struct ScaleAndShiftParams {
    input: Param,
    scale: Param,
    shift: Param,
}

#[derive(Debug)]
struct ScaleAndShiftModule {
    sample: f32,
    params: ScaleAndShiftParams,
}

impl ScaleAndShiftModule {
    fn update(&mut self, patch_map: &PatchMap) -> () {
        let input = self.params.input.get_value(patch_map);
        let scale = self.params.scale.get_value_or(patch_map, 5.0);
        let shift = self.params.shift.get_value(patch_map);
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
        let mut params_map = HashMap::new();
        let ref params = self.module.lock().unwrap().params;
        params_map.insert(INPUT.to_owned(), params.input.clone());
        params_map.insert(SCALE.to_owned(), params.scale.clone());
        params_map.insert(SHIFT.to_owned(), params.shift.clone());
        ModuleState {
            module_type: NAME.to_owned(),
            id: self.id.clone(),
            params: params_map,
        }
    }

    fn update_param(&self, param_name: &String, new_param: Param) -> Result<()> {
        todo!()
    }
}

pub const SCHEMA: ModuleSchema = ModuleSchema {
    name: NAME,
    description: "attenuate, invert, offset",
    params: &[
        PortSchema {
            name: INPUT,
            description: "signal input",
        },
        PortSchema {
            name: SCALE,
            description: "scale factor",
        },
        PortSchema {
            name: SHIFT,
            description: "shift amount",
        },
    ],
    outputs: &[PortSchema {
        name: OUTPUT,
        description: "signal output",
    }],
};

fn constructor(id: &String, params: Value) -> Result<Box<dyn Sampleable>> {
    let params = serde_json::from_value(params)?;
    Ok(Box::new(ScaleAndShift {
        id: id.clone(),
        sample: Mutex::new(0.0),
        module: Mutex::new(ScaleAndShiftModule {
            params,
            sample: 0.0,
        }),
    }))
}

pub fn install_constructor(map: &mut HashMap<String, SampleableConstructor>) {
    map.insert(NAME.into(), Box::new(constructor));
}
