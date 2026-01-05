
import type { ModuleSchema, ModuleState, PatchGraph, Scope } from "@modular/core";
import type { ProcessedModuleSchema } from "./paramsSchema";
import { processSchemas } from "./paramsSchema";

/**
 * GraphBuilder manages the construction of a PatchGraph from DSL code.
 * It tracks modules, generates deterministic IDs, and builds the final graph.
 */
export class GraphBuilder {

  private modules: Map<string, ModuleState> = new Map();
  private counters: Map<string, number> = new Map();
  private schemas: ProcessedModuleSchema[] = [];
  private schemaByName: Map<string, ProcessedModuleSchema> = new Map();
  private scopes: Scope[] = [];


  constructor(schemas: ModuleSchema[]) {
    this.schemas = processSchemas(schemas);
    this.schemaByName = new Map(this.schemas.map((s) => [s.name, s]));
  }

  /**
   * Generate a deterministic ID for a module type
   */
  private generateId(moduleType: string, explicitId?: string): string {
    if (explicitId) {
      return explicitId;
    }

    let counter = (this.counters.get(moduleType) || 0) + 1;
    let id = `${moduleType}-${counter}`;

    // If the generated ID is already taken (e.g. by an explicit ID),
    // keep incrementing until we find a free one.
    while (this.modules.has(id)) {
      counter++;
      id = `${moduleType}-${counter}`;
    }

    this.counters.set(moduleType, counter);
    return id;
  }

  /**
   * Add or update a module in the graph
   */
  addModule(moduleType: string, explicitId?: string): ModuleNode {
    const id = this.generateId(moduleType, explicitId);

    if (this.modules.has(id)) {
      throw new Error(`Duplicate module id: ${id}`);
    }

    // Check if module type exists in schemas
    const schema = this.schemaByName.get(moduleType);
    if (!schema) {
      throw new Error(`Unknown module type: ${moduleType}`);
    }

    // Initialize module params: default all signal params to disconnected.
    // Other params are left unset unless the DSL sets them explicitly.
    const params: Record<string, unknown> = {};
    for (const param of schema.params) {
      if (param.kind === 'signal') {
        params[param.name] = { type: 'disconnected' };
      } else if (param.kind === 'signalArray') {
        // Required arrays (e.g. sum.signals) should be valid by default.
        params[param.name] = [];
      }
    }

    const moduleState: ModuleState = {
      id,
      moduleType,
      idIsExplicit: Boolean(explicitId),
      params,
    };

    this.modules.set(id, moduleState);
    return new ModuleNode(this, id, moduleType, schema);
  }

  /**
   * Get a module by ID
   */
  getModule(id: string): ModuleState | undefined {
    return this.modules.get(id);
  }

  /**
   * Set a parameter value for a module
   */
  setParam(moduleId: string, paramName: string, value: unknown): void {
    const module = this.modules.get(moduleId);
    if (!module) {
      throw new Error(`Module not found: ${moduleId}`);
    }
    module.params[paramName] = value;
  }



  /**
   * Build the final PatchGraph
   */
  toPatch(): PatchGraph {
    return {
      modules: Array.from(this.modules.values()),
      scopes: Array.from(this.scopes),
    };
  }

  /**
   * Reset the builder state
   */
  reset(): void {
    this.modules.clear();
    this.scopes = [];
    this.counters.clear();
  }


  addScope(value: ModuleOutput | ModuleNode, msPerFrame: number = 500, triggerThreshold?: number) {
    let realTriggerThreshold: number | undefined = triggerThreshold ? (triggerThreshold * 1000) : undefined
    const output = value instanceof ModuleNode ? value.o : value;
    this.scopes.push({
      item: {
        type: 'ModuleOutput',
        moduleId: output.moduleId,
        portName: output.portName,
      },
      msPerFrame,
      triggerThreshold: realTriggerThreshold
    });
  }
}

type Value = number | ModuleOutput | ModuleNode;
/**
 * ModuleNode represents a module instance in the DSL with fluent API
 */
export class ModuleNode {
  // Dynamic parameter methods will be added via Proxy
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  [key: string]: any;

  readonly builder: GraphBuilder;
  readonly id: string;
  readonly moduleType: string;
  readonly schema: ProcessedModuleSchema;

  constructor(
    builder: GraphBuilder,
    id: string,
    moduleType: string,
    schema: ProcessedModuleSchema
  ) {
    this.builder = builder;
    this.id = id;
    this.moduleType = moduleType;
    this.schema = schema;
    // Create a proxy to intercept parameter method calls
    const proxy = new Proxy(this, {
      get(target, prop: string) {
        // Check if it's a param name (derived from schemars JSON schema)
        const param = target.schema.paramsByName[prop];
        console.log('Accessed prop:', prop, 'param:', param);
        if (param && Object.hasOwn(target.schema.paramsByName, prop)) {
          return (...args: unknown[]) => {
            if (args.length > 1) {
              target._setParam(prop, args);
            } else {
              target._setParam(prop, args[0]);
            }
            return proxy;
          };
        }

        // Check if it's an output name
        const outputSchema =
          target.schema.outputs.find(o => o.name === prop)
          ?? (prop === "output"
            ? target.schema.outputs.find(o => o.default) ?? target.schema.outputs[0]
            : undefined);
        if (outputSchema) {
          return target._output(outputSchema.name);
        }

        if (prop in target) {
          // eslint-disable-next-line @typescript-eslint/no-explicit-any
          const thing = (target as any)[prop];
          if (typeof thing === 'function') {
            return thing.bind(target);
          }
          return thing;
        }

        return undefined;
      }
    });

    return proxy as unknown as ModuleNode;
  }

  get o(): ModuleOutput {
    const defaultOutput = this.schema.outputs.find(o => o.default);
    if (!defaultOutput) {
      throw new Error(`Module ${this.moduleType} has no default output`);
    }
    return this._output(defaultOutput.name);
  }

  scale(value: Value): ModuleNode {
    return this.o.scale(value);
  }

  shift(value: Value): ModuleNode {
    return this.o.shift(value);
  }

  scope(msPerFrame: number = 500, triggerThreshold?: number): this {
    this.builder.addScope(this, msPerFrame, triggerThreshold);
    return this;
  }


  _setParam(paramName: string, value: unknown): this {
    this.builder.setParam(this.id, paramName, replaceSignals(value));
    return this
  }



  /**
   * Get an output port of this module
   */
  _output(portName: string): ModuleOutput {
    // Verify output exists
    const outputSchema = this.schema.outputs.find(o => o.name === portName);
    if (!outputSchema) {
      throw new Error(`Module ${this.moduleType} does not have output: ${portName}`);
    }
    return new ModuleOutput(this.builder, this.id, portName);
  }

  toString(): string {
    return this.o.toString();
  }
}

/**
 * ModuleOutput represents an output port that can be connected or transformed
 */
export class ModuleOutput {
  readonly builder: GraphBuilder;
  readonly moduleId: string;
  readonly portName: string;

  constructor(
    builder: GraphBuilder,
    moduleId: string,
    portName: string
  ) {
    this.builder = builder;
    this.moduleId = moduleId;
    this.portName = portName;
  }

  /**
   * Scale this output by a factor
   */
  scale(factor: Value): ModuleNode {
    const scaleNode = this.builder.addModule('scaleAndShift');
    scaleNode._setParam('input', this);
    scaleNode._setParam('scale', factor);
    return scaleNode;
  }

  /**
   * Shift this output by an offset
   */
  shift(offset: Value): ModuleNode {
    const shiftNode = this.builder.addModule('scaleAndShift');
    shiftNode._setParam('input', this);
    shiftNode._setParam('shift', offset);
    return shiftNode;
  }

  scope(msPerFrame: number = 500, triggerThreshold?: number): this {
    this.builder.addScope(this, msPerFrame, triggerThreshold);
    return this;
  }

  toString(): string {
    return `module(${this.moduleId}:${this.portName})`;
  }
}

type Replacer = (key: string, value: unknown) => unknown;

export function replaceValues(input: unknown, replacer: Replacer): unknown {
  function walk(key: string, value: unknown): unknown {
    const replaced = replacer(key, value);

    // Match JSON.stringify behavior
    if (replaced === undefined) {
      return undefined;
    }

    if (typeof replaced !== "object" || replaced === null) {
      return replaced;
    }

    if (Array.isArray(replaced)) {
      return replaced
        .map((v, i) => walk(String(i), v))
        .filter(v => v !== undefined);
    }

    const out: Record<string, unknown> = {};
    for (const [key, value] of Object.entries(replaced)) {
      const v = walk(key, value);
      if (v !== undefined) {
        out[key] = v;
      }
    }
    return out;
  }

  // JSON.stringify starts with key ""
  return walk("", input);
}


function replaceSignals(input: unknown): unknown {
  return replaceValues(input, (_key, value) => {
    // Replace Signal instances with their JSON representation
    if (value instanceof ModuleNode || value instanceof ModuleOutput) {
      return valueToSignal(value);
    } else {
      return value
    }
  })
}

function valueToSignal(value: Value): unknown {
  if (value instanceof ModuleNode) {
    value = value.o;
  }
  let signal: unknown;
  if (value instanceof ModuleOutput) {
    signal = {
      type: 'cable',
      module: value.moduleId,
      port: value.portName,
    };
  } else {
    signal = value
  }

  return signal
}