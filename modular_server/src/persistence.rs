use modular_core::types::PatchGraph;

/// Create an empty default patch
pub fn create_default_patch() -> PatchGraph {
    PatchGraph {
        modules: vec![],
        tracks: vec![],
        scopes: vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_default_patch() {
        let patch = create_default_patch();
        assert!(patch.modules.is_empty());
        assert!(patch.tracks.is_empty());
    }
}
