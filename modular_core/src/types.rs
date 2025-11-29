use anyhow::Result;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;
use std::{
    collections::HashMap,
    sync::{self, Arc},
};

use crate::patch::Patch;

// Serde helpers for Duration
mod duration_millis {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(duration.as_millis() as u64)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let millis = u64::deserialize(deserializer)?;
        Ok(Duration::from_millis(millis))
    }
}

mod option_duration_millis {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Option<Duration>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match duration {
            Some(d) => serializer.serialize_some(&(d.as_millis() as u64)),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Duration>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let millis = Option::<u64>::deserialize(deserializer)?;
        Ok(millis.map(Duration::from_millis))
    }
}

lazy_static! {
    pub static ref ROOT_ID: String = String::from("root");
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
    fn get_schema() -> Vec<PortSchema>;
}

pub trait Sampleable: Send + Sync {
    fn get_id(&self) -> &String;
    fn tick(&self) -> ();
    fn update(&self) -> ();
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

pub type SampleableMap = HashMap<String, Arc<Box<dyn Sampleable>>>;

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
    Track {
        track: sync::Weak<InternalTrack>,
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
                    && module1.upgrade().map(|module| module.get_id().clone())
                        == module2.upgrade().map(|module| module.get_id().clone())
            }
            (InternalParam::Track { track: track1 }, InternalParam::Track { track: track2 }) => {
                track1.upgrade().map(|track| track.id.clone()) == track2.upgrade().map(|track| track.id.clone())
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
                    module: module.get_id().clone(),
                    port: port.clone(),
                },
                None => Param::Disconnected,
            },
            InternalParam::Track { track } => match track.upgrade() {
                Some(track) => Param::Track { track: track.id.clone() },
                None => Param::Disconnected,
            },
            InternalParam::Disconnected => Param::Disconnected,
        }
    }
    pub fn get_value(&self) -> f32 {
        self.get_value_or(0.0)
    }
    pub fn get_value_or(&self, default: f32) -> f32 {
        self.get_value_optional().unwrap_or(default)
    }
    fn get_value_optional(&self) -> Option<f32> {
        match self {
            InternalParam::Value { value } => Some(*value),
            InternalParam::Note { value } => Some((*value as f32 - 21.0) / 12.0),
            InternalParam::Cable { module, port } => match module.upgrade() {
                Some(module) => match module.get_sample(port) {
                    Ok(sample) => Some(sample),
                    Err(_) => None,
                },
                None => None,
            },
            InternalParam::Track { track } => match track.upgrade() {
                Some(track) => match track.get_value_optional() {
                    Some(sample) => Some(sample),
                    None => None,
                },
                None => None,
            },
            InternalParam::Disconnected => None,
        }
    }
}

impl Default for InternalParam {
    fn default() -> Self {
        InternalParam::Disconnected
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "param_type", rename_all = "kebab-case")]
pub enum Param {
    Value { value: f32 },
    Note { value: u8 },
    Cable { module: String, port: String },
    Track { track: String },
    Disconnected,
}

impl Param {
    pub fn to_internal_param(&self, patch: &Patch) -> InternalParam {
        match self {
            Param::Value { value } => InternalParam::Value { value: *value },
            Param::Note { value } => InternalParam::Note { value: *value },
            Param::Cable { module, port } => match patch.sampleables.get(module) {
                Some(module) => InternalParam::Cable {
                    module: Arc::downgrade(module),
                    port: port.clone(),
                },
                None => InternalParam::Disconnected,
            },
            Param::Track { track } => match patch.tracks.get(track) {
                Some(track) => InternalParam::Track {
                    track: Arc::downgrade(track),
                },
                None => InternalParam::Disconnected,
            },
            Param::Disconnected => InternalParam::Disconnected,
        }
    }
}

#[derive(PartialEq)]
pub struct InternalKeyframe {
    id: String,
    track_id: String,
    pub time: Duration,
    pub param: InternalParam,
}

impl InternalKeyframe {
    pub fn new(id: String, track_id: String, time: Duration, param: InternalParam) -> Self {
        InternalKeyframe {
            id,
            track_id,
            time,
            param,
        }
    }
    pub fn get_id(&self) -> String {
        self.id.clone()
    }
    pub fn to_keyframe(&self) -> Keyframe {
        Keyframe {
            id: self.id.clone(),
            track_id: self.track_id.clone(),
            time: self.time,
            param: self.param.to_param(),
        }
    }
}

impl PartialOrd for InternalKeyframe {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.time.partial_cmp(&other.time)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Keyframe {
    pub id: String,
    pub track_id: String,
    #[serde(with = "duration_millis")]
    pub time: Duration,
    pub param: Param,
}

impl Keyframe {
    pub fn new(id: String, track_id: String, time: Duration, param: Param) -> Self {
        Keyframe {
            id,
            track_id,
            time,
            param,
        }
    }
    pub fn get_id(&self) -> &String {
        &self.id
    }
    pub fn to_internal_keyframe(&self, patch: &Patch) -> InternalKeyframe {
        InternalKeyframe {
            id: self.id.clone(),
            track_id: self.track_id.clone(),
            time: self.time,
            param: self.param.to_internal_param(patch),
        }
    }
}

impl PartialOrd for Keyframe {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.time.partial_cmp(&other.time)
    }
}

#[derive(Debug, Clone, Copy, PartialOrd, PartialEq, Ord, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Playmode {
    Once,
    Loop,
}

struct InnerTrack {
    playhead: Duration,
    length: Duration,
    play_mode: Playmode,
    playhead_idx: usize,
    keyframes: Vec<InternalKeyframe>,
}

impl InnerTrack {
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

    pub fn add_keyframe(&mut self, keyframe: InternalKeyframe) {
        match self.keyframes.iter().position(|k| k.id == keyframe.id) {
            Some(idx) => {
                self.keyframes[idx] = keyframe;
            }
            None => {
                self.keyframes.push(keyframe);
            }
        }
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

    pub fn remove_keyframe(&mut self, id: String) -> Option<InternalKeyframe> {
        match self.keyframes.iter().position(|k| k.id == id) {
            Some(idx) => {
                let ret = Some(self.keyframes.remove(idx));
                let playhead = self.playhead;
                self.playhead = Duration::from_nanos(0);
                self.playhead_idx = 0;
                self.seek(playhead);
                ret
            }
            None => None,
        }
    }

    pub fn tick(&mut self, delta: &Duration) -> Option<f32> {
        self.seek(self.playhead + *delta);
        match self.keyframes.get(self.playhead_idx) {
            Some(keyframe) => keyframe.param.get_value_optional(),
            None => None,
        }
    }

    pub fn update(&mut self, update: &TrackUpdate) {
        if let Some(play_mode) = update.play_mode {
            self.play_mode = play_mode;
        }
        if let Some(length) = update.length {
            self.length = length;
            if self.playhead > self.length {
                self.playhead = self.length
            }
            self.seek(self.playhead);
        }
    }
}

pub struct InternalTrack {
    id: String,
    inner_track: Mutex<InnerTrack>,
    sample: Mutex<Option<f32>>,
}

impl InternalTrack {
    pub fn new(id: String) -> Self {
        InternalTrack {
            id,
            inner_track: Mutex::new(InnerTrack {
                playhead: Duration::from_nanos(0),
                playhead_idx: 0,
                length: Duration::from_nanos(0),
                play_mode: Playmode::Once,
                keyframes: Vec::new(),
            }),
            sample: Mutex::new(None),
        }
    }

    pub fn seek(&self, playhead: Duration) {
        self.inner_track
            .try_lock_for(Duration::from_millis(10))
            .unwrap()
            .seek(playhead)
    }

    pub fn add_keyframe(&self, keyframe: InternalKeyframe) {
        self.inner_track
            .try_lock_for(Duration::from_millis(10))
            .unwrap()
            .add_keyframe(keyframe)
    }

    pub fn remove_keyframe(&self, id: String) -> Option<InternalKeyframe> {
        self.inner_track
            .try_lock_for(Duration::from_millis(10))
            .unwrap()
            .remove_keyframe(id)
    }

    pub fn tick(&self, delta: &Duration) {
        *(self.sample.try_lock_for(Duration::from_millis(10)).unwrap()) = self
            .inner_track
            .try_lock_for(Duration::from_millis(10))
            .unwrap()
            .tick(delta);
    }

    pub fn update(&self, update: &TrackUpdate) {
        self.inner_track
            .try_lock_for(Duration::from_millis(10))
            .unwrap()
            .update(update);
    }

    pub fn get_value_optional(&self) -> Option<f32> {
        *self.sample.try_lock_for(Duration::from_millis(10)).unwrap()
    }

    pub fn to_track(&self) -> Track {
        let inner_track = self
            .inner_track
            .try_lock_for(Duration::from_millis(10))
            .unwrap();
        Track {
            id: self.id.clone(),
            playhead: inner_track.playhead,
            length: inner_track.length,
            play_mode: inner_track.play_mode,
            keyframes: inner_track
                .keyframes
                .iter()
                .map(|k| k.to_keyframe())
                .collect(),
        }
    }
}

pub type TrackMap = HashMap<String, Arc<InternalTrack>>;

#[derive(Debug, Clone, PartialOrd, PartialEq, Serialize, Deserialize)]
pub struct Track {
    pub id: String,
    #[serde(with = "duration_millis")]
    pub playhead: Duration,
    #[serde(with = "duration_millis")]
    pub length: Duration,
    pub play_mode: Playmode,
    pub keyframes: Vec<Keyframe>,
}

#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq, Serialize, Deserialize)]
pub struct TrackUpdate {
    #[serde(default, with = "option_duration_millis", skip_serializing_if = "Option::is_none")]
    pub length: Option<Duration>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub play_mode: Option<Playmode>,
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Serialize, Deserialize)]
pub struct PortSchema {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Serialize, Deserialize)]
pub struct ModuleSchema {
    pub name: String,
    pub description: String,
    pub params: Vec<PortSchema>,
    pub outputs: Vec<PortSchema>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModuleState {
    pub id: String,
    pub module_type: String,
    pub params: HashMap<String, Param>,
}

pub type SampleableConstructor = Box<dyn Fn(&String, f32) -> Result<Arc<Box<dyn Sampleable>>>>;
