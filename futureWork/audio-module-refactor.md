Plan: VCV Rack Audio Architecture Understanding
VCV Rack decouples the engine sample rate from audio device rates using a master module pattern and per-module sample rate conversion. One Audio module "drives" the engine while others buffer and resample independently.

Key Architecture Points
Master Module Drives the Engine

Only one Audio module is the "master" at a time (Audio.cpp:62-63)
The master calls Engine::stepBlock() in its processBuffer() callback to advance the DSP graph
If no master exists, a fallback thread runs the engine at ~60Hz
Sample Rate Conversion Per-Port

Each AudioPort has two Speex resamplers: inputSrc (engine→device) and outputSrc (device→engine)
Device audio is converted to/from the engine sample rate regardless of device's native rate
Quality setting of 6 balances latency vs. quality
Ring Buffer Strategy

Two 32768-frame DoubleRingBuffers per port: engineInputBuffer and engineOutputBuffer
Buffers absorb timing differences between device callbacks and engine processing
Overflow protection clears buffers when they exceed ~2x expected frames to cap latency
Multiple Devices Coexist

Secondary (non-master) Audio modules do not call stepBlock()—they just buffer samples
Each module independently resamples between its device rate and the shared engine rate
Multiple ports can subscribe to the same Device for shared channel access
Further Considerations
How does "Auto" sample rate work? The master module calls setSuggestedSampleRate(deviceSampleRate) when engine is set to auto—so engine adopts the master's device rate.
Latency tradeoffs? The ring buffer size (32768) and quality=6 resampler are tunable; lower quality = less latency but more aliasing.
Would you like a deeper dive into any specific subsystem (e.g., the Speex resampler DSP, the fallback thread, or how modules claim master status)?

Plan: Rearchitect Audio to Support Multi-Device with Per-Module SRC
Migrate from a single global audio stream to VCV Rack-style architecture where Audio modules in the DSL own device streams and perform per-module sample rate conversion to a fixed engine rate.

Steps
Add sample rate conversion crate — integrate rubato in Cargo.toml for high-quality async resampling between device and engine rates.

Create AudioDevice DSP module — new file in dsp implementing a Sampleable that owns cpal streams, ring buffers, and SRC instances (similar to VCV's AudioPort template).

Refactor Synthesizer to delegate streams — modify audio.rs to remove single-stream ownership; instead, the engine processes at a fixed rate and AudioDevice modules spawn/manage their own cpal streams.

Add master module coordination — introduce EngineContext with master-device election logic; master's processBuffer callback drives Patch::process_frame(), non-masters just buffer samples.

Expose DSL factories — add audioIn(deviceId, channels) and audioOut(deviceId, channels) in factories.ts and corresponding Rust constructors so users can instantiate multiple audio I/O modules in patches.

Wire IPC for per-module device config — extend ipcTypes.ts with channels to enumerate devices and pass device IDs into patch module params.

Further Considerations
Engine timing source? VCV Rack uses the master audio callback to drive stepBlock(). Do you want the master device callback to drive engine ticks, or keep a separate high-priority thread with ring-buffer handoff?

Latency vs. quality tradeoff? Rubato supports different resampler types (SincFixedIn, SincFixedOut, FastFFT). SincFixedIn is lowest latency; want to make this configurable per-module?

Backward compatibility? Current patches assume implicit audio routing via out(). Should we auto-insert a default audioOut() when none is present, or require explicit audio modules going forward?
