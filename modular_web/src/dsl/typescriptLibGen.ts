import type { ModuleSchema } from "../types/generated/ModuleSchema";

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
type DSLValue = number | ModuleOutput | ModuleNode | TrackNode;
type DSLDataValue = string | number | boolean;

interface ModuleOutput {
  readonly moduleId: string;
  readonly portName: string;
  scale(factor: DSLValue): ModuleNode;
  shift(offset: DSLValue): ModuleNode;
}

interface ModuleNode {
  readonly id: string;
  readonly moduleType: string;
  readonly o: ModuleOutput;
  scale(value: DSLValue): ModuleNode;
  shift(value: DSLValue): ModuleNode;
}

interface TrackNode {
    /**
     * Set the interpolation type for this track.
     */
    interpolation(interpolation: "linear" | "step" | "cubic" | "exponential"): TrackNode;

    /**
     * Set the playhead control value for this track.
     *
     * The value range [-5.0, 5.0] maps linearly to the normalized time range [0.0, 1.0].
     */
    playhead(value: DSLValue): TrackNode;

    /**
     * Add a keyframe at the given normalized time in [0.0, 1.0].
     */
    addKeyframe(time: number, value: DSLValue): TrackNode;

    scale(value: DSLValue): ModuleNode;
    shift(value: DSLValue): ModuleNode;
}

// Helper functions exposed by the DSL runtime
declare function hz(frequency: number): number;
declare function note(noteName: string): number;
declare function bpm(beatsPerMinute: number): number;
declare function track(id?: string): TrackNode;
declare function scope(target: ModuleOutput | ModuleNode | TrackNode):  ModuleOutput | ModuleNode | TrackNode;
`;

function escapeDocComment(text: string): string {
    return text
        .replace(/\*\//g, '*\\/')
        .replace(/\r?\n\s*/g, ' ')
        .trim();
}

function sanitizeIdentifier(name: string): string {
    let id = name.replace(
        /[^a-zA-Z0-9_$]+(.)?/g,
        (_match, chr: string | undefined) => (chr ? chr.toUpperCase() : '')
    );
    if (!/^[A-Za-z_$]/.test(id)) {
        id = `_${id}`;
    }
    return id || '_';
}

function makeNodeInterfaceName(factoryName: string): string {
    const id = sanitizeIdentifier(factoryName);
    return id.charAt(0).toUpperCase() + id.slice(1) + 'Node';
}

function generateSchemaLib(schemas: ModuleSchema[]): string {
    const lines: string[] = [];
    lines.push('// DSL declarations generated from ModuleSchema');

    for (const schema of schemas) {
        const factoryName = sanitizeIdentifier(schema.name);
        const nodeInterfaceName = makeNodeInterfaceName(factoryName);

        lines.push('');
        if (schema.description) {
            lines.push(`/** ${escapeDocComment(schema.description)} */`);
        }
        lines.push(`interface ${nodeInterfaceName} extends ModuleNode {`);

        for (const param of schema.signalParams) {
            const paramName = sanitizeIdentifier(param.name);
            const doc = escapeDocComment(param.description);
            if (doc) {
                lines.push(`  /** ${doc} */`);
            }
            lines.push(`  ${paramName}(value: DSLValue): this;`);
        }

        for (const param of schema.dataParams) {
            const paramName = sanitizeIdentifier(param.name);
            const doc = escapeDocComment(param.description);
            if (doc) {
                lines.push(`  /** ${doc} */`);
            }
            lines.push(`  ${paramName}(value: DSLDataValue): this;`);
        }

        for (const output of schema.outputs) {
            const propName = sanitizeIdentifier(output.name);
            const doc = escapeDocComment(output.description);
            if (doc) {
                lines.push(`  /** ${doc} */`);
            }
            lines.push(`  readonly ${propName}: ModuleOutput;`);
        }

        lines.push('}');
        lines.push('');
        if (schema.description) {
            lines.push(`/** ${escapeDocComment(schema.description)} */`);
        }
        lines.push(
            `declare function ${factoryName}(id?: string): ${nodeInterfaceName};`
        );
    }

    const signalSchema = schemas.find((s) => s.name === 'signal');
    if (signalSchema) {
        const factoryName = sanitizeIdentifier(signalSchema.name);
        const nodeInterfaceName = makeNodeInterfaceName(factoryName);
        lines.push('');
        lines.push("/** Root output helper bound to the 'signal' module. */");
        lines.push(`declare const out: ${nodeInterfaceName};`);
    } else {
        lines.push('');
        lines.push('/** Root output helper. */');
        lines.push('declare const out: ModuleNode;');
    }

    const clockSchema = schemas.find((s) => s.name === 'clock');
    if (clockSchema) {
        const factoryName = sanitizeIdentifier(clockSchema.name);
        const nodeInterfaceName = makeNodeInterfaceName(factoryName);
        lines.push('');
        lines.push("/** Default clock module running at 120 BPM. */");
        lines.push(`declare const rootClock: ${nodeInterfaceName};`);
    }

    return lines.join('\n');
}

export function buildLibSource(schemas: ModuleSchema[]): string {
    // console.log('buildLibSource schemas:', schemas);
    const schemaLib = generateSchemaLib(schemas);
    return `declare global {\n${BASE_LIB_SOURCE}\n\n${schemaLib} \n}\n\n export {};\n`;
}
