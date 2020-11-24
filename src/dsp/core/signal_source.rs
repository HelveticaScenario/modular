use crate::types::{Sampleable, SampleableConstructor};
use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::HashMap, sync::Mutex};

const NAME: &str = "SignalSource";

#[derive(Serialize, Deserialize, Debug)]
struct SignalSourceParams {

}

struct SignalSource {
    id: String,
    current_sample: Mutex<f32>,
    next_sample: Mutex<f32>,
    params: SignalSourceParams
}

impl Sampleable for SignalSource {
    fn tick(&self) -> () {
        *self.current_sample.try_lock().unwrap() = *self.next_sample.try_lock().unwrap();
    }

    fn update(&self, _patch: &std::collections::HashMap<String, Box<dyn Sampleable>>) -> () {
        // no-op
    }

    fn get_sample(&self, port: &String) -> anyhow::Result<f32> {
        if port != "output" {
            return Err(anyhow!("Signal Source with id {} has no port {}", self.id, port))
        }
        Ok(*self.current_sample.try_lock().unwrap())
    }
}

fn signal_source_constructor(id: &String, params: Value) -> Result<Box<dyn Sampleable>> {
    let signal_source_params = serde_json::from_value(params)?;
    Ok(Box::new(SignalSource {
        id: id.clone(),
        current_sample: Mutex::new(0.0),
        next_sample: Mutex::new(0.0),
        params: signal_source_params,
    }))
}

pub fn install_constructor(map: &mut HashMap<String, SampleableConstructor>) {
    map.insert(NAME.into(), Box::new(signal_source_constructor));
}
