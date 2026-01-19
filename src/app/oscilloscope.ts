import type { ScopeItem } from '@modular/core';

export const scopeKeyFromSubscription = (subscription: ScopeItem) => {
    const { moduleId, portName } = subscription;
    return `:module:${moduleId}:${portName}`;
};

export const drawOscilloscope = (
    data: Float32Array,
    canvas: HTMLCanvasElement,
) => {
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const w = canvas.width;
    const h = canvas.height;

    // Get theme colors from CSS variables
    const styles = getComputedStyle(document.documentElement);
    const bgColor = styles.getPropertyValue('--bg-primary').trim() || '#0a0a0a';
    const borderColor =
        styles.getPropertyValue('--border-subtle').trim() || '#222222';
    const mutedColor =
        styles.getPropertyValue('--text-muted').trim() || '#555555';
    const accentColor =
        styles.getPropertyValue('--accent-primary').trim() || '#4ec9b0';

    ctx.fillStyle = bgColor;
    ctx.fillRect(0, 0, w, h);

    const midY = h / 2;
    const maxAbsAmplitude = 10;
    const pixelsPerUnit = h / 2 / maxAbsAmplitude;

    // Subtle grid line
    ctx.strokeStyle = borderColor;
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.moveTo(0, midY);
    ctx.lineTo(w, midY);
    ctx.stroke();

    if (!data || data.length === 0) {
        ctx.fillStyle = mutedColor;
        ctx.font = '13px "Fira Code", monospace';
        ctx.textAlign = 'center';
        ctx.fillText('~', w / 2, midY);
        return;
    }

    const windowSize = 1024;
    const startIndex = 0;
    const sampleCount = Math.min(windowSize, data.length);

    if (sampleCount < 2) {
        return;
    }

    // Accent color for waveform
    ctx.strokeStyle = accentColor;
    ctx.lineWidth = 1.5;
    ctx.beginPath();

    const stepX = w / (windowSize - 1);

    for (let i = 0; i < sampleCount; i++) {
        const x = stepX * i;
        const rawSample = data[startIndex + i];
        const s = Math.max(-maxAbsAmplitude, Math.min(maxAbsAmplitude, rawSample));
        const y = midY - s * pixelsPerUnit;

        if (i === 0) {
            ctx.moveTo(x, y);
        } else {
            ctx.lineTo(x, y);
        }
    }

    ctx.stroke();
};