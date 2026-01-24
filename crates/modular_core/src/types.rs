use napi::Env;
use napi::Result;
use napi::bindgen_prelude::{FromNapiValue, Object, ToNapiValue};
use napi_derive::napi;
use regex::Regex;
use rust_music_theory::note::{Notes, Pitch};
use rust_music_theory::scale::Scale;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::borrow::Cow;
use std::fmt::Debug;
use std::ops::{Add, Deref, Div, Mul, Sub};
use std::result::Result as StdResult;
use std::{
    collections::HashMap,
    sync::{self, Arc},
};

use crate::patch::Patch;
use crate::poly::PolySignal;

lazy_static! {
    pub static ref ROOT_ID: String = "root".into();
    pub static ref ROOT_OUTPUT_PORT: String = "output".into();
    pub static ref ROOT_CLOCK_ID: String = "root_clock".into();
    static ref RE_HZ: Regex = Regex::new(r"^(-?\d*\.?\d+)hz$").unwrap();
    static ref RE_MIDI: Regex = Regex::new(r"^(-?\d*\.?\d+)m$").unwrap();
    static ref RE_SCALE: Regex = Regex::new(r"^(-?\d*\.?\d+)s\(([^:]+):([^)]+)\)$").unwrap();
    static ref RE_NOTE: Regex = Regex::new(r"^([A-Ga-g])([#b]?)(-?\d+)?$").unwrap();
}

pub trait MessageHandler {
    fn handled_message_tags(&self) -> &'static [MessageTag] {
        &[]
    }

    fn handle_message(&self, _message: &Message) -> Result<()> {
        Ok(())
    }
}

pub trait StatefulModule {
    fn get_state(&self) -> Option<serde_json::Value> {
        None
    }
}

pub trait Sampleable: MessageHandler + Send + Sync {
    fn get_id(&self) -> &String;
    fn tick(&self) -> ();
    fn update(&self) -> ();
    /// Get polyphonic sample output for a port.
    fn get_poly_sample(&self, port: &String) -> Result<PolySignal>;
    fn get_module_type(&self) -> String;
    fn try_update_params(&self, params: serde_json::Value) -> Result<()>;
    fn connect(&self, patch: &Patch);
    fn get_state(&self) -> Option<serde_json::Value> {
        None
    }
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

struct ParsedNote {
    pitch: Pitch,
    octave: i32,
}

fn parse_note_str(s: &str) -> StdResult<ParsedNote, String> {
    let caps = RE_NOTE
        .captures(s)
        .ok_or("Invalid note format".to_string())?;
    let name = &caps[1];
    let acc = &caps[2];
    let octave: i32 = caps
        .get(3)
        .map(|m| m.as_str().parse().unwrap_or(3))
        .unwrap_or(3);

    let pitch_str = format!("{}{}", name, acc);
    let pitch = Pitch::from_str(&pitch_str).ok_or("Invalid pitch".to_string())?;
    Ok(ParsedNote { pitch, octave })
}

fn parse_signal_string(s: &str) -> StdResult<f32, String> {
    if let Some(caps) = RE_HZ.captures(s) {
        let hz: f32 = caps[1]
            .parse()
            .map_err(|_| "Invalid frequency number".to_string())?;
        if hz <= 0.0 {
            return Err("Frequency must be positive".to_string());
        }
        let volts = (hz / 55.0).log2();
        return Ok(volts);
    }

    if let Some(caps) = RE_MIDI.captures(s) {
        let midi: f32 = caps[1]
            .parse()
            .map_err(|_| "Invalid MIDI number".to_string())?;
        let volts = (midi - 33.0) / 12.0;
        return Ok(volts);
    }

    if let Some(caps) = RE_SCALE.captures(s) {
        let val: f32 = caps[1]
            .parse()
            .map_err(|_| "Invalid scale interval number".to_string())?;
        let root_str = &caps[2];
        let scale_str = &caps[3];

        let root_note = parse_note_str(root_str)?;
        let scale_def = format!("{} {}", root_note.pitch, scale_str);
        let scale =
            Scale::from_regex(&scale_def).map_err(|_| "Invalid scale definition".to_string())?;

        let interval_idx = val.floor() as i64;
        let cents = (val - interval_idx as f32) * 100.0;

        let notes = scale.notes();
        let note_len = notes.len();
        if note_len == 0 {
            return Err("Scale has no notes".to_string());
        }

        let effective_len = if note_len > 1 && notes[0].pitch == notes[note_len - 1].pitch {
            note_len - 1
        } else {
            note_len
        };
        let len = effective_len as i64;

        let scale_root_octave = notes[0].octave as i32;

        let (octave_shift, note_idx) = if interval_idx >= 0 {
            ((interval_idx / len), (interval_idx % len) as usize)
        } else {
            let abs_idx = (-interval_idx - 1) as i64;
            let octave_down = (abs_idx / len) + 1;
            let note_from_end = (abs_idx % len) as usize;
            (-octave_down, len as usize - 1 - note_from_end)
        };

        let base_note = &notes[note_idx];
        let relative_octave = (base_note.octave as i32) - scale_root_octave;
        let target_octave = (root_note.octave as i32) + relative_octave + (octave_shift as i32);

        let pc_val = base_note.pitch.into_u8();

        let midi = (target_octave as f32 + 2.0) * 12.0 + (pc_val as f32);
        let midi_with_cents = midi + (cents / 100.0);

        let volts = (midi_with_cents - 33.0) / 12.0;
        return Ok(volts);
    }

    if let Ok(note) = parse_note_str(s) {
        let pc_val = note.pitch.into_u8();
        let midi = (note.octave as f32 + 2.0) * 12.0 + (pc_val as f32);
        let volts = (midi - 33.0) / 12.0;
        return Ok(volts);
    }

    Err("Invalid signal format".to_string())
}

#[derive(Clone, Debug, Default)]
pub enum Signal {
    /// Static voltage value(s) - mono is just channels=1
    Volts(PolySignal),
    /// Cable connection to another module's output
    Cable {
        module: String,
        module_ptr: std::sync::Weak<Box<dyn Sampleable>>,
        port: String,
    },
    #[default]
    Disconnected,
}

// Custom serde deserialization to allow a bare number as shorthand for volts.
//
// Examples accepted:
// - 0.5                      -> Signal::Volts(PolySignal::mono(0.5))
// - [0.5, 1.0, 1.5]          -> Signal::Volts(PolySignal::poly(&[0.5, 1.0, 1.5]))
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
            Number(f64),
            NumberArray(Vec<f64>),
            String(String),
            StringArray(Vec<String>),
            Tagged(SignalTagged),
        }

        #[derive(Deserialize)]
        #[serde(
            tag = "type",
            rename_all = "camelCase",
            rename_all_fields = "camelCase"
        )]
        enum SignalTagged {
            Cable { module: String, port: String },
            Disconnected,
        }

        match SignalDe::deserialize(deserializer)? {
            SignalDe::Number(value) => Ok(Signal::Volts(PolySignal::mono(value as f32))),
            SignalDe::NumberArray(values) => Ok(Signal::Volts(PolySignal::poly(
                &values.into_iter().map(|v| v as f32).collect::<Vec<_>>(),
            ))),
            SignalDe::String(s) => parse_signal_string(&s)
                .map(|v| Signal::Volts(PolySignal::mono(v)))
                .map_err(serde::de::Error::custom),
            SignalDe::StringArray(items) => {
                let mut voltages = Vec::with_capacity(items.len());
                for item in items {
                    let v = parse_signal_string(&item).map_err(serde::de::Error::custom)?;
                    voltages.push(v);
                }
                Ok(Signal::Volts(PolySignal::poly(&voltages)))
            }
            SignalDe::Tagged(tagged) => Ok(match tagged {
                SignalTagged::Cable { module, port } => Signal::Cable {
                    module,
                    module_ptr: sync::Weak::new(),
                    port,
                },
                SignalTagged::Disconnected => Signal::Disconnected,
            }),
        }
    }
}

#[derive(JsonSchema)]
#[serde(untagged)]
#[allow(dead_code)]
enum SignalSchema {
    Number(f64),
    NumberArray(Vec<f64>),
    String(String),
    StringArray(Vec<String>),
    Tagged(SignalTaggedSchema),
}

#[derive(JsonSchema)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[allow(dead_code)]
enum SignalTaggedSchema {
    Cable { module: String, port: String },
    Disconnected,
}

impl JsonSchema for Signal {
    fn schema_name() -> Cow<'static, str> {
        Cow::Borrowed("Signal")
    }

    fn json_schema(r#gen: &mut schemars::SchemaGenerator) -> schemars::Schema {
        SignalSchema::json_schema(r#gen)
    }
}

impl Signal {
    /// Get the full polyphonic signal.
    pub fn get_poly_signal(&self) -> PolySignal {
        match self {
            Signal::Volts(poly) => *poly,
            Signal::Cable {
                module_ptr, port, ..
            } => match module_ptr.upgrade() {
                Some(module_ptr) => module_ptr.get_poly_sample(port).unwrap_or_default(),
                None => PolySignal::default(),
            },
            Signal::Disconnected => PolySignal::default(),
        }
    }

    /// Check if the signal is disconnected
    pub fn is_disconnected(&self) -> bool {
        matches!(self, Signal::Disconnected)
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
            (Signal::Volts(poly1), Signal::Volts(poly2)) => poly1 == poly2,
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
    /// Get polyphonic sample output for a port.
    fn get_poly_sample(&self, port: &str) -> Option<PolySignal>;
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::from_str;

    #[test]
    fn test_signal_deserialization_volts() {
        let s: Signal = from_str("0.5").unwrap();
        match s {
            Signal::Volts(poly) => assert_eq!(poly.get(0), 0.5),
            _ => panic!("Expected Volts"),
        }
    }

    #[test]
    fn test_signal_deserialization_hz() {
        // 55Hz is 0V
        let s: Signal = from_str("\"55hz\"").unwrap();
        match s {
            Signal::Volts(poly) => assert!((poly.get(0) - 0.0).abs() < 1e-6),
            _ => panic!("Expected Volts"),
        }

        // 110Hz is 1V (one octave up)
        let s: Signal = from_str("\"110hz\"").unwrap();
        match s {
            Signal::Volts(poly) => assert!((poly.get(0) - 1.0).abs() < 1e-6),
            _ => panic!("Expected Volts"),
        }
    }

    #[test]
    fn test_signal_deserialization_midi() {
        // MIDI 33 (A0) is 0V
        let s: Signal = from_str("\"33m\"").unwrap();
        match s {
            Signal::Volts(poly) => assert!((poly.get(0) - 0.0).abs() < 1e-6),
            _ => panic!("Expected Volts"),
        }

        // MIDI 45 (A1) is 1V
        let s: Signal = from_str("\"45m\"").unwrap();
        match s {
            Signal::Volts(poly) => assert!((poly.get(0) - 1.0).abs() < 1e-6),
            _ => panic!("Expected Volts"),
        }
    }

    #[test]
    fn test_signal_deserialization_note() {
        // A0 is 0V
        let s: Signal = from_str("\"A0\"").unwrap();
        match s {
            Signal::Volts(poly) => assert!((poly.get(0) - 0.0).abs() < 1e-6),
            _ => panic!("Expected Volts"),
        }

        // A-1 is -1V
        let s: Signal = from_str("\"A-1\"").unwrap();
        match s {
            Signal::Volts(poly) => assert!((poly.get(0) + 1.0).abs() < 1e-6),
            _ => panic!("Expected Volts"),
        }

        // A1 is 1V
        let s: Signal = from_str("\"A1\"").unwrap();
        match s {
            Signal::Volts(poly) => assert!((poly.get(0) - 1.0).abs() < 1e-6),
            _ => panic!("Expected Volts"),
        }

        // C4 (Middle C) -> MIDI 72 -> (72-33)/12 = 3.25V
        let s: Signal = from_str("\"C4\"").unwrap();
        match s {
            Signal::Volts(poly) => assert!((poly.get(0) - 3.25).abs() < 1e-6),
            _ => panic!("Expected Volts"),
        }

        // Sharps/Flats
        // A#0 -> MIDI 34 -> 1/12 V
        let s: Signal = from_str("\"A#0\"").unwrap();
        match s {
            Signal::Volts(poly) => assert!((poly.get(0) - (1.0 / 12.0)).abs() < 1e-6),
            _ => panic!("Expected Volts"),
        }
    }

    #[test]
    fn test_signal_deserialization_scale() {
        // 0s(A0:Major) -> treat as root -> A0 -> 0V
        let s: Signal = from_str("\"0s(A0:Major)\"").unwrap();
        match s {
            Signal::Volts(poly) => assert!((poly.get(0) - 0.0).abs() < 1e-6),
            _ => panic!("Expected Volts"),
        }

        // 1s(A0:Major) -> 2nd interval -> B0 -> 2 semitones -> 2/12 V
        let s: Signal = from_str("\"1s(A0:Major)\"").unwrap();
        match s {
            Signal::Volts(poly) => assert!((poly.get(0) - (2.0 / 12.0)).abs() < 1e-6),
            _ => panic!("Expected Volts"),
        }

        // 2s(A0:Major) -> 3rd interval -> C#0 -> 4 semitones -> 4/12 V
        let s: Signal = from_str("\"2s(A0:Major)\"").unwrap();
        match s {
            Signal::Volts(poly) => assert!((poly.get(0) - (4.0 / 12.0)).abs() < 1e-6),
            _ => panic!("Expected Volts"),
        }

        // 8s(A0:Major) -> 9th interval -> B1 -> 14 semitones -> 14/12 V
        let s: Signal = from_str("\"8s(A0:Major)\"").unwrap();
        match s {
            Signal::Volts(poly) => assert!((poly.get(0) - (14.0 / 12.0)).abs() < 1e-6),
            _ => panic!("Expected Volts"),
        }

        // Cents
        // 1.5s(A0:Major) -> 2nd interval + 50 cents -> 2.5 semitones -> 2.5/12 V
        let s: Signal = from_str("\"1.5s(a0:maj)\"").unwrap();
        match s {
            Signal::Volts(poly) => assert!((poly.get(0) - (2.5 / 12.0)).abs() < 1e-6),
            _ => panic!("Expected Volts"),
        }

        // Negative degrees wrap to lower octave
        // -1s(A0:Major) -> G#-1 -> one semitone below A0 -> -1/12 V
        let s: Signal = from_str("\"-1s(A0:Major)\"").unwrap();
        match s {
            Signal::Volts(poly) => assert!((poly.get(0) - (-1.0 / 12.0)).abs() < 1e-6),
            _ => panic!("Expected Volts"),
        }
    }

    #[test]
    fn test_signal_deserialization_errors() {
        assert!(from_str::<Signal>("\"invalid\"").is_err());
        assert!(from_str::<Signal>("\"-10hz\"").is_err());
    }
}
