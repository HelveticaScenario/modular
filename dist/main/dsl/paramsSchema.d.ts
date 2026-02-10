import { ModuleSchema } from "@modular/core";
/**
 * A small, pragmatic subset of JSON Schema (as emitted by `schemars`).
 * We intentionally keep this minimal and treat unknown shapes as `unknown`.
 */
export type JsonSchema = {
    $ref?: string;
    $schema?: string;
    title?: string;
    description?: string;
    type?: "object" | "string" | "number" | "integer" | "boolean" | "array" | "null";
    properties?: Record<string, JsonSchema>;
    required?: string[];
    oneOf?: JsonSchema[];
    anyOf?: JsonSchema[];
    allOf?: JsonSchema[];
    enum?: unknown[];
    const?: unknown;
    items?: JsonSchema | JsonSchema[];
    definitions?: Record<string, JsonSchema>;
    [k: string]: any;
};
export type ParamKind = "signal" | "polySignal" | "signalArray" | "number" | "string" | "boolean" | "unknown";
export type ParamDescriptor = {
    name: string;
    kind: ParamKind;
    description?: string;
    optional: boolean;
    enumValues?: string[];
};
export type ProcessedModuleSchema = ModuleSchema & {
    params: ParamDescriptor[];
    paramsByName: Record<string, ParamDescriptor>;
};
export declare function processModuleSchema(schema: ModuleSchema): ProcessedModuleSchema;
export declare function processSchemas(schemas: ModuleSchema[]): ProcessedModuleSchema[];
