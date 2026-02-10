export type ScopeCallEnd = {
    startIndex: number;
    endIndex: number;
    endLine: number;
    endLineText: string;
};
export declare function findScopeCallEndLines(code: string): ScopeCallEnd[];
