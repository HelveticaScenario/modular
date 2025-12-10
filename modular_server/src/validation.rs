use modular_core::types::{ModuleSchema, Param, PatchGraph, ScopeItem};
use std::collections::{HashMap, HashSet};

use crate::protocol::ValidationError;

/// Validate a patch against the module schemas
/// Returns all validation errors found (not just the first)
///
/// Validates:
/// - All module types exist in the schema
/// - All cable source/target modules exist in the patch
/// - All cable source ports (outputs) exist on their respective modules
/// - All cable target ports (params) exist on their respective modules
///
/// Note: Cycles in the graph are allowed (not an error condition)
/// Note: Any output can be routed to any param (no type compatibility checking needed)
pub fn validate_patch(
    patch: &PatchGraph,
    schemas: &[ModuleSchema],
) -> Result<(), Vec<ValidationError>> {
    let mut errors = Vec::new();

    // Build schema lookup map
    let schema_map: HashMap<&str, &ModuleSchema> =
        schemas.iter().map(|s| (s.name.as_str(), s)).collect();

    // Build module and track ID lookup sets
    let module_ids: HashSet<&str> = patch.modules.iter().map(|m| m.id.as_str()).collect();
    let track_ids: HashSet<&str> = patch.tracks.iter().map(|t| t.id.as_str()).collect();

    for module in &patch.modules {
        // Check if module type exists
        if !schema_map.contains_key(module.module_type.as_str()) {
            errors.push(ValidationError::with_location(
                "module_type",
                format!("Unknown module type '{}'", module.module_type),
                format!("modules.{}", module.id),
            ));
            // Skip param validation for unknown module types since we can't know
            // which parameters are valid. Cable target validation will still occur
            // when other modules reference this one.
            continue;
        }

        let schema = schema_map.get(module.module_type.as_str()).unwrap();

        // Build param name set from schema
        let valid_params: HashSet<&str> = schema.params.iter().map(|p| p.name.as_str()).collect();

        // Validate each param
        for (param_name, param) in &module.params {
            // Check if param name is valid for this module type
            if !valid_params.contains(param_name.as_str()) {
                errors.push(ValidationError::with_location(
                    format!("params.{}", param_name),
                    format!(
                        "Parameter '{}' not found on module type '{}'",
                        param_name, module.module_type
                    ),
                    format!("modules.{}.params.{}", module.id, param_name),
                ));
            }

            // Check cable connections
            if let Param::Cable {
                module: target_module,
                port,
            } = param
            {
                // Check if source module exists
                if !module_ids.contains(target_module.as_str()) {
                    errors.push(ValidationError::with_location(
                        format!("params.{}.module", param_name),
                        format!("Module '{}' not found for cable source", target_module),
                        format!("modules.{}.params.{}", module.id, param_name),
                    ));
                } else {
                    // Check if source port exists on the source module
                    let source_module = patch.modules.iter().find(|m| m.id == *target_module);
                    if let Some(source) = source_module {
                        if let Some(source_schema) = schema_map.get(source.module_type.as_str()) {
                            let valid_outputs: HashSet<&str> = source_schema
                                .outputs
                                .iter()
                                .map(|o| o.name.as_str())
                                .collect();
                            if !valid_outputs.contains(port.as_str()) {
                                errors.push(ValidationError::with_location(
                                    format!("params.{}.port", param_name),
                                    format!(
                                        "Output port '{}' not found on module type '{}'",
                                        port, source.module_type
                                    ),
                                    format!("modules.{}.params.{}", module.id, param_name),
                                ));
                            }
                        }
                    }
                }
            }

            // Check track references
            if let Param::Track { track: track_id } = param {
                // Track validation: check it's not empty
                if track_id.is_empty() {
                    errors.push(ValidationError::with_location(
                        "track",
                        "Track ID cannot be empty".to_string(),
                        format!("modules.{}.params.{}", module.id, param_name),
                    ));
                }
            }
        }
    }

    // Validate scopes (declarative audio subscriptions)
    for (idx, scope) in patch.scopes.iter().enumerate() {
        let location = format!("scopes[{}]", idx);
        match scope {
            ScopeItem::ModuleOutput {
                module_id,
                port_name,
            } => {
                if !module_ids.contains(module_id.as_str()) {
                    errors.push(ValidationError::with_location(
                        "scopes.module_id",
                        format!("Module '{}' not found for scope", module_id),
                        location.clone(),
                    ));
                    continue;
                }

                if let Some(module) = patch.modules.iter().find(|m| m.id == *module_id) {
                    if let Some(module_schema) = schema_map.get(module.module_type.as_str()) {
                        let valid_outputs: HashSet<&str> = module_schema
                            .outputs
                            .iter()
                            .map(|o| o.name.as_str())
                            .collect();

                        if !valid_outputs.contains(port_name.as_str()) {
                            errors.push(ValidationError::with_location(
                                "scopes.port_name",
                                format!(
                                    "Output port '{}' not found on module type '{}'",
                                    port_name, module.module_type
                                ),
                                location.clone(),
                            ));
                        }
                    }
                }
            }
            ScopeItem::Track { track_id } => {
                if !track_ids.contains(track_id.as_str()) {
                    errors.push(ValidationError::with_location(
                        "scopes.track_id",
                        format!("Track '{}' not found for scope", track_id),
                        location.clone(),
                    ));
                }
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use modular_core::types::{ModuleState, OutputSchema, ParamSchema};

    fn create_test_schemas() -> Vec<ModuleSchema> {
        vec![
            ModuleSchema {
                name: "sine-oscillator".to_string(),
                description: "A sine wave oscillator".to_string(),
                params: vec![
                    ParamSchema {
                        name: "freq".to_string(),
                        description: "frequency".to_string(),
                    },
                    ParamSchema {
                        name: "phase".to_string(),
                        description: "phase".to_string(),
                    },
                ],
                outputs: vec![OutputSchema {
                    name: "output".to_string(),
                    description: "signal output".to_string(),
                    default: false,
                }],
            },
            ModuleSchema {
                name: "signal".to_string(),
                description: "A signal".to_string(),
                params: vec![ParamSchema {
                    name: "source".to_string(),
                    description: "signal input".to_string(),
                }],
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
        let mut params = HashMap::new();
        params.insert("freq".to_string(), Param::Value { value: 4.0 });

        let patch = PatchGraph {
            modules: vec![ModuleState {
                id: "sine-1".to_string(),
                module_type: "sine-oscillator".to_string(),
                params,
            }],
            tracks: vec![],
            scopes: vec![],
        };

        let result = validate_patch(&patch, &schemas);
        assert!(result.is_ok());
    }

    #[test]
    fn test_unknown_module_type() {
        let schemas = create_test_schemas();
        let patch = PatchGraph {
            modules: vec![ModuleState {
                id: "foo-1".to_string(),
                module_type: "unknown-module".to_string(),
                params: HashMap::new(),
            }],
            tracks: vec![],
            scopes: vec![],
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
        let mut params = HashMap::new();
        params.insert("unknown_param".to_string(), Param::Value { value: 1.0 });

        let patch = PatchGraph {
            modules: vec![ModuleState {
                id: "sine-1".to_string(),
                module_type: "sine-oscillator".to_string(),
                params,
            }],
            tracks: vec![],
            scopes: vec![],
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
        let mut params = HashMap::new();
        params.insert(
            "source".to_string(),
            Param::Cable {
                module: "nonexistent".to_string(),
                port: "output".to_string(),
            },
        );

        let patch = PatchGraph {
            modules: vec![ModuleState {
                id: "root".to_string(),
                module_type: "signal".to_string(),
                params,
            }],
            tracks: vec![],
            scopes: vec![],
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

        let mut sine_params = HashMap::new();
        sine_params.insert("freq".to_string(), Param::Value { value: 4.0 });

        let mut root_params = HashMap::new();
        root_params.insert(
            "source".to_string(),
            Param::Cable {
                module: "sine-1".to_string(),
                port: "invalid_port".to_string(),
            },
        );

        let patch = PatchGraph {
            modules: vec![
                ModuleState {
                    id: "sine-1".to_string(),
                    module_type: "sine-oscillator".to_string(),
                    params: sine_params,
                },
                ModuleState {
                    id: "root".to_string(),
                    module_type: "signal".to_string(),
                    params: root_params,
                },
            ],
            tracks: vec![],
            scopes: vec![],
        };

        let result = validate_patch(&patch, &schemas);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert!(
            errors[0]
                .message
                .contains("Output port 'invalid_port' not found")
        );
    }

    #[test]
    fn test_valid_cable_connection() {
        let schemas = create_test_schemas();

        let mut sine_params = HashMap::new();
        sine_params.insert("freq".to_string(), Param::Value { value: 4.0 });

        let mut signal_params = HashMap::new();
        signal_params.insert(
            "source".to_string(),
            Param::Cable {
                module: "sine-1".to_string(),
                port: "output".to_string(),
            },
        );

        let patch = PatchGraph {
            modules: vec![
                ModuleState {
                    id: "sine-1".to_string(),
                    module_type: "sine-oscillator".to_string(),
                    params: sine_params,
                },
                ModuleState {
                    id: "signal-1".to_string(),
                    module_type: "signal".to_string(),
                    params: signal_params,
                },
            ],
            tracks: vec![],
            scopes: vec![],
        };

        assert!(validate_patch(&patch, &schemas).is_ok());
    }

    #[test]
    fn test_multiple_errors() {
        let schemas = create_test_schemas();
        let mut params = HashMap::new();
        params.insert("unknown1".to_string(), Param::Value { value: 1.0 });
        params.insert("unknown2".to_string(), Param::Value { value: 2.0 });

        let patch = PatchGraph {
            modules: vec![ModuleState {
                id: "sine-1".to_string(),
                module_type: "sine-oscillator".to_string(),
                params,
            }],
            tracks: vec![],
            scopes: vec![],
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
        };

        assert!(validate_patch(&patch, &schemas).is_ok());
    }
}
