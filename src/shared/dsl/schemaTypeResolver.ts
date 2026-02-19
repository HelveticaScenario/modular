/**
 * Shared JSON Schema → TypeScript type expression resolver.
 *
 * Used by both the main-process type generation (typescriptLibGen)
 * and the renderer help window to display human-readable param types.
 */

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export type JSONSchema = any;

function isValidIdentifier(name: string): boolean {
    return /^[$A-Z_][0-9A-Z_$]*$/i.test(name);
}

function renderPropertyKey(name: string): string {
    return isValidIdentifier(name) ? name : JSON.stringify(name);
}

/**
 * Resolve a `$ref` pointer against the root schema's `$defs`.
 * Returns the well-known Signal/Poly<Signal>/Mono<Signal> sentinel strings
 * for the three signal types, otherwise the raw resolved sub-schema.
 */
export function resolveRef(
    ref: string,
    rootSchema: JSONSchema,
): JSONSchema | 'Signal' | 'Poly<Signal>' | 'Mono<Signal>' {
    if (ref === 'Signal') return 'Signal';

    const defsPrefix = '#/$defs/';
    if (!ref.startsWith(defsPrefix)) {
        throw new Error(`Unsupported $ref: ${ref}`);
    }

    const defName = ref.slice(defsPrefix.length);
    if (defName === 'Signal') return 'Signal';
    if (defName === 'PolySignal') return 'Poly<Signal>';
    if (defName === 'MonoSignal') return 'Mono<Signal>';

    const defs = rootSchema?.$defs;
    if (!defs || typeof defs !== 'object') {
        throw new Error(`Unresolved $ref: ${ref}`);
    }

    const resolved = defs[defName];
    if (!resolved) {
        throw new Error(`Unresolved $ref: ${ref}`);
    }

    if (resolved?.title === 'Signal') return 'Signal';
    if (resolved?.title === 'PolySignal') return 'Poly<Signal>';
    if (resolved?.title === 'MonoSignal') return 'Mono<Signal>';
    return resolved;
}

/**
 * Information about a single variant in an enum-style JSON Schema.
 */
export interface EnumVariantInfo {
    /** The JSON-serialized const value (e.g., `"vaVcf"`) */
    value: string;
    /** The raw const value */
    rawValue: unknown;
    /** Description from the Rust `///` doc comment on the variant, if any */
    description?: string;
}

/**
 * Extract enum variant information (including descriptions) from a JSON Schema node.
 *
 * Returns an array of `EnumVariantInfo` if the schema represents an enum,
 * or `null` if it does not.  Follows `$ref` pointers, handling the
 * Signal/Poly/Mono sentinel types gracefully (returns `null` for those).
 */
export function getEnumVariants(
    schema: JSONSchema,
    rootSchema: JSONSchema,
): EnumVariantInfo[] | null {
    if (
        schema === null ||
        schema === undefined ||
        typeof schema === 'boolean'
    ) {
        return null;
    }

    // Follow $ref
    if (schema.$ref) {
        const resolved = resolveRef(String(schema.$ref), rootSchema);
        // Sentinel strings mean it's a signal type, not an enum
        if (typeof resolved === 'string') return null;
        return getEnumVariants(resolved, rootSchema);
    }

    // oneOf / anyOf with all-const variants (schemars output for documented enums)
    const variants = schema.oneOf || schema.anyOf;
    if (Array.isArray(variants)) {
        const isEnum = variants.every((v: JSONSchema) => v.const !== undefined);
        if (isEnum) {
            return variants.map((v: JSONSchema) => ({
                value: JSON.stringify(v.const),
                rawValue: v.const,
                description: v.description as string | undefined,
            }));
        }
        return null;
    }

    // Bare enum array (schemars output for undocumented enums)
    if (Array.isArray(schema.enum) && schema.enum.length > 0) {
        return schema.enum.map((v: unknown) => ({
            value: JSON.stringify(v),
            rawValue: v,
            description: undefined,
        }));
    }

    return null;
}

/**
 * Convert a JSON Schema node into a human-readable TypeScript type expression.
 *
 * Handles: `$ref` (Signal/PolySignal/MonoSignal + arbitrary defs), `oneOf`/`anyOf`
 * enum patterns, `allOf`, nullable type arrays, primitive types, object types,
 * array/tuple types, and `enum` schemas.
 */
export function schemaToTypeExpr(
    schema: JSONSchema,
    rootSchema: JSONSchema,
): string {
    if (schema === null || schema === undefined) {
        throw new Error('Unsupported schema: null/undefined');
    }
    if (typeof schema === 'boolean') {
        throw new Error('Unsupported schema: boolean schema');
    }

    // Handle oneOf/anyOf - check if all variants resolve to Signal
    if (schema.oneOf || schema.anyOf) {
        const variants = schema.oneOf || schema.anyOf;
        if (Array.isArray(variants)) {
            // Check if this is an enum (all variants have 'const')
            const isEnum = variants.every(
                (v: JSONSchema) => v.const !== undefined,
            );
            if (isEnum) {
                return variants
                    .map((v: JSONSchema) => JSON.stringify(v.const))
                    .join(' | ');
            }

            const types = variants.map((v: JSONSchema) => {
                try {
                    return schemaToTypeExpr(v, rootSchema);
                } catch {
                    return 'any';
                }
            });
            // If all variants are Signal, return Signal
            if (types.every((t) => t === 'Signal')) {
                return 'Poly<Signal>';
            }
            // If it's a mix but includes Signal[], treat as Signal (for Poly<Signal>)
            if (types.includes('Signal') && types.includes('Signal[]')) {
                return 'Poly<Signal>';
            }

            // Otherwise, return the union of all variant types
            return types.length > 0 ? types.join(' | ') : 'any';
        }
        return 'any';
    }
    if (schema.allOf) {
        return 'any';
    }
    if (Array.isArray(schema.type)) {
        // Handle union type arrays like ["integer", "null"] or ["string", "null"]
        const types = schema.type as string[];
        const nonNullTypes = types.filter((t: string) => t !== 'null');
        if (nonNullTypes.length === 1) {
            // This is a nullable type, treat it as the non-null type (optional in TS)
            const singleType = nonNullTypes[0];
            if (singleType === 'integer' || singleType === 'number')
                return 'number';
            if (singleType === 'string') return 'string';
            if (singleType === 'boolean') return 'boolean';
        }
        // Fall back to union of all non-null types
        const mapped = nonNullTypes.map((t: string) => {
            if (t === 'integer' || t === 'number') return 'number';
            if (t === 'string') return 'string';
            if (t === 'boolean') return 'boolean';
            return 'any';
        });
        return mapped.length > 0 ? mapped.join(' | ') : 'any';
    }

    if (schema.$ref) {
        const resolved = resolveRef(String(schema.$ref), rootSchema);
        if (resolved === 'Signal') return 'Signal';
        if (resolved === 'Poly<Signal>') return 'Poly<Signal>';
        if (resolved === 'Mono<Signal>') return 'Mono<Signal>';
        return schemaToTypeExpr(resolved, rootSchema);
    }

    if (schema.enum) {
        if (!Array.isArray(schema.enum) || schema.enum.length === 0) {
            throw new Error('Unsupported enum schema');
        }
        return schema.enum
            .map((v: JSONSchema) => JSON.stringify(v))
            .join(' | ');
    }

    const type = schema.type;

    if (type === 'integer' || type === 'number') return 'number';
    if (type === 'string') return 'string';
    if (type === 'boolean') return 'boolean';

    const looksLikeObject =
        type === 'object' ||
        (!!schema.properties && typeof schema.properties === 'object');
    if (looksLikeObject) {
        const props = schema.properties;
        if (!props || typeof props !== 'object') return '{}';

        const requiredSet = new Set<string>(
            Array.isArray(schema.required) ? schema.required : [],
        );
        const entries = Object.entries(props as Record<string, JSONSchema>);
        if (entries.length === 0) return '{}';

        const parts: string[] = [];
        for (const [propName, propSchema] of entries) {
            const optional = requiredSet.has(propName) ? '' : '?';
            parts.push(
                `${renderPropertyKey(propName)}${optional}: ${schemaToTypeExpr(propSchema, rootSchema)}`,
            );
        }
        return `{ ${parts.join('; ')} }`;
    }

    if (type === 'array') {
        if (Array.isArray(schema.prefixItems)) {
            const items = schema.prefixItems as JSONSchema[];
            const tuple = items
                .map((s: JSONSchema) => schemaToTypeExpr(s, rootSchema))
                .join(', ');
            return `[${tuple}]`;
        }
        if (schema.items) {
            return `${schemaToTypeExpr(schema.items, rootSchema)}[]`;
        }
        throw new Error('Unsupported array schema: missing items/prefixItems');
    }

    if (type === undefined) {
        // If there's a $ref we didn't catch, or other structural hints, try to handle
        if (schema.$ref) {
            const resolved = resolveRef(String(schema.$ref), rootSchema);
            if (resolved === 'Signal') return 'Signal';
            if (resolved === 'Poly<Signal>') return 'Poly<Signal>';
            if (resolved === 'Mono<Signal>') return 'Mono<Signal>';
            return schemaToTypeExpr(resolved, rootSchema);
        }
        // Schema with only 'const' (used in tagged unions)
        if (schema.const !== undefined) {
            return JSON.stringify(schema.const);
        }
        console.error(
            'Schema with missing type:',
            JSON.stringify(schema, null, 2),
        );
        throw new Error('Unsupported schema: missing type');
    }

    throw new Error(`Unsupported scalar type: ${type}`);
}
