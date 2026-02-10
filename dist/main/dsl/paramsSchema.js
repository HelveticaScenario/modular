"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.processModuleSchema = processModuleSchema;
exports.processSchemas = processSchemas;
function isPlainObject(value) {
    return typeof value === "object" && value !== null && !Array.isArray(value);
}
function asJsonSchema(value) {
    if (typeof value === "boolean") {
        return { const: value };
    }
    if (!isPlainObject(value))
        return null;
    return value;
}
function getByJsonPointer(root, pointer) {
    // Only support local refs.
    if (!pointer.startsWith("#/"))
        return null;
    const parts = pointer
        .slice(2)
        .split("/")
        .map((p) => p.replace(/~1/g, "/").replace(/~0/g, "~"));
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    let cur = root;
    for (const part of parts) {
        if (!isPlainObject(cur))
            return null;
        cur = cur[part];
        if (cur === undefined)
            return null;
    }
    return asJsonSchema(cur);
}
function deref(root, schema, seen = new Set()) {
    const ref = schema.$ref;
    if (!ref)
        return schema;
    if (seen.has(ref))
        return schema;
    const resolved = getByJsonPointer(root, ref);
    if (!resolved)
        return schema;
    seen.add(ref);
    return deref(root, resolved, seen);
}
function mergeObjectSchemas(a, b) {
    const out = { ...a, ...b };
    if (a.properties || b.properties) {
        out.properties = { ...(a.properties ?? {}), ...(b.properties ?? {}) };
    }
    if (a.required || b.required) {
        out.required = Array.from(new Set([...(a.required ?? []), ...(b.required ?? [])]));
    }
    if (!out.description)
        out.description = a.description ?? b.description;
    return out;
}
function resolveAndMerge(root, schema) {
    const resolved = deref(root, schema);
    if (resolved.allOf && Array.isArray(resolved.allOf) && resolved.allOf.length > 0) {
        return resolved.allOf
            .map((s) => resolveAndMerge(root, s))
            .reduce((acc, cur) => mergeObjectSchemas(acc, cur), { ...resolved, allOf: undefined });
    }
    return resolved;
}
function extractTypeTag(schema) {
    if (typeof schema.const === "string")
        return schema.const;
    if (Array.isArray(schema.enum)) {
        const str = schema.enum.find((v) => typeof v === "string");
        return typeof str === "string" ? str : null;
    }
    return null;
}
/**
 * Detects if a schema is a Signal type.
 * Signal schema is: anyOf [number, string, SignalTaggedSchema]
 * where SignalTaggedSchema is oneOf [cable, disconnected]
 */
function isSignalParamSchema(root, schema) {
    const resolved = resolveAndMerge(root, schema);
    // Check for direct $ref to Signal
    if (schema.$ref === "#/$defs/Signal")
        return true;
    // Check for title indicating Signal
    if (resolved.title === "Signal")
        return true;
    // Check for the actual Signal schema structure:
    // anyOf: [number, string, SignalTaggedSchema ref]
    const union = resolved.oneOf ?? resolved.anyOf;
    if (!union || !Array.isArray(union))
        return false;
    let hasNumber = false;
    let hasString = false;
    let hasTaggedSchema = false;
    for (const branch of union) {
        const b = resolveAndMerge(root, branch);
        // Check for number type
        if (b.type === "number" || b.type === "integer") {
            hasNumber = true;
            continue;
        }
        // Check for string type
        if (b.type === "string") {
            hasString = true;
            continue;
        }
        // Check for SignalTaggedSchema (oneOf with cable/disconnected variants)
        const taggedUnion = b.oneOf ?? b.anyOf;
        if (taggedUnion && Array.isArray(taggedUnion)) {
            const tags = new Set();
            for (const taggedBranch of taggedUnion) {
                const tb = resolveAndMerge(root, taggedBranch);
                if (tb.type === "object" && tb.properties?.type) {
                    const tag = extractTypeTag(resolveAndMerge(root, tb.properties.type));
                    if (tag)
                        tags.add(tag);
                }
            }
            // SignalTaggedSchema has "cable" and "disconnected" variants
            if (tags.has("cable") && tags.has("disconnected")) {
                hasTaggedSchema = true;
            }
        }
    }
    // Signal schema should have at least number and the tagged schema
    // (string is optional for the parse_signal_string format)
    return hasNumber && hasTaggedSchema;
}
function isSignalArrayParamSchema(root, schema) {
    const resolved = resolveAndMerge(root, schema);
    if (resolved.type !== "array")
        return false;
    const items = resolved.items;
    if (!items)
        return false;
    if (Array.isArray(items)) {
        // Tuple validation; treat as signal array if all entries are signals.
        return items.every((it) => {
            const s = resolveAndMerge(root, it);
            return isSignalParamSchema(root, s);
        });
    }
    const itemSchema = resolveAndMerge(root, items);
    return isSignalParamSchema(root, itemSchema);
}
/**
 * Detects PolySignal schema: either a single Signal OR an array of Signals.
 * PolySignal in Rust serializes as anyOf with Signal and Signal[] variants.
 */
function isPolySignalParamSchema(root, schema) {
    // Check for direct $ref to PolySignal
    if (schema.$ref === "#/$defs/PolySignal")
        return true;
    const resolved = resolveAndMerge(root, schema);
    // Check for title indicating PolySignal
    if (resolved.title === "PolySignal")
        return true;
    // Check for anyOf pattern: [Signal, Signal[]]
    const union = resolved.oneOf ?? resolved.anyOf;
    if (!union || !Array.isArray(union) || union.length < 2)
        return false;
    let hasSignal = false;
    let hasSignalArray = false;
    for (const branch of union) {
        const b = resolveAndMerge(root, branch);
        if (isSignalParamSchema(root, b)) {
            hasSignal = true;
        }
        else if (isSignalArrayParamSchema(root, b)) {
            hasSignalArray = true;
        }
    }
    return hasSignal && hasSignalArray;
}
function extractStringEnum(schema) {
    const resolved = schema;
    // Standard enum format
    if (resolved.type === "string" && Array.isArray(resolved.enum) && resolved.enum.every((v) => typeof v === "string")) {
        return resolved.enum;
    }
    if (Array.isArray(resolved.enum) && resolved.enum.every((v) => typeof v === "string")) {
        return resolved.enum;
    }
    // oneOf with const values format (serde rename_all enums)
    const union = resolved.oneOf ?? resolved.anyOf;
    if (union && Array.isArray(union)) {
        const values = [];
        for (const branch of union) {
            if (branch.type === "string" && typeof branch.const === "string") {
                values.push(branch.const);
            }
        }
        if (values.length === union.length && values.length > 0) {
            return values;
        }
    }
    return undefined;
}
/**
 * Checks if a schema represents a string enum (oneOf with const string values)
 */
function isStringEnumSchema(schema) {
    const union = schema.oneOf ?? schema.anyOf;
    if (!union || !Array.isArray(union) || union.length === 0)
        return false;
    return union.every((branch) => branch.type === "string" && typeof branch.const === "string");
}
function inferKind(root, schema) {
    // Check polySignal first since it's a union of Signal | Signal[]
    if (isPolySignalParamSchema(root, schema))
        return "polySignal";
    if (isSignalParamSchema(root, schema))
        return "signal";
    if (isSignalArrayParamSchema(root, schema))
        return "signalArray";
    const resolved = resolveAndMerge(root, schema);
    switch (resolved.type) {
        case "number":
        case "integer":
            return "number";
        case "string":
            return "string";
        case "boolean":
            return "boolean";
        default:
            break;
    }
    // If schema forgot `type` but has string enum, treat as string.
    if (Array.isArray(resolved.enum) && resolved.enum.every((v) => typeof v === "string")) {
        return "string";
    }
    // Check for oneOf with const string values (serde rename_all enums)
    if (isStringEnumSchema(resolved)) {
        return "string";
    }
    return "unknown";
}
function processModuleSchema(schema) {
    const root = asJsonSchema(schema.paramsSchema) ?? {};
    const rootResolved = resolveAndMerge(root, root);
    const obj = rootResolved.type === "object" ? rootResolved : null;
    const properties = obj?.properties ?? {};
    const required = new Set(obj?.required ?? []);
    const params = Object.entries(properties).map(([name, s]) => {
        const resolved = resolveAndMerge(root, s);
        const enumValues = extractStringEnum(resolved);
        const inferedKind = inferKind(root, s);
        return {
            name,
            kind: inferedKind,
            description: resolved.description,
            optional: !required.has(name),
            enumValues,
        };
    });
    const paramsByName = {};
    for (const p of params)
        paramsByName[p.name] = p;
    return {
        ...schema,
        params,
        paramsByName,
    };
}
function processSchemas(schemas) {
    return schemas.map(processModuleSchema);
}
//# sourceMappingURL=paramsSchema.js.map