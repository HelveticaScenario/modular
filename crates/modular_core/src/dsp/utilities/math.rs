use crate::dsp::utils::{hz_to_voct_f64, voct_to_hz_f64};
use crate::poly::MonoSignal;
use crate::types::{ClockMessages, Connect, Signal};
use fasteval::{Compiler, Evaler, Instruction};
use napi::Result;
use regex::Regex;
use schemars::JsonSchema;
use serde::{Deserialize, Deserializer};
use std::collections::BTreeMap;
use std::sync::Weak;

#[derive(Default, JsonSchema)]
#[serde(transparent)]
#[schemars(transparent)]
struct MathExpressionParam {
    #[allow(dead_code)]
    source: String,

    #[serde(skip)]
    #[schemars(skip)]
    signals: Vec<Signal>,

    #[serde(skip)]
    #[schemars(skip)]
    slab: fasteval::Slab,

    #[serde(skip)]
    #[schemars(skip)]
    instruction: Instruction,
}

impl<'de> Deserialize<'de> for MathExpressionParam {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let source = String::deserialize(deserializer)?;

        // Parse source to find module(id:port:channel)
        // Replace with module(index)
        // Store signals

        let re = Regex::new(r"module\(([a-zA-Z0-9\-_]+):([a-zA-Z0-9\-_]+):(\d+)\)")
            .map_err(serde::de::Error::custom)?;
        let mut signals = Vec::new();

        // We need to replace all occurrences.
        let result = re.replace_all(&source, |caps: &regex::Captures| {
            let module = caps[1].to_string();
            let port = caps[2].to_string();
            let channel: usize = caps[3].parse().unwrap_or(0);
            signals.push(Signal::Cable {
                module,
                module_ptr: Weak::default(),
                port,
                channel,
            });
            format!("module{}", signals.len() - 1)
        });

        let mut slab = fasteval::Slab::new();
        let parser = fasteval::Parser::new();
        let instruction = match parser.parse(&result, &mut slab.ps) {
            Err(e) => {
                return Err(serde::de::Error::custom(format!(
                    "Failed to parse expression: {}",
                    e
                )));
            }
            Ok(expression) => expression.from(&slab.ps).compile(&slab.ps, &mut slab.cs),
        };

        Ok(MathExpressionParam {
            source,
            signals,
            slab,
            instruction,
        })
    }
}

impl Connect for MathExpressionParam {
    fn connect(&mut self, patch: &crate::Patch) {
        for signal in &mut self.signals {
            signal.connect(patch);
        }
    }
}

#[derive(Deserialize, Default, JsonSchema, ChannelCount)]
#[serde(default, rename_all = "camelCase")]
struct MathParams {
    expression: MathExpressionParam,
    x: MonoSignal,
    y: MonoSignal,
    z: MonoSignal,
}

impl Connect for MathParams {
    fn connect(&mut self, patch: &crate::Patch) {
        Connect::connect(&mut self.expression, patch);
        Connect::connect(&mut self.x, patch);
        Connect::connect(&mut self.y, patch);
        Connect::connect(&mut self.z, patch);
    }
}

#[derive(Outputs, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct MathOutputs {
    #[output("output", "result of the expression", default)]
    output: f32,
}

#[module(
    name = "$math",
    description = "Math expression evaluator",
    args(expression)
)]
pub struct Math {
    outputs: MathOutputs,
    params: MathParams,

    // State
    phase: f32,
    loop_index: usize,
    running: bool,
}

impl Default for Math {
    fn default() -> Self {
        Self {
            outputs: MathOutputs::default(),
            params: MathParams::default(),
            phase: 0.0,
            loop_index: 0,
            running: true,
            _channel_count: 0,
        }
    }
}

message_handlers!(impl Math {
    Clock(m) => Math::on_clock_message,
});

impl Math {
    fn update(&mut self, sample_rate: f32) {
        // Update time
        if self.running {
            self.phase += 1.0 / sample_rate;
            if self.phase >= 1.0 {
                self.phase -= 1.0;
                self.loop_index += 1;
            }
        }

        self.outputs.output = self.eval().unwrap_or(0.0) as f32;
    }

    fn eval(&mut self) -> std::result::Result<f64, fasteval::Error> {
        let x = self.params.x.get_value_or(0.0) as f64;
        let y = self.params.y.get_value_or(0.0) as f64;
        let z = self.params.z.get_value_or(0.0) as f64;
        let t = self.phase as f64 + self.loop_index as f64;
        let signals = self
            .params
            .expression
            .signals
            .iter()
            .map(|s| s.get_value_or(0.0) as f64)
            .collect::<Vec<_>>();

        let mut btree = BTreeMap::new();
        btree.insert("x".to_string(), x);
        btree.insert("y".to_string(), y);
        btree.insert("z".to_string(), z);
        btree.insert("t".to_string(), t);
        for (i, val) in signals.iter().enumerate() {
            btree.insert(format!("module{}", i).to_string(), *val);
        }

        let mut cb = move |name: &str, args: Vec<f64>| -> Option<f64> {
            if let Some(val) = btree.get(name) {
                return Some(*val);
            }
            match name {
                "vToHz" => args.first().map(|v| voct_to_hz_f64(*v)),
                "hzToV" => args.first().map(|v| hz_to_voct_f64(*v)),

                // A wildcard to handle all undefined names:
                _ => None,
            }
        };
        // let mut ns = fasteval::CachedCallbackNamespace::new(cb);

        Ok({
            let evaler = &self.params.expression.instruction;
            if let fasteval::IConst(c) = evaler {
                *c
            } else {
                evaler.eval(&self.params.expression.slab, &mut cb)?
            }
        })
    }

    fn on_clock_message(&mut self, m: &ClockMessages) -> Result<()> {
        match m {
            ClockMessages::Start => {
                self.running = true;
                self.phase = 0.0;
                self.loop_index = 0;
            }
            ClockMessages::Stop => {
                self.running = false;
            }
        }
        Ok(())
    }
}
