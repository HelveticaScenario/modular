use std::collections::{HashMap, HashSet};
use modular_core::types::{ModuleSchema, Param, PatchGraph};
use serde::{Deserialize, Serialize};

/// A validation error with detailed information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
    pub location: Option<String>,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(ref location) = self.location {
            write!(f, "{}: {} (at {})", self.field, self.message, location)
        } else {
            write!(f, "{}: {}", self.field, self.message)
        }
    }
}

/// Result of patch validation
pub type ValidationResult = Result<(), Vec<ValidationError>>;

/// Validate a patch against the module schemas
/// 
/// Validates:
/// - All module types exist in the schema
/// - All cable source/target modules exist in the patch
/// - All cable source ports (outputs) exist on their respective modules
/// - All cable target ports (params) exist on their respective modules
/// 
/// Note: Cycles in the graph are allowed (not an error condition)
/// Note: Any output can be routed to any param (no type compatibility checking needed)
pub fn validate_patch(patch: &PatchGraph, schemas: &[ModuleSchema]) -> ValidationResult {
    let mut errors = Vec::new();
    
    // Build a map of module types to their schemas for efficient lookup
    let schema_map: HashMap<&str, &ModuleSchema> = schemas
        .iter()
        .map(|s| (s.name.as_str(), s))
        .collect();
    
    // Build a set of module IDs in the patch
    let module_ids: HashSet<&str> = patch.modules.iter().map(|m| m.id.as_str()).collect();
    
    for module in &patch.modules {
        let location = format!("modules.{}", module.id);
        
        // Check if module type exists in schema
        let module_schema = match schema_map.get(module.module_type.as_str()) {
            Some(schema) => Some(*schema),
            None => {
                errors.push(ValidationError {
                    field: "module_type".to_string(),
                    message: format!("Unknown module type '{}'", module.module_type),
                    location: Some(location.clone()),
                });
                None
            }
        };
        
        // Check each parameter
        for (param_name, param) in &module.params {
            let param_location = format!("{}.params.{}", location, param_name);
            
            // Check if param name exists on the module type
            if let Some(schema) = module_schema {
                let param_exists = schema.params.iter().any(|p| p.name == *param_name);
                if !param_exists {
                    errors.push(ValidationError {
                        field: param_name.clone(),
                        message: format!(
                            "Parameter '{}' not found on module type '{}'",
                            param_name, module.module_type
                        ),
                        location: Some(param_location.clone()),
                    });
                }
            }
            
            // Validate cable connections
            if let Param::Cable { module: cable_module, port: cable_port } = param {
                // Check if source module exists
                if !module_ids.contains(cable_module.as_str()) {
                    errors.push(ValidationError {
                        field: "module".to_string(),
                        message: format!("Module '{}' not found for cable source", cable_module),
                        location: Some(param_location.clone()),
                    });
                } else {
                    // Check if source port exists on the source module
                    if let Some(source_module) = patch.modules.iter().find(|m| m.id == *cable_module) {
                        if let Some(source_schema) = schema_map.get(source_module.module_type.as_str()) {
                            let port_exists = source_schema.outputs.iter().any(|o| o.name == *cable_port);
                            if !port_exists {
                                errors.push(ValidationError {
                                    field: "port".to_string(),
                                    message: format!(
                                        "Output port '{}' not found on module type '{}'",
                                        cable_port, source_module.module_type
                                    ),
                                    location: Some(param_location.clone()),
                                });
                            }
                        }
                    }
                }
            }
            
            // Validate track connections
            if let Param::Track { track } = param {
                // Track validation would go here if we had track schema
                // For now, just check it's not empty
                if track.is_empty() {
                    errors.push(ValidationError {
                        field: "track".to_string(),
                        message: "Track ID cannot be empty".to_string(),
                        location: Some(param_location),
                    });
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
    use modular_core::types::{ModuleState, PortSchema};

    fn create_test_schemas() -> Vec<ModuleSchema> {
        vec![
            ModuleSchema {
                name: "sine-oscillator".to_string(),
                description: "Sine wave oscillator".to_string(),
                params: vec![
                    PortSchema {
                        name: "freq".to_string(),
                        description: "Frequency in v/oct".to_string(),
                    },
                    PortSchema {
                        name: "phase".to_string(),
                        description: "Phase offset".to_string(),
                    },
                ],
                outputs: vec![PortSchema {
                    name: "output".to_string(),
                    description: "Audio output".to_string(),
                }],
            },
            ModuleSchema {
                name: "signal".to_string(),
                description: "Signal output to audio".to_string(),
                params: vec![PortSchema {
                    name: "input".to_string(),
                    description: "Audio input".to_string(),
                }],
                outputs: vec![PortSchema {
                    name: "output".to_string(),
                    description: "Audio output".to_string(),
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
        };
        
        assert!(validate_patch(&patch, &schemas).is_ok());
    }

    #[test]
    fn test_unknown_module_type() {
        let schemas = create_test_schemas();
        let patch = PatchGraph {
            modules: vec![ModuleState {
                id: "unknown-1".to_string(),
                module_type: "unknown-type".to_string(),
                params: HashMap::new(),
            }],
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
        };
        
        let result = validate_patch(&patch, &schemas);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("Parameter 'unknown_param' not found"));
    }

    #[test]
    fn test_cable_to_nonexistent_module() {
        let schemas = create_test_schemas();
        let mut params = HashMap::new();
        params.insert(
            "input".to_string(),
            Param::Cable {
                module: "nonexistent".to_string(),
                port: "output".to_string(),
            },
        );
        
        let patch = PatchGraph {
            modules: vec![ModuleState {
                id: "signal-1".to_string(),
                module_type: "signal".to_string(),
                params,
            }],
        };
        
        let result = validate_patch(&patch, &schemas);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.message.contains("not found for cable source")));
    }

    #[test]
    fn test_cable_to_nonexistent_port() {
        let schemas = create_test_schemas();
        
        let mut sine_params = HashMap::new();
        sine_params.insert("freq".to_string(), Param::Value { value: 4.0 });
        
        let mut signal_params = HashMap::new();
        signal_params.insert(
            "input".to_string(),
            Param::Cable {
                module: "sine-1".to_string(),
                port: "nonexistent_port".to_string(),
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
        };
        
        let result = validate_patch(&patch, &schemas);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.message.contains("Output port 'nonexistent_port' not found")));
    }

    #[test]
    fn test_valid_cable_connection() {
        let schemas = create_test_schemas();
        
        let mut sine_params = HashMap::new();
        sine_params.insert("freq".to_string(), Param::Value { value: 4.0 });
        
        let mut signal_params = HashMap::new();
        signal_params.insert(
            "input".to_string(),
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
        };
        
        assert!(validate_patch(&patch, &schemas).is_ok());
    }

    #[test]
    fn test_multiple_errors() {
        let schemas = create_test_schemas();
        let mut params = HashMap::new();
        params.insert("unknown_param".to_string(), Param::Value { value: 1.0 });
        params.insert(
            "input".to_string(),
            Param::Cable {
                module: "nonexistent".to_string(),
                port: "output".to_string(),
            },
        );
        
        let patch = PatchGraph {
            modules: vec![
                ModuleState {
                    id: "unknown-1".to_string(),
                    module_type: "unknown-type".to_string(),
                    params: HashMap::new(),
                },
                ModuleState {
                    id: "signal-1".to_string(),
                    module_type: "signal".to_string(),
                    params,
                },
            ],
        };
        
        let result = validate_patch(&patch, &schemas);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        // Should have multiple errors
        assert!(errors.len() >= 2);
    }

    #[test]
    fn test_empty_patch_is_valid() {
        let schemas = create_test_schemas();
        let patch = PatchGraph {
            modules: Vec::new(),
        };
        
        assert!(validate_patch(&patch, &schemas).is_ok());
    }
}
