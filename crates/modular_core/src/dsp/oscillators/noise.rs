use napi::Result;
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Deserialize, Default, JsonSchema, Connect, ChannelCount)]
#[serde(default)]
struct NoiseParams {
    /// color of the noise: white, pink, brown
    color: NoiseKind,
}

#[derive(Clone, Copy, Deserialize, JsonSchema, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum NoiseKind {
    White,
    Pink,
    Brown,
}

impl Default for NoiseKind {
    fn default() -> Self {
        NoiseKind::White
    }
}

impl crate::types::Connect for NoiseKind {
    fn connect(&mut self, _patch: &crate::Patch) {}
}

#[derive(Default)]
struct PinkFilter {
    b0: f32,
    b1: f32,
    b2: f32,
    b3: f32,
    b4: f32,
    b5: f32,
    b6: f32,
}

impl PinkFilter {
    fn process(&mut self, white: f32) -> f32 {
        self.b0 = 0.99886 * self.b0 + white * 0.0555179;
        self.b1 = 0.99332 * self.b1 + white * 0.0750759;
        self.b2 = 0.96900 * self.b2 + white * 0.1538520;
        self.b3 = 0.86650 * self.b3 + white * 0.3104856;
        self.b4 = 0.55000 * self.b4 + white * 0.5329522;
        self.b5 = -0.7616 * self.b5 - white * 0.0168980;
        self.b6 = white * 0.5362;

        let pink =
            self.b0 + self.b1 + self.b2 + self.b3 + self.b4 + self.b5 + self.b6 + white * 0.115926;
        (pink * 0.11).clamp(-1.0, 1.0)
    }

    fn reset(&mut self) {
        *self = Self::default();
    }
}

#[derive(Default)]
struct LcgRng {
    state: u64,
}

impl LcgRng {
    fn next(&mut self) -> f32 {
        self.state = self.state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let bits = (self.state >> 32) as u32;
        let value = bits as f32 / u32::MAX as f32;
        value * 2.0 - 1.0
    }
}

#[derive(Module)]
#[module("noise", "Noise generator with selectable color")]
#[args(color)]
pub struct Noise {
    outputs: NoiseOutputs,
    params: NoiseParams,
    generator: LcgRng,
    pink: PinkFilter,
    brown: f32,
    last_noise_type: NoiseKind,
}

#[derive(Outputs, JsonSchema)]
struct NoiseOutputs {
    #[output("output", "signal output", default, range = (-5.0, 5.0))]
    sample: f32,
}

impl Default for Noise {
    fn default() -> Self {
        Self {
            outputs: NoiseOutputs::default(),
            params: NoiseParams::default(),
            generator: LcgRng {
                state: 0x1234_5678_9abc_def0,
            },
            pink: PinkFilter::default(),
            brown: 0.0,
            last_noise_type: NoiseKind::default(),
        }
    }
}

impl Noise {
    fn refresh_kind(&mut self) {
        if self.last_noise_type != self.params.color {
            self.last_noise_type = self.params.color;
            self.pink.reset();
            self.brown = 0.0;
        }
    }

    fn process_brown(&mut self, white: f32) -> f32 {
        self.brown = (self.brown + white * 0.02).clamp(-1.0, 1.0);
        self.brown
    }

    fn update(&mut self, _sample_rate: f32) -> () {
        // self.params
        self.refresh_kind();
        let white = self.generator.next();
        let colored = match self.params.color {
            NoiseKind::White => white,
            NoiseKind::Pink => self.pink.process(white),
            NoiseKind::Brown => self.process_brown(white),
        };

        self.outputs.sample = colored.clamp(-1.0, 1.0) * 5.0;
    }
}

message_handlers!(impl Noise {});
