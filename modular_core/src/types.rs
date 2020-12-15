use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;
use std::{
    collections::HashMap,
    sync::{self, Arc},
};
use uuid::Uuid;

lazy_static! {
    pub static ref ROOT_ID: Uuid = Uuid::nil();
    pub static ref ROOT_OUTPUT_PORT: String = "output".into();
}

pub trait Params {
    fn get_params_state(&self) -> HashMap<String, Param>;
    fn update_param(
        &mut self,
        param_name: &String,
        new_param: &InternalParam,
        module_name: &str,
    ) -> Result<()>;
    fn get_schema() -> &'static [PortSchema];
}

pub trait Sampleable: Send + Sync {
    fn get_id(&self) -> Uuid;
    fn tick(&self) -> ();
    fn update(&self, sample_rate: f32) -> ();
    fn get_sample(&self, port: &String) -> Result<f32>;
    fn get_state(&self) -> ModuleState;
    fn update_param(&self, param_name: &String, new_param: &InternalParam) -> Result<()>;
}

pub trait Module {
    fn install_constructor(map: &mut HashMap<String, SampleableConstructor>);
    fn get_schema() -> ModuleSchema;
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub module_type: String,
    pub params: Value,
}

pub type SampleableMap = HashMap<Uuid, Arc<Box<dyn Sampleable>>>;

#[derive(Clone)]
pub enum InternalParam {
    Value {
        value: f32,
    },
    Note {
        value: u8,
    },
    Cable {
        module: sync::Weak<Box<dyn Sampleable>>,
        port: String,
    },
    Disconnected,
}

impl PartialEq for InternalParam {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (InternalParam::Value { value: value1 }, InternalParam::Value { value: value2 }) => {
                *value1 == *value2
            }
            (InternalParam::Note { value: value1 }, InternalParam::Note { value: value2 }) => {
                *value1 == *value2
            }
            (
                InternalParam::Cable {
                    module: module1,
                    port: port1,
                },
                InternalParam::Cable {
                    module: module2,
                    port: port2,
                },
            ) => {
                *port1 == *port2
                    && module1.upgrade().map(|module| module.get_id())
                        == module2.upgrade().map(|module| module.get_id())
            }
            (InternalParam::Disconnected, InternalParam::Disconnected) => true,
            _ => false,
        }
    }
}

impl InternalParam {
    pub fn to_param(&self) -> Param {
        match self {
            InternalParam::Value { value } => Param::Value { value: *value },
            InternalParam::Note { value } => Param::Note { value: *value },
            InternalParam::Cable { module, port } => match module.upgrade() {
                Some(module) => Param::Cable {
                    module: module.get_id(),
                    port: port.clone(),
                },
                None => Param::Disconnected,
            },
            InternalParam::Disconnected => Param::Disconnected,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "param_type", rename_all = "kebab-case")]
pub enum Param {
    Value { value: f32 },
    Note { value: u8 },
    Cable { module: Uuid, port: String },
    Disconnected,
}

impl Param {
    pub fn to_internal_param(&self, patch_map: &SampleableMap) -> InternalParam {
        match self {
            Param::Value { value } => InternalParam::Value { value: *value },
            Param::Note { value } => InternalParam::Note { value: *value },
            Param::Cable { module, port } => {
                match patch_map.get(module) {
                    Some(module) => InternalParam::Cable {
                        module: Arc::downgrade(module),
                        port: port.clone(),
                    },
                    None => InternalParam::Disconnected,
                }
                // InternalParam::Cable {
                //     module:
                // }
            }
            Param::Disconnected => InternalParam::Disconnected,
        }
    }
}

impl InternalParam {
    pub fn get_value(&self) -> f32 {
        self.get_value_or(0.0)
    }
    pub fn get_value_or(&self, default: f32) -> f32 {
        match self {
            InternalParam::Value { value } => *value,
            InternalParam::Note { value } => (*value as f32 - 21.0) / 12.0,
            InternalParam::Cable { module, port } => {
                if let Some(m) = module.upgrade() {
                    m.get_sample(port).unwrap_or(default)
                } else {
                    default
                }
            }
            InternalParam::Disconnected => default,
        }
    }
}

impl Default for InternalParam {
    fn default() -> Self {
        InternalParam::Disconnected
    }
}

#[derive(PartialEq)]
pub struct Keyframe {
    id: Uuid,
    pub time: Duration,
    pub param: InternalParam,
}

impl Keyframe {
    pub fn new(id: Uuid, time: Duration, param: InternalParam) -> Self {
        Keyframe { id, time, param }
    }
    pub fn get_id(&self) -> Uuid {
        self.id
    }
}

impl PartialOrd for Keyframe {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.time.partial_cmp(&other.time)
    }
}

pub enum Playmode {
    Once,
    Loop,
}

pub struct Track {
    id: Uuid,
    module: sync::Weak<Box<dyn Sampleable>>,
    port: String,
    playhead: Duration,
    length: Duration,
    play_mode: Playmode,
    playhead_idx: usize,
    keyframes: Vec<Keyframe>,
}

impl Track {
    pub fn new(id: Uuid, module: sync::Weak<Box<dyn Sampleable>>, port: String) -> Self {
        Track {
            id,
            module,
            port,
            playhead: Duration::from_nanos(0),
            playhead_idx: 0,
            length: Duration::from_nanos(0),
            play_mode: Playmode::Once,
            keyframes: Vec::new(),
        }
    }

    pub fn seek(&mut self, mut playhead: Duration) {
        if self.length < playhead {
            match self.play_mode {
                Playmode::Once => {
                    self.playhead = self.length;
                    self.playhead_idx = (self.keyframes.len() - 1).max(0);
                    return;
                }
                Playmode::Loop => {
                    while self.length < playhead {
                        playhead -= self.length;
                    }
                    self.playhead = playhead;
                }
            }
        } else {
            self.playhead = playhead;
        }
        let len = self.keyframes.len();
        while self.playhead_idx < len {
            let curr = self.keyframes.get(self.playhead_idx).unwrap();
            let next = self.keyframes.get(self.playhead_idx + 1);
            match next {
                Some(next) => {
                    let curr_is_behind_or_equal = self.playhead >= curr.time;
                    let next_is_ahead = self.playhead < next.time;
                    match (curr_is_behind_or_equal, next_is_ahead) {
                        (true, true) => return,
                        (true, false) => {
                            self.playhead_idx += 1;
                        }
                        (false, true) => {
                            if self.playhead_idx == 0 {
                                return;
                            } else {
                                self.playhead_idx -= 1;
                            }
                        }
                        (false, false) => {
                            unreachable!()
                        }
                    };
                }
                None => {
                    if self.playhead < curr.time {
                        if self.playhead_idx == 0 {
                            return;
                        } else {
                            self.playhead_idx -= 1;
                        }
                    } else {
                        return;
                    }
                }
            }
        }
    }

    pub fn add_keyframe(&mut self, keyframe: Keyframe) {
        self.keyframes.push(keyframe);
        self.keyframes.sort_by_key(|a| a.time);

        {
            let time = self.keyframes.last().unwrap().time;
            if self.length < time {
                self.length = time
            }
        }
        let playhead = self.playhead;
        self.playhead = Duration::from_nanos(0);
        self.playhead_idx = 0;
        self.seek(playhead);
    }

    pub fn remove_keyframe(&mut self, id: Uuid) -> Keyframe {
        let keyframe = self
            .keyframes
            .remove(self.keyframes.iter().position(|k| k.id == id).unwrap());

        let playhead = self.playhead;
        self.playhead = Duration::from_nanos(0);
        self.playhead_idx = 0;
        self.seek(playhead);

        keyframe
    }

    pub fn update(&mut self, delta: Duration) {
        self.seek(self.playhead + delta);
        
    }
}

pub type TrackMap = HashMap<Uuid, Track>;

#[derive(Debug, Clone)]
pub struct PortSchema {
    pub name: &'static str,
    pub description: &'static str,
}

#[derive(Debug, Clone)]
pub struct ModuleSchema {
    pub name: &'static str,
    pub description: &'static str,
    pub params: &'static [PortSchema],
    pub outputs: &'static [PortSchema],
}

#[derive(Debug, Clone)]
pub struct ModuleState {
    pub id: Uuid,
    pub module_type: String,
    pub params: HashMap<String, Param>,
}

pub type SampleableConstructor = Box<dyn Fn(&Uuid) -> Result<Arc<Box<dyn Sampleable>>>>;
