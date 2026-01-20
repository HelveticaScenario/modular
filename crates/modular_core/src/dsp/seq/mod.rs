use std::collections::HashMap;

use crate::types::{Module, ModuleSchema, ParamsValidator, SampleableConstructor};

pub mod seq;
pub mod seq_legacy;

pub fn install_constructors(map: &mut HashMap<String, SampleableConstructor>) {
    seq::Seq::install_constructor(map);
    seq_legacy::SeqLegacy::install_constructor(map);
}

pub fn install_param_validators(map: &mut HashMap<String, ParamsValidator>) {
    seq::Seq::install_params_validator(map);
    seq_legacy::SeqLegacy::install_params_validator(map);
}

pub fn schemas() -> Vec<ModuleSchema> {
    vec![seq::Seq::get_schema(), seq_legacy::SeqLegacy::get_schema()]
}
