import type { ScopeItem, ScopeStats } from '@modular/core';
export declare const scopeKeyFromSubscription: (subscription: ScopeItem) => string;
export interface ScopeDrawOptions {
    range?: [number, number];
    stats?: ScopeStats;
}
export declare const drawOscilloscope: (channels: Float32Array[], canvas: HTMLCanvasElement, options?: ScopeDrawOptions) => void;
