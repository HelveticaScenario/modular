use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

lazy_static! {
    pub static ref ROOT_ID: String = "ROOT".into();
    pub static ref ROOT_OUTPUT_PORT: String = "output".into();
}

pub trait Sampleable: Send {
    fn tick(&self) -> ();
    fn update(&self, patch_map: &PatchMap, sample_rate: f32) -> ();
    fn get_sample(&self, port: &String) -> Result<f32>;
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub module_type: String,
    pub params: Value,
}

pub type PatchMap = HashMap<String, Box<dyn Sampleable>>;

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "param_type")]
pub enum Param {
    Value { value: f32 },
    Note { value: u8 },
    Cable { module: String, port: String },
}

impl Param {
    pub fn get_value(&self, patch_map: &PatchMap) -> f32 {
        self.get_value_or(patch_map, 0.0)
    }
    pub fn get_value_or(&self, patch_map: &PatchMap, default: f32) -> f32 {
        match self {
            Param::Value { value } => *value,
            Param::Note { value } => (*value as f32 - 21.0) / 12.0,
            Param::Cable { module, port } => {
                if let Some(m) = patch_map.get(module) {
                    m.get_sample(port).unwrap_or(default)
                } else {
                    default
                }
            }
        }
    }
}

pub struct ParamSchema {
    pub name: &'static str,
    pub description: &'static str,
    pub required: bool
}

pub struct OutputSchema {
    pub name: &'static str,
    pub description: &'static str,
}

pub struct ModuleSchema {
    pub name: &'static str,
    pub description: &'static str,
    pub params: &'static [ParamSchema],
    pub outputs: &'static [OutputSchema],
}

pub type SampleableConstructor = Box<dyn Fn(&String, Value) -> Result<Box<dyn Sampleable>>>;
