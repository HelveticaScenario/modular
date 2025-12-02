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

/// One-pole lowpass filter for parameter smoothing to prevent clicking
/// Coefficient of 0.99 gives roughly 5ms smoothing time at 48kHz
const SMOOTHING_COEFF: f32 = 0.99;

pub fn smooth_value(current: f32, target: f32) -> f32 {
    current * SMOOTHING_COEFF + target * (1.0 - SMOOTHING_COEFF)
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
        module: sync::Weak<Box<dyn Sampleable>>,
        port: String,
    },
    Track {
        track: sync::Weak<InternalTrack>,
    },
    Disconnected,
}

impl std::fmt::Debug for InternalParam {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InternalParam::Value { value } => f.debug_struct("Value").field("value", value).finish(),
            InternalParam::Note { value } => f.debug_struct("Note").field("value", value).finish(),
            InternalParam::Cable { port, .. } => f.debug_struct("Cable").field("port", port).finish(),
            InternalParam::Track { .. } => f.debug_struct("Track").finish(),
            InternalParam::Disconnected => write!(f, "Disconnected"),
        }
    }
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PatchGraph {
    pub modules: Vec<ModuleState>,
}

pub type SampleableConstructor = Box<dyn Fn(&String, f32) -> Result<Arc<Box<dyn Sampleable>>>>;

#[cfg(test)]
mod tests {
    use super::*;

    // Tests for smooth_value function
    #[test]
    fn test_smooth_value_converges_to_target() {
        let target = 1.0;
        let mut current = 0.0;
        
        // Apply smoothing many times
        for _ in 0..1000 {
            current = smooth_value(current, target);
        }
        
        // Should converge close to target
        assert!((current - target).abs() < 0.01, "Expected {} to be close to {}", current, target);
    }

    #[test]
    fn test_smooth_value_no_change_at_target() {
        let target = 5.0;
        let current = 5.0;
        let result = smooth_value(current, target);
        assert!((result - target).abs() < 0.0001, "Value at target should stay at target");
    }

    #[test]
    fn test_smooth_value_gradual_change() {
        let target = 10.0;
        let current = 0.0;
        let result = smooth_value(current, target);
        
        // Should move towards target but not reach it immediately
        assert!(result > current, "Should move towards positive target");
        assert!(result < target, "Should not immediately reach target");
    }

    // Tests for InternalParam
    #[test]
    fn test_internal_param_value_get_value() {
        let param = InternalParam::Value { value: 3.5 };
        assert!((param.get_value() - 3.5).abs() < 0.0001);
    }

    #[test]
    fn test_internal_param_value_get_value_or() {
        let param = InternalParam::Value { value: 2.0 };
        assert!((param.get_value_or(5.0) - 2.0).abs() < 0.0001);
    }

    #[test]
    fn test_internal_param_disconnected_get_value_or() {
        let param = InternalParam::Disconnected;
        assert!((param.get_value_or(5.0) - 5.0).abs() < 0.0001);
    }

    #[test]
    fn test_internal_param_disconnected_get_value() {
        let param = InternalParam::Disconnected;
        assert!((param.get_value() - 0.0).abs() < 0.0001);
    }

    #[test]
    fn test_internal_param_note_conversion() {
        // MIDI note 69 = A4 = 440Hz, which is 4 v/oct
        let param = InternalParam::Note { value: 69 };
        let value = param.get_value();
        // (69 - 21) / 12 = 48 / 12 = 4.0
        assert!((value - 4.0).abs() < 0.0001);
    }

    #[test]
    fn test_internal_param_note_to_voct_middle_c() {
        // MIDI note 60 = C4
        let param = InternalParam::Note { value: 60 };
        let value = param.get_value();
        // (60 - 21) / 12 = 39 / 12 = 3.25
        assert!((value - 3.25).abs() < 0.0001);
    }

    #[test]
    fn test_internal_param_default() {
        let param = InternalParam::default();
        assert!(matches!(param, InternalParam::Disconnected));
    }

    // Tests for InternalParam to Param conversion
    #[test]
    fn test_internal_param_value_to_param() {
        let internal = InternalParam::Value { value: 1.5 };
        let param = internal.to_param();
        assert!(matches!(param, Param::Value { value } if (value - 1.5).abs() < 0.0001));
    }

    #[test]
    fn test_internal_param_note_to_param() {
        let internal = InternalParam::Note { value: 60 };
        let param = internal.to_param();
        assert!(matches!(param, Param::Note { value: 60 }));
    }

    #[test]
    fn test_internal_param_disconnected_to_param() {
        let internal = InternalParam::Disconnected;
        let param = internal.to_param();
        assert!(matches!(param, Param::Disconnected));
    }

    // Tests for InternalParam equality
    #[test]
    fn test_internal_param_value_equality() {
        let a = InternalParam::Value { value: 1.0 };
        let b = InternalParam::Value { value: 1.0 };
        let c = InternalParam::Value { value: 2.0 };
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn test_internal_param_note_equality() {
        let a = InternalParam::Note { value: 60 };
        let b = InternalParam::Note { value: 60 };
        let c = InternalParam::Note { value: 72 };
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn test_internal_param_disconnected_equality() {
        let a = InternalParam::Disconnected;
        let b = InternalParam::Disconnected;
        assert_eq!(a, b);
    }

    #[test]
    fn test_internal_param_different_types_not_equal() {
        let value = InternalParam::Value { value: 60.0 };
        let note = InternalParam::Note { value: 60 };
        let disconnected = InternalParam::Disconnected;
        assert_ne!(value.clone(), note.clone());
        assert_ne!(value.clone(), disconnected.clone());
        assert_ne!(note, disconnected);
    }

    // Tests for Param serialization
    #[test]
    fn test_param_value_serialization() {
        let param = Param::Value { value: 4.0 };
        let json = serde_json::to_string(&param).unwrap();
        assert!(json.contains("value"));
        assert!(json.contains("4.0") || json.contains("4"));
    }

    #[test]
    fn test_param_note_serialization() {
        let param = Param::Note { value: 69 };
        let json = serde_json::to_string(&param).unwrap();
        assert!(json.contains("69"));
    }

    #[test]
    fn test_param_cable_serialization() {
        let param = Param::Cable { 
            module: "sine-1".to_string(), 
            port: "output".to_string() 
        };
        let json = serde_json::to_string(&param).unwrap();
        assert!(json.contains("sine-1"));
        assert!(json.contains("output"));
    }

    #[test]
    fn test_param_deserialization_roundtrip() {
        let original = Param::Value { value: 3.14 };
        let json = serde_json::to_string(&original).unwrap();
        let restored: Param = serde_json::from_str(&json).unwrap();
        assert_eq!(original, restored);
    }

    // Tests for Keyframe
    #[test]
    fn test_keyframe_new() {
        let kf = Keyframe::new(
            "kf-1".to_string(),
            "track-1".to_string(),
            Duration::from_millis(500),
            Param::Value { value: 1.0 },
        );
        assert_eq!(kf.id, "kf-1");
        assert_eq!(kf.track_id, "track-1");
        assert_eq!(kf.time, Duration::from_millis(500));
    }

    #[test]
    fn test_keyframe_get_id() {
        let kf = Keyframe::new(
            "kf-abc".to_string(),
            "track-1".to_string(),
            Duration::from_millis(100),
            Param::Value { value: 2.0 },
        );
        assert_eq!(kf.get_id(), &"kf-abc".to_string());
    }

    #[test]
    fn test_keyframe_partial_ord() {
        let kf1 = Keyframe::new(
            "kf-1".to_string(),
            "track-1".to_string(),
            Duration::from_millis(100),
            Param::Value { value: 1.0 },
        );
        let kf2 = Keyframe::new(
            "kf-2".to_string(),
            "track-1".to_string(),
            Duration::from_millis(200),
            Param::Value { value: 2.0 },
        );
        assert!(kf1 < kf2);
    }

    // Tests for Playmode
    #[test]
    fn test_playmode_serialization() {
        let once = Playmode::Once;
        let looped = Playmode::Loop;
        
        let once_json = serde_json::to_string(&once).unwrap();
        let loop_json = serde_json::to_string(&looped).unwrap();
        
        assert!(once_json.contains("once"));
        assert!(loop_json.contains("loop"));
    }

    #[test]
    fn test_playmode_deserialization() {
        let once: Playmode = serde_json::from_str("\"once\"").unwrap();
        let looped: Playmode = serde_json::from_str("\"loop\"").unwrap();
        assert_eq!(once, Playmode::Once);
        assert_eq!(looped, Playmode::Loop);
    }

    // Tests for ModuleState
    #[test]
    fn test_module_state_serialization() {
        let mut params = HashMap::new();
        params.insert("freq".to_string(), Param::Value { value: 4.0 });
        
        let state = ModuleState {
            id: "sine-1".to_string(),
            module_type: "sine-oscillator".to_string(),
            params,
        };
        
        let json = serde_json::to_string(&state).unwrap();
        assert!(json.contains("sine-1"));
        assert!(json.contains("sine-oscillator"));
    }

    #[test]
    fn test_module_state_equality() {
        let mut params1 = HashMap::new();
        params1.insert("freq".to_string(), Param::Value { value: 4.0 });
        
        let mut params2 = HashMap::new();
        params2.insert("freq".to_string(), Param::Value { value: 4.0 });
        
        let state1 = ModuleState {
            id: "sine-1".to_string(),
            module_type: "sine-oscillator".to_string(),
            params: params1,
        };
        let state2 = ModuleState {
            id: "sine-1".to_string(),
            module_type: "sine-oscillator".to_string(),
            params: params2,
        };
        
        assert_eq!(state1, state2);
    }

    // Tests for PatchGraph
    #[test]
    fn test_patch_graph_empty() {
        let patch = PatchGraph { modules: vec![] };
        assert!(patch.modules.is_empty());
    }

    #[test]
    fn test_patch_graph_with_modules() {
        let state = ModuleState {
            id: "test-1".to_string(),
            module_type: "signal".to_string(),
            params: HashMap::new(),
        };
        let patch = PatchGraph { modules: vec![state] };
        assert_eq!(patch.modules.len(), 1);
    }

    #[test]
    fn test_patch_graph_serialization_roundtrip() {
        let mut params = HashMap::new();
        params.insert("source".to_string(), Param::Disconnected);
        
        let original = PatchGraph {
            modules: vec![
                ModuleState {
                    id: "signal-1".to_string(),
                    module_type: "signal".to_string(),
                    params,
                },
            ],
        };
        
        let json = serde_json::to_string(&original).unwrap();
        let restored: PatchGraph = serde_json::from_str(&json).unwrap();
        assert_eq!(original, restored);
    }

    // Tests for TrackUpdate
    #[test]
    fn test_track_update_partial() {
        let update = TrackUpdate {
            length: Some(Duration::from_secs(10)),
            play_mode: None,
        };
        assert_eq!(update.length, Some(Duration::from_secs(10)));
        assert_eq!(update.play_mode, None);
    }

    #[test]
    fn test_track_update_full() {
        let update = TrackUpdate {
            length: Some(Duration::from_secs(5)),
            play_mode: Some(Playmode::Loop),
        };
        assert_eq!(update.length, Some(Duration::from_secs(5)));
        assert_eq!(update.play_mode, Some(Playmode::Loop));
    }

    // Tests for PortSchema
    #[test]
    fn test_port_schema_equality() {
        let a = PortSchema {
            name: "output".to_string(),
            description: "Main output".to_string(),
        };
        let b = PortSchema {
            name: "output".to_string(),
            description: "Main output".to_string(),
        };
        assert_eq!(a, b);
    }

    // Tests for ModuleSchema
    #[test]
    fn test_module_schema_creation() {
        let schema = ModuleSchema {
            name: "sine-oscillator".to_string(),
            description: "A sine wave generator".to_string(),
            params: vec![
                PortSchema { name: "freq".to_string(), description: "Frequency in v/oct".to_string() },
            ],
            outputs: vec![
                PortSchema { name: "output".to_string(), description: "Audio output".to_string() },
            ],
        };
        assert_eq!(schema.name, "sine-oscillator");
        assert_eq!(schema.params.len(), 1);
        assert_eq!(schema.outputs.len(), 1);
    }

    // Tests for ROOT_ID and ROOT_OUTPUT_PORT
    #[test]
    fn test_root_id_constant() {
        assert_eq!(*ROOT_ID, "root");
    }

    #[test]
    fn test_root_output_port_constant() {
        assert_eq!(*ROOT_OUTPUT_PORT, "output");
    }

    // Tests for Duration serialization helpers
    #[test]
    fn test_keyframe_duration_serialization() {
        let kf = Keyframe::new(
            "test".to_string(),
            "track".to_string(),
            Duration::from_millis(1500),
            Param::Value { value: 1.0 },
        );
        let json = serde_json::to_string(&kf).unwrap();
        // Duration should serialize as milliseconds
        assert!(json.contains("1500"));
    }

    #[test]
    fn test_track_duration_serialization() {
        let track = Track {
            id: "test-track".to_string(),
            playhead: Duration::from_millis(500),
            length: Duration::from_millis(10000),
            play_mode: Playmode::Loop,
            keyframes: vec![],
        };
        let json = serde_json::to_string(&track).unwrap();
        assert!(json.contains("500"));
        assert!(json.contains("10000"));
    }
}
