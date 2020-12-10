use std::{sync::Arc, collections::HashMap};
use std::sync::Mutex;

use anyhow::{anyhow, Result};
use uuid::Uuid;

use crate::{
    dsp::utils::wrap,
    dsp::{
        consts::{LUT_SINE, LUT_SINE_SIZE},
        utils::{clamp, interpolate},
    },
    types::{ModuleSchema, ModuleState, InternalParam, PortSchema, Sampleable, SampleableConstructor},
};

const NAME: &str = "sine-oscillator";
const OUTPUT: &str = "output";
const FREQ: &str = "freq";
const PHASE: &str = "phase";

#[derive( Default)]
struct SineOscillatorParams {
    freq: InternalParam,
    phase: InternalParam,
}

struct SineOscillatorModule {
    sample: f32,
    phase: f32,
    params: SineOscillatorParams,
}

impl SineOscillatorModule {
    fn update(&mut self,  sample_rate: f32) -> () {
        if self.params.phase != InternalParam::Disconnected {
            self.sample = wrap(0.0..1.0, self.params.phase.get_value());
        } else {
            let voltage = clamp(
                self.params.freq.get_value_or(4.0),
                12.0,
                0.0,
            );
            let frequency = 27.5f32 * 2.0f32.powf(voltage) / sample_rate;
            // let frequency = semitones_to_ratio(voltage * 12.0) * 220.0 / SAMPLE_RATE * 100.0;
            self.phase += frequency;
            if self.phase >= 1.0 {
                self.phase -= 1.0;
            }
            self.sample = 5.0 * interpolate(LUT_SINE, self.phase, LUT_SINE_SIZE);
        }
    }
}

struct SineOscillator {
    id: Uuid,
    sample: Mutex<f32>,
    module: Mutex<SineOscillatorModule>,
}

impl Sampleable for SineOscillator {
    fn tick(&self) -> () {
        *self.sample.try_lock().unwrap() = self.module.try_lock().unwrap().sample;
    }

    fn update(&self,  sample_rate: f32) -> () {
        self.module
            .try_lock()
            .unwrap()
            .update( sample_rate);
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
        let mut param_map = HashMap::new();
        let ref params = self.module.lock().unwrap().params;
        param_map.insert(FREQ.to_owned(), params.freq.to_param());
        param_map.insert(PHASE.to_owned(), params.phase.to_param());
        ModuleState {
            module_type: NAME.to_owned(),
            id: self.id.clone(),
            params: param_map,
        }
    }

    fn update_param(&self, param_name: &String, new_param: InternalParam) -> Result<()> {
        match param_name.as_str() {
            FREQ => {
                self.module.lock().unwrap().params.freq = new_param;
                Ok(())
            }
            PHASE => {
                self.module.lock().unwrap().params.phase = new_param;
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
    description: "A sine wave oscillator",
    params: &[
        PortSchema {
            name: FREQ,
            description: "frequency in v/oct",
        },
        PortSchema {
            name: PHASE,
            description: "the phase of the oscillator, overrides freq if present",
        },
    ],
    outputs: &[PortSchema {
        name: OUTPUT,
        description: "signal output",
    }],
};

fn constructor(id: &Uuid) -> Result<Arc<Box<dyn Sampleable>>> {
    Ok(Arc::new(Box::new(SineOscillator {
        id: id.clone(),
        sample: Mutex::new(0.0),
        module: Mutex::new(SineOscillatorModule {
            params: SineOscillatorParams::default(),
            sample: 0.0,
            phase: 0.0,
        }),
    })))
}

pub fn install_constructor(map: &mut HashMap<String, SampleableConstructor>) {
    map.insert(NAME.into(), Box::new(constructor));
}
