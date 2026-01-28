//! Macro for generating Mutable Instruments engine wrapper modules.
//!
//! All MI engine modules share nearly identical structure. This macro generates
//! the params struct, outputs struct, main module struct, and implementation
//! from a concise declaration.

/// Generates a complete MI engine wrapper module.
///
/// # Syntax
///
/// ```ignore
/// mi_engine_module! {
///     name: "mi.modulename",
///     doc: "Module description",
///     struct_name: ModuleName,
///     engine_type: EngineName,           // plain type, EngineName<'static>, or EngineName<'a> for lifetime
///     engine_path: mi_plaits_dsp::engine::engine_module::EngineName,
///     constructor: new(),                // or new(BLOCK_SIZE)
///     output_range: (-1.0, 1.0),
///     output_doc: "main output description",
///     aux_range: (-1.0, 1.0),
///     aux_doc: "auxiliary output description",
///     params: {
///         freq: "frequency in v/oct",
///         timbre: "timbre parameter description",
///         morph: "morph parameter description",
///         harmonics: "harmonics parameter description",
///         sync: "sync input description",
///     }
/// }
/// ```
#[macro_export]
macro_rules! mi_engine_module {
    // NOTE: More specific patterns (with lifetimes) MUST come BEFORE less specific patterns ($engine_type:ty)
    // because Rust macros match the first matching arm.

    // Variant WITH parameterized lifetime ('a) - struct gets lifetime param
    (
        name: $module_name:literal,
        doc: $module_doc:literal,
        struct_name: $struct_name:ident,
        engine_type: $engine_type:ident<'a>,
        engine_path: $engine_path:path,
        constructor: new(),
        output_range: ($out_min:expr, $out_max:expr),
        output_doc: $output_doc:literal,
        aux_range: ($aux_min:expr, $aux_max:expr),
        aux_doc: $aux_doc:literal,
        params: {
            freq: $freq_doc:literal,
            timbre: $timbre_doc:literal,
            morph: $morph_doc:literal,
            harmonics: $harmonics_doc:literal,
            sync: $sync_doc:literal $(,)?
        } $(,)?
    ) => {
        $crate::mi_engine_module_impl! {
            @with_lifetime
            name: $module_name,
            doc: $module_doc,
            struct_name: $struct_name,
            engine_type: $engine_type,
            engine_path: $engine_path,
            constructor: { $engine_type::new() },
            output_range: ($out_min, $out_max),
            output_doc: $output_doc,
            aux_range: ($aux_min, $aux_max),
            aux_doc: $aux_doc,
            params: {
                freq: $freq_doc,
                timbre: $timbre_doc,
                morph: $morph_doc,
                harmonics: $harmonics_doc,
                sync: $sync_doc,
            }
        }
    };

    // Variant WITH parameterized lifetime ('a) AND BLOCK_SIZE constructor
    (
        name: $module_name:literal,
        doc: $module_doc:literal,
        struct_name: $struct_name:ident,
        engine_type: $engine_type:ident<'a>,
        engine_path: $engine_path:path,
        constructor: new(BLOCK_SIZE),
        output_range: ($out_min:expr, $out_max:expr),
        output_doc: $output_doc:literal,
        aux_range: ($aux_min:expr, $aux_max:expr),
        aux_doc: $aux_doc:literal,
        params: {
            freq: $freq_doc:literal,
            timbre: $timbre_doc:literal,
            morph: $morph_doc:literal,
            harmonics: $harmonics_doc:literal,
            sync: $sync_doc:literal $(,)?
        } $(,)?
    ) => {
        $crate::mi_engine_module_impl! {
            @with_lifetime
            name: $module_name,
            doc: $module_doc,
            struct_name: $struct_name,
            engine_type: $engine_type,
            engine_path: $engine_path,
            constructor: { $engine_type::new(BLOCK_SIZE) },
            output_range: ($out_min, $out_max),
            output_doc: $output_doc,
            aux_range: ($aux_min, $aux_max),
            aux_doc: $aux_doc,
            params: {
                freq: $freq_doc,
                timbre: $timbre_doc,
                morph: $morph_doc,
                harmonics: $harmonics_doc,
                sync: $sync_doc,
            }
        }
    };

    // Variant WITH 'static lifetime on engine (struct has NO lifetime param)
    (
        name: $module_name:literal,
        doc: $module_doc:literal,
        struct_name: $struct_name:ident,
        engine_type: $engine_type:ident<'static>,
        engine_path: $engine_path:path,
        constructor: new(),
        output_range: ($out_min:expr, $out_max:expr),
        output_doc: $output_doc:literal,
        aux_range: ($aux_min:expr, $aux_max:expr),
        aux_doc: $aux_doc:literal,
        params: {
            freq: $freq_doc:literal,
            timbre: $timbre_doc:literal,
            morph: $morph_doc:literal,
            harmonics: $harmonics_doc:literal,
            sync: $sync_doc:literal $(,)?
        } $(,)?
    ) => {
        $crate::mi_engine_module_impl! {
            @static_lifetime
            name: $module_name,
            doc: $module_doc,
            struct_name: $struct_name,
            engine_type: $engine_type,
            engine_path: $engine_path,
            constructor: { $engine_type::new() },
            output_range: ($out_min, $out_max),
            output_doc: $output_doc,
            aux_range: ($aux_min, $aux_max),
            aux_doc: $aux_doc,
            params: {
                freq: $freq_doc,
                timbre: $timbre_doc,
                morph: $morph_doc,
                harmonics: $harmonics_doc,
                sync: $sync_doc,
            }
        }
    };

    // Variant WITH 'static lifetime on engine AND BLOCK_SIZE constructor
    (
        name: $module_name:literal,
        doc: $module_doc:literal,
        struct_name: $struct_name:ident,
        engine_type: $engine_type:ident<'static>,
        engine_path: $engine_path:path,
        constructor: new(BLOCK_SIZE),
        output_range: ($out_min:expr, $out_max:expr),
        output_doc: $output_doc:literal,
        aux_range: ($aux_min:expr, $aux_max:expr),
        aux_doc: $aux_doc:literal,
        params: {
            freq: $freq_doc:literal,
            timbre: $timbre_doc:literal,
            morph: $morph_doc:literal,
            harmonics: $harmonics_doc:literal,
            sync: $sync_doc:literal $(,)?
        } $(,)?
    ) => {
        $crate::mi_engine_module_impl! {
            @static_lifetime
            name: $module_name,
            doc: $module_doc,
            struct_name: $struct_name,
            engine_type: $engine_type,
            engine_path: $engine_path,
            constructor: { $engine_type::new(BLOCK_SIZE) },
            output_range: ($out_min, $out_max),
            output_doc: $output_doc,
            aux_range: ($aux_min, $aux_max),
            aux_doc: $aux_doc,
            params: {
                freq: $freq_doc,
                timbre: $timbre_doc,
                morph: $morph_doc,
                harmonics: $harmonics_doc,
                sync: $sync_doc,
            }
        }
    };

    // Variant WITHOUT lifetime (most common) - MUST come AFTER lifetime variants
    (
        name: $module_name:literal,
        doc: $module_doc:literal,
        struct_name: $struct_name:ident,
        engine_type: $engine_type:ty,
        engine_path: $engine_path:path,
        constructor: new(),
        output_range: ($out_min:expr, $out_max:expr),
        output_doc: $output_doc:literal,
        aux_range: ($aux_min:expr, $aux_max:expr),
        aux_doc: $aux_doc:literal,
        params: {
            freq: $freq_doc:literal,
            timbre: $timbre_doc:literal,
            morph: $morph_doc:literal,
            harmonics: $harmonics_doc:literal,
            sync: $sync_doc:literal $(,)?
        } $(,)?
    ) => {
        $crate::mi_engine_module_impl! {
            @no_lifetime
            name: $module_name,
            doc: $module_doc,
            struct_name: $struct_name,
            engine_type: $engine_type,
            engine_path: $engine_path,
            constructor: { <$engine_type>::new() },
            output_range: ($out_min, $out_max),
            output_doc: $output_doc,
            aux_range: ($aux_min, $aux_max),
            aux_doc: $aux_doc,
            params: {
                freq: $freq_doc,
                timbre: $timbre_doc,
                morph: $morph_doc,
                harmonics: $harmonics_doc,
                sync: $sync_doc,
            }
        }
    };

    // Variant WITHOUT lifetime, WITH BLOCK_SIZE constructor
    (
        name: $module_name:literal,
        doc: $module_doc:literal,
        struct_name: $struct_name:ident,
        engine_type: $engine_type:ty,
        engine_path: $engine_path:path,
        constructor: new(BLOCK_SIZE),
        output_range: ($out_min:expr, $out_max:expr),
        output_doc: $output_doc:literal,
        aux_range: ($aux_min:expr, $aux_max:expr),
        aux_doc: $aux_doc:literal,
        params: {
            freq: $freq_doc:literal,
            timbre: $timbre_doc:literal,
            morph: $morph_doc:literal,
            harmonics: $harmonics_doc:literal,
            sync: $sync_doc:literal $(,)?
        } $(,)?
    ) => {
        $crate::mi_engine_module_impl! {
            @no_lifetime
            name: $module_name,
            doc: $module_doc,
            struct_name: $struct_name,
            engine_type: $engine_type,
            engine_path: $engine_path,
            constructor: { <$engine_type>::new(BLOCK_SIZE) },
            output_range: ($out_min, $out_max),
            output_doc: $output_doc,
            aux_range: ($aux_min, $aux_max),
            aux_doc: $aux_doc,
            params: {
                freq: $freq_doc,
                timbre: $timbre_doc,
                morph: $morph_doc,
                harmonics: $harmonics_doc,
                sync: $sync_doc,
            }
        }
    };
}

/// Internal implementation macro - generates the actual code
#[macro_export]
macro_rules! mi_engine_module_impl {
    // NO LIFETIME variant
    (
        @no_lifetime
        name: $module_name:literal,
        doc: $module_doc:literal,
        struct_name: $struct_name:ident,
        engine_type: $engine_type:ty,
        engine_path: $engine_path:path,
        constructor: { $constructor:expr },
        output_range: ($out_min:expr, $out_max:expr),
        output_doc: $output_doc:literal,
        aux_range: ($aux_min:expr, $aux_max:expr),
        aux_doc: $aux_doc:literal,
        params: {
            freq: $freq_doc:literal,
            timbre: $timbre_doc:literal,
            morph: $morph_doc:literal,
            harmonics: $harmonics_doc:literal,
            sync: $sync_doc:literal $(,)?
        } $(,)?
    ) => {
        paste::paste! {
            use napi::Result;
            use schemars::JsonSchema;
            use serde::Deserialize;

            use crate::{
                dsp::utils::voct_to_midi,
                types::{Clickless, Signal},
            };
            use mi_plaits_dsp::engine::{Engine, EngineParameters, TriggerState};
            use $engine_path;

            const BLOCK_SIZE: usize = 1;

            #[derive(Deserialize, Default, JsonSchema, Connect, ChannelCount)]
            #[serde(default)]
            struct [<$struct_name Params>] {
                #[doc = $freq_doc]
                freq: Signal,
                #[doc = $timbre_doc]
                timbre: Signal,
                #[doc = $morph_doc]
                morph: Signal,
                #[doc = $harmonics_doc]
                harmonics: Signal,
                #[doc = $sync_doc]
                sync: Signal,
            }

            #[derive(Outputs, JsonSchema)]
            struct [<$struct_name Outputs>] {
                #[output("main", $output_doc, range = ($out_min, $out_max))]
                sample: f32,
                #[output("aux", $aux_doc, range = ($aux_min, $aux_max))]
                aux: f32,
            }

            #[doc = $module_doc]
            #[derive(Default, Module)]
            #[module($module_name, $module_doc)]
            #[args(freq)]
            pub struct $struct_name {
                outputs: [<$struct_name Outputs>],
                engine: Option<$engine_type>,
                buffer_out: Vec<f32>,
                buffer_aux: Vec<f32>,
                buffer_pos: usize,
                last_sync: f32,
                sample_rate: f32,
                freq: Clickless,
                timbre: Clickless,
                morph: Clickless,
                harmonics: Clickless,
                params: [<$struct_name Params>],
            }

            impl $struct_name {
                fn update(&mut self, sample_rate: f32) {
                    if self.engine.is_none() || (self.sample_rate - sample_rate).abs() > 0.1 {
                        let mut engine = $constructor;
                        engine.init(sample_rate);
                        self.engine = Some(engine);
                        self.sample_rate = sample_rate;
                        self.buffer_out = vec![0.0; BLOCK_SIZE];
                        self.buffer_aux = vec![0.0; BLOCK_SIZE];
                        self.buffer_pos = BLOCK_SIZE;
                    }

                    if self.buffer_pos >= BLOCK_SIZE {
                        self.render_block(sample_rate);
                        self.buffer_pos = 0;
                    }

                    self.outputs.sample = self.buffer_out[self.buffer_pos];
                    self.outputs.aux = self.buffer_aux[self.buffer_pos];

                    self.buffer_pos += 1;
                }

                fn render_block(&mut self, sample_rate: f32) {
                    if let Some(ref mut engine) = self.engine {
                        self.freq
                            .update(self.params.freq.get_value_or(4.0).clamp(-10.0, 10.0));
                        self.timbre
                            .update(self.params.timbre.get_value_or(2.5).clamp(0.0, 5.0));
                        self.morph
                            .update(self.params.morph.get_value_or(2.5).clamp(0.0, 5.0));
                        self.harmonics
                            .update(self.params.harmonics.get_value_or(2.5).clamp(0.0, 5.0));

                        let midi_note = voct_to_midi(*self.freq);

                        let timbre_norm = (*self.timbre) / 5.0;
                        let morph_norm = (*self.morph) / 5.0;
                        let harmonics_norm = (*self.harmonics) / 5.0;

                        let trigger_state = if self.params.sync == Signal::Disconnected {
                            TriggerState::Unpatched
                        } else {
                            let sync_val = self.params.sync.get_value_or(0.0);
                            if sync_val > 0.0 && self.last_sync <= 0.0 {
                                self.last_sync = sync_val;
                                TriggerState::RisingEdge
                            } else if sync_val > 0.0 {
                                TriggerState::High
                            } else {
                                self.last_sync = sync_val;
                                TriggerState::Low
                            }
                        };

                        let engine_params = EngineParameters {
                            trigger: trigger_state,
                            note: midi_note,
                            timbre: timbre_norm,
                            morph: morph_norm,
                            harmonics: harmonics_norm,
                            accent: 1.0,
                            a0_normalized: 55.0 / sample_rate,
                        };

                        let mut already_enveloped = false;
                        engine.render(
                            &engine_params,
                            &mut self.buffer_out,
                            &mut self.buffer_aux,
                            &mut already_enveloped,
                        );
                    }
                }
            }

            message_handlers!(impl $struct_name {});
        }
    };

    // STATIC LIFETIME variant - engine uses 'static, struct has no lifetime param
    (
        @static_lifetime
        name: $module_name:literal,
        doc: $module_doc:literal,
        struct_name: $struct_name:ident,
        engine_type: $engine_type:ident,
        engine_path: $engine_path:path,
        constructor: { $constructor:expr },
        output_range: ($out_min:expr, $out_max:expr),
        output_doc: $output_doc:literal,
        aux_range: ($aux_min:expr, $aux_max:expr),
        aux_doc: $aux_doc:literal,
        params: {
            freq: $freq_doc:literal,
            timbre: $timbre_doc:literal,
            morph: $morph_doc:literal,
            harmonics: $harmonics_doc:literal,
            sync: $sync_doc:literal $(,)?
        } $(,)?
    ) => {
        paste::paste! {
            use napi::Result;
            use schemars::JsonSchema;
            use serde::Deserialize;

            use crate::{
                dsp::utils::voct_to_midi,
                types::{Clickless, Signal},
            };
            use mi_plaits_dsp::engine::{Engine, EngineParameters, TriggerState};
            use $engine_path;

            const BLOCK_SIZE: usize = 1;

            #[derive(Deserialize, Default, JsonSchema, Connect, ChannelCount)]
            #[serde(default)]
            struct [<$struct_name Params>] {
                #[doc = $freq_doc]
                freq: Signal,
                #[doc = $timbre_doc]
                timbre: Signal,
                #[doc = $morph_doc]
                morph: Signal,
                #[doc = $harmonics_doc]
                harmonics: Signal,
                #[doc = $sync_doc]
                sync: Signal,
            }

            #[derive(Outputs, JsonSchema)]
            struct [<$struct_name Outputs>] {
                #[output("output", $output_doc, range = ($out_min, $out_max))]
                sample: f32,
                #[output("aux", $aux_doc, range = ($aux_min, $aux_max))]
                aux: f32,
            }

            #[doc = $module_doc]
            #[derive(Default, Module)]
            #[module($module_name, $module_doc)]
            #[args(freq)]
            pub struct $struct_name {
                outputs: [<$struct_name Outputs>],
                engine: Option<$engine_type<'static>>,
                buffer_out: Vec<f32>,
                buffer_aux: Vec<f32>,
                buffer_pos: usize,
                last_sync: f32,
                sample_rate: f32,
                freq: Clickless,
                timbre: Clickless,
                morph: Clickless,
                harmonics: Clickless,
                params: [<$struct_name Params>],
            }

            impl $struct_name {
                fn update(&mut self, sample_rate: f32) {
                    if self.engine.is_none() || (self.sample_rate - sample_rate).abs() > 0.1 {
                        let mut engine = $constructor;
                        engine.init(sample_rate);
                        self.engine = Some(engine);
                        self.sample_rate = sample_rate;
                        self.buffer_out = vec![0.0; BLOCK_SIZE];
                        self.buffer_aux = vec![0.0; BLOCK_SIZE];
                        self.buffer_pos = BLOCK_SIZE;
                    }

                    if self.buffer_pos >= BLOCK_SIZE {
                        self.render_block(sample_rate);
                        self.buffer_pos = 0;
                    }

                    self.outputs.sample = self.buffer_out[self.buffer_pos];
                    self.outputs.aux = self.buffer_aux[self.buffer_pos];

                    self.buffer_pos += 1;
                }

                fn render_block(&mut self, sample_rate: f32) {
                    if let Some(ref mut engine) = self.engine {
                        self.freq
                            .update(self.params.freq.get_value_or(4.0).clamp(-10.0, 10.0));
                        self.timbre
                            .update(self.params.timbre.get_value_or(2.5).clamp(0.0, 5.0));
                        self.morph
                            .update(self.params.morph.get_value_or(2.5).clamp(0.0, 5.0));
                        self.harmonics
                            .update(self.params.harmonics.get_value_or(2.5).clamp(0.0, 5.0));

                        let midi_note = voct_to_midi(*self.freq);

                        let timbre_norm = (*self.timbre) / 5.0;
                        let morph_norm = (*self.morph) / 5.0;
                        let harmonics_norm = (*self.harmonics) / 5.0;

                        let trigger_state = if self.params.sync == Signal::Disconnected {
                            TriggerState::Unpatched
                        } else {
                            let sync_val = self.params.sync.get_value_or(0.0);
                            if sync_val > 0.0 && self.last_sync <= 0.0 {
                                self.last_sync = sync_val;
                                TriggerState::RisingEdge
                            } else if sync_val > 0.0 {
                                TriggerState::High
                            } else {
                                self.last_sync = sync_val;
                                TriggerState::Low
                            }
                        };

                        let engine_params = EngineParameters {
                            trigger: trigger_state,
                            note: midi_note,
                            timbre: timbre_norm,
                            morph: morph_norm,
                            harmonics: harmonics_norm,
                            accent: 1.0,
                            a0_normalized: 55.0 / sample_rate,
                        };

                        let mut already_enveloped = false;
                        engine.render(
                            &engine_params,
                            &mut self.buffer_out,
                            &mut self.buffer_aux,
                            &mut already_enveloped,
                        );
                    }
                }
            }

            message_handlers!(impl $struct_name {});
        }
    };

    // WITH LIFETIME variant
    (
        @with_lifetime
        name: $module_name:literal,
        doc: $module_doc:literal,
        struct_name: $struct_name:ident,
        engine_type: $engine_type:ident,
        engine_path: $engine_path:path,
        constructor: { $constructor:expr },
        output_range: ($out_min:expr, $out_max:expr),
        output_doc: $output_doc:literal,
        aux_range: ($aux_min:expr, $aux_max:expr),
        aux_doc: $aux_doc:literal,
        params: {
            freq: $freq_doc:literal,
            timbre: $timbre_doc:literal,
            morph: $morph_doc:literal,
            harmonics: $harmonics_doc:literal,
            sync: $sync_doc:literal $(,)?
        } $(,)?
    ) => {
        paste::paste! {
            use napi::Result;
            use schemars::JsonSchema;
            use serde::Deserialize;

            use crate::{
                dsp::utils::voct_to_midi,
                types::{Clickless, Signal},
            };
            use mi_plaits_dsp::engine::{Engine, EngineParameters, TriggerState};
            use $engine_path;

            const BLOCK_SIZE: usize = 1;

            #[derive(Deserialize, Default, JsonSchema, Connect, ChannelCount)]
            #[serde(default)]
            struct [<$struct_name Params>] {
                #[doc = $freq_doc]
                freq: Signal,
                #[doc = $timbre_doc]
                timbre: Signal,
                #[doc = $morph_doc]
                morph: Signal,
                #[doc = $harmonics_doc]
                harmonics: Signal,
                #[doc = $sync_doc]
                sync: Signal,
            }

            #[derive(Outputs, JsonSchema)]
            struct [<$struct_name Outputs>] {
                #[output("output", $output_doc, range = ($out_min, $out_max))]
                sample: f32,
                #[output("aux", $aux_doc, range = ($aux_min, $aux_max))]
                aux: f32,
            }

            #[doc = $module_doc]
            #[derive(Default, Module)]
            #[module($module_name, $module_doc)]
            #[args(freq)]
            pub struct $struct_name<'a> {
                outputs: [<$struct_name Outputs>],
                engine: Option<$engine_type<'a>>,
                buffer_out: Vec<f32>,
                buffer_aux: Vec<f32>,
                buffer_pos: usize,
                last_sync: f32,
                sample_rate: f32,
                freq: Clickless,
                timbre: Clickless,
                morph: Clickless,
                harmonics: Clickless,
                params: [<$struct_name Params>],
            }

            impl<'a> $struct_name<'a> {
                fn update(&mut self, sample_rate: f32) {
                    if self.engine.is_none() || (self.sample_rate - sample_rate).abs() > 0.1 {
                        let mut engine = $constructor;
                        engine.init(sample_rate);
                        self.engine = Some(engine);
                        self.sample_rate = sample_rate;
                        self.buffer_out = vec![0.0; BLOCK_SIZE];
                        self.buffer_aux = vec![0.0; BLOCK_SIZE];
                        self.buffer_pos = BLOCK_SIZE;
                    }

                    if self.buffer_pos >= BLOCK_SIZE {
                        self.render_block(sample_rate);
                        self.buffer_pos = 0;
                    }

                    self.outputs.sample = self.buffer_out[self.buffer_pos];
                    self.outputs.aux = self.buffer_aux[self.buffer_pos];

                    self.buffer_pos += 1;
                }

                fn render_block(&mut self, sample_rate: f32) {
                    if let Some(ref mut engine) = self.engine {
                        self.freq
                            .update(self.params.freq.get_value_or(4.0).clamp(-10.0, 10.0));
                        self.timbre
                            .update(self.params.timbre.get_value_or(2.5).clamp(0.0, 5.0));
                        self.morph
                            .update(self.params.morph.get_value_or(2.5).clamp(0.0, 5.0));
                        self.harmonics
                            .update(self.params.harmonics.get_value_or(2.5).clamp(0.0, 5.0));

                        let midi_note = voct_to_midi(*self.freq);

                        let timbre_norm = (*self.timbre) / 5.0;
                        let morph_norm = (*self.morph) / 5.0;
                        let harmonics_norm = (*self.harmonics) / 5.0;

                        let trigger_state = if self.params.sync == Signal::Disconnected {
                            TriggerState::Unpatched
                        } else {
                            let sync_val = self.params.sync.get_value_or(0.0);
                            if sync_val > 0.0 && self.last_sync <= 0.0 {
                                self.last_sync = sync_val;
                                TriggerState::RisingEdge
                            } else if sync_val > 0.0 {
                                TriggerState::High
                            } else {
                                self.last_sync = sync_val;
                                TriggerState::Low
                            }
                        };

                        let engine_params = EngineParameters {
                            trigger: trigger_state,
                            note: midi_note,
                            timbre: timbre_norm,
                            morph: morph_norm,
                            harmonics: harmonics_norm,
                            accent: 1.0,
                            a0_normalized: 55.0 / sample_rate,
                        };

                        let mut already_enveloped = false;
                        engine.render(
                            &engine_params,
                            &mut self.buffer_out,
                            &mut self.buffer_aux,
                            &mut already_enveloped,
                        );
                    }
                }
            }

            message_handlers!(impl $struct_name {});
        }
    };
}

pub use mi_engine_module;
pub use mi_engine_module_impl;
