use std::collections::HashMap;
use std::sync::Mutex;

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::types::{
    ModuleSchema, ModuleState, Param, PatchMap, PortSchema, Sampleable, SampleableConstructor,
};

const NAME: &str = "sum";
const INPUT_1: &str = "input-1";
const INPUT_2: &str = "input-2";
const INPUT_3: &str = "input-3";
const INPUT_4: &str = "input-4";
const OUTPUT: &str = "output";

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(default)]
struct SumParams {
    input1: Param,
    input2: Param,
    input3: Param,
    input4: Param,
}

#[derive(Debug)]
struct SumModule {
    sample: f32,
    params: SumParams,
}

impl SumModule {
    fn update(&mut self, patch_map: &PatchMap) -> () {
        let inputs = [
            &self.params.input1,
            &self.params.input2,
            &self.params.input3,
            &self.params.input4,
        ];

        self.sample = inputs
            .iter()
            .fold(0.0, |acc, x| acc + x.get_value(patch_map))
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
        param_map.insert(INPUT_1.to_owned(), params.input1.clone());
        param_map.insert(INPUT_2.to_owned(), params.input2.clone());
        param_map.insert(INPUT_3.to_owned(), params.input3.clone());
        param_map.insert(INPUT_4.to_owned(), params.input4.clone());
        ModuleState {
            module_type: NAME.to_owned(),
            id: self.id.clone(),
            params: param_map,
        }
    }

    fn update_param(&self, param_name: &String, new_param: Param) -> Result<()> {
        match param_name.as_str() {
            INPUT_1 => {
                self.module.lock().unwrap().params.input1 = new_param;
                Ok(())
            }
            INPUT_2 => {
                self.module.lock().unwrap().params.input2 = new_param;
                Ok(())
            }
            INPUT_3 => {
                self.module.lock().unwrap().params.input3 = new_param;
                Ok(())
            }
            INPUT_4 => {
                self.module.lock().unwrap().params.input4 = new_param;
                Ok(())
            }
            _ => Err(anyhow!("{} is not a valid param name for {}", param_name, NAME)),
        }
    }
}

pub const SCHEMA: ModuleSchema = ModuleSchema {
    name: NAME,
    description: "A 4 channel signal adder",
    params: &[
        PortSchema {
            name: INPUT_1,
            description: "a signal input",
        },
        PortSchema {
            name: INPUT_2,
            description: "a signal input",
        },
        PortSchema {
            name: INPUT_3,
            description: "a signal input",
        },
        PortSchema {
            name: INPUT_4,
            description: "a signal input",
        },
    ],
    outputs: &[PortSchema {
        name: OUTPUT,
        description: "signal output",
    }],
};

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
