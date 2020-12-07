use crate::types::{
    ModuleSchema, ModuleState, Param, PatchMap, PortSchema, Sampleable, SampleableConstructor,
};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::HashMap, sync::Mutex};

const NAME: &str = "signal";
const SOURCE: &str = "source";
const OUTPUT: &str = "output";

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(default)]
struct SignalParams {
    source: Param,
}
struct SignalModule {
    sample: f32,
    params: SignalParams,
}

impl SignalModule {
    fn update(&mut self, patch_map: &PatchMap) -> () {
        self.sample = self.params.source.get_value(patch_map);
    }
}

struct Signal {
    id: String,
    sample: Mutex<f32>,
    module: Mutex<SignalModule>,
}

impl Sampleable for Signal {
    fn tick(&self) -> () {
        *self.sample.try_lock().unwrap() = self.module.try_lock().unwrap().sample;
    }

    fn update(&self, patch_map: &PatchMap, _sample_rate: f32) -> () {
        self.module.try_lock().unwrap().update(patch_map);
    }

    fn get_sample(&self, port: &String) -> Result<f32> {
        if port != OUTPUT {
            return Err(anyhow!(
                "Signal Destination with id {} has no port {}",
                self.id,
                port
            ));
        }
        Ok(*self.sample.try_lock().unwrap())
    }

    fn get_state(&self) -> crate::types::ModuleState {
        let mut params_map = HashMap::new();
        let ref params = self.module.lock().unwrap().params;
        params_map.insert(SOURCE.to_owned(), params.source.clone());

        ModuleState {
            module_type: NAME.to_owned(),
            id: self.id.clone(),
            params: params_map,
        }
    }

    fn update_param(&self, param_name: &String, new_param: Param) -> Result<()> {
        match param_name.as_str() {
            SOURCE => {
                self.module.lock().unwrap().params.source = new_param;
                Ok(())
            }
            _ => Err(anyhow!("{} is not a valid param name for {}", param_name, NAME)),
        }
    }
}

pub const SCHEMA: ModuleSchema = ModuleSchema {
    name: NAME,
    description: "a signal",
    params: &[PortSchema {
        name: SOURCE,
        description: "source",
    }],
    outputs: &[PortSchema {
        name: OUTPUT,
        description: "signal output",
    }],
};

fn constructor(id: &String, params: Value) -> Result<Box<dyn Sampleable>> {
    let params = serde_json::from_value(params)?;
    Ok(Box::new(Signal {
        id: id.clone(),
        sample: Mutex::new(0.0),
        module: Mutex::new(SignalModule {
            sample: 0.0,
            params,
        }),
    }))
}

pub fn install_constructor(map: &mut HashMap<String, SampleableConstructor>) {
    map.insert(NAME.into(), Box::new(constructor));
}
