# Glow Effect Design

**Date:** 2026-03-05

## Overview

Add a subtle ambient glow to the Monaco editor text and oscilloscope canvases, with a momentary burst when the audio thread confirms a patch has been applied.

## Requirements

- Constant baseline glow on editor text and oscilloscope waveforms.
- Glow intensity increases as a smooth in-and-out pulse when the audio thread confirms a patch.
- Burst fires on audio thread confirmation (`lastAppliedUpdateId` advance), not on button press.
- Configurable: enable/disable, intensity (0–100%), burst duration (200–2000ms).

## Config Schema

New `GlowConfig` interface added to `AppConfig` in `src/shared/ipcTypes.ts`:

```ts
export interface GlowConfig {
    enabled?: boolean; // default: true
    intensity?: number; // 0–1, default: 0.5
    burstDuration?: number; // milliseconds, default: 800
}
```

Added to `AppConfig` as `glow?: GlowConfig`. Matching Zod schema added in `main.ts`. `ensureConfigExists` writes `enabled: true` as the only default; intensity and duration fall back to constants at runtime.

## Architecture

### Approach: CSS custom property + RAF (Option A)

A single `--glow-intensity` CSS variable on `:root` is written every animation frame from the existing RAF loop. Monaco text shadow and canvas `shadowBlur` both read the same computed value. No React re-renders during animation.

## Components

### `src/renderer/app/glowAnimation.ts` (new file)

Encapsulates all glow animation state and logic:

```ts
export interface GlowAnimator {
    notifyPatchQueued: (updateId: number) => void;
    tick: (
        now: number,
        lastAppliedUpdateId: number,
        config: GlowConfig,
    ) => number; // returns intensity 0–1
}

export function createGlowAnimator(): GlowAnimator;
```

Internally owns:

- `pendingGlowUpdateId` — the `updateId` from the most recent successful `executeDSL` result.
- `burstStartTime` — `performance.now()` timestamp when the burst was triggered, or `null`.

Per-frame intensity formula:

- `baselineIntensity = config.intensity * 0.25`
- `burstIntensity = config.intensity`
- `t = clamp((now - burstStartTime) / config.burstDuration, 0, 1)`
- `intensity = baselineIntensity + (burstIntensity - baselineIntensity) * sin(t * π)`

When glow is disabled or intensity is 0, returns 0 immediately.

### `src/renderer/App.tsx` (modified)

- Constructs `useRef(createGlowAnimator())`.
- Calls `animator.notifyPatchQueued(updateId)` after every successful `executeDSL` result.
- RAF loop calls `animator.tick(now, transport.lastAppliedUpdateId, glowConfig)`, writes the returned value to `document.documentElement.style.setProperty('--glow-intensity', intensity.toFixed(4))`, and passes `intensity` to `drawOscilloscope`.
- Reads `glowConfig` from React state (populated from `electronAPI.config.read()` and `onChange`).

### `src/renderer/App.css` (modified)

```css
.monaco-editor .view-lines .view-line {
    text-shadow: 0 0 calc(var(--glow-intensity, 0) * 8px) var(--accent-primary);
}
```

Max blur radius of 8px at full intensity.

### `src/renderer/app/oscilloscope.ts` (modified)

`drawOscilloscope` gets an optional `glowIntensity` parameter. Applied only to the waveform stroke:

```ts
ctx.shadowBlur = glowIntensity * 12; // 12px max
ctx.shadowColor = accentPrimaryColor;
// ... stroke path ...
ctx.shadowBlur = 0; // reset — does not affect labels or reference lines
```

### `src/renderer/components/EditorSettingsTab.tsx` (modified)

Three controls added under a "Glow" heading at the bottom of the Editor tab:

| Control        | Type                      | Range                |
| -------------- | ------------------------- | -------------------- |
| Enable Glow    | `<input type="checkbox">` | —                    |
| Intensity      | `<input type="range">`    | 0–100 (stored 0–1)   |
| Burst Duration | `<input type="range">`    | 200–2000ms, step 100 |

Intensity and Burst Duration are disabled when glow is disabled. All controls auto-save immediately via `electronAPI.config.write` on change, matching existing Editor tab behavior. The tab already has `overflow-y: auto` in `.settings-body`, so no layout changes are needed.

### `src/shared/ipcTypes.ts` (modified)

Add `GlowConfig` interface and `glow?: GlowConfig` field to `AppConfig`.

### `src/main/main.ts` (modified)

Add `GlowConfigSchema` Zod schema. Add `glow` key to `AppConfigSchema`. Update `ensureConfigExists` to write `glow: { enabled: true }` as the default.

## Data Flow

```
executeDSL succeeds
  → animator.notifyPatchQueued(updateId)

RAF frame
  → getTransportState() → transport.lastAppliedUpdateId
  → animator.tick(now, lastAppliedUpdateId, glowConfig) → intensity
  → document.documentElement.style.setProperty('--glow-intensity', intensity)
  → drawOscilloscope(..., { glowIntensity: intensity })
      → ctx.shadowBlur = intensity * 12

CSS (computed each frame via var())
  → .view-line { text-shadow: 0 0 calc(var(--glow-intensity) * 8px) var(--accent-primary) }
```

## Testing

- Unit test `createGlowAnimator`: verify baseline intensity when no burst is active, burst curve shape at t=0/0.5/1.0, early re-trigger resets the burst, disabled config returns 0.
- Manual visual check: baseline glow visible on editor text and oscilloscope; burst fires on audio thread confirm (not on keypress); settings controls update glow in real time.
