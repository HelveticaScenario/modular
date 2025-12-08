import type { ModuleSchema } from "../types/generated/ModuleSchema";

// Base library: minimal JS built-ins plus core DSL types and helpers.


// Keep this in sync with BASE_LIB_SOURCE in src/lsp/tsWorker.ts.
const BASE_LIB_SOURCE = `
// Core DSL types used by the generated declarations
type DSLValue = number | ModuleOutput | ModuleNode | TrackNode;

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
declare function track(id?: string): TrackNode;
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

        for (const param of schema.params) {
            const paramName = sanitizeIdentifier(param.name);
            const doc = escapeDocComment(param.description);
            if (doc) {
                lines.push(`  /** ${doc} */`);
            }
            lines.push(`  ${paramName}(value: DSLValue): this;`);
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

    return lines.join('\n');
}

export function buildLibSource(schemas: ModuleSchema[]): string {
    // console.log('buildLibSource schemas:', schemas);
    const schemaLib = generateSchemaLib(schemas);
    console.log('Generated DSL lib source:\n', schemaLib);
    return `declare global {\n${BASE_LIB_SOURCE}\n\n${schemaLib} \n}\n\n export {};\n`;
}
