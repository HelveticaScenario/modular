# Spectral Warp for `$wavetable`

**Date:** 2026-04-20
**Status:** Proposed

## Overview

Add a `SpectralTable` param type and a `spectral` parameter to `$wavetable`. Spectral warp
manipulates the frequency-domain representation of a wavetable frame before playback, enabling
timbral effects that are impossible with phase warp alone (harmonic stretch, smear, low/high
pass, phase dispersion, Shepard tone). The API mirrors the existing `Table` / `phase` design
exactly. When `spectral` is absent the oscillator's existing pre-baked mipmap path is
completely unchanged.

---

## DSL API

```js
// Static spectral warp
$wavetable(wav, pitch, pos, { spectral: $spectral.smear(0.6) }).out()

// Signal-driven amount — modulatable per voice
$wavetable(wav, pitch, pos, { spectral: $spectral.harmonicScale(lfo.out) }).out()

// Chained warps via pipe (applied left to right)
$wavetable(wav, pitch, pos, {
  spectral: $spectral.pipe($spectral.smear(0.3), $spectral.lowPass(env.out))
}).out()

// Combined with phase warp
$wavetable(wav, pitch, pos, {
  phase:    $table.bend(0.5),
  spectral: $spectral.smear(lfo.out),
}).out()
```

`$spectral` variants:

| Factory                     | Effect                                                          |
| --------------------------- | --------------------------------------------------------------- |
| `$spectral.smear(amount)`   | Running-average amplitude blur across harmonics                 |
| `$spectral.lowPass(cutoff)` | Zero bins above exponentially-scaled cutoff                     |
| `$spectral.highPass(cutoff)`| Zero bins below exponentially-scaled cutoff                     |
| `$spectral.harmonicScale(shift)` | Stretch harmonics apart linearly (inharmonic at high values) |
| `$spectral.inharmonicScale(mult)` | Log-scale harmonic stretch (bell/gong timbres)          |
| `$spectral.phaseDisperse(amount)` | Rotate phases by quadratic function of harmonic index   |
| `$spectral.shepard(shift)`  | Blend harmonic i with harmonic 2i for Shepard tone effect       |
| `$spectral.pipe(a, b)`      | Apply `a` then `b` to the same spectrum                         |

All `amount`/`cutoff`/`shift`/`mult` fields accept either a literal float or a `PolySignal`,
identical to `Table` variants.

---

## Architecture

### Why not pre-compute

Pre-computing mip pyramids for every `(frame, warp_amount)` pair is impractical:
`256 frames × 256 warp steps × 4MB per wavetable = 256GB`. Instead, spectral warp runs the
mip pyramid IFFT at block rate (every 32 samples), once per unique
`(position_channel, warp_channel)` pair. The pre-baked pyramid in `PreparedWavetable` is used
unchanged when `spectral` is absent.

### FM anti-aliasing is preserved

The block-rate IFFT generates a full mip pyramid (all levels) for the current warped spectrum.
Per-sample, `mipmap_level_for_freq(fm_modulated_freq)` selects the correct level exactly as
today. FM pitch changes still drive per-sample mip selection; only the *content* of those mip
levels is stale by up to 32 samples, which is inaudible for spectral warp parameters.

### Deduplication

The number of IFFT runs per block equals `max(position.channels(), spectral.channels())`.
Voice `ch` reads from slot `min(ch, num_slots - 1)`. In the common case (mono position,
mono warp amount) this is **one IFFT run regardless of voice count**. Per-voice warp
modulation (polyphonic spread) costs one run per voice.

---

## New Types

### `SpectralTable` (in `types.rs`, alongside `Table`)

```rust
pub enum SpectralTable {
    Smear           { amount: PolySignal },
    LowPass         { cutoff: PolySignal },
    HighPass        { cutoff: PolySignal },
    HarmonicScale   { shift:  PolySignal },
    InharmonicScale { mult:   PolySignal },
    PhaseDisperse   { amount: PolySignal },
    Shepard         { shift:  PolySignal },
    Pipe { first: Box<SpectralTable>, second: Box<SpectralTable> },
}

impl SpectralTable {
    /// Apply this warp to a full-bandwidth complex spectrum in-place.
    /// `spectrum` has length `num_harmonics = frame_size / 2 + 1`.
    /// Pure arithmetic, allocation-free, audio-thread safe.
    pub fn apply(&self, spectrum: &mut [Complex<f32>], channel: usize);

    pub fn channels(&self) -> usize;
    pub fn connect(&mut self, patch: &Patch);
}
```

`Pipe::apply` calls `first.apply(spectrum, channel)` then `second.apply(spectrum, channel)` —
a single spectrum pass, one IFFT afterward.

### Spectral morph functions (`dsp/oscillators/spectral_morph.rs`)

Pure functions, signature `fn name(spectrum: &mut [Complex<f32>], amount: f32)`. Ported
directly from Vital's `spectral_morph.h`. Each modifies the spectrum in place — no
allocation, no IFFT.

---

## Changes to `PreparedWavetable`

Add per-frame frequency-domain storage used by the block-rate IFFT path:

```rust
pub struct PreparedWavetable {
    // --- existing fields unchanged ---
    pub levels: Vec<Vec<f32>>,
    pub frame_size: usize,
    pub table_size: usize,   // frame_size * OVERSAMPLE
    pub frame_count: usize,
    pub mipmap_count: usize,
    pub base_frequency: f32,

    // --- new: frequency-domain data for spectral warp ---
    /// Complex spectrum per frame, positive-frequency bins only.
    /// Layout: `freq_frames[frame_idx]` has length `frame_size / 2 + 1`.
    /// Populated during `from_wav_data`, kept alive for block-rate IFFT.
    pub freq_frames: Vec<Vec<Complex<f32>>>,
}
```

`freq_frames` is populated from the same forward FFT already computed in `from_wav_data` —
no extra FFT passes. Memory cost: `frame_count × (frame_size/2 + 1) × 8 bytes`. For 256
frames at 2048-sample frame size: `256 × 1025 × 8 ≈ 2.1 MB` per loaded wavetable.

---

## Changes to `WavetableOscParams`

```rust
pub(crate) struct WavetableOscParams {
    // --- existing fields unchanged ---
    pub(crate) wav:      Wav,
    pub(crate) pitch:    PolySignal,
    pub(crate) position: Option<PolySignal>,
    pub(crate) fm:       Option<PolySignal>,
    pub(crate) fm_mode:  FmMode,
    pub(crate) phase:    Option<Table>,
    pub(crate) prepared: Option<PreparedWavetable>,

    // --- new ---
    /// Spectral warp applied before the block-rate IFFT.
    #[deserr(default)]
    pub(crate) spectral: Option<SpectralTable>,
}
```

Channel count derivation gains `spectral.channels()` in the max.

---

## Changes to `WavetableOscState`

```rust
struct WavetableOscState {
    // --- existing ---
    channels: [ChannelState; PORT_MAX_CHANNELS],

    // --- new: block-rate IFFT ---

    /// Counts samples since last IFFT. Reset to 0 when it hits SPECTRAL_BLOCK_SIZE.
    block_counter: usize,

    /// One slot per unique (position_channel, warp_channel) pair.
    /// Allocated by `prepare_spectral_state` when `spectral` becomes Some.
    spectral_slots: Vec<SpectralSlot>,
}

const SPECTRAL_BLOCK_SIZE: usize = 32;

struct SpectralSlot {
    /// Interpolated + warped complex spectrum. Length = frame_size / 2 + 1.
    spectrum: Vec<Complex<f32>>,
    /// Per-level complex working buffers for the IFFT. Level k has length
    /// `(table_size >> k)`, halving each level. Pre-allocated; reused each block.
    padded: Vec<Vec<Complex<f32>>>,
    /// Per-level IFFT scratch buffers, sized to match each plan's scratch requirement.
    scratch: Vec<Vec<Complex<f32>>>,
    /// Rendered mip levels. `warped_levels[k]` has length `table_size >> k`.
    /// Each level is half the size of the previous, matching the spectral bandwidth
    /// reduction. Higher pitches need fewer samples to represent one cycle cleanly.
    warped_levels: Vec<Vec<f32>>,
}
```

`SpectralSlot` is allocated on the main thread in `prepare_resources_impl` after
`PreparedWavetable` is built (sizes depend on `frame_size`, `table_size`, `mipmap_count`).
Audio thread never allocates.

IFFT plans (one per mip level, sizes `table_size >> k`) are stored in `WavetableOscState`
as `ifft_plans: Vec<Arc<dyn Fft<f32>>>`, also allocated in `prepare_resources_impl`.

The total samples across all levels is a geometric series:
`table_size + table_size/2 + table_size/4 + ... = 2 × table_size`
regardless of mip count. This bounds both memory and CPU to ~2× the cost of a single
full-size IFFT rather than `mip_count ×`.

---

## Block-Rate IFFT Logic (audio thread)

Called at the top of `update()` when `spectral.is_some()`:

```
if block_counter == 0:
    num_slots = max(position.channels(), spectral.channels())
    for slot_idx in 0..num_slots:
        ch = slot_idx
        pos_v  = position.value_or(ch, 0.0)
        frame_f = pos_v / 5.0 * (frame_count - 1)

        // 1. Frequency-domain frame interpolation
        f0 = frame_f.floor() as usize
        f1 = min(f0 + 1, frame_count - 1)
        t  = frame_f - f0 as f32
        for i in 0..num_harmonics:
            slot.spectrum[i] = lerp(freq_frames[f0][i], freq_frames[f1][i], t)

        // 2. Spectral warp (in-place, no alloc)
        spectral.apply(&mut slot.spectrum, ch)

        // 3. Mip pyramid — one IFFT per level, each half the size of the previous
        for level in 0..mipmap_count:
            level_table_size = table_size >> level
            cutoff = num_harmonics >> level
            // copy spectrum into padded[level], zero above cutoff,
            // zero-pad to level_table_size (oversampling shrinks with level)
            build_padded_spectrum(&slot.spectrum, &mut slot.padded[level], cutoff, level_table_size)
            ifft_plans[level].process_with_scratch(&mut slot.padded[level], &mut slot.scratch[level])
            inv_n_level = 1.0 / (level_table_size / OVERSAMPLE) as f32
            for i in 0..level_table_size:
                slot.warped_levels[level][i] = slot.padded[level][i].re * inv_n_level

block_counter = (block_counter + 1) % SPECTRAL_BLOCK_SIZE
```

### Per-sample read (unchanged interface)

```
slot_idx = min(ch, num_slots - 1)
level    = prepared.mipmap_level_for_freq(freq)   // per-sample, FM-modulated
sample   = read_warped_sample(&spectral_slots[slot_idx], level, warped_phase)
```

`read_warped_sample` reads from `slot.warped_levels[level]` using
`phase * (table_size >> level)` as the index (single interpolated frame, no frame
crossfade needed). Each level has a different table size so the index scaling is
level-dependent.

---

## Fast Path Unchanged

When `params.spectral.is_none()`, `block_counter` and `spectral_slots` are never touched.
`update()` falls through to the existing `prepared.read_sample(level, frame_f, warped_phase)`
call. Zero overhead for non-spectral use.

---

## Memory Summary

| What | Size |
|---|---|
| `freq_frames` added to `PreparedWavetable` | `frame_count × num_harmonics × 8 B` ≈ 2.1 MB for 256 frames |
| Per `SpectralSlot` (spectrum + padded + warped_levels) | `2 × table_size × 4 B` ≈ 131 KB for 2048 frame (geometric series) |
| Max slots = `PORT_MAX_CHANNELS` (16) | 16 × 131 KB ≈ 2.1 MB worst case |
| In practice (mono warp, 1 slot) | 131 KB |

All allocated on the main thread in `prepare_resources_impl`. Audio thread: zero allocation.

---

## CPU Summary (at 44.1 kHz, 2048-sample frames, 11 mip levels, OVERSAMPLE=8)

| Scenario | Cost per 32-sample block |
|---|---|
| No spectral warp | 0 (existing path) |
| 1 slot (mono position + mono warp) | ~2 × IFFT(16384) equivalent ≈ ~10 μs |
| N voices, same warp amount | 1 slot → same ~10 μs |
| N voices, poly warp amount | N × ~10 μs |

Block budget at 44.1 kHz: 32 × 22.7 μs = 726 μs. Single-slot spectral warp costs ~1.4%.

The halved-table-size mip pyramid means the total IFFT work across all 11 levels equals
~2 × IFFT(16384) due to the geometric series
`IFFT(16384) + IFFT(8192) + IFFT(4096) + ... ≈ 2 × IFFT(16384)`.

Note that `PreparedWavetable` still uses same-size tables for all mip levels. If that
pre-bake cost ever matters (it runs once at load time, not at audio rate) it can be
updated to match independently.

---

## New Files

| File | Contents |
|---|---|
| `crates/modular_core/src/dsp/oscillators/spectral_morph.rs` | Pure warp fns on `&mut [Complex<f32>]` |

## Modified Files

| File | Change |
|---|---|
| `crates/modular_core/src/types.rs` | Add `SpectralTable` enum alongside `Table` |
| `crates/modular_core/src/dsp/oscillators/wavetable_prep.rs` | Add `freq_frames` to `PreparedWavetable` |
| `crates/modular_core/src/dsp/oscillators/wavetable.rs` | Add `spectral` param, `SpectralSlot` state, block-rate IFFT logic |
| `crates/modular_core/src/dsp/oscillators/mod.rs` | Declare `spectral_morph` module |
| `crates/modular/index.d.ts` + `schemas.json` | Regenerated via `yarn generate-lib` |
| `src/main/dsl/factories.ts` | Add `$spectral` factory object |
| `generated/dsl.d.ts` | Regenerated |
