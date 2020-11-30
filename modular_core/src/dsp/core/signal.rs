use crate::types::{Param, PatchMap, Sampleable, SampleableConstructor};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::HashMap, sync::Mutex};

const NAME: &str = "Signal";

#[derive(Serialize, Deserialize, Debug)]
struct SignalParams {
    source: Param
}
struct Signal {
    id: String,
    current_sample: Mutex<f32>,
    next_sample: Mutex<f32>,
    params: SignalParams,
}

impl Sampleable for Signal {
    fn tick(&self) -> () {
        *self.current_sample.try_lock().unwrap() = *self.next_sample.try_lock().unwrap();
    }

    fn update(&self, patch_map: &PatchMap, _sample_rate: f32) -> () {
        *self.next_sample.try_lock().unwrap() = self.params.source.get_value(patch_map)
    }

    fn get_sample(&self, port: &String) -> Result<f32> {
        if port != "output" {
            return Err(anyhow!(
                "Signal Destination with id {} has no port {}",
                self.id,
                port
            ));
        }
        Ok(*self.current_sample.try_lock().unwrap())
    }
}

fn constructor(id: &String, params: Value) -> Result<Box<dyn Sampleable>> {
    let params = serde_json::from_value(params)?;
    Ok(Box::new(Signal {
        id: id.clone(),
        current_sample: Mutex::new(0.0),
        next_sample: Mutex::new(0.0),
        params,
    }))
}

pub fn install_constructor(map: &mut HashMap<String, SampleableConstructor>) {
    map.insert(NAME.into(), Box::new(constructor));
}
