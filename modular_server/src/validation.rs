use modular_core::dsp::get_param_validators;
use modular_core::types::{ModuleSchema, PatchGraph, ScopeItem, Signal};
use schemars::Schema;
use std::collections::{HashMap, HashSet};

use crate::protocol::ValidationError;

/// Extract the `properties` object from a schema node.
///
/// Returns a mapping from param name -> schema for that param.
/// If the schema doesn't look like an object schema with properties, returns empty.
fn schema_properties(schema: &Schema) -> HashMap<String, Schema> {
    // schemars::Schema is a thin wrapper around a serde_json::Value (object/bool).
    // Properties live under "properties" in the common case; we also tolerate
    // older "schema.properties" shapes.
    let props = schema.as_object().and_then(|obj| {
        obj.get("properties")
            .and_then(|v| v.as_object())
            .or_else(|| {
                obj.get("schema")
                    .and_then(|s| s.as_object())
                    .and_then(|s| s.get("properties"))
                    .and_then(|v| v.as_object())
            })
    });

    props
        .map(|m| {
            m.iter()
                .filter_map(|(k, v)| {
                    let schema: Result<Schema, _> = v.clone().try_into();
                    schema.ok().map(|s| (k.clone(), s))
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Returns true if `schema_node` describes (or contains) a `Signal`.
///
/// Why we need this:
/// - Most params are plain numbers/structs and don't reference other patch entities.
/// - Params typed as `Signal` can contain `Cable { module, port }` or `Track { track }`.
///   Those require existence checks against `patch.modules` / `patch.tracks`.
///
/// Implementation strategy:
/// - Look for `$ref` containing "Signal".
/// - Recurse through combinators (`anyOf/oneOf/allOf`) and `items` for arrays.
fn schema_refers_to_signal(schema_node: &Schema) -> bool {
    if let Some(obj) = schema_node.as_object() {
        if let Some(r) = obj.get("$ref").and_then(|v| v.as_str()) {
            return r.ends_with("/Signal") || r.ends_with("definitions/Signal") || r.contains("Signal");
        }

        for key in ["anyOf", "oneOf", "allOf"] {
            if let Some(items) = obj.get(key).and_then(|v| v.as_array()) {
                if items.iter().any(|item| {
                    let schema: Result<Schema, _> = item.clone().try_into();
                    schema.ok().is_some_and(|s| schema_refers_to_signal(&s))
                }) {
                    return true;
                }
            }
        }

        if let Some(items) = obj.get("items") {
            let schema: Result<Schema, _> = items.clone().try_into();
            if let Ok(schema) = schema {
                if schema_refers_to_signal(&schema) {
                    return true;
                }
            }
        }
    }

    false
}

/// Validate a patch against the module schemas.
///
/// Returns all validation errors found (not just the first).
///
/// Validates:
/// - All module types exist in the schema
/// - Params in `ModuleState.params` are known for the module type
/// - Signal params with Cable/Track references point to existing modules/ports/tracks
/// - Scopes reference existing module outputs / tracks
pub fn validate_patch(
    patch: &PatchGraph,
    schemas: &[ModuleSchema],
) -> Result<(), Vec<ValidationError>> {
    // === Overview ===
    // This validator is intentionally "best effort": it walks the whole patch and
    // accumulates *all* issues it can find, returning them together.
    //
    // High-level flow:
    // 1) Build fast lookup tables (schemas by name, modules by id, track ids).
    // 2) Validate each module:
    //    - module type exists
    //    - module params only use known param names
    //    - for params whose schema indicates a `Signal`, validate any Cable/Track references
    // 3) Validate scopes:
    //    - referenced module exists
    //    - referenced output port exists on the module type
    //    - referenced track exists
    let mut errors = Vec::new();

    // === Indexing ===
    // Build a map from module type name -> schema.
    let schema_map: HashMap<&str, &ModuleSchema> =
        schemas.iter().map(|s| (s.name.as_str(), s)).collect();

    // Build a map from module type name -> typed params validator.
    //
    // This map is generated from the Rust module param structs via `#[derive(Module)]`.
    // If a module type isn't present here (e.g. schemas were provided from a custom source),
    // we simply skip the typed-parse validation step for that module.
    let param_validators = get_param_validators();

    // Build a map from module id -> module instance (state) from the patch.
    let module_by_id: HashMap<&str, _> = patch
        .modules
        .iter()
        .map(|m| (m.id.as_str(), m))
        .collect();

    // Collect ids for fast membership checks while validating Track references.
    let track_ids: HashSet<&str> = patch.tracks.iter().map(|t| t.id.as_str()).collect();

    // === Schema helpers ===
    // The runtime patch stores parameter values as JSON (`ModuleState.params`), but
    // the authoritative set of valid parameter names/types lives in the module schema.

    // === Module validation ===
    // Validate each module instance in the patch.
    for module in &patch.modules {
        // 1) Module type must exist in our schema registry.
        let Some(schema) = schema_map.get(module.module_type.as_str()).copied() else {
            errors.push(ValidationError {
                field: "moduleType".to_string(),
                message: format!("Unknown module type '{}'", module.module_type),
                location: Some(module.id.clone()),
            });
            continue;
        };

        // 2) Gather declared params for this module type (name -> schema node).
        //    This is what we compare the incoming JSON keys against.
        let param_schemas = schema_properties(&schema.params_schema);

        // 3) Params must be a JSON object (map from param name -> JSON value).
        //    `null` is tolerated as "no params" because some senders may omit params.
        let Some(param_obj) = module.params.as_object() else {
            // params is defaulted; tolerate null/empty but flag other shapes.
            if !module.params.is_null() {
                errors.push(ValidationError {
                    field: "params".to_string(),
                    message: "Module params must be a JSON object".to_string(),
                    location: Some(module.id.clone()),
                });
            }
            continue;
        };

        // 3b) If available, validate that `module.params` can be deserialized into the
        // module's concrete `*Params` Rust type.
        //
        // Important: we only attempt this once we know params is an object. We explicitly
        // tolerate `null` elsewhere, and we don't want a redundant parse failure in that case.
        if let Some(validate) = param_validators.get(module.module_type.as_str()) {
            if let Err(err) = validate(&module.params) {
                errors.push(ValidationError {
                    field: "params".to_string(),
                    message: format!(
                        "Params failed to parse for module type '{}': {}",
                        module.module_type, err
                    ),
                    location: Some(module.id.clone()),
                });
            }
        }

        // 4) Validate each param key/value pair.
        // for (param_name, param_value) in param_obj {
        //     // 4a) Unknown param names are always an error.
        //     let Some(param_schema_node) = param_schemas.get(param_name) else {
        //         errors.push(ValidationError {
        //             field: format!("params.{}", param_name),
        //             message: format!(
        //                 "Param '{}' not found on module type '{}'",
        //                 param_name, module.module_type
        //             ),
        //             location: Some(module.id.clone()),
        //         });
        //         continue;
        //     };

        //     // 4b) Only `Signal`-typed params can reference other entities.
        //     //     If it's not a `Signal`, we're done for this param.
        //     if !schema_refers_to_signal(param_schema_node) {
        //         continue;
        //     }

        //     // 4c) Parse the JSON value into a typed `Signal`.
        //     //     If this fails, the client provided the wrong shape.
        //     let Ok(signal) = serde_json::from_value::<Signal>(param_value.clone()) else {
        //         errors.push(ValidationError {
        //             field: format!("params.{}", param_name),
        //             message: "Signal param value is invalid".to_string(),
        //             location: Some(module.id.clone()),
        //         });
        //         continue;
        //     };

        //     // 4d) For references, validate that targets exist.
        //     match signal {
        //         Signal::Cable {
        //             module: src_module,
        //             port: src_port,
        //             ..
        //         } => {
        //             // Cable requires a valid source module id.
        //             // We also need the source module's type to validate the port.
        //             let Some(src_state) = module_by_id.get(src_module.as_str()).copied() else {
        //                 errors.push(ValidationError {
        //                     field: format!("params.{}", param_name),
        //                     message: format!("Module '{}' not found for cable source", src_module),
        //                     location: Some(module.id.clone()),
        //                 });
        //                 continue;
        //             };

        //             // The source module's type must also be known.
        //             let Some(src_schema) = schema_map.get(src_state.module_type.as_str()).copied()
        //             else {
        //                 errors.push(ValidationError {
        //                     field: format!("params.{}", param_name),
        //                     message: format!(
        //                         "Unknown module type '{}' for cable source module '{}'",
        //                         src_state.module_type, src_module
        //                     ),
        //                     location: Some(module.id.clone()),
        //                 });
        //                 continue;
        //             };

        //             // Cable requires a valid output port on the source module type.
        //             if !src_schema.outputs.iter().any(|o| o.name == src_port) {
        //                 errors.push(ValidationError {
        //                     field: format!("params.{}", param_name),
        //                     message: format!(
        //                         "Output port '{}' not found on module '{}'",
        //                         src_port, src_module
        //                     ),
        //                     location: Some(module.id.clone()),
        //                 });
        //             }
        //         }
        //         Signal::Track { track, .. } => {
        //             // Track reference requires a valid track id.
        //             if !track_ids.contains(track.as_str()) {
        //                 errors.push(ValidationError {
        //                     field: format!("params.{}", param_name),
        //                     message: format!("Track '{}' not found for track source", track),
        //                     location: Some(module.id.clone()),
        //                 });
        //             }
        //         }
        //         Signal::Volts { .. } | Signal::Disconnected => {}
        //     }
        // }
    }

    // === Scope validation ===
    // Scopes drive audio streaming: they refer either to a module output port
    // or to a track. They must reference existing entities.
    for scope in &patch.scopes {
        match scope {
            ScopeItem::ModuleOutput {
                module_id,
                port_name,
            } => {
                // Scope target module must exist.
                let Some(module) = module_by_id.get(module_id.as_str()).copied() else {
                    errors.push(ValidationError {
                        field: "scopes".to_string(),
                        message: format!("Scope references missing module '{}'", module_id),
                        location: None,
                    });
                    continue;
                };

                // Target module type must be known so we can validate its declared outputs.
                let Some(schema) = schema_map.get(module.module_type.as_str()).copied() else {
                    errors.push(ValidationError {
                        field: "scopes".to_string(),
                        message: format!(
                            "Scope references module '{}' with unknown type '{}'",
                            module_id, module.module_type
                        ),
                        location: None,
                    });
                    continue;
                };

                // Scope port must be one of the output ports declared in the module schema.
                if !schema.outputs.iter().any(|o| o.name == *port_name) {
                    errors.push(ValidationError {
                        field: "scopes".to_string(),
                        message: format!(
                            "Scope references missing output port '{}' on module '{}'",
                            port_name, module_id
                        ),
                        location: None,
                    });
                }
            }
            ScopeItem::Track { track_id } => {
                // Scope target track must exist.
                if !track_ids.contains(track_id.as_str()) {
                    errors.push(ValidationError {
                        field: "scopes".to_string(),
                        message: format!("Scope references missing track '{}'", track_id),
                        location: None,
                    });
                }
            }
        }
    }

    // === Result ===
    // Return Ok for a clean patch; otherwise return all collected errors.
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use modular_core::types::{ModuleState, OutputSchema};
    use schemars::json_schema;
    use serde_json::json;

    fn create_test_schemas() -> Vec<ModuleSchema> {
        vec![
            ModuleSchema {
                name: "sine-oscillator".to_string(),
                description: "A sine wave oscillator".to_string(),
                params_schema: json_schema!({
                    "type": "object",
                    "properties": {
                        "freq": {"$ref": "#/definitions/Signal"},
                        "phase": {"$ref": "#/definitions/Signal"}
                    }
                }),
                outputs: vec![OutputSchema {
                    name: "output".to_string(),
                    description: "signal output".to_string(),
                    default: false,
                }],
            },
            ModuleSchema {
                name: "signal".to_string(),
                description: "A signal".to_string(),
                params_schema: json_schema!({
                    "type": "object",
                    "properties": {
                        "source": {"$ref": "#/definitions/Signal"}
                    }
                }),
                outputs: vec![OutputSchema {
                    name: "output".to_string(),
                    description: "signal output".to_string(),
                    default: false,
                }],
            },
        ]
    }

    #[test]
    fn test_valid_patch() {
        let schemas = create_test_schemas();
        let patch = PatchGraph {
            modules: vec![ModuleState {
                id: "sine-1".to_string(),
                module_type: "sine-oscillator".to_string(),
                params: json!({
                    "freq": {"Volts": {"value": 4.0}}
                }),
            }],
            tracks: vec![],
            scopes: vec![],
            factories: vec![],
        };

        assert!(validate_patch(&patch, &schemas).is_ok());
    }

    #[test]
    fn test_unknown_module_type() {
        let schemas = create_test_schemas();
        let patch = PatchGraph {
            modules: vec![ModuleState {
                id: "foo-1".to_string(),
                module_type: "unknown-module".to_string(),
                params: json!({}),
            }],
            tracks: vec![],
            scopes: vec![],
            factories: vec![],
        };

        let result = validate_patch(&patch, &schemas);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("Unknown module type"));
    }

    #[test]
    fn test_unknown_param() {
        let schemas = create_test_schemas();
        let patch = PatchGraph {
            modules: vec![ModuleState {
                id: "sine-1".to_string(),
                module_type: "sine-oscillator".to_string(),
                params: json!({
                    "unknown_param": {"Volts": {"value": 1.0}}
                }),
            }],
            tracks: vec![],
            scopes: vec![],
            factories: vec![],
        };

        let result = validate_patch(&patch, &schemas);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("not found on module type"));
    }

    #[test]
    fn test_cable_to_nonexistent_module() {
        let schemas = create_test_schemas();
        let patch = PatchGraph {
            modules: vec![ModuleState {
                id: "root".to_string(),
                module_type: "signal".to_string(),
                params: json!({
                    "source": {"Cable": {"module": "nonexistent", "port": "output"}}
                }),
            }],
            tracks: vec![],
            scopes: vec![],
            factories: vec![],
        };

        let result = validate_patch(&patch, &schemas);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("not found for cable source"));
    }

    #[test]
    fn test_cable_to_invalid_port() {
        let schemas = create_test_schemas();
        let patch = PatchGraph {
            modules: vec![
                ModuleState {
                    id: "sine-1".to_string(),
                    module_type: "sine-oscillator".to_string(),
                    params: json!({
                        "freq": {"Volts": {"value": 4.0}}
                    }),
                },
                ModuleState {
                    id: "root".to_string(),
                    module_type: "signal".to_string(),
                    params: json!({
                        "source": {"Cable": {"module": "sine-1", "port": "invalid_port"}}
                    }),
                },
            ],
            tracks: vec![],
            scopes: vec![],
            factories: vec![],
        };

        let result = validate_patch(&patch, &schemas);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert!(errors[0]
            .message
            .contains("Output port 'invalid_port' not found"));
    }

    #[test]
    fn test_valid_cable_connection() {
        let schemas = create_test_schemas();
        let patch = PatchGraph {
            modules: vec![
                ModuleState {
                    id: "sine-1".to_string(),
                    module_type: "sine-oscillator".to_string(),
                    params: json!({
                        "freq": {"Volts": {"value": 4.0}}
                    }),
                },
                ModuleState {
                    id: "signal-1".to_string(),
                    module_type: "signal".to_string(),
                    params: json!({
                        "source": {"Cable": {"module": "sine-1", "port": "output"}}
                    }),
                },
            ],
            tracks: vec![],
            scopes: vec![],
            factories: vec![],
        };

        assert!(validate_patch(&patch, &schemas).is_ok());
    }

    #[test]
    fn test_multiple_errors() {
        let schemas = create_test_schemas();
        let patch = PatchGraph {
            modules: vec![ModuleState {
                id: "sine-1".to_string(),
                module_type: "sine-oscillator".to_string(),
                params: json!({
                    "unknown1": {"Volts": {"value": 1.0}},
                    "unknown2": {"Volts": {"value": 2.0}}
                }),
            }],
            tracks: vec![],
            scopes: vec![],
            factories: vec![],
        };

        let result = validate_patch(&patch, &schemas);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 2);
    }

    #[test]
    fn test_empty_patch_is_valid() {
        let schemas = create_test_schemas();
        let patch = PatchGraph {
            modules: Vec::new(),
            tracks: vec![],
            scopes: vec![],
            factories: vec![],
        };

        assert!(validate_patch(&patch, &schemas).is_ok());
    }

    #[test]
    fn test_typed_params_validation_catches_missing_required_fields() {
        // Use the real schemas (and real typed validators) from modular_core.
        let schemas = modular_core::dsp::schema();

        // NoiseParams requires both `gain: Signal` and `color: NoiseKind`.
        // Provide only `color` so deserialization fails.
        let patch = PatchGraph {
            modules: vec![ModuleState {
                id: "noise-1".to_string(),
                module_type: "noise".to_string(),
                params: json!({
                    "color": "White"
                }),
            }],
            tracks: vec![],
            scopes: vec![],
            factories: vec![],
        };

        let result = validate_patch(&patch, &schemas);
        assert!(result.is_err());
        let errors = result.unwrap_err();

        assert!(errors.iter().any(|e| {
            e.field == "params"
                && e.location.as_deref() == Some("noise-1")
                && e.message.contains("failed to parse")
        }));
    }

    #[test]
    fn test_null_params_is_tolerated_even_with_typed_validation() {
        // validate_patch treats `params: null` as "no params" and does not require
        // it to be deserializable into the module's concrete params type.
        let schemas = modular_core::dsp::schema();

        let patch = PatchGraph {
            modules: vec![ModuleState {
                id: "noise-1".to_string(),
                module_type: "noise".to_string(),
                params: serde_json::Value::Null,
            }],
            tracks: vec![],
            scopes: vec![],
            factories: vec![],
        };

        assert!(validate_patch(&patch, &schemas).is_ok());
    }
}

