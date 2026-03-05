# Glow Effect Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a configurable ambient glow to the Monaco editor text and oscilloscope canvases, with a momentary burst when the audio thread confirms a patch has been applied.

**Architecture:** A CSS custom property `--glow-intensity` (0–1) is written to `:root` every animation frame from the existing RAF loop in `App.tsx`. All visual effects read this single value: Monaco text uses a `text-shadow` CSS rule; the oscilloscope canvas uses `ctx.shadowBlur` inside `drawOscilloscope`. Burst animation logic lives in a standalone `glowAnimation.ts` module that the RAF loop drives via a `tick()` call.

**Tech Stack:** TypeScript, React, plain CSS, Canvas 2D API, Vitest (unit tests), Electron config system.

---

### Task 1: Add `GlowConfig` to the shared config types and Zod schema

**Files:**

- Modify: `src/shared/ipcTypes.ts`
- Modify: `src/main/main.ts`

**Step 1: Add `GlowConfig` interface and field to `AppConfig`**

In `src/shared/ipcTypes.ts`, add the new interface after the existing `PrettierConfig` interface, then add the field to `AppConfig`:

```ts
export interface GlowConfig {
    enabled?: boolean;
    intensity?: number; // 0–1
    burstDuration?: number; // milliseconds
}
```

In `AppConfig`, add:

```ts
glow?: GlowConfig;
```

**Step 2: Add Zod schema in `src/main/main.ts`**

Find `AppConfigSchema` (the `z.object({...})` block). Add after the `prettier` field:

```ts
glow: z
    .object({
        enabled: z.boolean().optional(),
        intensity: z.number().min(0).max(1).optional(),
        burstDuration: z.number().min(200).max(2000).optional(),
    })
    .optional(),
```

**Step 3: Add default in `ensureConfigExists`**

In `main.ts`, find `ensureConfigExists`. In the default config object written to disk, add:

```ts
glow: { enabled: true },
```

**Step 4: Run typecheck to verify no type errors**

```bash
yarn typecheck
```

Expected: no errors.

**Step 5: Commit**

```bash
git add src/shared/ipcTypes.ts src/main/main.ts
git commit -m "feat: add GlowConfig to AppConfig schema"
```

---

### Task 2: Create `glowAnimation.ts` with unit tests (TDD)

**Files:**

- Create: `src/renderer/app/glowAnimation.ts`
- Create: `src/renderer/app/__tests__/glowAnimation.test.ts`

**Step 1: Write the failing tests**

Create `src/renderer/app/__tests__/glowAnimation.test.ts`:

```ts
import { describe, test, expect } from 'vitest';
import { createGlowAnimator, type ResolvedGlowConfig } from '../glowAnimation';

const cfg = (
    overrides: Partial<ResolvedGlowConfig> = {},
): ResolvedGlowConfig => ({
    enabled: true,
    intensity: 1.0,
    burstDuration: 800,
    ...overrides,
});

describe('createGlowAnimator', () => {
    test('returns baseline intensity when no burst is pending or active', () => {
        const animator = createGlowAnimator();
        // baseline = intensity * 0.25 = 0.25
        expect(animator.tick(1000, 5, cfg())).toBeCloseTo(0.25);
    });

    test('returns 0 when disabled', () => {
        const animator = createGlowAnimator();
        expect(animator.tick(0, 0, cfg({ enabled: false }))).toBe(0);
    });

    test('returns 0 when intensity is 0', () => {
        const animator = createGlowAnimator();
        expect(animator.tick(0, 0, cfg({ intensity: 0 }))).toBe(0);
    });

    test('burst fires when lastAppliedUpdateId reaches pendingUpdateId', () => {
        const animator = createGlowAnimator();
        animator.notifyPatchQueued(3);
        // tick at t=0: burst starts — sin(0) = 0, so at baseline
        expect(animator.tick(0, 3, cfg())).toBeCloseTo(0.25);
    });

    test('does not fire burst before lastAppliedUpdateId catches up', () => {
        const animator = createGlowAnimator();
        animator.notifyPatchQueued(5);
        // lastAppliedUpdateId is still 4, pending not yet confirmed
        expect(animator.tick(0, 4, cfg())).toBeCloseTo(0.25);
    });

    test('reaches full intensity at burst midpoint (sin(PI/2) = 1)', () => {
        const animator = createGlowAnimator();
        animator.notifyPatchQueued(1);
        animator.tick(0, 1, cfg()); // burst starts at now=0
        // midpoint: 400ms into 800ms burst → t=0.5 → sin(PI/2) = 1
        expect(animator.tick(400, 1, cfg())).toBeCloseTo(1.0);
    });

    test('returns to baseline at burst end (sin(PI) ≈ 0)', () => {
        const animator = createGlowAnimator();
        animator.notifyPatchQueued(1);
        animator.tick(0, 1, cfg()); // burst starts at now=0
        // end: 800ms into 800ms burst → t=1 → sin(PI) ≈ 0
        expect(animator.tick(800, 1, cfg())).toBeCloseTo(0.25, 1);
    });

    test('re-trigger resets burst start time', () => {
        const animator = createGlowAnimator();
        animator.notifyPatchQueued(1);
        animator.tick(0, 1, cfg()); // first burst starts at 0
        animator.tick(200, 1, cfg()); // 200ms in (quarter-way)

        // new patch queued and confirmed at 300ms
        animator.notifyPatchQueued(2);
        animator.tick(300, 2, cfg()); // second burst starts at 300ms

        // midpoint of second burst: 300 + 400 = 700ms → sin(PI/2) = 1
        expect(animator.tick(700, 2, cfg())).toBeCloseTo(1.0);
    });
});
```

**Step 2: Run tests to verify they all fail**

```bash
yarn test:unit --reporter=verbose src/renderer/app/__tests__/glowAnimation.test.ts
```

Expected: all tests fail with "Cannot find module '../glowAnimation'".

**Step 3: Implement `glowAnimation.ts`**

Create `src/renderer/app/glowAnimation.ts`:

```ts
export interface ResolvedGlowConfig {
    enabled: boolean;
    intensity: number; // 0–1
    burstDuration: number; // ms
}

export interface GlowAnimator {
    notifyPatchQueued: (updateId: number) => void;
    tick: (
        now: number,
        lastAppliedUpdateId: number,
        config: ResolvedGlowConfig,
    ) => number;
}

export function createGlowAnimator(): GlowAnimator {
    let pendingUpdateId: number | null = null;
    let burstStartTime: number | null = null;

    return {
        notifyPatchQueued(updateId) {
            pendingUpdateId = updateId;
        },

        tick(now, lastAppliedUpdateId, config) {
            if (!config.enabled || config.intensity === 0) return 0;

            // Check if the audio thread has confirmed a pending patch
            if (
                pendingUpdateId !== null &&
                lastAppliedUpdateId >= pendingUpdateId
            ) {
                burstStartTime = now;
                pendingUpdateId = null;
            }

            const baseline = config.intensity * 0.25;

            if (burstStartTime === null) return baseline;

            const t = Math.min(
                (now - burstStartTime) / config.burstDuration,
                1,
            );
            const burst = config.intensity;
            const intensity =
                baseline + (burst - baseline) * Math.sin(t * Math.PI);

            if (t >= 1) burstStartTime = null;

            return intensity;
        },
    };
}
```

**Step 4: Run tests to verify they all pass**

```bash
yarn test:unit --reporter=verbose src/renderer/app/__tests__/glowAnimation.test.ts
```

Expected: all 8 tests pass.

**Step 5: Commit**

```bash
git add src/renderer/app/glowAnimation.ts src/renderer/app/__tests__/glowAnimation.test.ts
git commit -m "feat: add glowAnimation module with unit tests"
```

---

### Task 3: Add `glowIntensity` to `drawOscilloscope`

**Files:**

- Modify: `src/renderer/app/oscilloscope.ts`

**Step 1: Add `glowIntensity` to `ScopeDrawOptions`**

In `src/renderer/app/oscilloscope.ts`, find the `ScopeDrawOptions` interface (lines 21–29) and add:

```ts
glowIntensity?: number; // 0–1, optional — defaults to 0
```

**Step 2: Apply `shadowBlur` to the waveform stroke**

In the body of `drawOscilloscope`, find where the waveform path is stroked (look for `ctx.stroke()` on the waveform path). Immediately before the `ctx.stroke()` call, add:

```ts
ctx.shadowBlur = (options.glowIntensity ?? 0) * 12;
ctx.shadowColor = accentPrimaryColor; // already declared above in the function
```

Immediately after `ctx.stroke()`, reset:

```ts
ctx.shadowBlur = 0;
```

This ensures only the waveform stroke gets the glow; reference lines, labels, and background are unaffected.

**Step 3: Run typecheck**

```bash
yarn typecheck
```

Expected: no errors. (`glowIntensity` is optional so all existing call sites are unaffected.)

**Step 4: Commit**

```bash
git add src/renderer/app/oscilloscope.ts
git commit -m "feat: add glowIntensity parameter to drawOscilloscope"
```

---

### Task 4: Add Monaco text-shadow CSS rule

**Files:**

- Modify: `src/renderer/App.css`

**Step 1: Add the glow rule**

At the bottom of `src/renderer/App.css`, add:

```css
/* Glow effect — driven by --glow-intensity CSS variable (0–1) set by the RAF loop */
.monaco-editor .view-lines .view-line {
    text-shadow: 0 0 calc(var(--glow-intensity, 0) * 8px) var(--accent-primary);
}
```

**Step 2: Commit**

```bash
git add src/renderer/App.css
git commit -m "feat: add glow text-shadow CSS rule for Monaco editor"
```

---

### Task 5: Wire glow into `App.tsx` (RAF loop + config state)

**Files:**

- Modify: `src/renderer/App.tsx`

This task has the most changes. Read `src/renderer/App.tsx` fully before making edits.

**Step 1: Import `createGlowAnimator` and `ResolvedGlowConfig`**

At the top of `App.tsx`, add the import:

```ts
import {
    createGlowAnimator,
    type ResolvedGlowConfig,
} from './app/glowAnimation';
```

Also import `GlowConfig` from shared types if not already imported:

```ts
import type { ..., GlowConfig } from '../shared/ipcTypes';
```

**Step 2: Add resolved-config helper**

Near the top of the `App` component function (with other helpers), add:

```ts
function resolveGlowConfig(cfg: GlowConfig | undefined): ResolvedGlowConfig {
    return {
        enabled: cfg?.enabled ?? true,
        intensity: cfg?.intensity ?? 0.5,
        burstDuration: cfg?.burstDuration ?? 800,
    };
}
```

**Step 3: Add glow state and animator ref**

Inside the `App` component, alongside the other `useRef` and `useState` calls, add:

```ts
const glowAnimatorRef = useRef(createGlowAnimator());
const [glowConfig, setGlowConfig] = useState<GlowConfig>({});
```

**Step 4: Read glow config on mount and subscribe to changes**

Find the `useEffect` that reads config on mount (or add one). Add a config read for glow:

```ts
useEffect(() => {
    void electronAPI.config.read().then((cfg) => {
        setGlowConfig(cfg.glow ?? {});
    });
    const unsub = electronAPI.config.onChange((cfg) => {
        setGlowConfig(cfg.glow ?? {});
    });
    return unsub;
}, []);
```

**Step 5: Notify the animator after successful `executeDSL`**

Find all call sites where `executeDSL` is called and the result is processed on success. After each `setScopeViews`/`setSliderDefs` call that marks a successful patch apply, add:

```ts
if (result.updateId != null) {
    glowAnimatorRef.current.notifyPatchQueued(result.updateId);
}
```

**Step 6: Update the RAF loop**

Find the `tick` function inside the `useEffect(() => { if (isClockRunning) ... }, [isClockRunning])` block (around line 419).

At the very top of the `tick` function, before the `Promise.all`, capture the current time:

```ts
const now = performance.now();
```

Inside the `.then(([scopeData, transport]) => {` handler, after the existing `setTransportState(transport)` call (around line 493), add:

```ts
const glowIntensity = glowAnimatorRef.current.tick(
    now,
    transport.lastAppliedUpdateId,
    resolveGlowConfig(glowConfig),
);
document.documentElement.style.setProperty(
    '--glow-intensity',
    glowIntensity.toFixed(4),
);
```

Then find the `drawOscilloscope` call (around line 481) and pass the intensity:

```ts
drawOscilloscope(channels, canvas, {
    range: [rangeMin, rangeMax],
    stats: { ... },
    glowIntensity,
});
```

Note: `glowIntensity` is declared above the canvas loop, so it's in scope here. If the canvas loop runs before the transport block in the code, move the `glowIntensity` declaration above the canvas loop or restructure so transport is processed first. Read the actual file to determine the correct order before editing.

**Step 7: Run typecheck**

```bash
yarn typecheck
```

Expected: no errors.

**Step 8: Commit**

```bash
git add src/renderer/App.tsx
git commit -m "feat: wire glow animation into App RAF loop"
```

---

### Task 6: Add glow controls to the Editor settings tab

**Files:**

- Modify: `src/renderer/components/EditorSettingsTab.tsx`

Read `src/renderer/components/EditorSettingsTab.tsx` fully before editing to understand the existing prop types and how auto-save is wired.

**Step 1: Confirm the component's prop type includes `config` and `onChange`/`onConfigChange`**

The tab receives the current `AppConfig` and a write callback. Find how the existing controls (e.g., font size range input) call `electronAPI.config.write(...)` and follow the same pattern for glow controls.

**Step 2: Add glow section at the bottom of the tab's JSX**

After the last existing `settings-section` div (Cursor Style), add:

```tsx
<div className="settings-section">
    <h3>Glow</h3>
    <div className="settings-row">
        <label>
            <input
                type="checkbox"
                checked={config.glow?.enabled ?? true}
                onChange={(e) =>
                    void electronAPI.config.write({
                        glow: {
                            ...(config.glow ?? {}),
                            enabled: e.target.checked,
                        },
                    })
                }
            />
            Enable glow
        </label>
    </div>
    <div className="settings-row">
        <label>Intensity</label>
        <input
            type="range"
            min={0}
            max={100}
            step={1}
            disabled={!(config.glow?.enabled ?? true)}
            value={Math.round((config.glow?.intensity ?? 0.5) * 100)}
            onInput={(e) =>
                void electronAPI.config.write({
                    glow: {
                        ...(config.glow ?? {}),
                        intensity:
                            Number((e.target as HTMLInputElement).value) / 100,
                    },
                })
            }
        />
        <span>{Math.round((config.glow?.intensity ?? 0.5) * 100)}%</span>
    </div>
    <div className="settings-row">
        <label>Burst duration</label>
        <input
            type="range"
            min={200}
            max={2000}
            step={100}
            disabled={!(config.glow?.enabled ?? true)}
            value={config.glow?.burstDuration ?? 800}
            onInput={(e) =>
                void electronAPI.config.write({
                    glow: {
                        ...(config.glow ?? {}),
                        burstDuration: Number(
                            (e.target as HTMLInputElement).value,
                        ),
                    },
                })
            }
        />
        <span>{config.glow?.burstDuration ?? 800}ms</span>
    </div>
</div>
```

Adjust to match the exact prop names and write pattern used by the existing controls — read the file before editing.

**Step 3: Run typecheck**

```bash
yarn typecheck
```

Expected: no errors.

**Step 4: Run the app and verify settings controls update the glow in real time**

```bash
yarn start
```

Open Settings → Editor tab. Confirm the three glow controls appear and are functional. Apply a patch and observe the burst.

**Step 5: Run the full test suite**

```bash
yarn test:unit
```

Expected: all unit tests pass.

**Step 6: Commit**

```bash
git add src/renderer/components/EditorSettingsTab.tsx
git commit -m "feat: add glow controls to Editor settings tab"
```
