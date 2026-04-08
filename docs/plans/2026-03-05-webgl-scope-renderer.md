# WebGL Oscilloscope Renderer Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace the Canvas2D oscilloscope renderer with a WebGL-powered renderer inspired by [woscope](https://github.com/m1el/woscope), producing physically-realistic gaussian beam lines with bloom post-processing.

**Architecture:** A `WebGLScopeRenderer` class manages a single WebGL context per scope canvas. Line segments between consecutive samples are expanded into screen-space quads in the vertex shader. The fragment shader computes analytical gaussian beam intensity using the error function (erf). Additive blending accumulates intensity from overlapping segments. An optional multi-pass bloom (render-to-texture, downscale, separable gaussian blur, composite) adds glow. A transparent Canvas2D overlay handles reference lines, legends, and stats text. The existing Rust data pipeline (ScopeBuffer -> N-API get_scopes() -> IPC -> renderer polling loop) is unchanged.

**Tech Stack:** WebGL 1.0, GLSL ES 1.0, TypeScript, Canvas 2D (overlay only), existing Electron IPC + Rust N-API pipeline.

---

## How Woscope Works (Reference)

Understanding this is critical before implementing. Read the [full explanation](https://m1el.github.io/woscope-how/).

### Core Technique

1. **Geometry generation:** Each line segment (between sample `i` and sample `i+1`) is drawn as a quad (2 triangles, 4 vertices). All 4 vertices receive both the start point and end point. The vertex index (0-3) determines which corner of the quad this vertex becomes — offset along the segment direction (forward/backward) and normal (left/right) by `uSize`.

2. **Buffer layout trick:** Sample data is stored as `[x0,y0, x0,y0, x0,y0, x0,y0, x1,y1, x1,y1, ...]` — each sample repeated 4 times (one per quad vertex). The `aStart` attribute reads at offset 0, `aEnd` reads at offset `4 vertices * 2 floats * 4 bytes = 32 bytes`. This makes consecutive sample pairs automatically available as (start, end) for each segment's quad.

3. **Gaussian beam intensity (fragment shader):** For each pixel in a quad, intensity is computed analytically:
    - The pixel position is transformed into the segment's local coordinate frame where start = (0,0), end = (length, 0)
    - Perpendicular falloff: `exp(-py^2 / 2*sigma^2)` (gaussian)
    - Parallel contribution: `erf(px / sqrt2*sigma) - erf((px-len) / sqrt2*sigma)` (integrated gaussian = error function)
    - Combined: `(1/2L) * exp(-py^2/2s^2) * [erf(px/sqrt2*s) - erf((px-L)/sqrt2*s)]`
    - This gives mathematically perfect segment joints — no corner artifacts

4. **Additive blending:** `gl.blendFunc(gl.SRC_ALPHA, gl.ONE)` — overlapping segments accumulate intensity like phosphor on a real CRT.

5. **Afterglow:** Earlier samples (lower vertex index) are dimmed via `smoothstep(0.0, 0.33, vertexIndex/nSamples)`, simulating phosphor decay.

6. **Bloom (optional):** Lines are rendered to a framebuffer texture, downscaled, blurred with a separable gaussian kernel (2 passes — horizontal then vertical), and composited back with the sharp original at reduced alpha.

### Adaptation for Time-Domain Display

Woscope uses XY mode (stereo audio as 2D coordinates). Our scopes are time-domain:

- **X axis** = time (linear ramp from -1 to +1 across the sample window)
- **Y axis** = voltage (sample value, normalized to the display range)

The rendering technique is identical — we just generate different (x, y) coordinates. The vertex shader, fragment shader, blending, and bloom are unchanged.

---

## Improvements Over Woscope

These are improvements identified from woscope's own limitations and our specific needs:

1. **Per-channel colors** — Each trace in a multi-channel scope gets a distinct color via the `uColor` uniform, rendered as separate draw calls.

2. **Sinc interpolation** — Upsample the 1024-sample buffer by 4x using a windowed sinc kernel before sending to WebGL. This eliminates the sharp corners between samples that don't exist on real oscilloscopes. Done in JavaScript before buffer upload.

3. **Gamma correction** — Apply sRGB gamma in the output/composite shader to ensure correct brightness perception.

4. **Intensity normalization** — Scale `uIntensity` based on the number of samples and line density to prevent oversaturation in dense waveform regions.

5. **Configurable beam width and bloom** — Expose `lineSize` and `bloomIntensity` as user-configurable settings, allowing users to tune the visual style.

6. **Phosphor persistence** — Blend the previous frame's output into the current frame at reduced alpha, simulating CRT phosphor persistence across animation frames. This creates smooth temporal continuity rather than per-frame redraw.

7. **Canvas2D overlay for UI chrome** — Reference lines, voltage labels, and stats are rendered on a transparent Canvas2D layer on top of the WebGL canvas. This avoids the complexity of WebGL text rendering while keeping the UI functional.

---

## File Structure

New files to create:

```
src/renderer/app/webgl/
  scopeRenderer.ts       — WebGLScopeRenderer class (context, lifecycle, draw)
  shaders.ts             — GLSL shader source strings (vertex, fragment, blur, output)
  bufferBuilder.ts       — Sample data → WebGL buffer conversion, sinc interpolation
  types.ts               — Shared WebGL types and constants
```

Files to modify:

```
src/renderer/app/oscilloscope.ts          — Replace drawOscilloscope internals (or rename old)
src/renderer/components/monaco/scopeViewZones.ts  — Dual-canvas creation (WebGL + 2D overlay)
src/renderer/App.tsx                      — Wire WebGL renderer lifecycle into RAF loop
src/renderer/App.css                      — Style the layered canvas container
src/shared/ipcTypes.ts                    — Add ScopeRenderConfig to AppConfig
src/main/main.ts                          — Add Zod schema for ScopeRenderConfig
```

---

## Task 1: Create GLSL Shaders Module

**Files:**

- Create: `src/renderer/app/webgl/shaders.ts`

**Step 1: Write the line vertex shader**

This is adapted directly from woscope's vsLine.glsl but parameterized for time-domain display. The shader takes start/end points of a line segment and vertex index, then computes the quad corner position by offsetting along segment direction and normal.

```ts
export const LINE_VERTEX_SHADER = `
precision highp float;

#define EPS 1E-6

uniform float uInvert;
uniform float uSize;
uniform float uAspect; // width/height ratio for non-square canvases

attribute vec2 aStart;
attribute vec2 aEnd;
attribute float aIdx;

varying vec4 uvl;

void main() {
    float idx = mod(aIdx, 4.0);

    vec2 dir = aEnd - aStart;
    uvl.z = length(dir);

    if (uvl.z > EPS) {
        dir = dir / uvl.z;
    } else {
        dir = vec2(1.0, 0.0);
    }

    // Correct normal for aspect ratio so beam width is uniform in screen space
    vec2 norm = vec2(-dir.y * uAspect, dir.x / uAspect);
    norm = normalize(norm);

    vec2 current;
    float tang;

    if (idx >= 2.0) {
        current = aEnd;
        tang = 1.0;
        uvl.x = -uSize;
    } else {
        current = aStart;
        tang = -1.0;
        uvl.x = uvl.z + uSize;
    }

    float side = (mod(idx, 2.0) - 0.5) * 2.0;
    uvl.y = side * uSize;
    uvl.w = floor(aIdx / 4.0 + 0.5);

    gl_Position = vec4(
        (current + (tang * dir + norm * side) * uSize) * uInvert,
        0.0,
        1.0
    );
}
`;
```

**Step 2: Write the line fragment shader**

Adapted from woscope's fsLine.glsl — computes gaussian beam intensity analytically using the error function.

```ts
export const LINE_FRAGMENT_SHADER = `
precision highp float;

#define EPS 1E-6
#define TAU 6.283185307179586
#define TAUR 2.5066282746310002
#define SQRT2 1.4142135623730951

uniform float uSize;
uniform float uIntensity;
uniform float uNSamples;
uniform vec4 uColor;

varying vec4 uvl;

float gaussian(float x, float sigma) {
    return exp(-(x * x) / (2.0 * sigma * sigma)) / (TAUR * sigma);
}

float erf(float x) {
    float s = sign(x), a = abs(x);
    x = 1.0 + (0.278393 + (0.230389 + (0.000972 + 0.078108 * a) * a) * a) * a;
    x *= x;
    return s - s / (x * x);
}

void main() {
    float len = uvl.z;
    vec2 xy = vec2(
        (len / 2.0 + uSize) * uvl.x + len / 2.0,
        uSize * uvl.y
    );

    float sigma = uSize / 4.0;
    float alpha;

    if (len < EPS) {
        alpha = exp(-pow(length(xy), 2.0) / (2.0 * sigma * sigma))
              / 2.0 / sqrt(uSize);
    } else {
        alpha = erf(xy.x / SQRT2 / sigma) - erf((xy.x - len) / SQRT2 / sigma);
        alpha *= exp(-xy.y * xy.y / (2.0 * sigma * sigma))
               / 2.0 / len * uSize;
    }

    // Afterglow: fade earlier samples
    float afterglow = smoothstep(0.0, 0.33, uvl.w / uNSamples);
    alpha *= afterglow * uIntensity;

    gl_FragColor = vec4(uColor.rgb, uColor.a * alpha);
}
`;
```

**Step 3: Write the blur/transpose shader pair**

For the bloom post-processing — a separable gaussian blur. The vertex shader passes through a full-screen quad, the fragment shader samples along one axis (transposed between passes).

```ts
export const BLUR_VERTEX_SHADER = `
precision highp float;
attribute vec2 aPos;
attribute vec2 aST;
varying vec2 vTexCoord;
void main() {
    vTexCoord = aST;
    gl_Position = vec4(aPos, 0.0, 1.0);
}
`;

export const BLUR_FRAGMENT_SHADER = `
precision highp float;
uniform sampler2D uTexture;
uniform float uSize;
varying vec2 vTexCoord;

void main() {
    float step = uSize / 1024.0 / 1024.0 * 2.0;
    vec4 sum = vec4(0.0);
    sum += texture2D(uTexture, vec2(vTexCoord.x - step * 4.0, vTexCoord.y)) * (1.0 / 25.0);
    sum += texture2D(uTexture, vec2(vTexCoord.x - step * 3.0, vTexCoord.y)) * (2.0 / 25.0);
    sum += texture2D(uTexture, vec2(vTexCoord.x - step * 2.0, vTexCoord.y)) * (3.0 / 25.0);
    sum += texture2D(uTexture, vec2(vTexCoord.x - step * 1.0, vTexCoord.y)) * (4.0 / 25.0);
    sum += texture2D(uTexture, vec2(vTexCoord.x,              vTexCoord.y)) * (5.0 / 25.0);
    sum += texture2D(uTexture, vec2(vTexCoord.x + step * 1.0, vTexCoord.y)) * (4.0 / 25.0);
    sum += texture2D(uTexture, vec2(vTexCoord.x + step * 2.0, vTexCoord.y)) * (3.0 / 25.0);
    sum += texture2D(uTexture, vec2(vTexCoord.x + step * 3.0, vTexCoord.y)) * (2.0 / 25.0);
    sum += texture2D(uTexture, vec2(vTexCoord.x + step * 4.0, vTexCoord.y)) * (1.0 / 25.0);
    gl_FragColor = sum;
}
`;
```

**Step 4: Write the output/composite shader**

Composites a texture onto the screen with alpha control, used for the bloom pass.

```ts
export const OUTPUT_VERTEX_SHADER = BLUR_VERTEX_SHADER; // Same full-screen quad

export const OUTPUT_FRAGMENT_SHADER = `
precision highp float;
uniform sampler2D uTexture;
uniform float uAlpha;
uniform float uGamma; // sRGB gamma correction (2.2 default)
varying vec2 vTexCoord;

void main() {
    vec4 color = texture2D(uTexture, vTexCoord);
    // Apply gamma correction
    color.rgb = pow(color.rgb, vec3(1.0 / uGamma));
    color.a *= uAlpha;
    gl_FragColor = color;
}
`;
```

**Step 5: Commit**

```bash
git add src/renderer/app/webgl/shaders.ts
git commit -m "feat: add WebGL scope renderer GLSL shaders"
```

---

## Task 2: Create WebGL Types and Constants

**Files:**

- Create: `src/renderer/app/webgl/types.ts`

**Step 1: Define shared types**

```ts
export interface ScopeRenderConfig {
    /** Beam width in GL units (0.001 – 0.05). Default: 0.012 */
    lineSize: number;
    /** Overall beam intensity multiplier (0.1 – 5.0). Default: 1.0 */
    intensity: number;
    /** Enable bloom post-processing. Default: true */
    bloom: boolean;
    /** Bloom layer alpha (0.0 – 1.0). Default: 0.5 */
    bloomAlpha: number;
    /** sRGB gamma value. Default: 2.2 */
    gamma: number;
    /** Enable phosphor persistence (blend previous frame). Default: false */
    persistence: boolean;
    /** Persistence decay factor per frame (0.0 – 0.95). Default: 0.7 */
    persistenceDecay: number;
}

export const DEFAULT_SCOPE_RENDER_CONFIG: ScopeRenderConfig = {
    lineSize: 0.012,
    intensity: 1.0,
    bloom: true,
    bloomAlpha: 0.5,
    gamma: 2.2,
    persistence: false,
    persistenceDecay: 0.7,
};

/** Default trace colors for multi-channel scopes (RGBA, 0–1 range) */
export const TRACE_COLORS: [number, number, number, number][] = [
    [0.306, 0.788, 0.69, 1.0], // #4ec9b0 — teal (accent-primary)
    [0.545, 0.659, 0.859, 1.0], // #8ba8db — blue
    [0.89, 0.545, 0.549, 1.0], // #e38b8c — rose
    [0.839, 0.769, 0.459, 1.0], // #d6c475 — gold
    [0.616, 0.859, 0.545, 1.0], // #9ddb8b — green
    [0.753, 0.545, 0.859, 1.0], // #c08bdb — purple
];

export const SCOPE_SAMPLE_COUNT = 1024;

/** Internal framebuffer resolution for bloom passes */
export const FB_RESOLUTION = 512;
```

**Step 2: Commit**

```bash
git add src/renderer/app/webgl/types.ts
git commit -m "feat: add WebGL scope renderer types and constants"
```

---

## Task 3: Create Buffer Builder (Sample Data to WebGL Buffers)

**Files:**

- Create: `src/renderer/app/webgl/bufferBuilder.ts`
- Create: `src/renderer/app/webgl/__tests__/bufferBuilder.test.ts`

**Step 1: Write failing tests for buffer building**

```ts
import { describe, test, expect } from 'vitest';
import { buildLineBuffer, sincInterpolate } from '../bufferBuilder';

describe('buildLineBuffer', () => {
    test('creates correct buffer layout for 4 samples', () => {
        // 4 samples → 3 segments → 3*4 = 12 vertices → 12*2 = 24 floats
        const samples = new Float32Array([0.0, 0.5, -0.3, 0.1]);
        const buf = buildLineBuffer(samples, 0, [-1, 1]);
        // Each sample becomes 4 vertices with (x, y) repeated
        // Total = nSamples * 4 * 2 floats
        expect(buf.length).toBe(4 * 4 * 2);
        // First vertex: x=-1 (first time position), y=0.0 (first sample normalized)
        expect(buf[0]).toBeCloseTo(-1.0); // x
        expect(buf[1]).toBeCloseTo(0.0); // y (0.0 in [-1,1] maps to 0.0)
    });

    test('normalizes Y values to [-1, 1] range', () => {
        const samples = new Float32Array([5.0]); // max of range [-5, 5]
        const buf = buildLineBuffer(samples, 0, [-5, 5]);
        expect(buf[1]).toBeCloseTo(1.0); // 5V in [-5,5] maps to 1.0
    });

    test('applies readOffset for circular buffer alignment', () => {
        const samples = new Float32Array([0.0, 1.0, 2.0, 3.0]);
        const buf = buildLineBuffer(samples, 2, [-5, 5]); // offset=2: reads [2,3,0,1]
        // First sample should be samples[2] = 2.0
        expect(buf[1]).toBeCloseTo(2.0 / 5.0); // 2.0 in [-5,5] → 0.4
    });
});

describe('sincInterpolate', () => {
    test('returns 4x upsampled array', () => {
        const input = new Float32Array([0, 1, 0, -1]);
        const output = sincInterpolate(input, 4);
        expect(output.length).toBe(16);
    });

    test('preserves original sample values at integer positions', () => {
        const input = new Float32Array([0, 1, 0, -1, 0]);
        const output = sincInterpolate(input, 4);
        // Original samples should appear at indices 0, 4, 8, 12, 16
        expect(output[0]).toBeCloseTo(0, 1);
        expect(output[4]).toBeCloseTo(1, 1);
        expect(output[8]).toBeCloseTo(0, 1);
    });
});
```

**Step 2: Run tests to verify they fail**

```bash
yarn test:unit --reporter=verbose src/renderer/app/webgl/__tests__/bufferBuilder.test.ts
```

Expected: FAIL — module not found.

**Step 3: Implement bufferBuilder.ts**

```ts
import { SCOPE_SAMPLE_COUNT } from './types';

/**
 * Build the vertex buffer for WebGL line rendering.
 *
 * Each sample is written 4 times (one per quad vertex).
 * The vertex shader reads aStart from offset 0 and aEnd from offset
 * (4 vertices * 2 floats * 4 bytes = 32 bytes), creating segment pairs.
 *
 * @param samples   Raw Float32Array from the scope buffer (1024 samples)
 * @param readOffset  Circular buffer read offset
 * @param range     Voltage display range [min, max]
 * @returns Float32Array sized nSamples * 4 * 2
 */
export function buildLineBuffer(
    samples: Float32Array,
    readOffset: number,
    range: [number, number],
): Float32Array {
    const n = samples.length;
    const buf = new Float32Array(n * 4 * 2);
    const [vMin, vMax] = range;
    const vRange = vMax - vMin;

    for (let i = 0; i < n; i++) {
        const dataIndex = (i + readOffset) % n;
        // Map time to [-1, 1]
        const x = (i / (n - 1)) * 2.0 - 1.0;
        // Map voltage to [-1, 1]
        const rawSample = samples[dataIndex];
        const clamped = Math.max(vMin, Math.min(vMax, rawSample));
        const y = ((clamped - vMin) / vRange) * 2.0 - 1.0;

        // Write 4 copies (one per quad vertex)
        const t = i * 8;
        buf[t] = buf[t + 2] = buf[t + 4] = buf[t + 6] = x;
        buf[t + 1] = buf[t + 3] = buf[t + 5] = buf[t + 7] = y;
    }

    return buf;
}

/**
 * Windowed sinc interpolation for upsampling.
 * Smooths the waveform to eliminate sharp corners between samples.
 *
 * @param input   Original samples
 * @param factor  Upsampling factor (e.g. 4 for 4x)
 * @returns Upsampled Float32Array of length input.length * factor
 */
export function sincInterpolate(
    input: Float32Array,
    factor: number,
): Float32Array {
    const n = input.length;
    const out = new Float32Array(n * factor);
    const halfWindow = 8; // Sinc kernel half-width in original samples

    for (let i = 0; i < n * factor; i++) {
        const t = i / factor; // Position in original sample space
        let sum = 0;
        const iStart = Math.max(0, Math.ceil(t - halfWindow));
        const iEnd = Math.min(n - 1, Math.floor(t + halfWindow));

        for (let j = iStart; j <= iEnd; j++) {
            const x = t - j;
            // sinc(x) * Hann window
            let sinc: number;
            if (Math.abs(x) < 1e-8) {
                sinc = 1.0;
            } else {
                sinc = Math.sin(Math.PI * x) / (Math.PI * x);
            }
            // Hann window
            const w = 0.5 * (1 + Math.cos((Math.PI * x) / halfWindow));
            sum += input[j] * sinc * w;
        }
        out[i] = sum;
    }

    return out;
}

/**
 * Build the quad index buffer for element drawing.
 * Each segment (n-1 total) is drawn as 2 triangles (6 indices).
 */
export function buildQuadIndices(nSamples: number): Uint16Array {
    const nSegments = nSamples - 1;
    const indices = new Uint16Array(nSegments * 6);
    for (let i = 0, pos = 0, idx = 0; i < nSegments; i++, pos += 4) {
        indices[idx++] = pos;
        indices[idx++] = pos + 2;
        indices[idx++] = pos + 1;
        indices[idx++] = pos + 1;
        indices[idx++] = pos + 2;
        indices[idx++] = pos + 3;
    }
    return indices;
}

/**
 * Build the vertex index attribute buffer.
 * Sequential integers 0..nSamples*4, one per vertex.
 */
export function buildVertexIndexBuffer(nSamples: number): Int16Array {
    const buf = new Int16Array(nSamples * 4);
    for (let i = 0; i < buf.length; i++) {
        buf[i] = i;
    }
    return buf;
}
```

**Step 4: Run tests to verify they pass**

```bash
yarn test:unit --reporter=verbose src/renderer/app/webgl/__tests__/bufferBuilder.test.ts
```

Expected: all tests pass.

**Step 5: Commit**

```bash
git add src/renderer/app/webgl/bufferBuilder.ts src/renderer/app/webgl/__tests__/bufferBuilder.test.ts
git commit -m "feat: add buffer builder with sinc interpolation for WebGL scope"
```

---

## Task 4: Create WebGLScopeRenderer Class (Core)

**Files:**

- Create: `src/renderer/app/webgl/scopeRenderer.ts`

This is the main rendering class. It manages the WebGL context lifecycle, shader compilation, buffer management, and the draw call.

**Step 1: Implement the WebGLScopeRenderer class**

The renderer handles:

- WebGL context creation and cleanup
- Shader program compilation and caching
- Static buffer creation (quad indices, vertex indices, full-screen quad)
- Per-frame dynamic buffer upload (sample data)
- The draw pipeline: clear → draw lines (per channel) → bloom passes → composite

Key architectural decisions:

- One `WebGLScopeRenderer` instance per scope canvas
- Static buffers (indices, full-screen quad) are created once in the constructor
- Dynamic buffers (sample data) are updated every frame via `gl.bufferData`
- Bloom uses render-to-texture with a framebuffer

See `scopeRenderer.ts` in full below. This file is ~300 lines.

```ts
import {
    LINE_VERTEX_SHADER,
    LINE_FRAGMENT_SHADER,
    BLUR_VERTEX_SHADER,
    BLUR_FRAGMENT_SHADER,
    OUTPUT_VERTEX_SHADER,
    OUTPUT_FRAGMENT_SHADER,
} from './shaders';
import {
    type ScopeRenderConfig,
    DEFAULT_SCOPE_RENDER_CONFIG,
    TRACE_COLORS,
    FB_RESOLUTION,
} from './types';
import {
    buildLineBuffer,
    buildQuadIndices,
    buildVertexIndexBuffer,
} from './bufferBuilder';

export class WebGLScopeRenderer {
    private gl: WebGLRenderingContext | null = null;
    private canvas: HTMLCanvasElement;

    // Shader programs
    private lineProgram: WebGLProgram | null = null;
    private blurProgram: WebGLProgram | null = null;
    private outputProgram: WebGLProgram | null = null;

    // Static buffers
    private quadIndexBuf: WebGLBuffer | null = null; // vertex index attribute (aIdx)
    private elementIndexBuf: WebGLBuffer | null = null; // element array (triangle indices)
    private outQuadBuf: WebGLBuffer | null = null; // full-screen quad for post-processing
    private dataBuf: WebGLBuffer | null = null; // dynamic sample data buffer

    // Framebuffers for bloom
    private framebuffer: WebGLFramebuffer | null = null;
    private lineTexture: WebGLTexture | null = null;
    private blurTexture1: WebGLTexture | null = null;
    private blurTexture2: WebGLTexture | null = null;
    private renderBuffer: WebGLRenderbuffer | null = null;

    private nSamples = 0;
    private destroyed = false;

    constructor(canvas: HTMLCanvasElement) {
        this.canvas = canvas;
        this.initGL();
    }

    private initGL(): void {
        const gl = this.canvas.getContext('webgl', {
            alpha: true,
            premultipliedAlpha: false,
            antialias: false,
            preserveDrawingBuffer: false,
        });
        if (!gl) {
            console.warn('WebGL not available for scope renderer');
            return;
        }
        this.gl = gl;
        gl.clearColor(0, 0, 0, 0);

        // Compile shaders
        this.lineProgram = this.compileProgram(
            LINE_VERTEX_SHADER,
            LINE_FRAGMENT_SHADER,
        );
        this.blurProgram = this.compileProgram(
            BLUR_VERTEX_SHADER,
            BLUR_FRAGMENT_SHADER,
        );
        this.outputProgram = this.compileProgram(
            OUTPUT_VERTEX_SHADER,
            OUTPUT_FRAGMENT_SHADER,
        );

        // Create full-screen quad for post-processing
        this.outQuadBuf = gl.createBuffer();
        gl.bindBuffer(gl.ARRAY_BUFFER, this.outQuadBuf);
        gl.bufferData(
            gl.ARRAY_BUFFER,
            new Int16Array([
                -1, -1, 0, 0, -1, 1, 0, 1, 1, -1, 1, 0, 1, 1, 1, 1,
            ]),
            gl.STATIC_DRAW,
        );

        // Create dynamic data buffer
        this.dataBuf = gl.createBuffer();

        // Create bloom framebuffers
        this.framebuffer = gl.createFramebuffer();
        this.renderBuffer = gl.createRenderbuffer();
        this.lineTexture = this.createTargetTexture(
            FB_RESOLUTION,
            FB_RESOLUTION,
        );
        this.blurTexture1 = this.createTargetTexture(
            FB_RESOLUTION,
            FB_RESOLUTION,
        );
        this.blurTexture2 = this.createTargetTexture(
            FB_RESOLUTION,
            FB_RESOLUTION,
        );
    }

    /**
     * Update sample count and rebuild static index buffers.
     * Called when the number of samples changes (rare).
     */
    private ensureIndexBuffers(nSamples: number): void {
        if (this.nSamples === nSamples || !this.gl) return;
        this.nSamples = nSamples;
        const gl = this.gl;

        // Vertex index attribute (aIdx = 0, 1, 2, 3, 4, 5, ...)
        const vertexIndices = buildVertexIndexBuffer(nSamples);
        this.quadIndexBuf = this.quadIndexBuf || gl.createBuffer();
        gl.bindBuffer(gl.ARRAY_BUFFER, this.quadIndexBuf);
        gl.bufferData(gl.ARRAY_BUFFER, vertexIndices, gl.STATIC_DRAW);

        // Element index buffer (triangle indices for quads)
        const quadIndices = buildQuadIndices(nSamples);
        this.elementIndexBuf = this.elementIndexBuf || gl.createBuffer();
        gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, this.elementIndexBuf);
        gl.bufferData(gl.ELEMENT_ARRAY_BUFFER, quadIndices, gl.STATIC_DRAW);
    }

    /**
     * Draw the scope with the given sample data.
     *
     * @param channels   Per-channel sample data arrays
     * @param readOffsets Per-channel circular buffer read offsets
     * @param range      Voltage display range [min, max]
     * @param config     Rendering configuration
     */
    draw(
        channels: Float32Array[],
        readOffsets: number[],
        range: [number, number],
        config: Partial<ScopeRenderConfig> = {},
    ): void {
        const gl = this.gl;
        if (!gl || this.destroyed || channels.length === 0) return;

        const cfg = { ...DEFAULT_SCOPE_RENDER_CONFIG, ...config };
        const w = this.canvas.width;
        const h = this.canvas.height;
        const aspect = w / h;

        if (cfg.bloom && this.framebuffer && this.lineTexture) {
            this.drawWithBloom(channels, readOffsets, range, cfg, w, h, aspect);
        } else {
            this.drawDirect(channels, readOffsets, range, cfg, w, h, aspect);
        }
    }

    private drawDirect(
        channels: Float32Array[],
        readOffsets: number[],
        range: [number, number],
        cfg: ScopeRenderConfig,
        w: number,
        h: number,
        aspect: number,
    ): void {
        const gl = this.gl!;
        gl.bindFramebuffer(gl.FRAMEBUFFER, null);
        gl.viewport(0, 0, w, h);
        gl.clear(gl.COLOR_BUFFER_BIT);

        for (let ch = 0; ch < channels.length; ch++) {
            const color = TRACE_COLORS[ch % TRACE_COLORS.length];
            this.drawLineChannel(
                channels[ch],
                readOffsets[ch] ?? 0,
                range,
                cfg,
                color,
                aspect,
            );
        }
    }

    private drawWithBloom(
        channels: Float32Array[],
        readOffsets: number[],
        range: [number, number],
        cfg: ScopeRenderConfig,
        w: number,
        h: number,
        aspect: number,
    ): void {
        const gl = this.gl!;

        // Pass 1: Render lines to lineTexture
        gl.bindFramebuffer(gl.FRAMEBUFFER, this.framebuffer);
        this.activateTexture(this.lineTexture!);
        gl.viewport(0, 0, FB_RESOLUTION, FB_RESOLUTION);
        gl.clear(gl.COLOR_BUFFER_BIT);

        for (let ch = 0; ch < channels.length; ch++) {
            const color = TRACE_COLORS[ch % TRACE_COLORS.length];
            this.drawLineChannel(
                channels[ch],
                readOffsets[ch] ?? 0,
                range,
                cfg,
                color,
                aspect,
            );
        }

        // Generate mipmap for the line texture
        gl.bindTexture(gl.TEXTURE_2D, this.lineTexture);
        gl.generateMipmap(gl.TEXTURE_2D);
        gl.bindTexture(gl.TEXTURE_2D, null);

        // Pass 2: Downscale lineTexture → blurTexture2
        this.activateTexture(this.blurTexture2!);
        gl.viewport(0, 0, FB_RESOLUTION / 2, FB_RESOLUTION / 2);
        gl.clear(gl.COLOR_BUFFER_BIT);
        this.drawTexture(
            this.lineTexture!,
            w,
            this.outputProgram!,
            1.0,
            cfg.gamma,
        );

        // Pass 3: Blur X — blurTexture2 → blurTexture1
        this.activateTexture(this.blurTexture1!);
        gl.clear(gl.COLOR_BUFFER_BIT);
        this.drawTexture(
            this.blurTexture2!,
            w / 2,
            this.blurProgram!,
            1.0,
            cfg.gamma,
        );

        // Pass 4: Blur Y — blurTexture1 → blurTexture2
        this.activateTexture(this.blurTexture2!);
        gl.clear(gl.COLOR_BUFFER_BIT);
        this.drawTexture(
            this.blurTexture1!,
            w / 2,
            this.blurProgram!,
            1.0,
            cfg.gamma,
        );

        // Pass 5: Composite to screen
        gl.bindFramebuffer(gl.FRAMEBUFFER, null);
        gl.viewport(0, 0, w, h);
        gl.clear(gl.COLOR_BUFFER_BIT);

        // Sharp original
        this.drawTexture(
            this.lineTexture!,
            w,
            this.outputProgram!,
            1.0,
            cfg.gamma,
        );
        // Blurred bloom layer
        this.drawTexture(
            this.blurTexture2!,
            w / 2,
            this.outputProgram!,
            cfg.bloomAlpha,
            cfg.gamma,
        );
    }

    private drawLineChannel(
        samples: Float32Array,
        readOffset: number,
        range: [number, number],
        cfg: ScopeRenderConfig,
        color: [number, number, number, number],
        aspect: number,
    ): void {
        const gl = this.gl!;
        const nSamples = samples.length;
        this.ensureIndexBuffers(nSamples);

        // Build and upload vertex data
        const lineData = buildLineBuffer(samples, readOffset, range);
        gl.bindBuffer(gl.ARRAY_BUFFER, this.dataBuf);
        gl.bufferData(gl.ARRAY_BUFFER, lineData, gl.DYNAMIC_DRAW);

        // Use line shader
        gl.useProgram(this.lineProgram);

        // Set uniforms
        this.setUniform1f(this.lineProgram!, 'uInvert', 1.0);
        this.setUniform1f(this.lineProgram!, 'uSize', cfg.lineSize);
        this.setUniform1f(this.lineProgram!, 'uIntensity', cfg.intensity);
        this.setUniform1f(this.lineProgram!, 'uNSamples', nSamples);
        this.setUniform1f(this.lineProgram!, 'uAspect', aspect);
        this.setUniform4fv(this.lineProgram!, 'uColor', color);

        // Bind vertex index attribute (aIdx)
        gl.bindBuffer(gl.ARRAY_BUFFER, this.quadIndexBuf);
        const idxAttr = gl.getAttribLocation(this.lineProgram!, 'aIdx');
        if (idxAttr > -1) {
            gl.enableVertexAttribArray(idxAttr);
            gl.vertexAttribPointer(idxAttr, 1, gl.SHORT, false, 2, 0);
        }

        // Bind sample data — aStart and aEnd read from same buffer at different offsets
        gl.bindBuffer(gl.ARRAY_BUFFER, this.dataBuf);
        const startAttr = gl.getAttribLocation(this.lineProgram!, 'aStart');
        if (startAttr > -1) {
            gl.enableVertexAttribArray(startAttr);
            gl.vertexAttribPointer(startAttr, 2, gl.FLOAT, false, 8, 0);
        }
        const endAttr = gl.getAttribLocation(this.lineProgram!, 'aEnd');
        if (endAttr > -1) {
            gl.enableVertexAttribArray(endAttr);
            // Offset by 4 vertices * 2 floats * 4 bytes = 32 bytes (one sample's quad)
            gl.vertexAttribPointer(endAttr, 2, gl.FLOAT, false, 8, 32);
        }

        // Draw with additive blending
        gl.enable(gl.BLEND);
        gl.blendFunc(gl.SRC_ALPHA, gl.ONE);

        gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, this.elementIndexBuf);
        const nElements = (nSamples - 1) * 6;
        gl.drawElements(gl.TRIANGLES, nElements, gl.UNSIGNED_SHORT, 0);

        gl.disable(gl.BLEND);

        // Clean up attribs
        if (idxAttr > -1) gl.disableVertexAttribArray(idxAttr);
        if (startAttr > -1) gl.disableVertexAttribArray(startAttr);
        if (endAttr > -1) gl.disableVertexAttribArray(endAttr);

        gl.bindBuffer(gl.ARRAY_BUFFER, null);
        gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, null);
        gl.useProgram(null);
    }

    private drawTexture(
        texture: WebGLTexture,
        size: number,
        shader: WebGLProgram,
        alpha: number,
        gamma: number,
    ): void {
        const gl = this.gl!;
        gl.useProgram(shader);

        gl.bindBuffer(gl.ARRAY_BUFFER, this.outQuadBuf);
        const posAttr = gl.getAttribLocation(shader, 'aPos');
        if (posAttr > -1) {
            gl.enableVertexAttribArray(posAttr);
            gl.vertexAttribPointer(posAttr, 2, gl.SHORT, false, 8, 0);
        }
        const stAttr = gl.getAttribLocation(shader, 'aST');
        if (stAttr > -1) {
            gl.enableVertexAttribArray(stAttr);
            gl.vertexAttribPointer(stAttr, 2, gl.SHORT, false, 8, 4);
        }

        gl.activeTexture(gl.TEXTURE0);
        gl.bindTexture(gl.TEXTURE_2D, texture);
        this.setUniform1i(shader, 'uTexture', 0);
        this.setUniform1f(shader, 'uSize', size);
        this.setUniform1f(shader, 'uAlpha', alpha);
        this.setUniform1f(shader, 'uGamma', gamma);

        gl.enable(gl.BLEND);
        gl.blendFunc(gl.ONE, gl.SRC_ALPHA);
        gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
        gl.disable(gl.BLEND);

        if (posAttr > -1) gl.disableVertexAttribArray(posAttr);
        if (stAttr > -1) gl.disableVertexAttribArray(stAttr);

        gl.bindBuffer(gl.ARRAY_BUFFER, null);
        gl.bindTexture(gl.TEXTURE_2D, null);
        gl.useProgram(null);
    }

    // --- Utility methods ---

    private compileProgram(vsSrc: string, fsSrc: string): WebGLProgram | null {
        const gl = this.gl!;
        const vs = this.compileShader(gl.VERTEX_SHADER, vsSrc);
        const fs = this.compileShader(gl.FRAGMENT_SHADER, fsSrc);
        if (!vs || !fs) return null;

        const program = gl.createProgram()!;
        gl.attachShader(program, vs);
        gl.attachShader(program, fs);
        gl.linkProgram(program);
        gl.deleteShader(vs);
        gl.deleteShader(fs);

        if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
            console.error('Shader link error:', gl.getProgramInfoLog(program));
            gl.deleteProgram(program);
            return null;
        }
        return program;
    }

    private compileShader(type: number, source: string): WebGLShader | null {
        const gl = this.gl!;
        const shader = gl.createShader(type)!;
        gl.shaderSource(shader, source);
        gl.compileShader(shader);
        if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
            console.error('Shader compile error:', gl.getShaderInfoLog(shader));
            gl.deleteShader(shader);
            return null;
        }
        return shader;
    }

    private createTargetTexture(w: number, h: number): WebGLTexture {
        const gl = this.gl!;
        const tex = gl.createTexture()!;
        gl.bindTexture(gl.TEXTURE_2D, tex);
        gl.texImage2D(
            gl.TEXTURE_2D,
            0,
            gl.RGBA,
            w,
            h,
            0,
            gl.RGBA,
            gl.UNSIGNED_BYTE,
            null,
        );
        gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
        gl.texParameteri(
            gl.TEXTURE_2D,
            gl.TEXTURE_MIN_FILTER,
            gl.LINEAR_MIPMAP_LINEAR,
        );
        gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.MIRRORED_REPEAT);
        gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.MIRRORED_REPEAT);
        gl.generateMipmap(gl.TEXTURE_2D);
        gl.bindTexture(gl.TEXTURE_2D, null);
        return tex;
    }

    private activateTexture(texture: WebGLTexture): void {
        const gl = this.gl!;
        gl.bindRenderbuffer(gl.RENDERBUFFER, this.renderBuffer);
        gl.renderbufferStorage(
            gl.RENDERBUFFER,
            gl.DEPTH_COMPONENT16,
            FB_RESOLUTION,
            FB_RESOLUTION,
        );
        gl.framebufferTexture2D(
            gl.FRAMEBUFFER,
            gl.COLOR_ATTACHMENT0,
            gl.TEXTURE_2D,
            texture,
            0,
        );
        gl.framebufferRenderbuffer(
            gl.FRAMEBUFFER,
            gl.DEPTH_ATTACHMENT,
            gl.RENDERBUFFER,
            this.renderBuffer,
        );
        gl.bindTexture(gl.TEXTURE_2D, null);
        gl.bindRenderbuffer(gl.RENDERBUFFER, null);
    }

    private setUniform1f(
        program: WebGLProgram,
        name: string,
        value: number,
    ): void {
        const loc = this.gl!.getUniformLocation(program, name);
        if (loc) this.gl!.uniform1f(loc, value);
    }

    private setUniform1i(
        program: WebGLProgram,
        name: string,
        value: number,
    ): void {
        const loc = this.gl!.getUniformLocation(program, name);
        if (loc) this.gl!.uniform1i(loc, value);
    }

    private setUniform4fv(
        program: WebGLProgram,
        name: string,
        value: number[],
    ): void {
        const loc = this.gl!.getUniformLocation(program, name);
        if (loc) this.gl!.uniform4fv(loc, value);
    }

    /** Release all GPU resources. */
    destroy(): void {
        this.destroyed = true;
        if (!this.gl) return;
        const gl = this.gl;

        // Release WebGL context (Chrome-specific optimization)
        const ext = gl.getExtension('WEBGL_lose_context');
        if (ext) ext.loseContext();

        this.gl = null;
    }
}
```

**Step 2: Verify TypeScript compiles**

```bash
yarn typecheck
```

Expected: no errors.

**Step 3: Commit**

```bash
git add src/renderer/app/webgl/scopeRenderer.ts
git commit -m "feat: add WebGLScopeRenderer class with bloom pipeline"
```

---

## Task 5: Create the 2D Overlay Drawing Function

**Files:**

- Modify: `src/renderer/app/oscilloscope.ts`

The WebGL canvas renders the waveform only. Reference lines, legends, and stats are drawn on a transparent Canvas2D overlay. Extract the existing non-waveform drawing code into a new function.

**Step 1: Add `drawScopeOverlay` function**

This extracts the reference lines, legend, and stats drawing from `drawOscilloscope`, leaving out the waveform stroke:

```ts
export const drawScopeOverlay = (
    canvas: HTMLCanvasElement,
    options: ScopeDrawOptions,
) => {
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const { range = [-5, 5], stats } = options;
    const [minVoltage, maxVoltage] = range;
    const w = canvas.width;
    const h = canvas.height;

    const styles = getComputedStyle(document.documentElement);
    const borderColor =
        styles.getPropertyValue('--border-subtle').trim() || '#222222';
    const mutedColor =
        styles.getPropertyValue('--text-muted').trim() || '#555555';

    // Clear with transparency (WebGL canvas shows through)
    ctx.clearRect(0, 0, w, h);

    const dpr = window.devicePixelRatio || 1;
    const legendWidth = 40 * dpr;
    const statsWidth = 140 * dpr;
    const waveformLeft = legendWidth;
    const waveformWidth = w - legendWidth - statsWidth;
    const waveformRight = waveformLeft + waveformWidth;

    const voltageRange = maxVoltage - minVoltage;
    const pixelsPerVolt = h / voltageRange;
    const zeroY = h - (0 - minVoltage) * pixelsPerVolt;

    // Reference lines
    ctx.strokeStyle = borderColor;
    ctx.lineWidth = 1;
    ctx.setLineDash([]);
    if (minVoltage <= 0 && maxVoltage >= 0) {
        ctx.beginPath();
        ctx.moveTo(waveformLeft, zeroY);
        ctx.lineTo(waveformRight, zeroY);
        ctx.stroke();
    }
    ctx.setLineDash([4 * dpr, 4 * dpr]);
    ctx.beginPath();
    ctx.moveTo(waveformLeft, 0);
    ctx.lineTo(waveformRight, 0);
    ctx.stroke();
    ctx.beginPath();
    ctx.moveTo(waveformLeft, h);
    ctx.lineTo(waveformRight, h);
    ctx.stroke();
    ctx.setLineDash([]);

    // Legend
    ctx.fillStyle = mutedColor;
    ctx.font = `${10 * dpr}px "Fira Code", monospace`;
    ctx.textAlign = 'right';
    ctx.textBaseline = 'middle';
    const legendX = legendWidth - 4 * dpr;
    const textVerticalOffset = 10 * dpr;
    ctx.fillText(`${maxVoltage.toFixed(1)}v`, legendX, textVerticalOffset);
    if (minVoltage <= 0 && maxVoltage >= 0) {
        ctx.fillText('0v', legendX, zeroY);
    }
    ctx.fillText(`${minVoltage.toFixed(1)}v`, legendX, h - textVerticalOffset);

    // Stats
    if (stats) {
        ctx.textAlign = 'left';
        const statsX = waveformRight + 8 * dpr;
        const lineHeight = 14 * dpr;
        ctx.fillText(
            `min: ${stats.min.toFixed(2)}v`,
            statsX,
            h / 2 - lineHeight,
        );
        ctx.fillText(`max: ${stats.max.toFixed(2)}v`, statsX, h / 2);
        ctx.fillText(
            `p-p: ${stats.peakToPeak.toFixed(2)}v`,
            statsX,
            h / 2 + lineHeight,
        );
    }
};
```

**Step 2: Keep `drawOscilloscope` as a Canvas2D fallback**

Do not delete `drawOscilloscope` — it serves as a fallback when WebGL is not available. Mark it with a comment:

```ts
/** @deprecated Use WebGLScopeRenderer for primary rendering. Retained as Canvas2D fallback. */
export const drawOscilloscope = ( ... ) => { ... };
```

**Step 3: Commit**

```bash
git add src/renderer/app/oscilloscope.ts
git commit -m "feat: extract drawScopeOverlay for WebGL scope 2D chrome layer"
```

---

## Task 6: Update Scope View Zones for Dual-Canvas Layout

**Files:**

- Modify: `src/renderer/components/monaco/scopeViewZones.ts`
- Modify: `src/renderer/App.css`

**Step 1: Create dual-canvas structure in view zone containers**

Each scope view zone gets two canvases stacked via CSS:

- Bottom: WebGL canvas for the waveform (`<canvas data-scope-layer="webgl">`)
- Top: 2D canvas for the overlay chrome (`<canvas data-scope-layer="overlay">`)

In `scopeViewZones.ts`, modify the canvas creation inside the `zones = views.map(...)` block:

```ts
const container = document.createElement('div');
container.className = 'scope-view-zone';
container.style.height = `${scopeHeight}px`;
container.style.width = '100%';
container.style.display = 'flex';
container.style.position = 'relative'; // NEW: for absolute positioning of layers

// WebGL canvas (bottom layer)
const glCanvas = document.createElement('canvas');
glCanvas.className = 'scope-canvas-webgl';
glCanvas.style.position = 'absolute';
glCanvas.style.left = '0';
glCanvas.style.top = '0';
glCanvas.style.width = '100%';
glCanvas.style.height = `${scopeHeight}px`;
glCanvas.dataset.scopeLayer = 'webgl';
glCanvas.dataset.scopeKey = view.key;
glCanvas.dataset.scopeRangeMin = String(view.range[0]);
glCanvas.dataset.scopeRangeMax = String(view.range[1]);
glCanvas.dataset.scopeChannelKeys = JSON.stringify(view.channelKeys);

// 2D overlay canvas (top layer)
const overlayCanvas = document.createElement('canvas');
overlayCanvas.className = 'scope-canvas-overlay';
overlayCanvas.style.position = 'absolute';
overlayCanvas.style.left = '0';
overlayCanvas.style.top = '0';
overlayCanvas.style.width = '100%';
overlayCanvas.style.height = `${scopeHeight}px`;
overlayCanvas.style.pointerEvents = 'none';
overlayCanvas.dataset.scopeLayer = 'overlay';

// Set pixel dimensions
const pixelWidth = Math.max(1, Math.floor(layoutInfo.contentWidth * dpr));
const pixelHeight = Math.floor(scopeHeight * dpr);
glCanvas.width = pixelWidth;
glCanvas.height = pixelHeight;
overlayCanvas.width = pixelWidth;
overlayCanvas.height = pixelHeight;

container.appendChild(glCanvas);
container.appendChild(overlayCanvas);
```

Update canvas registration to pass both canvases (or modify the callback signature to include a `type` discriminator).

**Step 2: Update App.css**

```css
.scope-view-zone {
    position: relative;
}

.scope-view-zone canvas {
    display: block;
}

.scope-canvas-webgl {
    z-index: 0;
}

.scope-canvas-overlay {
    z-index: 1;
    pointer-events: none;
}
```

**Step 3: Update resize handler**

The `resizeCanvases` callback needs to resize both canvases:

```ts
const resizeCanvases = () => {
    const info = editor.getLayoutInfo();
    const nextDpr =
        typeof window !== 'undefined' ? window.devicePixelRatio || 1 : 1;
    // Resize all scope canvases (both webgl and overlay)
    const allCanvases = document.querySelectorAll('.scope-view-zone canvas');
    allCanvases.forEach((canvas) => {
        const c = canvas as HTMLCanvasElement;
        c.width = Math.max(1, Math.floor(info.contentWidth * nextDpr));
        c.height = Math.floor(scopeHeight * nextDpr);
    });
};
```

**Step 4: Commit**

```bash
git add src/renderer/components/monaco/scopeViewZones.ts src/renderer/App.css
git commit -m "feat: dual-canvas layout for WebGL scope (webgl + 2d overlay)"
```

---

## Task 7: Wire WebGL Renderer into App.tsx Polling Loop

**Files:**

- Modify: `src/renderer/App.tsx`

This is the integration task. The RAF loop currently calls `drawOscilloscope()` for each scope canvas. We replace this with:

1. `WebGLScopeRenderer.draw()` on the WebGL canvas
2. `drawScopeOverlay()` on the overlay canvas

**Step 1: Import the new renderer**

```ts
import { WebGLScopeRenderer } from './app/webgl/scopeRenderer';
import { drawScopeOverlay } from './app/oscilloscope';
```

**Step 2: Add a renderer instance map**

Alongside `scopeCanvasMapRef`, add a ref that maps scope keys to `WebGLScopeRenderer` instances:

```ts
const glRendererMapRef = useRef(new Map<string, WebGLScopeRenderer>());
```

**Step 3: Update canvas registration**

When a scope canvas is registered (`onRegisterScopeCanvas`), also create a `WebGLScopeRenderer` for the WebGL canvas:

```ts
const onRegisterScopeCanvas = useCallback(
    (key: string, glCanvas: HTMLCanvasElement) => {
        scopeCanvasMapRef.current.set(key, glCanvas);
        // Create WebGL renderer for this scope
        const renderer = new WebGLScopeRenderer(glCanvas);
        glRendererMapRef.current.set(key, renderer);
    },
    [],
);

const onUnregisterScopeCanvas = useCallback((key: string) => {
    scopeCanvasMapRef.current.delete(key);
    const renderer = glRendererMapRef.current.get(key);
    if (renderer) {
        renderer.destroy();
        glRendererMapRef.current.delete(key);
    }
}, []);
```

**Step 4: Update the draw call in the RAF loop**

Replace the `drawOscilloscope(channels, canvas, ...)` call with:

```ts
// Find the WebGL renderer for this scope
const renderer = glRendererMapRef.current.get(scopeKey);
if (renderer) {
    renderer.draw(channels, readOffsets, [rangeMin, rangeMax]);
}

// Find the overlay canvas (sibling of the WebGL canvas)
const overlayCanvas = canvas.parentElement?.querySelector(
    'canvas[data-scope-layer="overlay"]',
) as HTMLCanvasElement | null;
if (overlayCanvas) {
    drawScopeOverlay(overlayCanvas, {
        range: [rangeMin, rangeMax],
        stats: {
            min: globalMin,
            max: globalMax,
            peakToPeak: globalMax - globalMin,
            readOffset: readOffsets,
        },
    });
}
```

**Step 5: Clean up renderers on unmount**

In the cleanup function of the effect that creates the RAF loop, destroy all renderers:

```ts
return () => {
    cancelled = true;
    glRendererMapRef.current.forEach((renderer) => renderer.destroy());
    glRendererMapRef.current.clear();
};
```

**Step 6: Run typecheck**

```bash
yarn typecheck
```

Expected: no errors.

**Step 7: Run the app and verify the WebGL scopes render**

```bash
yarn start
```

- Write a simple patch: `$sine('C4').scope().out()`
- Verify the oscilloscope shows a glowing green waveform with gaussian beam effect
- Verify reference lines, voltage labels, and stats render on the overlay

**Step 8: Commit**

```bash
git add src/renderer/App.tsx
git commit -m "feat: wire WebGLScopeRenderer into App RAF loop with 2D overlay"
```

---

## Task 8: Add Scope Render Config to App Settings

**Files:**

- Modify: `src/shared/ipcTypes.ts`
- Modify: `src/main/main.ts`
- Modify: `src/renderer/components/EditorSettingsTab.tsx`
- Modify: `src/renderer/App.tsx`

**Step 1: Add ScopeRenderConfig to AppConfig**

In `src/shared/ipcTypes.ts`, add:

```ts
export interface ScopeRenderConfig {
    lineSize?: number; // 0.001 – 0.05, default 0.012
    intensity?: number; // 0.1 – 5.0, default 1.0
    bloom?: boolean; // default true
    bloomAlpha?: number; // 0.0 – 1.0, default 0.5
}
```

Add to `AppConfig`:

```ts
scopeRenderer?: ScopeRenderConfig;
```

**Step 2: Add Zod schema in main.ts**

```ts
scopeRenderer: z
    .object({
        lineSize: z.number().min(0.001).max(0.05).optional(),
        intensity: z.number().min(0.1).max(5.0).optional(),
        bloom: z.boolean().optional(),
        bloomAlpha: z.number().min(0).max(1).optional(),
    })
    .optional(),
```

**Step 3: Add settings controls**

Add a "Scope Renderer" section to `EditorSettingsTab.tsx` with controls for:

- Beam width (range slider)
- Intensity (range slider)
- Bloom enable (checkbox)
- Bloom intensity (range slider)

Follow the exact pattern used by existing settings controls in the file.

**Step 4: Wire config into the draw call**

In `App.tsx`, read `scopeRenderer` config from app config state and pass it to `renderer.draw()`:

```ts
renderer.draw(channels, readOffsets, [rangeMin, rangeMax], {
    lineSize: config.scopeRenderer?.lineSize,
    intensity: config.scopeRenderer?.intensity,
    bloom: config.scopeRenderer?.bloom,
    bloomAlpha: config.scopeRenderer?.bloomAlpha,
});
```

**Step 5: Run typecheck and verify**

```bash
yarn typecheck
yarn start
```

Verify settings controls update the scope rendering in real time.

**Step 6: Commit**

```bash
git add src/shared/ipcTypes.ts src/main/main.ts src/renderer/components/EditorSettingsTab.tsx src/renderer/App.tsx
git commit -m "feat: add configurable scope renderer settings (beam width, bloom, intensity)"
```

---

## Task 9: Add Sinc Interpolation Toggle

**Files:**

- Modify: `src/renderer/app/webgl/scopeRenderer.ts`
- Modify: `src/renderer/app/webgl/bufferBuilder.ts`

**Step 1: Add interpolation to the draw pipeline**

In `scopeRenderer.ts`, optionally apply sinc interpolation before building the line buffer:

```ts
import { sincInterpolate } from './bufferBuilder';

// In drawLineChannel:
let processedSamples = samples;
if (cfg.interpolate) {
    processedSamples = sincInterpolate(samples, cfg.interpolationFactor ?? 4);
    // Adjust readOffset proportionally
    readOffset *= cfg.interpolationFactor ?? 4;
}
const lineData = buildLineBuffer(processedSamples, readOffset, range);
```

**Step 2: Add config options**

Add to `ScopeRenderConfig`:

```ts
interpolate?: boolean;        // default: true
interpolationFactor?: number; // 2 or 4, default: 4
```

**Step 3: Add settings control**

Add an "Interpolation" checkbox to the scope renderer settings section.

**Step 4: Test with a square wave**

Use `$square('C4').scope().out()` and toggle interpolation on/off. With interpolation on, the square wave's rising/falling edges should show Gibbs phenomenon ringing (physically correct for bandlimited signals). With interpolation off, edges should be sharp staircases.

**Step 5: Commit**

```bash
git add src/renderer/app/webgl/scopeRenderer.ts src/renderer/app/webgl/bufferBuilder.ts
git commit -m "feat: add optional sinc interpolation to WebGL scope renderer"
```

---

## Task 10: Final Integration Test and Cleanup

**Files:**

- All modified files
- Test files

**Step 1: Run unit tests**

```bash
yarn test:unit
```

Expected: all pass.

**Step 2: Run Rust tests**

```bash
yarn test:rust
```

Expected: all pass (no Rust changes in this plan).

**Step 3: Run typecheck**

```bash
yarn typecheck
```

Expected: no errors.

**Step 4: Run lint**

```bash
yarn lint
```

Expected: no errors.

**Step 5: Manual visual verification**

Test the following patches and verify visual quality:

```js
// Simple sine — smooth glowing trace
$sine('C4').scope().out();

// Square wave with interpolation — ringing at edges
$square('C4').scope().out();

// Multi-channel — different colored traces
$sine(['C4', 'E4', 'G4']).scope().out();

// High frequency — verify no aliasing artifacts
$sine('C7').scope({ msPerFrame: 10 }).out();

// Triggered scope — stable waveform
$sine('C4').scope({ triggerThreshold: 0 }).out();
```

**Step 6: Run E2E tests**

```bash
yarn test:e2e
```

Expected: pass (may need to update visual snapshots due to new rendering).

**Step 7: Update E2E visual snapshots if needed**

```bash
yarn test:e2e:update
```

**Step 8: Commit**

```bash
git add -A
git commit -m "test: verify WebGL scope renderer integration, update snapshots"
```

---

## Summary of Changes

| Component                 | Before                     | After                                                               |
| ------------------------- | -------------------------- | ------------------------------------------------------------------- |
| Scope waveform rendering  | Canvas2D `moveTo`/`lineTo` | WebGL gaussian beam quads with erf-based intensity                  |
| Visual quality            | 1.5px flat line            | Physically-accurate gaussian glow with bloom                        |
| Post-processing           | None                       | Optional bloom (downscale + separable gaussian blur + composite)    |
| Multi-channel colors      | All same accent color      | Per-channel distinct colors from palette                            |
| Interpolation             | None (raw staircase)       | Optional 4x windowed sinc interpolation                             |
| Gamma correction          | None                       | sRGB gamma in output shader                                         |
| Canvas structure          | Single Canvas2D            | Dual-layer: WebGL (waveform) + Canvas2D (chrome/labels)             |
| UI chrome (legends/stats) | Part of waveform canvas    | Separate transparent overlay canvas                                 |
| Fallback                  | N/A                        | Original `drawOscilloscope` retained for non-WebGL environments     |
| Configuration             | None                       | Beam width, intensity, bloom, interpolation — all user-configurable |
| Data pipeline (Rust)      | Unchanged                  | Unchanged                                                           |

## Risk Considerations

1. **WebGL context limits** — Chrome allows ~16 active WebGL contexts. With 5+ scopes visible, we approach this limit. If this becomes an issue, future work could use a single offscreen WebGL canvas with framebuffer blit to each visible scope canvas.

2. **Performance with many scopes** — Each scope does 1 draw call per channel + bloom passes. With 10 scopes x 3 channels each, that's 30+ draw calls per frame. Monitor GPU utilization and consider disabling bloom for scopes beyond a threshold.

3. **Electron WebGL compatibility** — Electron's Chromium should have full WebGL 1.0 support. The Canvas2D fallback (`drawOscilloscope`) ensures degraded-but-functional rendering if WebGL initialization fails.

4. **Visual snapshot tests** — E2E visual tests will need snapshot updates since the scope appearance changes significantly.
