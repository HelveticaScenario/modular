//! Audio input module - reads from the audio input ring buffer.
//!
//! This module allows reading audio from the system's audio input device.

use std::cell::UnsafeCell;
use std::sync::Arc;

use parking_lot::Mutex;

use crate::{
    poly::{PolyOutput, PORT_MAX_CHANNELS},
    types::{MessageHandler, WellKnownModule},
    Sampleable,
};

pub struct AudioIn {
    /// Shared with `Patch::audio_in`; kept for `insert_audio_in()` reconstruction.
    pub input: Arc<Mutex<PolyOutput>>,
    /// Pre-filled each block by `inject_audio_in_block` (§2 of the CPAL callback).
    /// One slot per sample; each slot holds all channels.
    /// Layout mirrors `BlockPort`: `block[sample_index][channel_index]`.
    ///
    /// # Safety
    ///
    /// Accessed only from the audio thread:
    ///   - Written during `inject_audio_in_block` (§2, before any processing).
    ///   - Read during `get_value_at` (§7, inside module processing).
    /// These phases are serialised on the same thread — no concurrent access.
    block: UnsafeCell<[[f32; PORT_MAX_CHANNELS]; 4096]>,
    /// Number of valid samples in `block` (= current CPAL block size).
    block_len: UnsafeCell<usize>,
}

impl Default for AudioIn {
    fn default() -> Self {
        Self {
            input: Arc::new(Mutex::new(PolyOutput::default())),
            block: UnsafeCell::new([[0.0f32; PORT_MAX_CHANNELS]; 4096]),
            block_len: UnsafeCell::new(0),
        }
    }
}

impl AudioIn {
    /// Create an `AudioIn` sharing an existing `input` Arc (used by `Patch::insert_audio_in`).
    pub fn with_input(input: Arc<Mutex<PolyOutput>>) -> Self {
        Self {
            input,
            block: UnsafeCell::new([[0.0f32; PORT_MAX_CHANNELS]; 4096]),
            block_len: UnsafeCell::new(0),
        }
    }
}

// SAFETY: See `block` field documentation above.
unsafe impl Sync for AudioIn {}

impl Sampleable for AudioIn {
    fn get_id(&self) -> &str {
        WellKnownModule::HiddenAudioIn.id()
    }

    fn tick(&self) {}

    /// Store the full input block so `get_value_at` can serve per-sample values.
    fn inject_audio_in_block(&self, block: &[[f32; PORT_MAX_CHANNELS]]) {
        let len = block.len().min(4096);
        unsafe {
            let stored = &mut *self.block.get();
            for (i, slot) in block.iter().take(len).enumerate() {
                stored[i] = *slot;
            }
            *self.block_len.get() = len;
        }
    }

    fn get_value_at(&self, _port: &str, ch: usize, index: usize) -> f32 {
        let len = unsafe { *self.block_len.get() };
        if index >= len || ch >= PORT_MAX_CHANNELS {
            return 0.0;
        }
        unsafe { (*self.block.get())[index][ch] }
    }

    fn get_module_type(&self) -> &str {
        WellKnownModule::HiddenAudioIn.id()
    }

    fn connect(&self, _patch: &crate::Patch) {}

    fn on_patch_update(&self) {}

    fn get_state(&self) -> Option<serde_json::Value> {
        None
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl MessageHandler for AudioIn {}
