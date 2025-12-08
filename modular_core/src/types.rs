use anyhow::Result;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;
use std::{
    collections::HashMap,
    sync::{self, Arc},
};
use ts_rs::TS;

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
    fn get_schema() -> Vec<ParamSchema>;
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
    Volts {
        value: f32,
    },
    Hz {
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
            InternalParam::Volts { value } => {
                f.debug_struct("Value").field("value", value).finish()
            }
            InternalParam::Hz { value } => f.debug_struct("Hz").field("value", value).finish(),
            InternalParam::Note { value } => f.debug_struct("Note").field("value", value).finish(),
            InternalParam::Cable { port, .. } => {
                f.debug_struct("Cable").field("port", port).finish()
            }
            InternalParam::Track { .. } => f.debug_struct("Track").finish(),
            InternalParam::Disconnected => write!(f, "Disconnected"),
        }
    }
}

impl PartialEq for InternalParam {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (InternalParam::Volts { value: value1 }, InternalParam::Volts { value: value2 }) => {
                *value1 == *value2
            }
            (InternalParam::Hz { value: value1 }, InternalParam::Hz { value: value2 }) => {
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
                track1.upgrade().map(|track| track.id.clone())
                    == track2.upgrade().map(|track| track.id.clone())
            }
            (InternalParam::Disconnected, InternalParam::Disconnected) => true,
            _ => false,
        }
    }
}

impl InternalParam {
    pub fn to_param(&self) -> Param {
        match self {
            InternalParam::Volts { value } => Param::Value { value: *value },
            InternalParam::Hz { value } => Param::Hz { value: *value },
            InternalParam::Note { value } => Param::Note { value: *value },
            InternalParam::Cable { module, port } => match module.upgrade() {
                Some(module) => Param::Cable {
                    module: module.get_id().clone(),
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
            InternalParam::Volts { value } => Some(*value),
            InternalParam::Hz { value } => Some(((*value).max(0.0) / 27.5).log2()),
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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, TS)]
#[ts(export, export_to = "../../modular_web/src/types/generated/")]
#[serde(tag = "param_type", rename_all = "kebab-case")]
pub enum Param {
    Value { value: f32 },
    Hz { value: f32 },
    Note { value: u8 },
    Cable { module: String, port: String },
    Track { track: String },
    Disconnected,
}

impl Param {
    pub fn to_internal_param(&self, patch: &Patch) -> InternalParam {
        match self {
            Param::Value { value } => InternalParam::Volts { value: *value },
            Param::Hz { value } => InternalParam::Hz { value: *value },
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
    /// Normalized time in the range [0.0, 1.0]
    pub time: f32,
    pub param: InternalParam,
}

impl InternalKeyframe {
    pub fn new(id: String, track_id: String, time: f32, param: InternalParam) -> Self {
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

#[derive(Debug, Clone, Copy, PartialOrd, PartialEq, Ord, Eq, Hash, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../modular_web/src/types/generated/")]
#[serde(rename_all = "camelCase")]
pub enum InterpolationType {
    Linear,
    Step,
    Cubic,
    Exponential,
}

impl Default for InterpolationType {
    fn default() -> Self {
        InterpolationType::Linear
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename = "TrackKeyframe")]
#[ts(export, export_to = "../../modular_web/src/types/generated/")]
pub struct Keyframe {
    pub id: String,
    pub track_id: String,
    /// Normalized time in the range [0.0, 1.0]
    pub time: f32,
    pub param: Param,
}

impl Keyframe {
    pub fn new(id: String, track_id: String, time: f32, param: Param) -> Self {
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

fn normalize_playhead_value_to_t(value: f32) -> f32 {
    // Map [-5.0, 5.0] linearly to [0.0, 1.0]
    ((value + 5.0) / 10.0).clamp(0.0, 1.0)
}

struct InnerTrack {
    interpolation_type: InterpolationType,
    keyframes: Vec<InternalKeyframe>,
}

impl InnerTrack {
    fn new(interpolation_type: InterpolationType) -> Self {
        InnerTrack {
            interpolation_type,
            keyframes: Vec::new(),
        }
    }

    fn set_interpolation_type(&mut self, interpolation_type: InterpolationType) {
        self.interpolation_type = interpolation_type;
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
        self.keyframes.sort_by(|a, b| {
            a.time
                .partial_cmp(&b.time)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    pub fn remove_keyframe(&mut self, id: String) -> Option<InternalKeyframe> {
        match self.keyframes.iter().position(|k| k.id == id) {
            Some(idx) => Some(self.keyframes.remove(idx)),
            None => None,
        }
    }

    /// Tick the track for the current playhead value and return the interpolated sample.
    ///
    /// `playhead_value` is expected to be in the range [-5.0, 5.0]. It will be
    /// mapped linearly to a normalized time in [0.0, 1.0].
    pub fn tick(&mut self, playhead_value: Option<f32>) -> Option<f32> {
        let playhead_value = playhead_value?;
        if self.keyframes.is_empty() {
            return None;
        }

        let t = normalize_playhead_value_to_t(playhead_value);

        // Single keyframe: always return its value
        if self.keyframes.len() == 1 {
            return self.keyframes[0].param.get_value_optional();
        }

        // Clamp to first/last keyframe times
        let first = &self.keyframes[0];
        if t <= first.time {
            return first.param.get_value_optional();
        }
        let last = self.keyframes.last().unwrap();
        if t >= last.time {
            return last.param.get_value_optional();
        }

        // Find the segment [curr, next] such that curr.time <= t <= next.time
        let mut idx = 0usize;
        for i in 0..(self.keyframes.len() - 1) {
            let curr = &self.keyframes[i];
            let next = &self.keyframes[i + 1];
            if t >= curr.time && t <= next.time {
                idx = i;
                break;
            }
        }

        let curr = &self.keyframes[idx];
        let next = &self.keyframes[idx + 1];

        let curr_value = curr.param.get_value_optional()?;
        let next_value = next.param.get_value_optional()?;

        let time_range = (next.time - curr.time).max(f32::EPSILON);
        let mut local_t = (t - curr.time) / time_range;
        local_t = local_t.clamp(0.0, 1.0);

        let interpolated = match self.interpolation_type {
            InterpolationType::Linear => curr_value + (next_value - curr_value) * local_t,
            InterpolationType::Step => curr_value,
            InterpolationType::Cubic => {
                let t2 = if local_t < 0.5 {
                    2.0 * local_t * local_t
                } else {
                    1.0 - (-2.0 * local_t + 2.0).powi(2) / 2.0
                };
                curr_value + (next_value - curr_value) * t2
            }
            InterpolationType::Exponential => {
                if curr_value > 0.0 && next_value > 0.0 {
                    curr_value * (next_value / curr_value).powf(local_t)
                } else {
                    curr_value + (next_value - curr_value) * local_t
                }
            }
        };

        Some(interpolated)
    }
}

pub struct InternalTrack {
    id: String,
    inner_track: Mutex<InnerTrack>,
    playhead_param: Mutex<InternalParam>,
    sample: Mutex<Option<f32>>,
}

impl InternalTrack {
    pub fn new(
        id: String,
        playhead_param: InternalParam,
        interpolation_type: InterpolationType,
    ) -> Self {
        InternalTrack {
            id,
            inner_track: Mutex::new(InnerTrack::new(interpolation_type)),
            playhead_param: Mutex::new(playhead_param),
            sample: Mutex::new(None),
        }
    }

    pub fn configure(&self, playhead_param: InternalParam, interpolation_type: InterpolationType) {
        {
            let mut inner = self
                .inner_track
                .try_lock_for(Duration::from_millis(10))
                .unwrap();
            inner.set_interpolation_type(interpolation_type);
        }
        *self
            .playhead_param
            .try_lock_for(Duration::from_millis(10))
            .unwrap() = playhead_param;
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

    pub fn tick(&self) {
        let playhead_value = self
            .playhead_param
            .try_lock_for(Duration::from_millis(10))
            .unwrap()
            .get_value_optional();

        let sample = self
            .inner_track
            .try_lock_for(Duration::from_millis(10))
            .unwrap()
            .tick(playhead_value);

        *(self.sample.try_lock_for(Duration::from_millis(10)).unwrap()) = sample;
    }

    pub fn get_value_optional(&self) -> Option<f32> {
        *self.sample.try_lock_for(Duration::from_millis(10)).unwrap()
    }

    pub fn to_track(&self) -> Track {
        let inner_track = self
            .inner_track
            .try_lock_for(Duration::from_millis(10))
            .unwrap();
        let playhead_param = self
            .playhead_param
            .try_lock_for(Duration::from_millis(10))
            .unwrap();
        Track {
            id: self.id.clone(),
            playhead: playhead_param.to_param(),
            interpolation_type: inner_track.interpolation_type,
            keyframes: inner_track
                .keyframes
                .iter()
                .map(|k| k.to_keyframe())
                .collect(),
        }
    }
}

pub type TrackMap = HashMap<String, Arc<InternalTrack>>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../modular_web/src/types/generated/")]
pub struct Track {
    pub id: String,
    /// Parameter controlling the playhead position in the range [-5.0, 5.0]
    pub playhead: Param,
    /// Interpolation type applied to all keyframes in this track
    pub interpolation_type: InterpolationType,
    pub keyframes: Vec<Keyframe>,
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../modular_web/src/types/generated/")]
pub struct ParamSchema {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../modular_web/src/types/generated/")]
pub struct OutputSchema {
    pub name: String,
    pub description: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub default: bool,
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../modular_web/src/types/generated/")]
pub struct ModuleSchema {
    pub name: String,
    pub description: String,
    pub params: Vec<ParamSchema>,
    pub outputs: Vec<OutputSchema>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../modular_web/src/types/generated/")]
pub struct ModuleState {
    pub id: String,
    pub module_type: String,
    pub params: HashMap<String, Param>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../modular_web/src/types/generated/")]
pub struct PatchGraph {
    pub modules: Vec<ModuleState>,
    pub tracks: Vec<Track>,
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
        assert!(
            (current - target).abs() < 0.01,
            "Expected {} to be close to {}",
            current,
            target
        );
    }

    #[test]
    fn test_smooth_value_no_change_at_target() {
        let target = 5.0;
        let current = 5.0;
        let result = smooth_value(current, target);
        assert!(
            (result - target).abs() < 0.0001,
            "Value at target should stay at target"
        );
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
        let param = InternalParam::Volts { value: 3.5 };
        assert!((param.get_value() - 3.5).abs() < 0.0001);
    }

    #[test]
    fn test_internal_param_value_get_value_or() {
        let param = InternalParam::Volts { value: 2.0 };
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
        let internal = InternalParam::Volts { value: 1.5 };
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
        let a = InternalParam::Volts { value: 1.0 };
        let b = InternalParam::Volts { value: 1.0 };
        let c = InternalParam::Volts { value: 2.0 };
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
        let value = InternalParam::Volts { value: 60.0 };
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
            port: "output".to_string(),
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
            0.5,
            Param::Value { value: 1.0 },
        );
        assert_eq!(kf.id, "kf-1");
        assert_eq!(kf.track_id, "track-1");
        assert_eq!(kf.time, 0.5);
    }

    #[test]
    fn test_keyframe_get_id() {
        let kf = Keyframe::new(
            "kf-abc".to_string(),
            "track-1".to_string(),
            0.1,
            Param::Value { value: 2.0 },
        );
        assert_eq!(kf.get_id(), &"kf-abc".to_string());
    }

    #[test]
    fn test_keyframe_partial_ord() {
        let kf1 = Keyframe::new(
            "kf-1".to_string(),
            "track-1".to_string(),
            0.1,
            Param::Value { value: 1.0 },
        );
        let kf2 = Keyframe::new(
            "kf-2".to_string(),
            "track-1".to_string(),
            0.2,
            Param::Value { value: 2.0 },
        );
        assert!(kf1 < kf2);
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
        let patch = PatchGraph {
            modules: vec![],
            tracks: vec![],
        };
        assert!(patch.modules.is_empty());
        assert!(patch.tracks.is_empty());
    }

    #[test]
    fn test_patch_graph_with_modules() {
        let state = ModuleState {
            id: "test-1".to_string(),
            module_type: "signal".to_string(),
            params: HashMap::new(),
        };
        let patch = PatchGraph {
            modules: vec![state],
            tracks: vec![],
        };
        assert_eq!(patch.modules.len(), 1);
    }

    #[test]
    fn test_patch_graph_serialization_roundtrip() {
        let mut params = HashMap::new();
        params.insert("source".to_string(), Param::Disconnected);

        let original = PatchGraph {
            modules: vec![ModuleState {
                id: "signal-1".to_string(),
                module_type: "signal".to_string(),
                params,
            }],
            tracks: vec![],
        };

        let json = serde_json::to_string(&original).unwrap();
        let restored: PatchGraph = serde_json::from_str(&json).unwrap();
        assert_eq!(original, restored);
    }

    // Tests for PortSchema
    #[test]
    fn test_port_schema_equality() {
        let a = OutputSchema {
            name: "output".to_string(),
            description: "Main output".to_string(),
            default: false,
        };
        let b = OutputSchema {
            name: "output".to_string(),
            description: "Main output".to_string(),
            default: false,
        };
        assert_eq!(a, b);
    }

    #[test]
    fn test_port_schema_default_serialization() {
        // Test that default: true is included in JSON
        let schema = OutputSchema {
            name: "output".to_string(),
            description: "Main output".to_string(),
            default: true,
        };
        let json = serde_json::to_string(&schema).unwrap();
        assert!(
            json.contains("\"default\":true"),
            "JSON should contain default:true"
        );

        // Test that default: false is omitted from JSON
        let schema_no_default = OutputSchema {
            name: "output".to_string(),
            description: "Main output".to_string(),
            default: false,
        };
        let json = serde_json::to_string(&schema_no_default).unwrap();
        assert!(
            !json.contains("default"),
            "JSON should not contain default field when false"
        );
    }

    #[test]
    fn test_port_schema_deserialization_without_default() {
        // Backward compatibility test - old JSON without default field
        let json = r#"{"name":"output","description":"Main output"}"#;
        let schema: OutputSchema = serde_json::from_str(json).unwrap();
        assert_eq!(schema.name, "output");
        assert_eq!(schema.description, "Main output");
        assert_eq!(
            schema.default, false,
            "default should be false when not present"
        );
    }

    #[test]
    fn test_port_schema_deserialization_with_default() {
        // Test deserialization with default: true
        let json = r#"{"name":"output","description":"Main output","default":true}"#;
        let schema: OutputSchema = serde_json::from_str(json).unwrap();
        assert_eq!(schema.default, true);

        // Test deserialization with default: false
        let json = r#"{"name":"output","description":"Main output","default":false}"#;
        let schema: OutputSchema = serde_json::from_str(json).unwrap();
        assert_eq!(schema.default, false);
    }

    // Tests for ModuleSchema
    #[test]
    fn test_module_schema_creation() {
        let schema = ModuleSchema {
            name: "sine-oscillator".to_string(),
            description: "A sine wave generator".to_string(),
            params: vec![ParamSchema {
                name: "freq".to_string(),
                description: "Frequency in v/oct".to_string(),
            }],
            outputs: vec![OutputSchema {
                name: "output".to_string(),
                description: "Audio output".to_string(),
                default: false,
            }],
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
    fn test_keyframe_time_serialization() {
        let kf = Keyframe::new(
            "test".to_string(),
            "track".to_string(),
            0.5,
            Param::Value { value: 1.0 },
        );
        let json = serde_json::to_string(&kf).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["time"].as_f64().unwrap(), 0.5f64);
    }

    #[test]
    fn test_track_serialization() {
        let track = Track {
            id: "test-track".to_string(),
            playhead: Param::Value { value: 0.0 },
            interpolation_type: InterpolationType::Linear,
            keyframes: vec![],
        };
        let json = serde_json::to_string(&track).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["id"].as_str().unwrap(), "test-track");
        assert_eq!(v["interpolation_type"].as_str().unwrap(), "linear");
    }
}
