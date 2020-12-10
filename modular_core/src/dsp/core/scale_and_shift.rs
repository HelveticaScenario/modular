use std::{sync::Arc, collections::HashMap};
use std::sync::Mutex;

use anyhow::{anyhow, Result};


use uuid::Uuid;

use crate::types::{
    InternalParam, ModuleSchema, ModuleState, PortSchema, Sampleable,
    SampleableConstructor,
};

const NAME: &str = "scale-and-shift";
const INPUT: &str = "input";
const SCALE: &str = "scale";
const SHIFT: &str = "shift";
const OUTPUT: &str = "output";

#[derive(Default)]
struct ScaleAndShiftParams {
    input: InternalParam,
    scale: InternalParam,
    shift: InternalParam,
}

struct ScaleAndShiftModule {
    sample: f32,
    params: ScaleAndShiftParams,
}

impl ScaleAndShiftModule {
    fn update(&mut self) -> () {
        let input = self.params.input.get_value();
        let scale = self.params.scale.get_value_or(5.0);
        let shift = self.params.shift.get_value();
        self.sample = input * (scale / 5.0) + shift
    }
}


struct ScaleAndShift {
    id: Uuid,
    sample: Mutex<f32>,
    module: Mutex<ScaleAndShiftModule>,
}

impl Sampleable for ScaleAndShift {
    fn tick(&self) -> () {
        *self.sample.try_lock().unwrap() = self.module.try_lock().unwrap().sample;
    }

    fn update(&self, _sample_rate: f32) -> () {
        self.module.try_lock().unwrap().update();
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
        let mut params_map = HashMap::new();
        let ref params = self.module.lock().unwrap().params;
        params_map.insert(INPUT.to_owned(), params.input.to_param());
        params_map.insert(SCALE.to_owned(), params.scale.to_param());
        params_map.insert(SHIFT.to_owned(), params.shift.to_param());
        ModuleState {
            module_type: NAME.to_owned(),
            id: self.id.clone(),
            params: params_map,
        }
    }

    fn update_param(&self, param_name: &String, new_param: InternalParam) -> Result<()> {
        match param_name.as_str() {
            INPUT => {
                self.module.lock().unwrap().params.input = new_param;
                Ok(())
            }
            SCALE => {
                self.module.lock().unwrap().params.scale = new_param;
                Ok(())
            }
            SHIFT => {
                self.module.lock().unwrap().params.shift = new_param;
                Ok(())
            }
            _ => Err(anyhow!(
                "{} is not a valid param name for {}",
                param_name,
                NAME
            )),
        }
    }

    fn get_id(&self) -> Uuid {
        self.id.clone()
    }
}

pub const SCHEMA: ModuleSchema = ModuleSchema {
    name: NAME,
    description: "attenuate, invert, offset",
    params: &[
        PortSchema {
            name: INPUT,
            description: "signal input",
        },
        PortSchema {
            name: SCALE,
            description: "scale factor",
        },
        PortSchema {
            name: SHIFT,
            description: "shift amount",
        },
    ],
    outputs: &[PortSchema {
        name: OUTPUT,
        description: "signal output",
    }],
};

fn constructor(id: &Uuid) -> Result<Arc<Box<dyn Sampleable>>> {
    Ok(Arc::new(Box::new(ScaleAndShift {
        id: id.clone(),
        sample: Mutex::new(0.0),
        module: Mutex::new(ScaleAndShiftModule {
            params: ScaleAndShiftParams::default(),
            sample: 0.0,
        }),
    })))
}

pub fn install_constructor(map: &mut HashMap<String, SampleableConstructor>) {
    map.insert(NAME.into(), Box::new(constructor));
}
