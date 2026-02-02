//! Macro for generating Mutable Instruments engine wrapper modules.
//!
//! All MI engine modules share nearly identical structure. This macro generates
//! the params struct, outputs struct, main module struct, and implementation
//! from a concise declaration.
//!
//! These modules are **polyphonic**: they support up to 16 voices, with each
//! voice having its own engine instance, render buffers, and trigger state.
//! The voice count is determined by the maximum channel count of the inputs.

/// Generates a complete polyphonic MI engine wrapper module.
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
///     output_doc: "main output description",
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
        output_doc: $output_doc:literal,
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
            output_doc: $output_doc,
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
        output_doc: $output_doc:literal,
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
            output_doc: $output_doc,
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
        output_doc: $output_doc:literal,
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
            output_doc: $output_doc,
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
        output_doc: $output_doc:literal,
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
            output_doc: $output_doc,
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
        output_doc: $output_doc:literal,
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
            output_doc: $output_doc,
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
        output_doc: $output_doc:literal,
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
            output_doc: $output_doc,
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

/// Internal implementation macro - generates the actual polyphonic code
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
        output_doc: $output_doc:literal,
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
            
            use schemars::JsonSchema;
            use serde::Deserialize;

            use $crate::{
                dsp::utils::voct_to_midi,
                poly::{PORT_MAX_CHANNELS, PolyOutput, PolySignal},
                types::Clickless,
            };
            use mi_plaits_dsp::engine::{Engine, EngineParameters, TriggerState};
            use $engine_path;

            const BLOCK_SIZE: usize = 1;

            #[derive(Deserialize, Default, JsonSchema, Connect, ChannelCount)]
            #[serde(default)]
            struct [<$struct_name Params>] {
                #[doc = $freq_doc]
                freq: PolySignal,
                #[doc = $timbre_doc]
                timbre: PolySignal,
                #[doc = $morph_doc]
                morph: PolySignal,
                #[doc = $harmonics_doc]
                harmonics: PolySignal,
                #[doc = $sync_doc]
                sync: PolySignal,
            }

            #[derive(Outputs, JsonSchema)]
            struct [<$struct_name Outputs>] {
                #[output("main", $output_doc, default)]
                sample: PolyOutput,
                #[output("aux", $aux_doc)]
                aux: PolyOutput,
            }

            /// Per-channel state for a single voice
            struct [<$struct_name ChannelState>] {
                engine: $engine_type,
                buffer_out: [f32; BLOCK_SIZE],
                buffer_aux: [f32; BLOCK_SIZE],
                last_sync: f32,
                freq: Clickless,
                timbre: Clickless,
                morph: Clickless,
                harmonics: Clickless,
            }

            impl Default for [<$struct_name ChannelState>] {
                fn default() -> Self {
                    Self {
                        engine: $constructor,
                        buffer_out: [0.0; BLOCK_SIZE],
                        buffer_aux: [0.0; BLOCK_SIZE],
                        last_sync: 0.0,
                        freq: Clickless::default(),
                        timbre: Clickless::default(),
                        morph: Clickless::default(),
                        harmonics: Clickless::default(),
                    }
                }
            }

            #[doc = $module_doc]
            #[derive(Module)]
            #[module($module_name, $module_doc)]
            #[has_init]
            #[args(freq)]
            pub struct $struct_name {
                outputs: [<$struct_name Outputs>],
                channels: [[<$struct_name ChannelState>]; PORT_MAX_CHANNELS],
                buffer_pos: usize,
                sample_rate: f32,
                params: [<$struct_name Params>],
            }

            impl Default for $struct_name {
                fn default() -> Self {
                    Self {
                        outputs: [<$struct_name Outputs>]::default(),
                        channels: std::array::from_fn(|_| [<$struct_name ChannelState>]::default()),
                        buffer_pos: BLOCK_SIZE, // Start exhausted to trigger initial render
                        sample_rate: 0.0,
                        params: [<$struct_name Params>]::default(),
                    }
                }
            }

            impl $struct_name {
                fn init(&mut self, sample_rate: f32) {
                    for state in self.channels.iter_mut() {
                        state.engine.init(sample_rate);
                    }
                    self.sample_rate = sample_rate;
                    self.buffer_pos = BLOCK_SIZE; // Force re-render
                }

                fn update(&mut self, sample_rate: f32) {
                    let num_channels = self.channel_count().max(1);

                    // Render all active voices when buffer is exhausted
                    if self.buffer_pos >= BLOCK_SIZE {
                        self.render_block(sample_rate, num_channels);
                        self.buffer_pos = 0;
                    }

                    // Copy current samples to outputs
                    let mut output = PolyOutput::default();
                    let mut aux_output = PolyOutput::default();
                    output.set_channels(num_channels);
                    aux_output.set_channels(num_channels);

                    for ch in 0..num_channels {
                        let state = &self.channels[ch];
                        output.set(ch, state.buffer_out[self.buffer_pos] * 5.0);
                        aux_output.set(ch, state.buffer_aux[self.buffer_pos] * 5.0);
                    }

                    self.outputs.sample = output;
                    self.outputs.aux = aux_output;

                    self.buffer_pos += 1;
                }

                fn render_block(&mut self, sample_rate: f32, num_channels: usize) {
                    for ch in 0..num_channels {
                        let state = &mut self.channels[ch];

                        // Get per-voice parameters with cycling
                        state.freq.update(self.params.freq.get_value_or(ch, 4.0));
                        state.timbre.update(self.params.timbre.get_value_or(ch, 2.5).clamp(0.0, 5.0));
                        state.morph.update(self.params.morph.get_value_or(ch, 2.5).clamp(0.0, 5.0));
                        state.harmonics.update(self.params.harmonics.get_value_or(ch, 2.5).clamp(0.0, 5.0));

                        let midi_note = voct_to_midi(*state.freq);

                        let timbre_norm = (*state.timbre) / 5.0;
                        let morph_norm = (*state.morph) / 5.0;
                        let harmonics_norm = (*state.harmonics) / 5.0;

                        // Per-voice trigger detection
                        let trigger_state = if self.params.sync.is_disconnected() {
                            TriggerState::Unpatched
                        } else {
                            let sync_val = self.params.sync.get_value_or(ch, 0.0);
                            if sync_val > 0.0 && state.last_sync <= 0.0 {
                                state.last_sync = sync_val;
                                TriggerState::RisingEdge
                            } else if sync_val > 0.0 {
                                state.last_sync = sync_val;
                                TriggerState::High
                            } else {
                                state.last_sync = sync_val;
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
                        state.engine.render(
                            &engine_params,
                            &mut state.buffer_out,
                            &mut state.buffer_aux,
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
        output_doc: $output_doc:literal,
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
            
            use schemars::JsonSchema;
            use serde::Deserialize;

            use $crate::{
                dsp::utils::voct_to_midi,
                poly::{PORT_MAX_CHANNELS, PolyOutput, PolySignal},
                types::Clickless,
            };
            use mi_plaits_dsp::engine::{Engine, EngineParameters, TriggerState};
            use $engine_path;

            const BLOCK_SIZE: usize = 1;

            #[derive(Deserialize, Default, JsonSchema, Connect, ChannelCount)]
            #[serde(default)]
            struct [<$struct_name Params>] {
                #[doc = $freq_doc]
                freq: PolySignal,
                #[doc = $timbre_doc]
                timbre: PolySignal,
                #[doc = $morph_doc]
                morph: PolySignal,
                #[doc = $harmonics_doc]
                harmonics: PolySignal,
                #[doc = $sync_doc]
                sync: PolySignal,
            }

            #[derive(Outputs, JsonSchema)]
            struct [<$struct_name Outputs>] {
                #[output("output", $output_doc, default)]
                sample: PolyOutput,
                #[output("aux", $aux_doc)]
                aux: PolyOutput,
            }

            /// Per-channel state for a single voice
            struct [<$struct_name ChannelState>] {
                engine: $engine_type<'static>,
                buffer_out: [f32; BLOCK_SIZE],
                buffer_aux: [f32; BLOCK_SIZE],
                last_sync: f32,
                freq: Clickless,
                timbre: Clickless,
                morph: Clickless,
                harmonics: Clickless,
            }

            impl Default for [<$struct_name ChannelState>] {
                fn default() -> Self {
                    Self {
                        engine: $constructor,
                        buffer_out: [0.0; BLOCK_SIZE],
                        buffer_aux: [0.0; BLOCK_SIZE],
                        last_sync: 0.0,
                        freq: Clickless::default(),
                        timbre: Clickless::default(),
                        morph: Clickless::default(),
                        harmonics: Clickless::default(),
                    }
                }
            }

            #[doc = $module_doc]
            #[derive(Module)]
            #[module($module_name, $module_doc)]
            #[args(freq)]
            pub struct $struct_name {
                outputs: [<$struct_name Outputs>],
                channels: [[<$struct_name ChannelState>]; PORT_MAX_CHANNELS],
                buffer_pos: usize,
                sample_rate: f32,
                params: [<$struct_name Params>],
            }

            impl Default for $struct_name {
                fn default() -> Self {
                    Self {
                        outputs: [<$struct_name Outputs>]::default(),
                        channels: std::array::from_fn(|_| [<$struct_name ChannelState>]::default()),
                        buffer_pos: BLOCK_SIZE, // Start exhausted to trigger initial render
                        sample_rate: 0.0,
                        params: [<$struct_name Params>]::default(),
                    }
                }
            }

            impl $struct_name {
                fn update(&mut self, sample_rate: f32) {
                    let num_channels = self.channel_count().max(1);

                    // Initialize engines if sample rate changed
                    if (self.sample_rate - sample_rate).abs() > 0.1 {
                        for state in self.channels.iter_mut() {
                            state.engine.init(sample_rate);
                        }
                        self.sample_rate = sample_rate;
                        self.buffer_pos = BLOCK_SIZE; // Force re-render
                    }

                    // Render all active voices when buffer is exhausted
                    if self.buffer_pos >= BLOCK_SIZE {
                        self.render_block(sample_rate, num_channels);
                        self.buffer_pos = 0;
                    }

                    // Copy current samples to outputs
                    let mut output = PolyOutput::default();
                    let mut aux_output = PolyOutput::default();
                    output.set_channels(num_channels);
                    aux_output.set_channels(num_channels);

                    for ch in 0..num_channels {
                        let state = &self.channels[ch];
                        output.set(ch, state.buffer_out[self.buffer_pos]);
                        aux_output.set(ch, state.buffer_aux[self.buffer_pos]);
                    }

                    self.outputs.sample = output;
                    self.outputs.aux = aux_output;

                    self.buffer_pos += 1;
                }

                fn render_block(&mut self, sample_rate: f32, num_channels: usize) {
                    for ch in 0..num_channels {
                        let state = &mut self.channels[ch];

                        // Get per-voice parameters with cycling
                        state.freq.update(self.params.freq.get_value_or(ch, 4.0));
                        state.timbre.update(self.params.timbre.get_value_or(ch, 2.5).clamp(0.0, 5.0));
                        state.morph.update(self.params.morph.get_value_or(ch, 2.5).clamp(0.0, 5.0));
                        state.harmonics.update(self.params.harmonics.get_value_or(ch, 2.5).clamp(0.0, 5.0));

                        let midi_note = voct_to_midi(*state.freq);

                        let timbre_norm = (*state.timbre) / 5.0;
                        let morph_norm = (*state.morph) / 5.0;
                        let harmonics_norm = (*state.harmonics) / 5.0;

                        // Per-voice trigger detection
                        let trigger_state = if self.params.sync.is_disconnected() {
                            TriggerState::Unpatched
                        } else {
                            let sync_val = self.params.sync.get_value_or(ch, 0.0);
                            if sync_val > 0.0 && state.last_sync <= 0.0 {
                                state.last_sync = sync_val;
                                TriggerState::RisingEdge
                            } else if sync_val > 0.0 {
                                state.last_sync = sync_val;
                                TriggerState::High
                            } else {
                                state.last_sync = sync_val;
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
                        state.engine.render(
                            &engine_params,
                            &mut state.buffer_out,
                            &mut state.buffer_aux,
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
        output_doc: $output_doc:literal,
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
            
            use schemars::JsonSchema;
            use serde::Deserialize;

            use crate::{
                dsp::utils::voct_to_midi,
                poly::{PORT_MAX_CHANNELS, PolyOutput, PolySignal},
                types::Clickless,
            };
            use mi_plaits_dsp::engine::{Engine, EngineParameters, TriggerState};
            use $engine_path;

            const BLOCK_SIZE: usize = 1;

            #[derive(Deserialize, Default, JsonSchema, Connect, ChannelCount)]
            #[serde(default)]
            struct [<$struct_name Params>] {
                #[doc = $freq_doc]
                freq: PolySignal,
                #[doc = $timbre_doc]
                timbre: PolySignal,
                #[doc = $morph_doc]
                morph: PolySignal,
                #[doc = $harmonics_doc]
                harmonics: PolySignal,
                #[doc = $sync_doc]
                sync: PolySignal,
            }

            #[derive(Outputs, JsonSchema)]
            struct [<$struct_name Outputs>] {
                #[output("output", $output_doc, default)]
                sample: PolyOutput,
                #[output("aux", $aux_doc)]
                aux: PolyOutput,
            }

            /// Per-channel state for a single voice
            struct [<$struct_name ChannelState>]<'a> {
                engine: $engine_type<'a>,
                buffer_out: [f32; BLOCK_SIZE],
                buffer_aux: [f32; BLOCK_SIZE],
                last_sync: f32,
                freq: Clickless,
                timbre: Clickless,
                morph: Clickless,
                harmonics: Clickless,
            }

            impl<'a> Default for [<$struct_name ChannelState>]<'a> {
                fn default() -> Self {
                    Self {
                        engine: $constructor,
                        buffer_out: [0.0; BLOCK_SIZE],
                        buffer_aux: [0.0; BLOCK_SIZE],
                        last_sync: 0.0,
                        freq: Clickless::default(),
                        timbre: Clickless::default(),
                        morph: Clickless::default(),
                        harmonics: Clickless::default(),
                    }
                }
            }

            #[doc = $module_doc]
            #[derive(Module)]
            #[module($module_name, $module_doc)]
            #[args(freq)]
            pub struct $struct_name<'a> {
                outputs: [<$struct_name Outputs>],
                channels: [[<$struct_name ChannelState>]<'a>; PORT_MAX_CHANNELS],
                buffer_pos: usize,
                sample_rate: f32,
                params: [<$struct_name Params>],
            }

            impl<'a> Default for $struct_name<'a> {
                fn default() -> Self {
                    Self {
                        outputs: [<$struct_name Outputs>]::default(),
                        channels: std::array::from_fn(|_| [<$struct_name ChannelState>]::default()),
                        buffer_pos: BLOCK_SIZE, // Start exhausted to trigger initial render
                        sample_rate: 0.0,
                        params: [<$struct_name Params>]::default(),
                    }
                }
            }

            impl<'a> $struct_name<'a> {
                fn update(&mut self, sample_rate: f32) {
                    let num_channels = self.channel_count().max(1);

                    // Initialize engines if sample rate changed
                    if (self.sample_rate - sample_rate).abs() > 0.1 {
                        for state in self.channels.iter_mut() {
                            state.engine.init(sample_rate);
                        }
                        self.sample_rate = sample_rate;
                        self.buffer_pos = BLOCK_SIZE; // Force re-render
                    }

                    // Render all active voices when buffer is exhausted
                    if self.buffer_pos >= BLOCK_SIZE {
                        self.render_block(sample_rate, num_channels);
                        self.buffer_pos = 0;
                    }

                    // Copy current samples to outputs
                    let mut output = PolyOutput::default();
                    let mut aux_output = PolyOutput::default();
                    output.set_channels(num_channels);
                    aux_output.set_channels(num_channels);

                    for ch in 0..num_channels {
                        let state = &self.channels[ch];
                        output.set(ch, state.buffer_out[self.buffer_pos]);
                        aux_output.set(ch, state.buffer_aux[self.buffer_pos]);
                    }

                    self.outputs.sample = output;
                    self.outputs.aux = aux_output;

                    self.buffer_pos += 1;
                }

                fn render_block(&mut self, sample_rate: f32, num_channels: usize) {
                    for ch in 0..num_channels {
                        let state = &mut self.channels[ch];

                        // Get per-voice parameters with cycling
                        state.freq.update(self.params.freq.get_value_or(ch, 4.0));
                        state.timbre.update(self.params.timbre.get_value_or(ch, 2.5).clamp(0.0, 5.0));
                        state.morph.update(self.params.morph.get_value_or(ch, 2.5).clamp(0.0, 5.0));
                        state.harmonics.update(self.params.harmonics.get_value_or(ch, 2.5).clamp(0.0, 5.0));

                        let midi_note = voct_to_midi(*state.freq);

                        let timbre_norm = (*state.timbre) / 5.0;
                        let morph_norm = (*state.morph) / 5.0;
                        let harmonics_norm = (*state.harmonics) / 5.0;

                        // Per-voice trigger detection
                        let trigger_state = if self.params.sync.is_disconnected() {
                            TriggerState::Unpatched
                        } else {
                            let sync_val = self.params.sync.get_value_or(ch, 0.0);
                            if sync_val > 0.0 && state.last_sync <= 0.0 {
                                state.last_sync = sync_val;
                                TriggerState::RisingEdge
                            } else if sync_val > 0.0 {
                                state.last_sync = sync_val;
                                TriggerState::High
                            } else {
                                state.last_sync = sync_val;
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
                        state.engine.render(
                            &engine_params,
                            &mut state.buffer_out,
                            &mut state.buffer_aux,
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
