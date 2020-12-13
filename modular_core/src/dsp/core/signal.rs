use crate::types::{InternalParam, Params};
use anyhow::{anyhow, Result};

const NAME: &str = "signal";
const SOURCE: &str = "source";
const OUTPUT: &str = "output";

#[derive(Default, Params)]
struct SignalParams {
    #[name("source")]
    #[description("signal input")]
    source: InternalParam,
}

#[derive(Default, Module)]
#[name("signal")]
#[description("a signal")]
pub struct Signal {
    #[output("output")]
    sample: f32,
    params: SignalParams,
}

impl Signal {
    fn update(&mut self, sample_rate: f32) -> () {
        self.sample = self.params.source.get_value();
    }
}

// struct Signal {
//     id: Uuid,
//     sample: Mutex<f32>,
//     module: Mutex<SignalModule>,
// }

// impl Sampleable for Signal {
//     fn tick(&self) -> () {
//         *self.sample.try_lock().unwrap() = self.module.try_lock().unwrap().sample;
//     }

//     fn update(&self,  _sample_rate: f32) -> () {
//         self.module.try_lock().unwrap().update();
//     }

//     fn get_sample(&self, port: &String) -> Result<f32> {
//         if port != OUTPUT {
//             return Err(anyhow!(
//                 "Signal Destination with id {} has no port {}",
//                 self.id,
//                 port
//             ));
//         }
//         Ok(*self.sample.try_lock().unwrap())
//     }

//     fn get_state(&self) -> ModuleState {
//         let mut params_map = HashMap::new();
//         let ref params = self.module.lock().unwrap().params;
//         params_map.insert(SOURCE.to_owned(), params.source.to_param());

//         ModuleState {
//             module_type: NAME.to_owned(),
//             id: self.id.clone(),
//             params: params_map,
//         }
//     }

//     fn update_param(&self, param_name: &String, new_param: InternalParam) -> Result<()> {
//         match param_name.as_str() {
//             SOURCE => {
//                 self.module.lock().unwrap().params.source = new_param;
//                 Ok(())
//             }
//             _ => Err(anyhow!("{} is not a valid param name for {}", param_name, NAME)),
//         }
//     }

//     fn get_id(&self) -> Uuid {
//         self.id.clone()
//     }
// }

// pub const SCHEMA: ModuleSchema = ModuleSchema {
//     name: NAME,
//     description: "a signal",
//     params: &[PortSchema {
//         name: SOURCE,
//         description: "source",
//     }],
//     outputs: &[PortSchema {
//         name: OUTPUT,
//         description: "signal output",
//     }],
// };

// fn constructor(id: &Uuid) -> Result<Arc<Box<dyn Sampleable>>>{
//     Ok(Arc::new(Box::new(Signal {
//         id: id.clone(),
//         sample: Mutex::new(0.0),
//         module: Mutex::new(SignalModule {
//             sample: 0.0,
//             params: SignalParams::default(),
//         }),
//     })))
// }

// pub fn install_constructor(map: &mut HashMap<String, SampleableConstructor>) {
//     map.insert(NAME.into(), Box::new(constructor));
// }
