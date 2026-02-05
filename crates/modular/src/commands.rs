//! Command queue types for audio thread communication.
//!
//! This module defines the commands sent from the main thread to the audio thread,
//! and the errors reported back from the audio thread.

use modular_core::types::{Message, ModuleIdRemap, Scope, ScopeItem};
use serde_json::Value;

/// A single atomic patch update - always processed as a complete unit.
///
/// This struct ensures the audio thread receives a complete, consistent batch of changes.
/// The main thread computes the entire diff and sends it as one unit.
pub struct PatchUpdate {
  /// Modules to insert (pre-constructed on main thread).
  /// The Box<dyn Sampleable> is Send so it can be transferred across thread boundary.
  pub inserts: Vec<(String, Box<dyn modular_core::types::Sampleable>)>,

  /// Module IDs to delete
  pub deletes: Vec<String>,

  /// ID remappings (applied before inserts/deletes)
  pub remaps: Vec<ModuleIdRemap>,

  /// Param updates for existing modules (module_id, params_json)
  pub param_updates: Vec<(String, Value)>,

  /// Scopes to add
  pub scope_adds: Vec<Scope>,

  /// Scopes to remove
  pub scope_removes: Vec<ScopeItem>,

  /// Scopes to update (existing scopes with new parameters)
  pub scope_updates: Vec<Scope>,

  /// Sample rate for new modules
  pub sample_rate: f32,
}

impl PatchUpdate {
  /// Create an empty patch update
  pub fn new(sample_rate: f32) -> Self {
    Self {
      inserts: Vec::new(),
      deletes: Vec::new(),
      remaps: Vec::new(),
      param_updates: Vec::new(),
      scope_adds: Vec::new(),
      scope_removes: Vec::new(),
      scope_updates: Vec::new(),
      sample_rate,
    }
  }

  /// Check if this update has any changes
  pub fn is_empty(&self) -> bool {
    self.inserts.is_empty()
      && self.deletes.is_empty()
      && self.remaps.is_empty()
      && self.param_updates.is_empty()
      && self.scope_adds.is_empty()
      && self.scope_removes.is_empty()
      && self.scope_updates.is_empty()
  }
}

/// Commands sent to audio thread via the command queue.
pub enum GraphCommand {
  /// Atomic patch update - all changes applied together
  PatchUpdate(PatchUpdate),

  /// MIDI/control messages (can be sent individually)
  DispatchMessage(Message),

  /// Transport control: start playback
  Start,

  /// Transport control: stop playback
  Stop,

  /// Clear the entire patch (used when stopped to reset state)
  ClearPatch,
}

/// Error types that can be reported from the audio thread back to the main thread.
#[derive(Debug, Clone)]
pub enum AudioError {
  /// Failed to update module parameters
  ParamUpdateFailed { module_id: String, message: String },

  /// Failed to dispatch a message
  MessageDispatchFailed { message: String },

  /// Module not found when trying to perform an operation
  ModuleNotFound { module_id: String },

  /// Generic error during patch processing
  PatchProcessingError { message: String },
}

impl std::fmt::Display for AudioError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      AudioError::ParamUpdateFailed { module_id, message } => {
        write!(f, "Failed to update params for {}: {}", module_id, message)
      }
      AudioError::MessageDispatchFailed { message } => {
        write!(f, "Failed to dispatch message: {}", message)
      }
      AudioError::ModuleNotFound { module_id } => {
        write!(f, "Module not found: {}", module_id)
      }
      AudioError::PatchProcessingError { message } => {
        write!(f, "Patch processing error: {}", message)
      }
    }
  }
}

impl std::error::Error for AudioError {}

/// Capacity for the command queue (main → audio)
pub const COMMAND_QUEUE_CAPACITY: usize = 1024;

/// Capacity for the error queue (audio → main)
pub const ERROR_QUEUE_CAPACITY: usize = 256;
