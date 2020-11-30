use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

pub trait Sampleable: Send {
    fn tick(&self) -> ();
    fn update(&self, patch: &HashMap<String, Box<dyn Sampleable>>, sample_rate: f32) -> ();
    fn get_sample(&self, port: &String) -> Result<f32>;
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub module_type: String,
    pub params: Value,
}

pub type Patch = HashMap<String, Box<dyn Sampleable>>;

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "param_type")]
pub enum Param {
    Value { value: f32 },
    Note { value: u8 },
    Cable { module: String, port: String },
}

impl Param {
    pub fn get_value(&self, patch: &Patch) -> f32 {
        self.get_value_or(patch, 0.0)
    }
    pub fn get_value_or(&self, patch: &Patch, default: f32) -> f32 {
        match self {
            Param::Value { value } => *value,
            Param::Note { value } => {
                (*value as f32 - 21.0) / 12.0
            }
            Param::Cable { module, port } => {
                if let Some(m) = patch.get(module) {
                    m.get_sample(port).unwrap_or(default)
                } else {
                    default
                }
            }
        }
    }
}

pub type SampleableConstructor = Box<dyn Fn(&String, Value) -> Result<Box<dyn Sampleable>>>;
