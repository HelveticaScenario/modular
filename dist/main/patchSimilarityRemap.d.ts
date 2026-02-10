import type { PatchGraph } from '../shared/ipcTypes';
export interface ReconcileOptions {
    matchThreshold?: number;
    ambiguityMargin?: number;
    debugLog?: (message: string) => void;
}
export interface ReconcileResult {
    appliedPatch: PatchGraph;
    moduleIdRemap: Record<string, string>;
}
export declare function reconcilePatchBySimilarity(desiredGraph: PatchGraph, currentGraph: PatchGraph | null, options?: ReconcileOptions): ReconcileResult;
