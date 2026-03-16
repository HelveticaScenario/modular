use deserr::{DeserializeError, ErrorKind, IntoValue, MergeWithError, ValuePointerRef};
use std::collections::HashMap;
use std::fmt;
use std::ops::ControlFlow;

/// Accumulates all deserialization errors with source location context.
///
/// Uses `ControlFlow::Continue` to collect all errors rather than failing on first.
#[derive(Debug, Clone, Default)]
pub struct ModuleParamErrors {
    /// All accumulated errors, each with field path and message.
    errors: Vec<ParamError>,
    /// Source spans extracted from `__argument_spans` field.
    spans: HashMap<String, (u32, u32)>,
}

#[derive(Debug, Clone)]
pub struct ParamError {
    pub field: String,
    pub message: String,
}

impl fmt::Display for ParamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.field.is_empty() {
            write!(f, "{}", self.message)
        } else {
            write!(f, "`{}`: {}", self.field, self.message)
        }
    }
}

impl fmt::Display for ModuleParamErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, error) in self.errors.iter().enumerate() {
            if i > 0 {
                write!(f, "; ")?;
            }
            write!(f, "{}", error)?;
        }
        Ok(())
    }
}

/// Helper to convert a [`ValuePointerRef`] to a dot-separated field path string.
fn location_to_field(location: ValuePointerRef<'_>) -> String {
    fn rec(location: ValuePointerRef<'_>) -> String {
        match location {
            ValuePointerRef::Origin => String::new(),
            ValuePointerRef::Key { key, prev } => {
                let prefix = rec(*prev);
                if prefix.is_empty() {
                    key.to_string()
                } else {
                    format!("{}.{}", prefix, key)
                }
            }
            ValuePointerRef::Index { index, prev } => {
                format!("{}[{}]", rec(*prev), index)
            }
        }
    }
    rec(location)
}

impl ModuleParamErrors {
    pub fn new(spans: HashMap<String, (u32, u32)>) -> Self {
        Self {
            errors: Vec::new(),
            spans,
        }
    }

    pub fn add(&mut self, field: String, message: String) {
        self.errors.push(ParamError { field, message });
    }

    pub fn into_errors(self) -> Vec<ParamError> {
        self.errors
    }

    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }

    /// Get the span for a field name, if source spans were provided.
    pub fn get_span_for_field(&self, field: &str) -> Option<(u32, u32)> {
        self.spans.get(field).copied()
    }
}

impl DeserializeError for ModuleParamErrors {
    fn error<V: IntoValue>(
        _self_: Option<Self>,
        error: ErrorKind<V>,
        location: ValuePointerRef<'_>,
    ) -> ControlFlow<Self, Self> {
        let field = location_to_field(location);
        let message = match error {
            ErrorKind::IncorrectValueKind { actual, accepted } => {
                let accepted_str: Vec<&str> = accepted
                    .iter()
                    .map(|k| match k {
                        deserr::ValueKind::Null => "null",
                        deserr::ValueKind::Boolean => "a boolean",
                        deserr::ValueKind::Integer => "a positive integer",
                        deserr::ValueKind::NegativeInteger => "a negative integer",
                        deserr::ValueKind::Float => "a number",
                        deserr::ValueKind::String => "a string",
                        deserr::ValueKind::Sequence => "an array",
                        deserr::ValueKind::Map => "an object",
                    })
                    .collect();
                let actual_str = match actual.kind() {
                    deserr::ValueKind::Null => "null",
                    deserr::ValueKind::Boolean => "a boolean",
                    deserr::ValueKind::Integer => "a positive integer",
                    deserr::ValueKind::NegativeInteger => "a negative integer",
                    deserr::ValueKind::Float => "a number",
                    deserr::ValueKind::String => "a string",
                    deserr::ValueKind::Sequence => "an array",
                    deserr::ValueKind::Map => "an object",
                };
                format!(
                    "expected {}, but found {}",
                    accepted_str.join(" or "),
                    actual_str
                )
            }
            ErrorKind::MissingField { field: f } => {
                format!("missing required parameter `{}`", f)
            }
            ErrorKind::UnknownKey { key, accepted } => {
                let accepted_str = accepted
                    .iter()
                    .map(|s| format!("`{}`", s))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!(
                    "unknown parameter `{}`: expected one of {}",
                    key, accepted_str
                )
            }
            ErrorKind::UnknownValue { value, accepted } => {
                let accepted_str = accepted
                    .iter()
                    .map(|s| format!("`{}`", s))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!(
                    "unknown value `{}`: expected one of {}",
                    value, accepted_str
                )
            }
            ErrorKind::BadSequenceLen {
                actual: _,
                expected,
            } => {
                format!("expected exactly {} elements", expected)
            }
            ErrorKind::Unexpected { msg } => msg.to_string(),
        };

        let mut errors = _self_.unwrap_or_default();
        errors.add(field, message);
        // Continue to accumulate more errors
        ControlFlow::Continue(errors)
    }
}

impl MergeWithError<ModuleParamErrors> for ModuleParamErrors {
    fn merge(
        self_: Option<Self>,
        other: ModuleParamErrors,
        _merge_location: ValuePointerRef<'_>,
    ) -> ControlFlow<Self, Self> {
        let mut merged = self_.unwrap_or_default();
        merged.errors.extend(other.errors);
        ControlFlow::Continue(merged)
    }
}

/// Allow merging any std::error::Error into ModuleParamErrors.
impl<E: std::error::Error> MergeWithError<E> for ModuleParamErrors {
    fn merge(
        self_: Option<Self>,
        other: E,
        merge_location: ValuePointerRef<'_>,
    ) -> ControlFlow<Self, Self> {
        ModuleParamErrors::error::<std::convert::Infallible>(
            self_,
            ErrorKind::Unexpected {
                msg: other.to_string(),
            },
            merge_location,
        )
    }
}

/// Helper to extract the content from a ControlFlow where both branches are the same type.
pub fn take_cf<E>(cf: ControlFlow<E, E>) -> E {
    match cf {
        ControlFlow::Break(e) | ControlFlow::Continue(e) => e,
    }
}
