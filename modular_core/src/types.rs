use anyhow::Result;
use parking_lot::Mutex;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;
use std::{
    collections::HashMap,
    sync::{self, Arc},
};
use ts_rs::TS;

use crate::patch::Patch;

lazy_static! {
    pub static ref ROOT_ID: String = "root".into();
    pub static ref ROOT_OUTPUT_PORT: String = "output".into();
    pub static ref ROOT_CLOCK_ID: String = "root_clock".into();
}

pub trait Sampleable: Send + Sync {
    fn get_id(&self) -> &String;
    fn tick(&self) -> ();
    fn update(&self) -> ();
    fn get_sample(&self, port: &String) -> Result<f32>;
    fn get_module_type(&self) -> String;
    fn try_update_params(&self, params: serde_json::Value) -> Result<()>;
    fn connect(&self, patch: &Patch);
}

pub trait Module {
    fn install_constructor(map: &mut HashMap<String, SampleableConstructor>);
    fn get_schema() -> ModuleSchema;

    /// Register this module's parameter validator in the provided map.
    ///
    /// The key is the module type string (e.g. "noise"). The value is a function
    /// that attempts to deserialize a JSON params object into the module's concrete
    /// `*Params` type.
    fn install_params_validator(map: &mut HashMap<String, ParamsValidator>);

    /// Validate a JSON params object by attempting to parse it as the module's concrete
    /// params type.
    ///
    /// This is intended for server-side patch validation before applying the patch.
    fn validate_params_json(params: &serde_json::Value) -> anyhow::Result<()>;
}

/// Function pointer type used to validate a module's `ModuleState.params`.
///
/// The validator should return Ok if deserialization into the module's concrete params type succeeds.
pub type ParamsValidator = fn(&serde_json::Value) -> anyhow::Result<()>;

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

pub trait Connect {
    fn connect(&mut self, patch: &Patch);
}

#[derive(Clone, Debug, Default, Serialize, TS, JsonSchema)]
#[serde(
    tag = "type",
    content = "data",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[ts(
    export,
    export_to = "../../modular_web/src/types/generated/",
    rename_all = "camelCase",
    tag = "type"
)]
pub enum Signal {
    #[ts(as = "f32")]
    Volts { value: f32 },
    Cable {
        module: String,
        #[ts(skip)]
        #[serde(skip)]
        module_ptr: sync::Weak<Box<dyn Sampleable>>,
        port: String,
    },
    Track {
        track: String,
        #[ts(skip)]
        #[serde(skip)]
        track_ptr: sync::Weak<Track>,
    },
    #[default]
    Disconnected,
}

// Custom serde deserialization to allow a bare number as shorthand for volts.
//
// Examples accepted:
// - 0.5                      -> Signal::Volts { value: 0.5 }
// - {"type":"volts","value":0.5} -> Signal::Volts { value: 0.5 }
//
// Note: This keeps the existing *serialized* representation unchanged (still tagged objects).
// If you also want JSON Schema / TS exports to reflect this shorthand, you'll need a custom
// JsonSchema/TS representation as well.
impl<'de> Deserialize<'de> for Signal {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum SignalDe {
            Number(f32),
            Tagged(SignalTagged),
        }

        #[derive(Deserialize)]
        #[serde(
            tag = "type",
            rename_all = "camelCase",
            rename_all_fields = "camelCase"
        )]
        enum SignalTagged {
            Volts { value: f32 },
            Cable { module: String, port: String },
            Track { track: String },
            Disconnected,
        }

        match SignalDe::deserialize(deserializer)? {
            SignalDe::Number(value) => Ok(Signal::Volts { value }),
            SignalDe::Tagged(tagged) => Ok(match tagged {
                SignalTagged::Volts { value } => Signal::Volts { value },
                SignalTagged::Cable { module, port } => Signal::Cable {
                    module,
                    module_ptr: sync::Weak::new(),
                    port,
                },
                SignalTagged::Track { track } => Signal::Track {
                    track,
                    track_ptr: sync::Weak::new(),
                },
                SignalTagged::Disconnected => Signal::Disconnected,
            }),
        }
    }
}

impl Signal {
    pub fn get_value(&self) -> f32 {
        self.get_value_or(0.0)
    }
    pub fn get_value_or(&self, default: f32) -> f32 {
        self.get_value_optional().unwrap_or(default)
    }
    fn get_value_optional(&self) -> Option<f32> {
        match self {
            Signal::Volts { value } => Some(*value),
            Signal::Cable {
                module_ptr, port, ..
            } => match module_ptr.upgrade() {
                Some(module_ptr) => match module_ptr.get_sample(port) {
                    Ok(sample) => Some(sample),
                    Err(_) => None,
                },
                None => None,
            },
            Signal::Track { track_ptr, .. } => match track_ptr.upgrade() {
                Some(track_ptr) => match track_ptr.get_value_optional() {
                    Some(sample) => Some(sample),
                    None => None,
                },
                None => None,
            },
            Signal::Disconnected => None,
        }
    }
}

impl Connect for Signal {
    fn connect(&mut self, patch: &Patch) {
        match self {
            Signal::Cable {
                module,
                module_ptr,
                port: _,
            } => {
                if let Some(sampleable) = patch.sampleables.get(module) {
                    *module_ptr = Arc::downgrade(sampleable);
                }
            }
            Signal::Track { track, track_ptr } => {
                if let Some(track_obj) = patch.tracks.get(track) {
                    *track_ptr = Arc::downgrade(track_obj);
                }
            }
            _ => {}
        }
    }
}

impl PartialEq for Box<dyn Sampleable> {
    fn eq(&self, other: &Self) -> bool {
        self.get_id() == other.get_id()
    }
}

impl PartialEq for Signal {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Signal::Volts { value: value1 }, Signal::Volts { value: value2 }) => {
                *value1 == *value2
            }
            (
                Signal::Cable {
                    module: module_1,
                    module_ptr: module_ptr_1,
                    port: port_1,
                },
                Signal::Cable {
                    module: module_2,
                    module_ptr: module_ptr_2,
                    port: port_2,
                },
            ) => {
                module_ptr_1.upgrade() == module_ptr_2.upgrade()
                    && port_1 == port_2
                    && module_1 == module_2
            }
            (
                Signal::Track {
                    track: track_1,
                    track_ptr: track_ptr_1,
                },
                Signal::Track {
                    track: track_2,
                    track_ptr: track_ptr_2,
                },
            ) => track_ptr_1.upgrade() == track_ptr_2.upgrade() && track_1 == track_2,
            (Signal::Disconnected, Signal::Disconnected) => true,
            _ => false,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(
    export,
    export_to = "../../modular_web/src/types/generated/",
    rename_all = "camelCase"
)]
pub struct TrackKeyframe {
    pub id: String,
    pub track_id: String,
    /// Normalized time in the range [0.0, 1.0]
    pub time: f32,
    pub signal: Signal,
}

impl Connect for TrackKeyframe {
    fn connect(&mut self, patch: &Patch) {
        self.signal.connect(patch);
    }
}

impl PartialOrd for TrackKeyframe {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.time.partial_cmp(&other.time)
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
struct InnerTrack {
    interpolation_type: InterpolationType,
    keyframes: Vec<TrackKeyframe>,
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

    // Keyframe should already be connected at this point
    pub fn add_keyframe(&mut self, keyframe: TrackKeyframe) {
        match self.keyframes.iter().position(|k| k.id == keyframe.id) {
            Some(idx) => {
                // Updating existing keyframe - check if time changed
                let old_time = self.keyframes[idx].time;
                self.keyframes[idx] = keyframe;

                // Only re-sort if the time changed and might affect ordering
                if old_time != self.keyframes[idx].time {
                    self.keyframes.sort_by(|a, b| {
                        a.time
                            .partial_cmp(&b.time)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    });
                }
            }
            None => {
                // New keyframe - insert in sorted position using binary search
                let insert_pos = self
                    .keyframes
                    .binary_search_by(|k| {
                        k.time
                            .partial_cmp(&keyframe.time)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .unwrap_or_else(|pos| pos);
                self.keyframes.insert(insert_pos, keyframe);
            }
        }
    }

    pub fn remove_keyframe(&mut self, id: String) -> Option<TrackKeyframe> {
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
            return self.keyframes[0].signal.get_value_optional();
        }

        // Clamp to first/last keyframe times
        let first = &self.keyframes[0];
        if t <= first.time {
            return first.signal.get_value_optional();
        }
        let last = self.keyframes.last().unwrap();
        if t >= last.time {
            return last.signal.get_value_optional();
        }

        // Find the segment [curr, next] such that curr.time <= t <= next.time
        // Use partition_point to find the first keyframe with time > t
        // Then back up one to get the last keyframe with time <= t
        let idx = self.keyframes.partition_point(|kf| kf.time <= t);

        // partition_point returns the index of the first element > t
        // So idx-1 is the last element <= t, which is the start of our interpolation segment
        let idx = if idx > 0 { idx - 1 } else { 0 };

        // Ensure idx is valid for the segment [idx, idx+1]
        let idx = idx.min(self.keyframes.len() - 2);

        let curr = &self.keyframes[idx];
        let next = &self.keyframes[idx + 1];

        let curr_value = curr.signal.get_value_optional()?;
        let next_value = next.signal.get_value_optional()?;

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

impl Connect for InnerTrack {
    fn connect(&mut self, patch: &Patch) {
        for keyframe in &mut self.keyframes {
            keyframe.connect(patch);
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(
    export,
    export_to = "../../modular_web/src/types/generated/",
    rename_all = "camelCase",
    rename = "Track"
)]
pub struct TrackProxy {
    pub id: String,
    /// Parameter controlling the playhead position in the range [-5.0, 5.0]
    pub playhead: Signal,
    /// Interpolation type applied to all keyframes in this track
    pub interpolation_type: InterpolationType,
    pub keyframes: Vec<TrackKeyframe>,
}

impl TryFrom<&TrackProxy> for Track {
    type Error = &'static str;
    fn try_from(proxy: &TrackProxy) -> Result<Self, Self::Error> {
        let track = Self::new(
            proxy.id.clone(),
            proxy.playhead.clone(),
            proxy.interpolation_type.clone(),
        );
        track.inner_track.lock().keyframes = proxy.keyframes.clone();
        Ok(track)
    }
}

// Required for `#[serde(from = "TrackProxy", into = "TrackProxy")]`
impl From<TrackProxy> for Track {
    fn from(proxy: TrackProxy) -> Self {
        // Currently infallible; keep a `From` impl for serde ergonomics.
        // NOTE: call the explicit `TryFrom<&TrackProxy>` impl to avoid the blanket
        // `TryFrom<U> for T where U: Into<T>` which would recurse back into this `From`.
        Self::try_from(&proxy).expect("TrackProxy -> Track conversion should be infallible")
    }
}

impl TryFrom<&Track> for TrackProxy {
    type Error = std::convert::Infallible;
    fn try_from(track: &Track) -> Result<Self, Self::Error> {
        let inner_track = track.inner_track.lock();
        let track_proxy = TrackProxy {
            id: track.id.clone(),
            playhead: track.playhead.lock().clone(),
            interpolation_type: inner_track.interpolation_type.clone(),
            keyframes: inner_track.keyframes.clone(),
        };
        Ok(track_proxy)
    }
}

// Required so `Track: Into<TrackProxy>` is satisfied (used by serde `into = "TrackProxy"`).
impl From<Track> for TrackProxy {
    fn from(track: Track) -> Self {
        // Avoid locking by consuming the mutexes.
        let Track {
            id,
            inner_track,
            playhead,
            sample: _,
        } = track;

        let inner_track = inner_track.into_inner();
        TrackProxy {
            id,
            playhead: playhead.into_inner(),
            interpolation_type: inner_track.interpolation_type,
            keyframes: inner_track.keyframes,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(from = "TrackProxy", into = "TrackProxy")]
pub struct Track {
    id: String,
    inner_track: Mutex<InnerTrack>,
    playhead: Mutex<Signal>,
    sample: Mutex<Option<f32>>,
}

impl Clone for Track {
    fn clone(&self) -> Self {
        Track {
            id: self.id.clone(),
            inner_track: Mutex::new(self.inner_track.try_lock().unwrap().clone()),
            playhead: Mutex::new(self.playhead.try_lock().unwrap().clone()),
            sample: Mutex::new(*self.sample.try_lock().unwrap()),
        }
    }
}

impl Track {
    pub fn new(id: String, playhead_param: Signal, interpolation_type: InterpolationType) -> Self {
        Track {
            id,
            inner_track: Mutex::new(InnerTrack::new(interpolation_type)),
            playhead: Mutex::new(playhead_param),
            sample: Mutex::new(None),
        }
    }

    pub fn configure(&self, playhead: Signal, interpolation_type: InterpolationType) {
        {
            let mut inner = self
                .inner_track
                .try_lock_for(Duration::from_millis(10))
                .unwrap();
            inner.set_interpolation_type(interpolation_type);
        }
        *self
            .playhead
            .try_lock_for(Duration::from_millis(10))
            .unwrap() = playhead;
    }

    pub fn add_keyframe(&self, keyframe: TrackKeyframe) {
        self.inner_track
            .try_lock_for(Duration::from_millis(10))
            .unwrap()
            .add_keyframe(keyframe)
    }

    pub fn remove_keyframe(&self, id: String) -> Option<TrackKeyframe> {
        self.inner_track
            .try_lock_for(Duration::from_millis(10))
            .unwrap()
            .remove_keyframe(id)
    }

    pub fn clear_keyframes(&self) {
        self.inner_track
            .try_lock_for(Duration::from_millis(10))
            .unwrap()
            .keyframes
            .clear();
    }

    pub fn tick(&self) {
        // Use try_lock in audio hot path to avoid timeout overhead
        // If we can't acquire locks, keep the previous sample value
        let playhead_value = match self.playhead.try_lock() {
            Some(guard) => guard.get_value_optional(),
            None => return, // Keep previous sample if locked
        };

        let sample = match self.inner_track.try_lock() {
            Some(mut guard) => guard.tick(playhead_value),
            None => return, // Keep previous sample if locked
        };

        if let Some(mut sample_guard) = self.sample.try_lock() {
            *sample_guard = sample;
        }
        // If sample lock fails, keep previous value (graceful degradation)
    }

    pub fn get_value_optional(&self) -> Option<f32> {
        // Use try_lock in audio hot path - return None if locked
        // sample is Mutex<Option<f32>>, so *guard is Option<f32>
        self.sample.try_lock().and_then(|guard| *guard)
    }

    pub fn connect(&self, patch: &Patch) {
        self.playhead
            .try_lock_for(Duration::from_millis(10))
            .unwrap()
            .connect(patch);
        self.inner_track
            .try_lock_for(Duration::from_millis(10))
            .unwrap()
            .connect(patch);
    }
}

impl PartialEq for Track {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

pub type TrackMap = HashMap<String, Arc<Track>>;

#[derive(
    Debug, Default, Clone, Copy, PartialOrd, PartialEq, Ord, Eq, Hash, Serialize, Deserialize, TS,
)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
#[ts(export, export_to = "../../modular_web/src/types/generated/")]
pub enum InterpolationType {
    #[default]
    Linear,
    Step,
    Cubic,
    Exponential,
}

fn normalize_playhead_value_to_t(value: f32) -> f32 {
    // Map [-5.0, 5.0] linearly to [0.0, 1.0]
    ((value + 5.0) / 10.0).clamp(0.0, 1.0)
}

pub enum Seq {
    Fast,
    Slow,
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../modular_web/src/types/generated/")]
pub struct SignalParamSchema {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../modular_web/src/types/generated/")]
pub struct OutputSchema {
    pub name: String,
    pub description: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub default: bool,
}

pub trait OutputStruct: Default + Send + Sync + 'static {
    fn copy_from(&mut self, other: &Self);
    fn get_sample(&self, port: &str) -> Option<f32>;
    fn schemas() -> Vec<OutputSchema>
    where
        Self: Sized;
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../modular_web/src/types/generated/")]
pub struct ModuleSchema {
    pub name: String,
    pub description: String,
    #[ts(type = "unknown")]
    pub params_schema: schemars::Schema,
    pub outputs: Vec<OutputSchema>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../modular_web/src/types/generated/")]
pub struct ModuleState {
    pub id: String,
    pub module_type: String,
    #[serde(default)]
    #[ts(type = "Record<string, unknown>")]
    pub params: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq, Hash)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[ts(export, export_to = "../../modular_web/src/types/generated/")]
pub enum ScopeItem {
    ModuleOutput {
        module_id: String,
        port_name: String,
    },
    Track {
        track_id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../modular_web/src/types/generated/")]
pub struct PatchGraph {
    pub modules: Vec<ModuleState>,
    #[serde(default)]
    pub tracks: Vec<TrackProxy>,
    #[serde(default)]
    pub scopes: Vec<ScopeItem>,
    #[serde(default)]
    pub factories: Vec<ModuleState>,
}

pub type SampleableConstructor = Box<dyn Fn(&String, f32) -> Result<Arc<Box<dyn Sampleable>>>>;
