use napi::Env;
use napi::Result;
use napi::bindgen_prelude::{FromNapiValue, Object, ToNapiValue};
use napi_derive::napi;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt::Debug;
use std::ops::{Add, Deref, Div, Mul, Sub};
use std::{
    collections::HashMap,
    sync::{self, Arc},
};

use crate::patch::Patch;

lazy_static! {
    pub static ref ROOT_ID: String = "root".into();
    pub static ref ROOT_OUTPUT_PORT: String = "output".into();
    pub static ref ROOT_CLOCK_ID: String = "root_clock".into();
}

pub trait MessageHandler {
    fn handled_message_tags(&self) -> &'static [MessageTag] {
        &[]
    }

    fn handle_message(&self, _message: &Message) -> Result<()> {
        Ok(())
    }
}

pub trait Sampleable: MessageHandler + Send + Sync {
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
    fn validate_params_json(params: &serde_json::Value) -> napi::Result<()>;
}

/// Function pointer type used to validate a module's `ModuleState.params`.
///
/// The validator should return Ok if deserialization into the module's concrete params type succeeds.
pub type ParamsValidator = fn(&serde_json::Value) -> napi::Result<()>;

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

#[derive(Debug, Default, Clone, Copy)]
pub struct Clickless {
    value: f32,
}
impl Clickless {
    pub fn update(&mut self, input: f32) {
        self.value = smooth_value(self.value, input);
    }
}

impl From<Clickless> for f32 {
    fn from(clickless: Clickless) -> Self {
        clickless.value
    }
}

impl From<f32> for Clickless {
    fn from(value: f32) -> Self {
        Clickless { value }
    }
}

impl Deref for Clickless {
    type Target = f32;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl Add<f32> for Clickless {
    type Output = f32;
    fn add(self, rhs: f32) -> Self::Output {
        self.value + rhs
    }
}

impl Add for Clickless {
    type Output = f32;
    fn add(self, rhs: Self) -> Self::Output {
        self.value + rhs.value
    }
}

impl Sub<f32> for Clickless {
    type Output = f32;
    fn sub(self, rhs: f32) -> Self::Output {
        self.value - rhs
    }
}

impl Sub for Clickless {
    type Output = f32;
    fn sub(self, rhs: Self) -> Self::Output {
        self.value - rhs.value
    }
}

impl Mul<f32> for Clickless {
    type Output = f32;
    fn mul(self, rhs: f32) -> Self::Output {
        self.value * rhs
    }
}

impl Mul for Clickless {
    type Output = f32;
    fn mul(self, rhs: Self) -> Self::Output {
        self.value * rhs.value
    }
}

impl Div<f32> for Clickless {
    type Output = f32;
    fn div(self, rhs: f32) -> Self::Output {
        self.value / rhs
    }
}

impl Div for Clickless {
    type Output = f32;
    fn div(self, rhs: Self) -> Self::Output {
        self.value / rhs.value
    }
}

pub trait Connect {
    fn connect(&mut self, patch: &Patch);
}

#[derive(Clone, Debug, Default, JsonSchema)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Signal {
    Volts {
        value: f32,
    },
    Cable {
        module: String,
        #[serde(skip)]
        module_ptr: std::sync::Weak<Box<dyn Sampleable>>,
        port: String,
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
                SignalTagged::Track { track } => Signal::Cable {
                    module: track,
                    module_ptr: sync::Weak::new(),
                    port: "output".to_string(),
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
    pub fn get_value_optional(&self) -> Option<f32> {
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
            (Signal::Disconnected, Signal::Disconnected) => true,
            _ => false,
        }
    }
}

#[derive(
    Debug,
    Default,
    Clone,
    Copy,
    PartialOrd,
    PartialEq,
    Ord,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    JsonSchema,
)]
#[serde(rename_all = "camelCase")]
pub enum InterpolationType {
    #[default]
    Linear,
    Step,
    SineIn,
    SineOut,
    SineInOut,
    QuadIn,
    QuadOut,
    QuadInOut,
    CubicIn,
    CubicOut,
    CubicInOut,
    QuartIn,
    QuartOut,
    QuartInOut,
    QuintIn,
    QuintOut,
    QuintInOut,
    ExpoIn,
    ExpoOut,
    ExpoInOut,
    CircIn,
    CircOut,
    CircInOut,
    BounceIn,
    BounceOut,
    BounceInOut,
}

pub enum Seq {
    Fast,
    Slow,
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignalParamSchema {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[napi(object)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SchemaContainer {
    pub schema: schemars::Schema,
}

impl ToNapiValue for SchemaContainer {
    unsafe fn to_napi_value(
        env: napi::sys::napi_env,
        val: Self,
    ) -> napi::Result<napi::sys::napi_value> {
        unsafe {
            return ToNapiValue::to_napi_value(
                env,
                serde_json::to_value(val.schema).map_err(|e| {
                    napi::Error::from_reason(format!("Failed to serialize schema: {}", e))
                })?,
            );
        }
    }
}

impl FromNapiValue for SchemaContainer {
    unsafe fn from_napi_value(
        env: napi::sys::napi_env,
        napi_val: napi::sys::napi_value,
    ) -> napi::Result<Self> {
        unsafe {
            FromNapiValue::from_napi_value(env, napi_val).and_then(|js_value: Object| {
                Ok(SchemaContainer {
                    schema: Env::from_raw(env).from_js_value(js_value).map_err(|e| {
                        napi::Error::from_reason(format!("Failed to parse schema: {}", e))
                    })?,
                })
            })
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[napi(object)]
pub struct PositionalArg {
    pub name: String,
    pub optional: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[napi(object)]
pub struct ModuleSchema {
    pub name: String,
    pub description: String,
    #[napi(ts_type = "Record<string, unknown>")]
    pub params_schema: SchemaContainer,
    pub outputs: Vec<OutputSchema>,
    pub positional_args: Vec<PositionalArg>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[napi(object)]
pub struct ModuleState {
    pub id: String,
    pub module_type: String,
    pub id_is_explicit: Option<bool>,
    // #[serde(default)]
    pub params: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[napi]
pub enum ScopeItem {
    ModuleOutput {
        module_id: String,
        port_name: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[napi(object)]
pub struct Scope {
    pub item: ScopeItem,
    pub ms_per_frame: u32,
    pub trigger_threshold: Option<i32>,
}

#[derive(Debug, Clone, PartialEq)]
// #[serde(rename_all = "camelCase")]
#[napi(object)]
pub struct PatchGraph {
    pub modules: Vec<ModuleState>,
    pub module_id_remaps: Option<Vec<ModuleIdRemap>>,
    // #[serde(default)]
    pub scopes: Vec<Scope>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[napi(object)]
pub struct ModuleIdRemap {
    pub from: String,
    pub to: String,
}

pub type SampleableConstructor = Box<dyn Fn(&String, f32) -> Result<Arc<Box<dyn Sampleable>>>>;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ClockMessages {
    Start,
    Stop,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, EnumTag, Serialize, Deserialize)]
#[serde(
    tag = "type",
    content = "data",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Message {
    Clock(ClockMessages),
    MidiNote(u8, bool), // (note number, on/off)
    MidiCC(u8, u8),     // (cc number, value)
}
