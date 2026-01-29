//! Audio input module - reads from the audio input ring buffer.
//!
//! This module allows reading audio from the system's audio input device.

use std::sync::Arc;

use napi::Result;
use parking_lot::Mutex;

use crate::{PORT_MAX_CHANNELS, Sampleable, poly::PolyOutput, types::MessageHandler};

#[derive(Default)]
pub struct AudioIn {
    pub input: Arc<Mutex<PolyOutput>>,
}

impl Sampleable for AudioIn {
    fn update(&self) {}

    fn get_id(&self) -> &str {
        "AUDIO_IN"
    }

    fn tick(&self) -> () {
        todo!()
    }

    fn get_poly_sample(&self, _port: &str) -> Result<PolyOutput> {
        Ok(*self.input.lock())
    }

    fn get_module_type(&self) -> &str {
        "AUDIO_IN"
    }

    fn try_update_params(&self, _params: serde_json::Value) -> Result<()> {
        Ok(())
    }

    fn connect(&self, _patch: &crate::Patch) {}

    fn on_patch_update(&self) {}

    fn get_state(&self) -> Option<serde_json::Value> {
        None
    }
}

impl MessageHandler for AudioIn {}
