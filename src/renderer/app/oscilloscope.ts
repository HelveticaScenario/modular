import type { ScopeItem, ScopeStats } from '@modular/core';

export const scopeKeyFromSubscription = (subscription: ScopeItem) => {
    const { moduleId, portName } = subscription;
    return `:module:${moduleId}:${portName}`;
};

export interface ScopeDrawOptions {
    scale?: number;
    stats?: ScopeStats;
}

export const drawOscilloscope = (
    channels: Float32Array[],
    canvas: HTMLCanvasElement,
    options: ScopeDrawOptions = {},
) => {
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const { scale = 5, stats } = options;
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

    const dpr = window.devicePixelRatio || 1;
    const legendWidth = 40 * dpr; // Reserve space for legend on left
    const statsWidth = 140 * dpr; // Reserve space for stats on right
    const waveformLeft = legendWidth;
    const waveformWidth = w - legendWidth - statsWidth;
    const waveformRight = waveformLeft + waveformWidth;

    const midY = h / 2;
    const maxAbsAmplitude = scale;
    const pixelsPerUnit = h / 2 / maxAbsAmplitude;

    // Draw reference lines
    ctx.strokeStyle = borderColor;
    ctx.lineWidth = 1;
    ctx.setLineDash([]);

    // Center line (0V) - solid
    ctx.beginPath();
    ctx.moveTo(waveformLeft, midY);
    ctx.lineTo(waveformRight, midY);
    ctx.stroke();

    // +scale and -scale lines - dashed
    ctx.setLineDash([4 * dpr, 4 * dpr]);
    const topY = midY - scale * pixelsPerUnit;
    const bottomY = midY + scale * pixelsPerUnit;

    ctx.beginPath();
    ctx.moveTo(waveformLeft, topY);
    ctx.lineTo(waveformRight, topY);
    ctx.stroke();

    ctx.beginPath();
    ctx.moveTo(waveformLeft, bottomY);
    ctx.lineTo(waveformRight, bottomY);
    ctx.stroke();

    ctx.setLineDash([]);

    // Draw legend on left
    ctx.fillStyle = mutedColor;
    ctx.font = `${10 * dpr}px "Fira Code", monospace`;
    ctx.textAlign = 'right';
    ctx.textBaseline = 'middle';

    const legendX = legendWidth - 4 * dpr;
    ctx.fillText(`+${scale}v`, legendX, topY);
    ctx.fillText('0v', legendX, midY);
    ctx.fillText(`-${scale}v`, legendX, bottomY);

    // Draw stats on right
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

    // Handle empty data
    if (
        !channels ||
        channels.length === 0 ||
        channels.every((ch) => ch.length === 0)
    ) {
        ctx.fillStyle = mutedColor;
        ctx.font = `${13 * dpr}px "Fira Code", monospace`;
        ctx.textAlign = 'center';
        ctx.textBaseline = 'middle';
        ctx.fillText('~', waveformLeft + waveformWidth / 2, midY);
        return;
    }

    const windowSize = 1024;

    // Draw all channels (same color, overlaid)
    ctx.strokeStyle = accentColor;
    ctx.lineWidth = 1.5 * dpr;

    for (const data of channels) {
        if (!data || data.length < 2) continue;

        const sampleCount = Math.min(windowSize, data.length);
        const stepX = waveformWidth / (windowSize - 1);

        ctx.beginPath();

        for (let i = 0; i < sampleCount; i++) {
            const x = waveformLeft + stepX * i;
            const rawSample = data[i];
            const s = Math.max(
                -maxAbsAmplitude,
                Math.min(maxAbsAmplitude, rawSample),
            );
            const y = midY - s * pixelsPerUnit;

            if (i === 0) {
                ctx.moveTo(x, y);
            } else {
                ctx.lineTo(x, y);
            }
        }

        ctx.stroke();
    }
};
