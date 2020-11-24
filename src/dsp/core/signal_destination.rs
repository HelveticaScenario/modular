use crate::types::{Param, Sampleable, SampleableConstructor};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::HashMap, sync::Mutex};

const NAME: &str = "SignalDestination";

#[derive(Serialize, Deserialize, Debug)]
struct SignalDestinationParams {
    source: Param
}
struct SignalDestination {
    id: String,
    current_sample: Mutex<f32>,
    next_sample: Mutex<f32>,
    params: SignalDestinationParams,
}

impl Sampleable for SignalDestination {
    fn tick(&self) -> () {
        *self.current_sample.try_lock().unwrap() = *self.next_sample.try_lock().unwrap();
    }

    fn update(&self, patch: &std::collections::HashMap<String, Box<dyn Sampleable>>) -> () {
        *self.next_sample.try_lock().unwrap() = self.params.source.get_value(patch)
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

fn signal_destination_constructor(id: &String, params: Value) -> Result<Box<dyn Sampleable>> {
    let signal_destination_params = serde_json::from_value(params)?;
    Ok(Box::new(SignalDestination {
        id: id.clone(),
        current_sample: Mutex::new(0.0),
        next_sample: Mutex::new(0.0),
        params: signal_destination_params,
    }))
}

pub fn install_constructor(map: &mut HashMap<String, SampleableConstructor>) {
    map.insert(NAME.into(), Box::new(signal_destination_constructor));
}
