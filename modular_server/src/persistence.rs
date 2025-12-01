use std::path::Path;
use modular_core::types::PatchGraph;

/// Save a patch to a YAML file
pub fn save_patch(path: &Path, patch: &PatchGraph) -> anyhow::Result<()> {
    let yaml = serde_yaml::to_string(patch)?;
    std::fs::write(path, yaml)?;
    Ok(())
}

/// Load a patch from a YAML file
pub fn load_patch(path: &Path) -> anyhow::Result<PatchGraph> {
    let yaml = std::fs::read_to_string(path)?;
    let patch: PatchGraph = serde_yaml::from_str(&yaml)?;
    Ok(patch)
}

/// Create a default empty patch
pub fn default_patch() -> PatchGraph {
    PatchGraph {
        modules: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use modular_core::types::{ModuleState, Param};
    use std::collections::HashMap;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_save_load_roundtrip() {
        let mut params = HashMap::new();
        params.insert("freq".to_string(), Param::Value { value: 4.0 });
        
        let patch = PatchGraph {
            modules: vec![
                ModuleState {
                    id: "sine-1".to_string(),
                    module_type: "sine-oscillator".to_string(),
                    params,
                },
            ],
        };
        
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();
        
        save_patch(path, &patch).unwrap();
        let loaded = load_patch(path).unwrap();
        
        assert_eq!(loaded.modules.len(), 1);
        assert_eq!(loaded.modules[0].id, "sine-1");
        assert_eq!(loaded.modules[0].module_type, "sine-oscillator");
    }

    #[test]
    fn test_load_yaml_format() {
        let yaml = r#"
modules:
  - id: sine-1
    module_type: sine-oscillator
    params:
      freq:
        param_type: value
        value: 4.0
  - id: signal
    module_type: signal
    params:
      input:
        param_type: cable
        module: sine-1
        port: output
"#;
        
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml.as_bytes()).unwrap();
        
        let loaded = load_patch(temp_file.path()).unwrap();
        
        assert_eq!(loaded.modules.len(), 2);
        assert_eq!(loaded.modules[0].id, "sine-1");
        assert_eq!(loaded.modules[1].id, "signal");
    }

    #[test]
    fn test_load_nonexistent_file() {
        let result = load_patch(Path::new("/nonexistent/path.yaml"));
        assert!(result.is_err());
    }

    #[test]
    fn test_load_invalid_yaml() {
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"invalid: yaml: content: [").unwrap();
        
        let result = load_patch(temp_file.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_default_patch() {
        let patch = default_patch();
        assert!(patch.modules.is_empty());
    }
}
