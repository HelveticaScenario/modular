//! SeqValue enum and SeqPatternParam for the pattern-based sequencer.
//!
//! `$cycle` accepts a `ParsedPattern` object built by the TypeScript
//! `$p(...)` helper. The patch-graph payload shape is:
//!
//! ```json
//! { "ast": <MiniAST>, "source": "<string>", "all_spans": [[start, end], ...] }
//! ```
//!
//! On deserialization this module runs `mini::convert::<SeqValue>(&ast)` to
//! build the runtime `Pattern<SeqValue>` and pre-computes 1000 cycles of
//! haps for the audio thread. The `source` and `all_spans` fields flow
//! through unchanged into Monaco pattern-span highlighting.
//!
//! `SeqValue` variants:
//! - `Voltage(f64)` — pre-converted V/Oct pitch. Bare numbers in
//!   mini-notation are interpreted as MIDI note numbers and converted
//!   here. Hz and note atoms also convert to voltage.
//! - `Rest` — silence / gate-low event.
//!
//! The previous `Signal { .. }` variant (backing the `module(id:port:ch)`
//! and `=` sample-and-hold notations) has been removed along with those
//! atom kinds.

use std::sync::Arc;

use deserr::{DeserializeError, Deserr, ErrorKind, IntoValue, Map, Sequence, ValuePointerRef};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    Patch,
    dsp::utils::midi_to_voct_f64,
    pattern_system::{
        DspHap, Pattern,
        mini::{
            FromMiniAtom, MiniAST,
            ast::AtomValue,
            convert::{ConvertError, HasRest},
        },
    },
    types::Connect,
};

/// A value in a sequence pattern.
#[derive(Clone, Debug)]
pub enum SeqValue {
    /// Pre-converted V/Oct voltage value.
    Voltage(f64),

    /// Rest — no value output, gate goes low.
    Rest,
}

impl SeqValue {
    /// Get the voltage (V/Oct) value. Returns None for Rest.
    pub fn to_voltage(&self) -> Option<f64> {
        match self {
            SeqValue::Voltage(v) => Some(*v),
            SeqValue::Rest => None,
        }
    }

    /// Check if this is a rest value.
    pub fn is_rest(&self) -> bool {
        matches!(self, SeqValue::Rest)
    }
}

/// Convert a note letter, accidental, and octave to MIDI note number.
fn note_to_midi(letter: char, accidental: Option<char>, octave: Option<i32>) -> Option<f64> {
    let base = match letter.to_ascii_lowercase() {
        'c' => 0,
        'd' => 2,
        'e' => 4,
        'f' => 5,
        'g' => 7,
        'a' => 9,
        'b' => 11,
        _ => return None,
    };

    let acc_offset = match accidental {
        Some('#') => 1,
        Some('b') => -1,
        _ => 0,
    };

    // Default octave to 4 if not specified
    let oct = octave.unwrap_or(4);
    Some(((oct + 1) * 12 + base + acc_offset) as f64)
}

impl FromMiniAtom for SeqValue {
    fn from_atom(atom: &AtomValue) -> Result<Self, ConvertError> {
        match atom {
            AtomValue::Number(n) => {
                // Treat bare number as MIDI note, convert to voltage at
                // parse time. Matches the historical behaviour before the
                // grammar shrink (when the Rust parser produced Midi(i)).
                Ok(SeqValue::Voltage(midi_to_voct_f64(*n)))
            }
            AtomValue::Hz(hz) => {
                // Convert Hz to MIDI then to voltage.
                let midi = 12.0 * (hz / 440.0).log2() + 69.0;
                Ok(SeqValue::Voltage(midi_to_voct_f64(midi)))
            }
            AtomValue::Note {
                letter,
                accidental,
                octave,
            } => {
                if let Some(midi) = note_to_midi(*letter, *accidental, *octave) {
                    Ok(SeqValue::Voltage(midi_to_voct_f64(midi)))
                } else {
                    Err(ConvertError::InvalidAtom(format!(
                        "Invalid note: {}{}{}",
                        letter,
                        accidental
                            .as_ref()
                            .map(|s| s.to_string())
                            .unwrap_or_default(),
                        octave.map(|o| o.to_string()).unwrap_or_default()
                    )))
                }
            }
        }
    }

    fn from_list(atoms: &[AtomValue]) -> Result<Self, ConvertError> {
        if atoms.len() == 1 {
            Self::from_atom(&atoms[0])
        } else {
            Err(ConvertError::ListNotSupported)
        }
    }

    fn combine_with_head(_head_atoms: &[AtomValue], _tail: &Self) -> Result<Self, ConvertError> {
        Err(ConvertError::ListNotSupported)
    }

    fn rest_value() -> Option<Self> {
        Some(SeqValue::Rest)
    }

    fn supports_rest() -> bool {
        true
    }
}

impl HasRest for SeqValue {
    fn rest_value() -> Self {
        SeqValue::Rest
    }
}

/// JSON payload shape delivered in the patch graph for
/// `SeqPatternParam` / `IntervalPatternParam`. Produced client-side by
/// the TypeScript `$p(...)` helper in `src/main/dsl/miniNotation/`.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct ParsedPatternPayload {
    /// The parsed AST.
    pub ast: MiniAST,
    /// The original mini-notation source string.
    pub source: String,
    /// Pre-computed leaf spans, used for Monaco tracked decorations.
    pub all_spans: Vec<(usize, usize)>,
}

impl ParsedPatternPayload {
    /// Build a payload by parsing a mini-notation string via the
    /// in-crate test parser. Integration tests in `tests/` need this
    /// (they're a separate crate, so `#[cfg(test)]` items in the lib
    /// aren't visible). The production path is the TypeScript `$p()`
    /// helper; this exists only so existing Rust fixtures don't need
    /// to hand-build ASTs node by node.
    #[doc(hidden)]
    pub fn parse_for_test(source: &str) -> Self {
        if source.is_empty() {
            return Self {
                ast: MiniAST::Sequence(Vec::new()),
                source: String::new(),
                all_spans: Vec::new(),
            };
        }
        let ast = crate::pattern_system::mini::parse_ast(source)
            .expect("test_parser should parse the fixture source");
        let all_spans = crate::pattern_system::mini::collect_leaf_spans(&ast);
        ParsedPatternPayload {
            ast,
            source: source.to_string(),
            all_spans,
        }
    }
}

#[cfg(test)]
impl From<&str> for ParsedPatternPayload {
    fn from(source: &str) -> Self {
        Self::parse_for_test(source)
    }
}

#[cfg(test)]
impl From<String> for ParsedPatternPayload {
    fn from(source: String) -> Self {
        Self::parse_for_test(&source)
    }
}

// Deserr bridge: round-trip via serde_json::Value. The payload is
// structurally complex (recursive MiniAST), so a hand-rolled deserr impl
// would duplicate the existing serde Deserialize impl. deserr's IntoValue
// is trivially convertible to serde_json::Value, so we lean on that.
impl<E: DeserializeError> Deserr<E> for ParsedPatternPayload {
    fn deserialize_from_value<V: IntoValue>(
        value: deserr::Value<V>,
        location: ValuePointerRef<'_>,
    ) -> Result<Self, E> {
        let json = value_to_json(value);
        serde_json::from_value::<ParsedPatternPayload>(json).map_err(|e| {
            deserr::take_cf_content(E::error::<V>(
                None,
                ErrorKind::Unexpected {
                    msg: format!("invalid parsed pattern payload: {e}"),
                },
                location,
            ))
        })
    }
}

/// Convert a deserr `Value` (backed by an arbitrary `IntoValue`) into a
/// `serde_json::Value`. Used by the `ParsedPatternPayload` bridge so we
/// don't have to reimplement `Deserialize` via `Deserr` by hand.
fn value_to_json<V: IntoValue>(value: deserr::Value<V>) -> serde_json::Value {
    match value {
        deserr::Value::Null => serde_json::Value::Null,
        deserr::Value::Boolean(b) => serde_json::Value::Bool(b),
        deserr::Value::Integer(i) => serde_json::Value::Number(i.into()),
        deserr::Value::NegativeInteger(i) => serde_json::Value::Number(i.into()),
        deserr::Value::Float(f) => serde_json::Number::from_f64(f)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        deserr::Value::String(s) => serde_json::Value::String(s),
        deserr::Value::Sequence(seq) => serde_json::Value::Array(
            seq.into_iter()
                .map(|v: V| value_to_json::<V>(v.into_value()))
                .collect(),
        ),
        deserr::Value::Map(map) => {
            let mut out = serde_json::Map::new();
            for (k, v) in map.into_iter() {
                out.insert(k, value_to_json::<V>(v.into_value()));
            }
            serde_json::Value::Object(out)
        }
    }
}

// Default for MiniAST — required so ParsedPatternPayload can derive Default.
// An empty sequence is the zero value; deserialization always overwrites it.
impl Default for MiniAST {
    fn default() -> Self {
        MiniAST::Sequence(Vec::new())
    }
}

/// Pattern parameter for `$cycle`.
///
/// Accepts a `ParsedPatternPayload` in the patch graph (a JSON object
/// with `{ ast, source, all_spans }` — see `ParsedPatternPayload`).
/// Parsing happens TypeScript-side; this struct only lowers the AST into
/// a runtime `Pattern<SeqValue>` and pre-computes cycles 0..999 of haps
/// for zero-allocation audio-thread access.
///
/// An empty string source, `{}` payload, or a missing `ast` field
/// produces the default (no pattern) — matches previous behaviour for
/// empty strings.
///
/// The JsonSchema is delegated to [`ParsedPatternPayload`] because that's
/// the shape actually carried on the wire; the runtime-derived fields
/// (`pattern`, `cached_haps`) never cross the IPC boundary.
#[derive(Clone, Default, Debug)]
pub struct SeqPatternParam {
    /// The source pattern string (used for Monaco highlighting).
    #[allow(dead_code)]
    source: String,

    /// The parsed pattern.
    pub(crate) pattern: Option<Pattern<SeqValue>>,

    /// All leaf spans in the pattern (character offsets in `source`).
    /// Flows through from the TS side's `collectLeafSpans` output.
    pub(crate) all_spans: Vec<(usize, usize)>,

    /// Pre-computed haps for cycles 0..999, populated at deserialization.
    /// Each element is an `Arc`-wrapped `Vec` of all haps intersecting
    /// that cycle. Cache-friendly for audio-thread access.
    pub(crate) cached_haps: Vec<Arc<Vec<DspHap<SeqValue>>>>,
}

impl JsonSchema for SeqPatternParam {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        ParsedPatternPayload::schema_name()
    }
    fn json_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        ParsedPatternPayload::json_schema(generator)
    }
}

impl SeqPatternParam {
    fn from_payload(payload: ParsedPatternPayload) -> Result<Self, String> {
        if payload.source.is_empty() {
            return Ok(Self::default());
        }
        let pattern =
            crate::pattern_system::mini::convert::<SeqValue>(&payload.ast).map_err(|e| e.to_string())?;

        // Pre-compute and cache haps for cycles 0..999.
        let cached_haps: Vec<Arc<Vec<DspHap<SeqValue>>>> = (0..1000)
            .map(|cycle| Arc::new(pattern.query_cycle_all(cycle)))
            .collect();

        Ok(Self {
            source: payload.source,
            pattern: Some(pattern),
            all_spans: payload.all_spans,
            cached_haps,
        })
    }

    /// Get the parsed pattern.
    pub fn pattern(&self) -> Option<&Pattern<SeqValue>> {
        self.pattern.as_ref()
    }

    /// Get the source pattern string (the evaluated pattern passed to `$p`).
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Get all leaf spans in the pattern (for frontend tracked decorations).
    pub fn all_spans(&self) -> &[(usize, usize)] {
        &self.all_spans
    }

    /// Get the pre-computed cached haps for cycles 0..999.
    pub fn cached_haps(&self) -> &[Arc<Vec<DspHap<SeqValue>>>] {
        &self.cached_haps
    }
}

// deserr implementation: reads the JSON `ParsedPatternPayload` shape
// (`{ ast, source, all_spans }`). Source-only strings are no longer
// accepted; the DSL must wrap mini-notation in `$p(...)` before passing
// to `$cycle`.
impl<E: DeserializeError> deserr::Deserr<E> for SeqPatternParam {
    fn deserialize_from_value<V: IntoValue>(
        value: deserr::Value<V>,
        location: ValuePointerRef<'_>,
    ) -> Result<Self, E> {
        let payload = ParsedPatternPayload::deserialize_from_value(value, location)?;
        Self::from_payload(payload).map_err(|e| {
            deserr::take_cf_content(E::error::<V>(
                None,
                ErrorKind::Unexpected { msg: e },
                location,
            ))
        })
    }
}

impl Connect for SeqPatternParam {
    fn connect(&mut self, _patch: &Patch) {
        // No signals in a pattern anymore — module references were removed
        // from the grammar in the `$p()` refactor.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pattern_system::Fraction;
    use crate::pattern_system::mini::parse;

    #[test]
    fn test_seq_value_to_voltage() {
        // C4 = MIDI 60 -> voltage
        let c4_voltage = midi_to_voct_f64(60.0);
        assert!((SeqValue::Voltage(c4_voltage).to_voltage().unwrap() - c4_voltage).abs() < 0.001);

        assert_eq!(SeqValue::Rest.to_voltage(), None);
    }

    #[test]
    fn test_note_to_midi_helper() {
        assert_eq!(note_to_midi('c', None, Some(4)), Some(60.0));
        assert_eq!(note_to_midi('c', None, None), Some(60.0));
        assert_eq!(note_to_midi('c', Some('#'), Some(4)), Some(61.0));
        assert_eq!(note_to_midi('a', None, Some(0)), Some(21.0));
        assert_eq!(note_to_midi('a', None, Some(1)), Some(33.0));
    }

    #[test]
    fn test_from_atom_number_to_voltage() {
        // Number is treated as MIDI and converted to voltage (1V/oct from C4).
        let n = SeqValue::from_atom(&AtomValue::Number(60.0)).unwrap();
        let expected_voltage = midi_to_voct_f64(60.0);
        assert!(matches!(n, SeqValue::Voltage(v) if (v - expected_voltage).abs() < 0.001));
    }

    #[test]
    fn test_from_atom_note_to_voltage() {
        let note = SeqValue::from_atom(&AtomValue::Note {
            letter: 'a',
            accidental: None,
            octave: Some(4),
        })
        .unwrap();
        let expected_a4_voltage = midi_to_voct_f64(69.0);
        assert!(matches!(note, SeqValue::Voltage(v) if (v - expected_a4_voltage).abs() < 0.001));
    }

    #[test]
    fn test_from_atom_hz_to_voltage() {
        // 440hz = A4 = MIDI 69
        let hz = SeqValue::from_atom(&AtomValue::Hz(440.0)).unwrap();
        let expected = midi_to_voct_f64(69.0);
        assert!(matches!(hz, SeqValue::Voltage(v) if (v - expected).abs() < 0.001));
    }

    #[test]
    fn test_note_octaves_different() {
        // Parse "a1 a2 a3 a4" and check each note has different voltage.
        let pattern: crate::pattern_system::Pattern<SeqValue> =
            parse("a1 a2 a3 a4").expect("should parse");

        let haps = pattern.query_arc(Fraction::from(0), Fraction::from(1));

        assert_eq!(haps.len(), 4);

        let voltages: Vec<f64> = haps.iter().filter_map(|h| h.value.to_voltage()).collect();

        // a1 = MIDI 33 = 0V, a2 = MIDI 45 = 1V, a3 = MIDI 57 = 2V, a4 = MIDI 69 = 3V
        let expected = [
            midi_to_voct_f64(33.0),
            midi_to_voct_f64(45.0),
            midi_to_voct_f64(57.0),
            midi_to_voct_f64(69.0),
        ];

        for (i, (actual, expected)) in voltages.iter().zip(expected.iter()).enumerate() {
            assert!(
                (actual - expected).abs() < 0.001,
                "a{} voltage mismatch",
                i + 1
            );
        }
    }

    #[test]
    fn test_seq_value_supports_rest() {
        assert!(SeqValue::supports_rest());
        assert!(<SeqValue as FromMiniAtom>::rest_value().is_some());
        assert!(matches!(
            <SeqValue as FromMiniAtom>::rest_value(),
            Some(SeqValue::Rest)
        ));
    }

    #[test]
    fn test_seq_value_has_rest_trait() {
        use crate::pattern_system::HasRest;
        let rest = <SeqValue as HasRest>::rest_value();
        assert!(matches!(rest, SeqValue::Rest));
    }

    #[test]
    fn test_seq_value_euclidean() {
        let pattern: crate::pattern_system::Pattern<SeqValue> =
            parse("c(2,4)").expect("should parse");

        let haps = pattern.query_arc(Fraction::from(0), Fraction::from(1));
        assert_eq!(haps.len(), 4);

        let notes: Vec<_> = haps.iter().filter(|h| !h.value.is_rest()).collect();
        let rests: Vec<_> = haps.iter().filter(|h| h.value.is_rest()).collect();

        assert_eq!(notes.len(), 2);
        assert_eq!(rests.len(), 2);
    }

    #[test]
    fn test_seq_value_rest_in_pattern() {
        let pattern: crate::pattern_system::Pattern<SeqValue> =
            parse("c4 ~ e4").expect("should parse");

        let haps = pattern.query_arc(Fraction::from(0), Fraction::from(1));
        assert_eq!(haps.len(), 3);
        assert!(haps[1].value.is_rest());
    }

    #[test]
    fn test_seq_value_degrade_in_pattern() {
        let pattern: crate::pattern_system::Pattern<SeqValue> =
            parse("c4?").expect("should parse");

        for i in 0..10 {
            let haps = pattern.query_arc(Fraction::from(i), Fraction::from(i + 1));
            assert_eq!(haps.len(), 1);
        }
    }

    #[test]
    fn test_seq_value_euclidean_in_pattern() {
        let pattern: crate::pattern_system::Pattern<SeqValue> =
            parse("c4(3,8)").expect("should parse");

        let haps = pattern.query_arc(Fraction::from(0), Fraction::from(1));
        assert_eq!(haps.len(), 8);

        let pulse_count = haps.iter().filter(|h| !h.value.is_rest()).count();
        assert_eq!(pulse_count, 3);
    }

    #[test]
    fn test_deserialize_payload_json_c4() {
        // Emulate what the DSL's $p("c4") would emit.
        let json = serde_json::json!({
            "ast": {
                "Pure": {
                    "node": { "Note": { "letter": "c", "accidental": null, "octave": 4 } },
                    "span": { "start": 0, "end": 2 }
                }
            },
            "source": "c4",
            "all_spans": [[0, 2]]
        });
        let param: SeqPatternParam = deserr::deserialize::<
            SeqPatternParam,
            _,
            crate::param_errors::ModuleParamErrors,
        >(json)
        .expect("should deserialize c4 payload");
        assert_eq!(param.source(), "c4");
    }

    #[test]
    fn test_deserialize_payload_json() {
        // Emulate what the DSL's $p() helper would emit.
        let json = serde_json::json!({
            "ast": { "Pure": { "node": { "Number": 60 }, "span": { "start": 0, "end": 2 } } },
            "source": "60",
            "all_spans": [[0, 2]]
        });
        let param: SeqPatternParam = deserr::deserialize::<
            SeqPatternParam,
            _,
            crate::param_errors::ModuleParamErrors,
        >(json)
        .expect("should deserialize");
        assert_eq!(param.source(), "60");
        assert_eq!(param.all_spans(), &[(0, 2)]);
        assert!(param.pattern().is_some());
        assert_eq!(param.cached_haps().len(), 1000);
    }

    #[test]
    fn test_empty_source_produces_default() {
        let json = serde_json::json!({
            "ast": { "Sequence": [] },
            "source": "",
            "all_spans": []
        });
        let param: SeqPatternParam = deserr::deserialize::<
            SeqPatternParam,
            _,
            crate::param_errors::ModuleParamErrors,
        >(json)
        .expect("should deserialize");
        assert!(param.pattern().is_none());
        assert_eq!(param.cached_haps().len(), 0);
    }
}
