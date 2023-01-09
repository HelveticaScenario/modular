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

lazy_static! {
    pub static ref ROOT_ID: String = "root".into();
    pub static ref ROOT_OUTPUT_PORT: String = "output".into();
}

pub trait Params {
    fn get_params_state(&self) -> HashMap<String, Param>;
    fn update_param(
        &mut self,
        param_name: &str,
        new_param: &InternalParam,
        module_name: &str,
    ) -> Result<()>;
    fn get_schema() -> &'static [PortSchema];
    fn regenerate_cables(&mut self, sampleable_map: &SampleableMap) -> ();
}

pub trait Sampleable: Send + Sync + HasId {
    fn tick(&self) -> ();
    fn update(&self) -> ();
    fn get_sample(&self, port: &str) -> Result<f32>;
    fn get_state(&self) -> ModuleState;
    fn update_param(&self, param_name: &str, new_param: &InternalParam) -> Result<()>;
    fn regenerate_cables(&self, sampleable_map: &SampleableMap) -> ();
}

pub trait Module {
    fn install_constructor(map: &mut HashMap<String, SampleableConstructor>);
    fn get_schema() -> ModuleSchema;
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub module_type: String,
    pub params: Value,
}

pub type SampleableMap = HashMap<String, Arc<Box<dyn Sampleable>>>;

pub trait HasId {
    fn get_id<'a>(&'a self) -> &'a str;
}

#[derive(Debug, Default)]
pub struct Connection<T>
where
    T: HasId,
{
    id: String,
    reference: sync::Weak<T>,
}

impl<T: HasId> PartialEq for Connection<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && self
                .reference
                .upgrade()
                .map(|track| track.get_id().to_owned())
                == other
                    .reference
                    .upgrade()
                    .map(|track| track.get_id().to_owned())
    }
}

impl<T: HasId> Clone for Connection<T> {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            reference: self.reference.clone(),
        }
    }
}

impl<T: HasId> Connection<T> {
    pub fn new(id: String, reference: &Arc<T>) -> Self {
        Self {
            id,
            reference: Arc::downgrade(reference),
        }
    }

    pub fn upgrade(&self) -> Option<Arc<T>> {
        self.reference.upgrade()
    }

    pub fn update_reference(&mut self, reference: &Arc<T>) {
        self.reference = Arc::downgrade(reference);
    }
}

impl<T: HasId> HasId for Connection<T> {
    fn get_id<'a>(&'a self) -> &'a str {
        &self.id
    }
}

impl HasId for Box<dyn Sampleable> {
    fn get_id<'a>(&'a self) -> &'a str {
        self.as_ref().get_id()
    }
}

#[derive(Clone)]
pub enum InternalParam {
    Value {
        value: f32,
    },
    Note {
        value: u8,
    },
    Cable {
        module: Connection<Box<dyn Sampleable>>,
        port: String,
    },
    Track {
        track: Connection<InternalTrack>,
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
            ) => *port1 == *port2 && module1 == module2,
            (InternalParam::Track { track: track1 }, InternalParam::Track { track: track2 }) => {
                *track1 == *track2
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
                    module: module.get_id().to_owned(),
                    port: port.clone(),
                },
                None => Param::Disconnected,
            },
            InternalParam::Track { track } => match track.upgrade() {
                Some(track) => Param::Track {
                    track: track.id.clone(),
                },
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
#[serde(tag = "type", rename_all = "camelCase")]
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
            Param::Cable { module: id, port } => match patch.sampleables.get(id) {
                Some(module) => InternalParam::Cable {
                    module: Connection::new(id.into(), module),
                    port: port.clone(),
                },
                None => InternalParam::Disconnected,
            },
            Param::Track { track: id } => match patch.tracks.get(id) {
                Some(track) => InternalParam::Track {
                    track: Connection::new(id.into(), track),
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
    pub fn new(id: &str, track_id: &str, time: Duration, param: InternalParam) -> Self {
        InternalKeyframe {
            id: id.into(),
            track_id: track_id.into(),
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

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]

pub struct Keyframe {
    pub id: String,
    pub track_id: String,
    pub time: Duration,
    pub param: Param,
}

impl Keyframe {
    pub fn new(id: &str, track_id: &str, time: Duration, param: Param) -> Self {
        Keyframe {
            id: id.into(),
            track_id: track_id.into(),
            time,
            param,
        }
    }
    pub fn get_id(&self) -> String {
        self.id.clone()
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
#[serde(rename_all = "camelCase")]
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

    pub fn remove_keyframe(&mut self, id: &str) -> Option<InternalKeyframe> {
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

impl HasId for InternalTrack {
    fn get_id<'a>(&'a self) -> &'a str {
        &self.id
    }
}

impl InternalTrack {
    pub fn new(id: &str) -> Self {
        InternalTrack {
            id: id.into(),
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

    pub fn remove_keyframe(&self, id: &str) -> Option<InternalKeyframe> {
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

    pub fn update_keyframes(&self, module_id: &str, module: &Option<&Arc<Box<dyn Sampleable>>>) {
        let m = module;
        for keyframe in self.inner_track.lock().keyframes.iter_mut() {
            if let InternalParam::Cable { ref mut module, .. } = keyframe.param {
                if module.get_id() == module_id {
                    if let Some(m) = m {
                        module.update_reference(m);
                    } else {
                        keyframe.param = InternalParam::Disconnected;
                    }
                }
            }
        }
    }
}

pub type TrackMap = HashMap<String, Arc<InternalTrack>>;

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub struct Track {
    pub id: String,
    pub playhead: Duration,
    pub length: Duration,
    pub play_mode: Playmode,
    pub keyframes: Vec<Keyframe>,
}

#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackUpdate {
    pub length: Option<Duration>,
    pub play_mode: Option<Playmode>,
}

#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PortSchema {
    pub name: &'static str,
    pub description: &'static str,
}

#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModuleSchema {
    pub name: &'static str,
    pub description: &'static str,
    pub params: &'static [PortSchema],
    pub outputs: &'static [PortSchema],
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModuleState {
    pub id: String,
    pub module_type: String,
    pub params: HashMap<String, Param>,
}

pub type SampleableConstructor = Box<dyn Fn(&str, f32) -> Result<Arc<Box<dyn Sampleable>>>>;
