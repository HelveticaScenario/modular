use anyhow::Result;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub trait Sampleable {
    fn tick(&mut self) -> ();
    fn update(&mut self, patch: &HashMap<String, Box<dyn Sampleable>>) -> ();
    fn get_sample(&self, port: &String) -> Result<f32>;
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    module_type: String,
    params: Value
}

type Patch = HashMap<String, Box<dyn Sampleable>>;

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "param_type")]
pub enum Param {
    Value { value: f32 },
    Cable { module: String, port: String }
}

impl Param {
    pub fn get_value(&self, patch: &Patch) -> f32 {
        match self {
            Param::Value { value } => *value,
            Param::Cable { module, port } =>  {
                if let Some(m) = patch.get(module) {
                    m.get_sample(port).unwrap_or_default()
                } else {
                    0.0
                }
            }
        }
    }
}

pub type SampleableConstructor = Box<dyn Fn(&String, Value) -> Result<Box<dyn Sampleable>>>;