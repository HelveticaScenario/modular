import { ModuleSchema } from "@modular/core";

const BASE_LIB_SOURCE = `
/** The **\`console\`** object provides access to the debugging console (e.g., the Web console in Firefox). */
/**
 * The **\`console\`** object provides access to the debugging console (e.g., the Web console in Firefox).
 *
 * [MDN Reference](https://developer.mozilla.org/docs/Web/API/console)
 */
interface Console {
    /**
     * The **\`console.assert()\`** static method writes an error message to the console if the assertion is false.
     *
     * [MDN Reference](https://developer.mozilla.org/docs/Web/API/console/assert_static)
     */
    assert(condition?: boolean, ...data: any[]): void;
    /**
     * The **\`console.clear()\`** static method clears the console if possible.
     *
     * [MDN Reference](https://developer.mozilla.org/docs/Web/API/console/clear_static)
     */
    clear(): void;
    /**
     * The **\`console.count()\`** static method logs the number of times that this particular call to \`count()\` has been called.
     *
     * [MDN Reference](https://developer.mozilla.org/docs/Web/API/console/count_static)
     */
    count(label?: string): void;
    /**
     * The **\`console.countReset()\`** static method resets counter used with console/count_static.
     *
     * [MDN Reference](https://developer.mozilla.org/docs/Web/API/console/countReset_static)
     */
    countReset(label?: string): void;
    /**
     * The **\`console.debug()\`** static method outputs a message to the console at the 'debug' log level.
     *
     * [MDN Reference](https://developer.mozilla.org/docs/Web/API/console/debug_static)
     */
    debug(...data: any[]): void;
    /**
     * The **\`console.dir()\`** static method displays a list of the properties of the specified JavaScript object.
     *
     * [MDN Reference](https://developer.mozilla.org/docs/Web/API/console/dir_static)
     */
    dir(item?: any, options?: any): void;
    /**
     * The **\`console.dirxml()\`** static method displays an interactive tree of the descendant elements of the specified XML/HTML element.
     *
     * [MDN Reference](https://developer.mozilla.org/docs/Web/API/console/dirxml_static)
     */
    dirxml(...data: any[]): void;
    /**
     * The **\`console.error()\`** static method outputs a message to the console at the 'error' log level.
     *
     * [MDN Reference](https://developer.mozilla.org/docs/Web/API/console/error_static)
     */
    error(...data: any[]): void;
    /**
     * The **\`console.group()\`** static method creates a new inline group in the Web console log, causing any subsequent console messages to be indented by an additional level, until console/groupEnd_static is called.
     *
     * [MDN Reference](https://developer.mozilla.org/docs/Web/API/console/group_static)
     */
    group(...data: any[]): void;
    /**
     * The **\`console.groupCollapsed()\`** static method creates a new inline group in the console.
     *
     * [MDN Reference](https://developer.mozilla.org/docs/Web/API/console/groupCollapsed_static)
     */
    groupCollapsed(...data: any[]): void;
    /**
     * The **\`console.groupEnd()\`** static method exits the current inline group in the console.
     *
     * [MDN Reference](https://developer.mozilla.org/docs/Web/API/console/groupEnd_static)
     */
    groupEnd(): void;
    /**
     * The **\`console.info()\`** static method outputs a message to the console at the 'info' log level.
     *
     * [MDN Reference](https://developer.mozilla.org/docs/Web/API/console/info_static)
     */
    info(...data: any[]): void;
    /**
     * The **\`console.log()\`** static method outputs a message to the console.
     *
     * [MDN Reference](https://developer.mozilla.org/docs/Web/API/console/log_static)
     */
    log(...data: any[]): void;
    /**
     * The **\`console.table()\`** static method displays tabular data as a table.
     *
     * [MDN Reference](https://developer.mozilla.org/docs/Web/API/console/table_static)
     */
    table(tabularData?: any, properties?: string[]): void;
    /**
     * The **\`console.time()\`** static method starts a timer you can use to track how long an operation takes.
     *
     * [MDN Reference](https://developer.mozilla.org/docs/Web/API/console/time_static)
     */
    time(label?: string): void;
    /**
     * The **\`console.timeEnd()\`** static method stops a timer that was previously started by calling console/time_static.
     *
     * [MDN Reference](https://developer.mozilla.org/docs/Web/API/console/timeEnd_static)
     */
    timeEnd(label?: string): void;
    /**
     * The **\`console.timeLog()\`** static method logs the current value of a timer that was previously started by calling console/time_static.
     *
     * [MDN Reference](https://developer.mozilla.org/docs/Web/API/console/timeLog_static)
     */
    timeLog(label?: string, ...data: any[]): void;
    timeStamp(label?: string): void;
    /**
     * The **\`console.trace()\`** static method outputs a stack trace to the console.
     *
     * [MDN Reference](https://developer.mozilla.org/docs/Web/API/console/trace_static)
     */
    trace(...data: any[]): void;
    /**
     * The **\`console.warn()\`** static method outputs a warning message to the console at the 'warning' log level.
     *
     * [MDN Reference](https://developer.mozilla.org/docs/Web/API/console/warn_static)
     */
    warn(...data: any[]): void;
}

declare var console: Console;

// Core DSL types used by the generated declarations
type Signal = number | ModuleOutput | ModuleNode | TrackNode;

interface ModuleOutput {
  readonly moduleId: string;
  readonly portName: string;
  scale(factor: Signal): ModuleNode;
  shift(offset: Signal): ModuleNode;
}

interface ModuleNode {
  readonly id: string;
  readonly moduleType: string;
  readonly o: ModuleOutput;
  scale(value: Signal): ModuleNode;
  shift(value: Signal): ModuleNode;
}

interface TrackNode {
    /**
     * Set the interpolation type for this track.
     */
    interpolation(interpolation: "linear" | "step" | "cubic" | "expo"): TrackNode;

    /**
     * Set the playhead control value for this track.
     *
     * The value maps linearly to the normalized time range [0.0, 1.0].
     */
    playhead(value: Signal): TrackNode;

    /**
     * Add a keyframe at the given normalized time in [0.0, 1.0].
     */
    addKeyframe(time: number, value: Signal): TrackNode;

    scale(value: Signal): ModuleNode;
    shift(value: Signal): ModuleNode;
}

type ASTNode =
  | { type: 'FastSubsequence', elements: Array<ASTNode> }
  | { type: 'SlowSubsequence', elements: Array<ASTNode> }
  | { type: 'RandomChoice', choices: Array<ASTNode> }
  | { type: 'NumericLiteral', value: number }
  | { type: 'Rest' }


interface PatternProgram {
  id: string
  elements: Array<ASTNode>
  seed: number
}

// Helper functions exposed by the DSL runtime
declare function hz(frequency: number): number;
declare function note(noteName: string): number;
declare function bpm(beatsPerMinute: number): number;
declare function track(id?: string): TrackNode;
declare function scope(target: ModuleOutput | ModuleNode | TrackNode, msPerFrame?: number, triggerThreshold?: number):  ModuleOutput | ModuleNode | TrackNode;
declare function pat(str: string): PatternProgram;
`;



export function buildLibSource(schemas: ModuleSchema[]): string {
    // console.log('buildLibSource schemas:', schemas);
    const schemaLib = generateDSL(schemas);
    return `declare global {\n${BASE_LIB_SOURCE}\n\n${schemaLib} \n}\n\n export {};\n`;
}

type JSONSchema = any;

type ClassSpec = {
    description?: string;
    outputs: Array<{ name: string; description?: string }>;
    properties: Array<{
        name: string;
        schema: JSONSchema;
        description?: string;
    }>;
    rootSchema: JSONSchema;
};

type NamespaceNode = {
    namespaces: Map<string, NamespaceNode>;
    classes: Map<string, ClassSpec>;
    order: Array<{ kind: "namespace" | "class"; name: string }>;
};

function makeNamespaceNode(): NamespaceNode {
    return {
        namespaces: new Map(),
        classes: new Map(),
        order: [],
    };
}

function isValidIdentifier(name: string): boolean {
    return /^[$A-Z_][0-9A-Z_$]*$/i.test(name);
}

function renderPropertyKey(name: string): string {
    return isValidIdentifier(name) ? name : JSON.stringify(name);
}

function renderReadonlyPropertyKey(name: string): string {
    return isValidIdentifier(name) ? name : `[${JSON.stringify(name)}]`;
}

function renderDocComment(description?: string, indent: string = ""): string[] {
    if (!description) return [];
    const lines = description.split(/\r?\n/);
    return [
        `${indent}/**`,
        ...lines.map((l) => `${indent} * ${l}`),
        `${indent} */`,
    ];
}

function extractParamNamesFromDoc(description?: string): string[] {
    if (!description) return [];
    const names: string[] = [];
    const re = /@param\s+([^\s]+)/g;
    for (const match of description.matchAll(re)) {
        names.push(match[1]);
    }
    return names;
}

function resolveRef(ref: string, rootSchema: JSONSchema): JSONSchema | "Signal" {
    if (ref === "Signal") return "Signal";

    const defsPrefix = "#/$defs/";
    if (!ref.startsWith(defsPrefix)) {
        throw new Error(`Unsupported $ref: ${ref}`);
    }

    const defName = ref.slice(defsPrefix.length);
    if (defName === "Signal") return "Signal";

    const defs = rootSchema?.$defs;
    if (!defs || typeof defs !== "object") {
        throw new Error(`Unresolved $ref: ${ref}`);
    }

    const resolved = defs[defName];
    if (!resolved) {
        throw new Error(`Unresolved $ref: ${ref}`);
    }

    if (resolved?.title === "Signal") return "Signal";
    return resolved;
}

function schemaToTypeExpr(schema: JSONSchema, rootSchema: JSONSchema): string {
    if (schema === null || schema === undefined) {
        throw new Error("Unsupported schema: null/undefined");
    }
    if (typeof schema === "boolean") {
        throw new Error("Unsupported schema: boolean schema");
    }
    if (schema.oneOf || schema.anyOf || schema.allOf) {
        console.log('schema:', schema);
        return "any";
        // throw new Error("Unsupported schema composition (oneOf/anyOf/allOf)");
    }
    if (Array.isArray(schema.type)) {
        throw new Error("Unsupported schema: union type array");
    }

    if (schema.$ref) {
        const resolved = resolveRef(String(schema.$ref), rootSchema);
        if (resolved === "Signal") return "Signal";
        return schemaToTypeExpr(resolved, rootSchema);
    }

    if (schema.enum) {
        if (!Array.isArray(schema.enum) || schema.enum.length === 0) {
            throw new Error("Unsupported enum schema");
        }
        return schema.enum.map((v: any) => JSON.stringify(v)).join(" | ");
    }

    const type = schema.type;

    if (type === "integer" || type === "number") return "number";
    if (type === "string") return "string";
    if (type === "boolean") return "boolean";

    const looksLikeObject = type === "object" || (!!schema.properties && typeof schema.properties === "object");
    if (looksLikeObject) {
        const props = schema.properties;
        if (!props || typeof props !== "object") return "{}";

        const requiredSet = new Set<string>(Array.isArray(schema.required) ? schema.required : []);
        const entries = Object.entries(props as Record<string, JSONSchema>);
        if (entries.length === 0) return "{}";

        const parts: string[] = [];
        for (const [propName, propSchema] of entries) {
            const optional = requiredSet.has(propName) ? "" : "?";
            parts.push(`${renderPropertyKey(propName)}${optional}: ${schemaToTypeExpr(propSchema, rootSchema)}`);
        }
        return `{ ${parts.join("; ")} }`;
    }

    if (type === "array") {
        if (Array.isArray(schema.prefixItems)) {
            const items = schema.prefixItems as JSONSchema[];
            const tuple = items.map((s) => schemaToTypeExpr(s, rootSchema)).join(", ");
            return `[${tuple}]`;
        }
        if (schema.items) {
            return `${schemaToTypeExpr(schema.items, rootSchema)}[]`;
        }
        throw new Error("Unsupported array schema: missing items/prefixItems");
    }

    if (type === undefined) {
        throw new Error("Unsupported schema: missing type");
    }

    throw new Error(`Unsupported scalar type: ${type}`);
}

function getMethodArgsForProperty(
    propertySchema: JSONSchema,
    rootSchema: JSONSchema,
    propertyDescription?: string
): Array<{ name: string; type: string }> {
    const paramNames = extractParamNamesFromDoc(propertyDescription);

    // Top-level tuple expansion into multiple arguments.
    if (
        propertySchema &&
        typeof propertySchema === "object" &&
        propertySchema.type === "array" &&
        Array.isArray(propertySchema.prefixItems)
    ) {
        const items: JSONSchema[] = propertySchema.prefixItems;
        return items.map((itemSchema, index) => {
            const name =
                paramNames.length > 0
                    ? (paramNames[index] ?? `arg${index + 1}`)
                    : `arg${index + 1}`;
            return { name, type: schemaToTypeExpr(itemSchema, rootSchema) };
        });
    }

    // Single-argument method.
    const name =
        paramNames.length > 0
            ? (paramNames[0] ?? "arg1")
            : "arg";
    return [{ name, type: schemaToTypeExpr(propertySchema, rootSchema) }];
}

function buildTreeFromSchemas(schemas: ModuleSchema[]): NamespaceNode {
    const root = makeNamespaceNode();

    for (const moduleSchema of schemas) {
        const fullName = String(moduleSchema.name).trim();
        if (!fullName) {
            throw new Error("ModuleSchema is missing a non-empty name");
        }

        const paramsSchema = moduleSchema.paramsSchema;
        if (!paramsSchema || typeof paramsSchema !== "object") {
            throw new Error(`ModuleSchema ${fullName} is missing paramsSchema`);
        }

        const parts = fullName.split(".").filter((p: string) => p.length > 0);
        if (parts.length === 0) {
            throw new Error(`Invalid ModuleSchema name: ${fullName}`);
        }

        const className = parts[parts.length - 1];
        const namespacePath = parts.slice(0, -1);

        let node = root;
        for (const ns of namespacePath) {
            let child = node.namespaces.get(ns);
            if (!child) {
                child = makeNamespaceNode();
                node.namespaces.set(ns, child);
                node.order.push({ kind: "namespace", name: ns });
            }
            node = child;
        }

        if (node.classes.has(className)) {
            throw new Error(`Duplicate class name detected: ${fullName}`);
        }
        if ('properties' in paramsSchema === false) {
            throw new Error(`ModuleSchema ${fullName} paramsSchema is missing properties`);
        }
        const propsObj = paramsSchema.properties;
        const propsEntries =
            propsObj && typeof propsObj === "object"
                ? Object.entries(propsObj as Record<string, JSONSchema>)
                : [];

        const properties = propsEntries.map(([name, propSchema]) => ({
            name,
            schema: propSchema,
            description: propSchema?.description,
        }));

        const outputs = (Array.isArray(moduleSchema.outputs) ? moduleSchema.outputs : []).map((o) => ({
            name: String(o?.name ?? "").trim(),
            description: o?.description,
        })).filter((o) => o.name.length > 0);

        node.classes.set(className, {
            description: moduleSchema.description,
            outputs,
            properties,
            rootSchema: paramsSchema,
        });
        node.order.push({ kind: "class", name: className });
    }

    return root;
}

function renderNodeInterfaceName(baseName: string): string {
    return baseName.endsWith("Node") ? baseName : `${baseName}Node`;
}

function capitalizeName(name: string): string {
    if (!name) return name;
    return name.charAt(0).toUpperCase() + name.slice(1);
}

function renderFactoryFunction(
    functionName: string,
    interfaceName: string,
    indent: string
): string[] {
    return [`${indent}export function ${functionName}(id?: string): ${interfaceName};`];
}

function getQualifiedNodeInterfaceType(moduleName: string): string {
    const parts = moduleName.split(".").filter((p) => p.length > 0);
    if (parts.length === 0) {
        throw new Error(`Invalid ModuleSchema name: ${moduleName}`);
    }
    const base = parts[parts.length - 1];
    const interfaceName = renderNodeInterfaceName(capitalizeName(base));
    const namespaces = parts.slice(0, -1);
    return namespaces.length > 0
        ? `${namespaces.join(".")}.${interfaceName}`
        : interfaceName;
}

function renderInterface(baseName: string, classSpec: ClassSpec, indent: string): string[] {
    const lines: string[] = [];
    lines.push(...renderDocComment(classSpec.description, indent));
    const interfaceName = renderNodeInterfaceName(capitalizeName(baseName));
    lines.push(`${indent}export interface ${interfaceName} extends ModuleNode {`);

    const seenOutputNames = new Set<string>();
    for (const output of classSpec.outputs) {
        if (!output.name) continue;
        if (output.name === "o") continue; // already exists on ModuleNode
        if (seenOutputNames.has(output.name)) continue;
        seenOutputNames.add(output.name);

        lines.push("");
        lines.push(...renderDocComment(output.description, indent + "  "));
        lines.push(`${indent}  readonly ${renderReadonlyPropertyKey(output.name)}: ModuleOutput;`);
    }

    for (const prop of classSpec.properties) {
        lines.push("");
        lines.push(...renderDocComment(prop.description, indent + "  "));
        const args = getMethodArgsForProperty(
            prop.schema,
            classSpec.rootSchema,
            prop.description
        );
        const argList = args.map((a) => `${a.name}: ${a.type}`).join(", ");
        lines.push(`${indent}  ${renderPropertyKey(prop.name)}(${argList}): this;`);
    }

    lines.push(`${indent}}`);
    lines.push("");
    lines.push(...renderFactoryFunction(baseName, interfaceName, indent));
    return lines;
}

function renderTree(node: NamespaceNode, indentLevel: number = 0): string[] {
    const indent = "  ".repeat(indentLevel);
    const lines: string[] = [];

    for (const item of node.order) {
        if (item.kind === "class") {
            const classSpec = node.classes.get(item.name);
            if (!classSpec) continue;
            lines.push(...renderInterface(item.name, classSpec, indent));
            lines.push("");
            continue;
        }

        const child = node.namespaces.get(item.name);
        if (!child) continue;
        lines.push(`${indent}export declare namespace ${item.name} {`);
        lines.push(...renderTree(child, indentLevel + 1));
        lines.push(`${indent}}`);
        lines.push("");
    }

    // Trim extra blank lines at this level.
    while (lines.length > 0 && lines[lines.length - 1] === "") {
        lines.pop();
    }
    return lines;
}

export function generateDSL(schemas: ModuleSchema[]): string {
    if (!Array.isArray(schemas)) {
        throw new Error("generateDSL expects an array of ModuleSchema");
    }
    const tree = buildTreeFromSchemas(schemas);
    const lines = renderTree(tree, 0);

    const signalSchema = schemas.find((s) => s.name === "signal");
    if (signalSchema) {
        lines.push("");
        lines.push("/** Root output helper bound to the 'signal' module. */");
        lines.push(
            `export declare const out: ${getQualifiedNodeInterfaceType(signalSchema.name)};`
        );
    } else {
        lines.push("");
        lines.push("/** Root output helper. */");
        lines.push("export declare const out: ModuleNode;");
    }

    const clockSchema = schemas.find((s) => s.name === "clock");
    if (clockSchema) {
        lines.push("");
        lines.push("/** Default clock module running at 120 BPM. */");
        lines.push(
            `export declare const rootClock: ${getQualifiedNodeInterfaceType(clockSchema.name)};`
        );
    }

    return lines.join("\n") + "\n";
}
