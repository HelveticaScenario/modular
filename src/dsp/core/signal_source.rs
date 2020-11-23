use anyhow::anyhow;
use crate::types::Sampleable;

pub struct SignalSource {
    id: String,
    current_value: f32,
    next_value: f32
}

impl Sampleable for SignalSource {
    fn tick(&mut self) -> () {
        self.current_value = self.next_value;
    }

    fn update(&mut self, _patch: &std::collections::HashMap<String, Box<dyn Sampleable>>) -> () {
        // no-op
    }

    fn get_sample(&self, port: &String) -> anyhow::Result<f32> {
        if port != "output" {
            return Err(anyhow!("Signal Source with id {} has no port {}", self.id, port))
        }
        Ok(self.current_value)
    }
}

// pub fn SignalSourceConstructor()