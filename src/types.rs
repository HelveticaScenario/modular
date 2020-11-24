use anyhow::Result;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub trait Sampleable: Send {
    fn tick(&self) -> ();
    fn update(&self, patch: &HashMap<String, Box<dyn Sampleable>>, sample_rate: f32) -> ();
    fn get_sample(&self, port: &String) -> Result<f32>;
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub module_type: String,
    pub params: Value
}

pub type Patch = HashMap<String, Box<dyn Sampleable>>;

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

