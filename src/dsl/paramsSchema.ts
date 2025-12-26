import type { ModuleSchema } from "../types/generated/ModuleSchema";

/**
 * A small, pragmatic subset of JSON Schema (as emitted by `schemars`).
 * We intentionally keep this minimal and treat unknown shapes as `unknown`.
 */
export type JsonSchema = {
  $ref?: string;
  $schema?: string;
  title?: string;
  description?: string;

  type?:
    | "object"
    | "string"
    | "number"
    | "integer"
    | "boolean"
    | "array"
    | "null";

  properties?: Record<string, JsonSchema>;
  required?: string[];

  oneOf?: JsonSchema[];
  anyOf?: JsonSchema[];
  allOf?: JsonSchema[];

  enum?: unknown[];
  const?: unknown;

  items?: JsonSchema | JsonSchema[];

  definitions?: Record<string, JsonSchema>;

  // Allow extra schemars/json-schema keywords without modeling them.
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  [k: string]: any;
};

export type ParamKind =
  | "signal"
  | "signalArray"
  | "number"
  | "string"
  | "boolean"
  | "unknown";

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

function isPlainObject(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function asJsonSchema(value: unknown): JsonSchema | null {
  if (typeof value === "boolean") {
    return { const: value };
  }
  if (!isPlainObject(value)) return null;
  return value as JsonSchema;
}

function getByJsonPointer(root: JsonSchema, pointer: string): JsonSchema | null {
  // Only support local refs.
  if (!pointer.startsWith("#/")) return null;
  const parts = pointer
    .slice(2)
    .split("/")
    .map((p) => p.replace(/~1/g, "/").replace(/~0/g, "~"));

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  let cur: any = root;
  for (const part of parts) {
    if (!isPlainObject(cur)) return null;
    cur = cur[part];
    if (cur === undefined) return null;
  }
  return asJsonSchema(cur);
}

function deref(root: JsonSchema, schema: JsonSchema, seen = new Set<string>()): JsonSchema {
  const ref = schema.$ref;
  if (!ref) return schema;
  if (seen.has(ref)) return schema;
  const resolved = getByJsonPointer(root, ref);
  if (!resolved) return schema;
  seen.add(ref);
  return deref(root, resolved, seen);
}

function mergeObjectSchemas(a: JsonSchema, b: JsonSchema): JsonSchema {
  const out: JsonSchema = { ...a, ...b };
  if (a.properties || b.properties) {
    out.properties = { ...(a.properties ?? {}), ...(b.properties ?? {}) };
  }
  if (a.required || b.required) {
    out.required = Array.from(new Set([...(a.required ?? []), ...(b.required ?? [])]));
  }
  if (!out.description) out.description = a.description ?? b.description;
  return out;
}

function resolveAndMerge(root: JsonSchema, schema: JsonSchema): JsonSchema {
  const resolved = deref(root, schema);

  if (resolved.allOf && Array.isArray(resolved.allOf) && resolved.allOf.length > 0) {
    return resolved.allOf
      .map((s) => resolveAndMerge(root, s))
      .reduce((acc, cur) => mergeObjectSchemas(acc, cur), { ...resolved, allOf: undefined });
  }

  return resolved;
}

function extractTypeTag(schema: JsonSchema): string | null {
  if (typeof schema.const === "string") return schema.const;
  if (Array.isArray(schema.enum)) {
    const str = schema.enum.find((v) => typeof v === "string");
    return typeof str === "string" ? str : null;
  }
  return null;
}

function isSignalParamSchema(root: JsonSchema, schema: JsonSchema): boolean {
  const resolved = resolveAndMerge(root, schema);
  const union = resolved.oneOf ?? resolved.anyOf;
  if (!union || !Array.isArray(union)) return false;

  const tags = new Set<string>();
  for (const branch of union) {
    const b = resolveAndMerge(root, branch);
    if (b.type !== "object" || !b.properties?.type) continue;
    const tag = extractTypeTag(resolveAndMerge(root, b.properties.type));
    if (tag) tags.add(tag);
  }

  // Current Signal variants in generated TS: volts/cable/track/disconnected
  const required = ["volts", "cable", "track", "disconnected"];
  return required.every((t) => tags.has(t));
}

function isSignalArrayParamSchema(root: JsonSchema, schema: JsonSchema): boolean {
  const resolved = resolveAndMerge(root, schema);
  if (resolved.type !== "array") return false;

  const items = resolved.items;
  if (!items) return false;

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

function extractStringEnum(schema: JsonSchema): string[] | undefined {
  const resolved = schema;
  if (resolved.type === "string" && Array.isArray(resolved.enum) && resolved.enum.every((v) => typeof v === "string")) {
    return resolved.enum as string[];
  }
  if (Array.isArray(resolved.enum) && resolved.enum.every((v) => typeof v === "string")) {
    return resolved.enum as string[];
  }
  return undefined;
}

function inferKind(root: JsonSchema, schema: JsonSchema): ParamKind {
  if (isSignalParamSchema(root, schema)) return "signal";
  if (isSignalArrayParamSchema(root, schema)) return "signalArray";

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

  return "unknown";
}

export function processModuleSchema(schema: ModuleSchema): ProcessedModuleSchema {
  const root = asJsonSchema(schema.paramsSchema) ?? {};
  const rootResolved = resolveAndMerge(root, root);

  const obj = rootResolved.type === "object" ? rootResolved : null;
  const properties = obj?.properties ?? {};
  const required = new Set(obj?.required ?? []);

  const params: ParamDescriptor[] = Object.entries(properties).map(([name, s]) => {
    const resolved = resolveAndMerge(root, s);
    const enumValues = extractStringEnum(resolved);
    return {
      name,
      kind: inferKind(root, resolved),
      description: resolved.description,
      optional: !required.has(name),
      enumValues,
    };
  });

  const paramsByName: Record<string, ParamDescriptor> = {};
  for (const p of params) paramsByName[p.name] = p;

  return {
    ...schema,
    params,
    paramsByName,
  };
}

export function processSchemas(schemas: ModuleSchema[]): ProcessedModuleSchema[] {
  return schemas.map(processModuleSchema);
}
