use anyhow::{Context, Result};
use modular_core::types::PatchGraph;
use std::fs;
use std::path::Path;

/// Save a patch to a YAML file
pub fn save_patch(path: &Path, patch: &PatchGraph) -> Result<()> {
    let yaml = serde_yaml::to_string(patch)
        .context("Failed to serialize patch to YAML")?;
    fs::write(path, yaml)
        .with_context(|| format!("Failed to write patch to {}", path.display()))?;
    Ok(())
}

/// Load a patch from a YAML file
pub fn load_patch(path: &Path) -> Result<PatchGraph> {
    let yaml = fs::read_to_string(path)
        .with_context(|| format!("Failed to read patch from {}", path.display()))?;
    let patch: PatchGraph = serde_yaml::from_str(&yaml)
        .context("Failed to parse patch YAML")?;
    Ok(patch)
}

/// Create an empty default patch
pub fn create_default_patch() -> PatchGraph {
    PatchGraph {
        modules: vec![],
        tracks: vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use modular_core::types::{ModuleState, Param};
    use std::collections::HashMap;
    use std::io::Write;
    use tempfile::{tempdir, NamedTempFile};
    
    #[test]
    fn test_save_load_roundtrip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test_patch.yaml");
        
        let mut params = HashMap::new();
        params.insert("freq".to_string(), Param::Value { value: 4.0 });
        
        let patch = PatchGraph {
            modules: vec![
                ModuleState {
                    id: "sine-1".to_string(),
                    module_type: "sine-oscillator".to_string(),
                    params,
                }
            ],
            tracks: vec![],
        };
        
        save_patch(&path, &patch).unwrap();
        let loaded = load_patch(&path).unwrap();
        
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
    fn test_create_default_patch() {
        let patch = create_default_patch();
        assert!(patch.modules.is_empty());
    }
}
