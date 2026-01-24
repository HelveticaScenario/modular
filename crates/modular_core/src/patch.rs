//! Core patch structure for DSP processing
//!
//! This module contains the core `Patch` struct which represents a graph of
//! connected audio modules. The patch contains sampleable modules and tracks
//! that can be processed to generate audio.

use crate::types::{Message, MessageTag, ROOT_ID, ROOT_OUTPUT_PORT, Sampleable, SampleableMap};

use std::collections::HashMap;
use std::sync::{Arc, Weak};

#[derive(Clone)]
struct MessageListenerRef {
    id: String,
    weak: Weak<Box<dyn Sampleable>>,
}

/// The core patch structure containing the DSP graph
pub struct Patch {
    pub sampleables: SampleableMap,
    message_listeners: HashMap<MessageTag, Vec<MessageListenerRef>>,
}

impl Patch {
    /// Create a new empty patch
    pub fn new(sampleables: SampleableMap) -> Self {
        let mut patch = Patch {
            sampleables,
            message_listeners: HashMap::new(),
        };
        patch.rebuild_message_listeners();
        patch
    }

    pub fn rebuild_message_listeners(&mut self) {
        self.message_listeners.clear();
        for (id, sampleable) in &self.sampleables {
            for tag in sampleable.handled_message_tags() {
                self.message_listeners
                    .entry(*tag)
                    .or_default()
                    .push(MessageListenerRef {
                        id: id.clone(),
                        weak: Arc::downgrade(sampleable),
                    });
            }
        }
    }

    /// Collect strong references to all modules currently in this patch that
    /// have registered to handle the given message tag.
    ///
    /// This method prunes stale entries. In particular, it will never return a
    /// module that is no longer present in `self.sampleables`, even if some
    /// other subsystem still holds a strong `Arc` to that module.
    pub fn message_listeners_for(&mut self, tag: MessageTag) -> Vec<Arc<Box<dyn Sampleable>>> {
        let Some(list) = self.message_listeners.get_mut(&tag) else {
            return Vec::new();
        };

        list.retain(|r| {
            if !self.sampleables.contains_key(&r.id) {
                return false;
            }
            r.weak.upgrade().is_some()
        });

        list.iter()
            .filter(|r| self.sampleables.contains_key(&r.id))
            .filter_map(|r| r.weak.upgrade())
            .collect()
    }

    pub fn dispatch_message(&mut self, message: &Message) -> napi::Result<()> {
        let listeners = self.message_listeners_for(message.tag());
        for s in listeners {
            s.handle_message(message)?;
        }
        Ok(())
    }

    /// Get the output sample from the root module
    pub fn get_output(&self) -> f32 {
        if let Some(root) = self.sampleables.get(&*ROOT_ID) {
            root.get_poly_sample(&ROOT_OUTPUT_PORT)
                .map(|p| p.get(0))
                .unwrap_or_default()
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    use napi::Result;
    use crate::types::MessageHandler;

    #[test]
    fn test_patch_new_empty() {
        let patch = Patch::new(HashMap::new());
        assert!(patch.sampleables.is_empty());
    }

    #[test]
    fn test_patch_get_output_no_root() {
        let patch = Patch::new(HashMap::new());
        let output = patch.get_output();
        assert!(
            (output - 0.0).abs() < 0.0001,
            "No root module should return 0.0"
        );
    }

    struct DummyMessageSampleable {
        id: String,
    }

    impl Sampleable for DummyMessageSampleable {
        fn get_id(&self) -> &String {
            &self.id
        }

        fn tick(&self) {}

        fn update(&self) {}

        fn get_poly_sample(&self, _port: &String) -> Result<crate::poly::PolySignal> {
            Ok(crate::poly::PolySignal::mono(0.0))
        }

        fn get_module_type(&self) -> String {
            "dummy".to_string()
        }

        fn try_update_params(&self, _params: serde_json::Value) -> Result<()> {
            Ok(())
        }

        fn connect(&self, _patch: &Patch) {}
    }

    impl MessageHandler for DummyMessageSampleable {
        fn handled_message_tags(&self) -> &'static [MessageTag] {
            &[MessageTag::MidiNote]
        }

        fn handle_message(&self, _message: &Message) -> Result<()> {
            Ok(())
        }
    }

    #[test]
    fn message_listeners_never_return_removed_modules() {
        let s: Arc<Box<dyn Sampleable>> = Arc::new(Box::new(DummyMessageSampleable {
            id: "m1".to_string(),
        }));

        let mut sampleables: SampleableMap = HashMap::new();
        sampleables.insert("m1".to_string(), Arc::clone(&s));
        let mut patch = Patch::new(sampleables);

        // Index should include it.
        assert_eq!(patch.message_listeners_for(MessageTag::MidiNote).len(), 1);

        // Remove from patch but keep an external strong ref (`s`).
        patch.sampleables.remove("m1");

        // Rebuild/prune and ensure it is not returned.
        assert_eq!(patch.message_listeners_for(MessageTag::MidiNote).len(), 0);
    }
}
