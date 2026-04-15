use deserr::Deserr;
use schemars::JsonSchema;

use crate::{
    dsp::utils::SchmittTrigger,
    poly::{MonoSignal, MonoSignalExt, PolyOutput},
    Wav,
};

fn sampler_derive_channel_count(params: &SamplerParams) -> usize {
    params.wav.channel_count()
}

#[derive(Clone, Deserr, JsonSchema, Connect, ChannelCount, SignalParams)]
#[serde(rename_all = "camelCase")]
#[deserr(rename_all = camelCase, deny_unknown_fields)]
struct SamplerParams {
    wav: Wav,
    /// Gate input — rising edge starts playback from the beginning.
    #[signal(type = gate, range = (0.0, 5.0))]
    gate: MonoSignal,
    /// Playback speed. 1.0 = normal, 2.0 = double speed, negative = reverse.
    #[signal(default = 1.0, range = (-4.0, 4.0))]
    #[deserr(default)]
    speed: Option<MonoSignal>,
}

#[derive(Outputs, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct SamplerOutputs {
    #[output("output", "sample playback output", default)]
    sample: PolyOutput,
}

#[derive(Default)]
struct SamplerState {
    position: f64,
    playing: bool,
    gate_trigger: SchmittTrigger,
}

/// One-shot sample player. Plays a loaded WAV file from the beginning on each
/// gate rising edge. Speed control allows pitch-shifting and reverse playback.
///
/// ```js
/// $sampler($wavs().kick, $pulse('4hz'))
/// $sampler($wavs().tables.pad, $clock.beat, { speed: 0.5 })
/// ```
#[module(name = "$sampler", channels_derive = sampler_derive_channel_count, args(wav, gate))]
pub struct Sampler {
    params: SamplerParams,
    outputs: SamplerOutputs,
    state: SamplerState,
}

impl Sampler {
    fn update(&mut self, _sample_rate: f32) {
        let channels = self.channel_count();
        let frame_count = self.params.wav.frame_count();

        // Detect gate rising edge
        let gate_val = self.params.gate.get_value();
        if self.state.gate_trigger.process(gate_val) {
            self.state.position = 0.0;
            self.state.playing = true;
        }

        if !self.state.playing || !self.params.wav.is_loaded() || frame_count == 0 {
            for ch in 0..channels {
                self.outputs.sample.set(ch, 0.0);
            }
            return;
        }

        let speed = self.params.speed.value_or(1.0) as f64;
        let max_frame = (frame_count - 1) as f64;

        // Check bounds
        if self.state.position < 0.0 || self.state.position > max_frame {
            self.state.playing = false;
            for ch in 0..channels {
                self.outputs.sample.set(ch, 0.0);
            }
            return;
        }

        // Read with Hermite interpolation
        let pos = self.state.position as f32;
        for ch in 0..channels {
            let value = read_interpolated_wav(&self.params.wav, ch, pos);
            self.outputs.sample.set(ch, value);
        }

        // Advance position
        self.state.position += speed;
    }
}

message_handlers!(impl Sampler {});

/// Hermite (4-point cubic) interpolation for WAV data.
/// Non-wrapping — returns 0.0 for out-of-bounds positions.
fn read_interpolated_wav(wav: &Wav, channel: usize, frame: f32) -> f32 {
    if !frame.is_finite() {
        return 0.0;
    }

    let frame_count = wav.frame_count();
    if frame_count == 0 {
        return 0.0;
    }

    let max_frame = (frame_count - 1) as f32;
    if frame < 0.0 || frame > max_frame {
        return 0.0;
    }

    let left = frame.floor() as usize;
    let frac = frame - left as f32;
    if frac <= f32::EPSILON {
        return wav.read(channel, left);
    }

    let i0 = left.saturating_sub(1);
    let i1 = left;
    let i2 = (left + 1).min(frame_count - 1);
    let i3 = (left + 2).min(frame_count - 1);

    let y0 = wav.read(channel, i0);
    let y1 = wav.read(channel, i1);
    let y2 = wav.read(channel, i2);
    let y3 = wav.read(channel, i3);

    let c0 = y1;
    let c1 = 0.5 * (y2 - y0);
    let c2 = y0 - 2.5 * y1 + 2.0 * y2 - 0.5 * y3;
    let c3 = 0.5 * (y3 - y0) + 1.5 * (y1 - y2);

    ((c3 * frac + c2) * frac + c1) * frac + c0
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::dsp::{get_constructors, get_params_deserializers};
    use crate::params::DeserializedParams;
    use crate::patch::Patch;
    use crate::types::{SampleBuffer, Sampleable, WavData};

    const SAMPLE_RATE: f32 = 48000.0;

    fn make_module(
        module_type: &str,
        id: &str,
        params: serde_json::Value,
    ) -> Arc<Box<dyn Sampleable>> {
        let constructors = get_constructors();
        let deserializers = get_params_deserializers();
        let deserializer = deserializers
            .get(module_type)
            .unwrap_or_else(|| panic!("no params deserializer for '{module_type}'"));
        let cached = deserializer(params)
            .unwrap_or_else(|e| panic!("params deserialization for '{module_type}' failed: {e}"));
        let deserialized = DeserializedParams {
            params: cached.params,
            argument_spans: Default::default(),
            channel_count: cached.channel_count,
        };
        constructors
            .get(module_type)
            .unwrap_or_else(|| panic!("no constructor for '{module_type}'"))(
            &id.to_string(),
            SAMPLE_RATE,
            deserialized,
        )
        .unwrap_or_else(|e| panic!("constructor for '{module_type}' failed: {e}"))
    }

    fn step(module: &dyn Sampleable) {
        module.tick();
        module.update();
    }

    fn make_test_wav(samples: Vec<Vec<f32>>) -> Arc<WavData> {
        Arc::new(WavData::new(
            SampleBuffer::from_samples(samples),
            SAMPLE_RATE,
        ))
    }

    #[test]
    fn sampler_outputs_silence_when_not_triggered() {
        let wav_data = make_test_wav(vec![vec![1.0, 2.0, 3.0, 4.0]]);
        let module = make_module(
            "$sampler",
            "s1",
            serde_json::json!({
                "wav": { "type": "wav_ref", "path": "test", "channels": 1 },
                "gate": 0.0,
                "speed": 1.0,
            }),
        );

        // Connect with wav_data in patch
        let mut patch = Patch::new();
        patch.wav_data.insert("test".to_string(), wav_data);
        module.connect(&patch);

        // Run a few samples — no trigger, should output silence
        for _ in 0..4 {
            step(&**module);
        }

        let output = module.get_poly_sample("output").unwrap();
        assert_eq!(output.get(0), 0.0);
    }

    #[test]
    fn sampler_plays_on_gate_rising_edge() {
        // 4-frame mono WAV: [1.0, 2.0, 3.0, 4.0]
        let wav_data = make_test_wav(vec![vec![1.0, 2.0, 3.0, 4.0]]);
        let module = make_module(
            "$sampler",
            "s2",
            serde_json::json!({
                "wav": { "type": "wav_ref", "path": "test", "channels": 1 },
                "gate": 5.0,
                "speed": 1.0,
            }),
        );

        let mut patch = Patch::new();
        patch.wav_data.insert("test".to_string(), wav_data);
        module.connect(&patch);

        // First tick: gate is high, Schmitt trigger detects rising edge, position resets to 0
        step(&**module);
        let out = module.get_poly_sample("output").unwrap();
        assert!(
            (out.get(0) - 1.0).abs() < 1e-6,
            "expected 1.0, got {}",
            out.get(0)
        );
    }

    #[test]
    fn sampler_outputs_silence_after_sample_ends() {
        let wav_data = make_test_wav(vec![vec![1.0, 2.0]]);
        let module = make_module(
            "$sampler",
            "s3",
            serde_json::json!({
                "wav": { "type": "wav_ref", "path": "test", "channels": 1 },
                "gate": 5.0,
                "speed": 1.0,
            }),
        );

        let mut patch = Patch::new();
        patch.wav_data.insert("test".to_string(), wav_data);
        module.connect(&patch);

        // Play through the 2-frame sample
        step(&**module); // frame 0 -> output 1.0
        step(&**module); // frame 1 -> output 2.0
        step(&**module); // frame 2 -> past end -> silence

        let out = module.get_poly_sample("output").unwrap();
        assert_eq!(out.get(0), 0.0, "should be silent after sample ends");
    }
}
