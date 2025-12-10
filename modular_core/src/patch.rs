//! Core patch structure for DSP processing
//!
//! This module contains the core `Patch` struct which represents a graph of
//! connected audio modules. The patch contains sampleable modules and tracks
//! that can be processed to generate audio.

use crate::types::{ROOT_ID, ROOT_OUTPUT_PORT, SampleableMap, TrackMap};

/// The core patch structure containing the DSP graph
pub struct Patch {
    pub sampleables: SampleableMap,
    pub tracks: TrackMap,
}

impl Patch {
    /// Create a new empty patch
    pub fn new(sampleables: SampleableMap, tracks: TrackMap) -> Self {
        Patch {
            sampleables,
            tracks,
        }
    }

    /// Get the current state of all modules in the patch
    pub fn get_state(&self) -> Vec<crate::types::ModuleState> {
        self.sampleables
            .iter()
            .map(|(_, module)| module.get_state())
            .collect()
    }

    /// Get the output sample from the root module
    pub fn get_output(&self) -> f32 {
        if let Some(root) = self.sampleables.get(&ROOT_ID) {
            root.get_sample(&ROOT_OUTPUT_PORT).unwrap_or_default()
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_patch_new_empty() {
        let patch = Patch::new(HashMap::new(), HashMap::new());
        assert!(patch.sampleables.is_empty());
        assert!(patch.tracks.is_empty());
    }

    #[test]
    fn test_patch_get_state_empty() {
        let patch = Patch::new(HashMap::new(), HashMap::new());
        let state = patch.get_state();
        assert!(state.is_empty());
    }

    #[test]
    fn test_patch_get_output_no_root() {
        let patch = Patch::new(HashMap::new(), HashMap::new());
        let output = patch.get_output();
        assert!(
            (output - 0.0).abs() < 0.0001,
            "No root module should return 0.0"
        );
    }
}
