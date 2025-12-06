import type { PatchGraph, ModuleState, Param, ModuleSchema } from '../types';

/**
 * GraphBuilder manages the construction of a PatchGraph from DSL code.
 * It tracks modules, generates deterministic IDs, and builds the final graph.
 */
export class GraphBuilder {
  private modules: Map<string, ModuleState> = new Map();
  private moduleCounters: Map<string, number> = new Map();
  private schemas: ModuleSchema[] = [];

  constructor(schemas: ModuleSchema[]) {
    this.schemas = schemas;
  }

  /**
   * Generate a deterministic ID for a module type
   */
  private generateId(moduleType: string, explicitId?: string): string {
    if (explicitId) {
      return explicitId;
    }

    const counter = (this.moduleCounters.get(moduleType) || 0) + 1;
    this.moduleCounters.set(moduleType, counter);
    return `${moduleType}-${counter}`;
  }

  /**
   * Add or update a module in the graph
   */
  addModule(moduleType: string, explicitId?: string): ModuleNode {
    const id = this.generateId(moduleType, explicitId);
    
    // Check if module type exists in schemas
    const schema = this.schemas.find(s => s.name === moduleType);
    if (!schema) {
      throw new Error(`Unknown module type: ${moduleType}`);
    }

    // Initialize module with disconnected params
    const params: Record<string, Param> = {};
    for (const param of schema.params) {
      params[param.name] = { param_type: 'disconnected' };
    }

    const moduleState: ModuleState = {
      id,
      module_type: moduleType,
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
  setParam(moduleId: string, paramName: string, value: Param): void {
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
      tracks: [],
    };
  }

  /**
   * Reset the builder state
   */
  reset(): void {
    this.modules.clear();
    this.moduleCounters.clear();
  }
}

/**
 * ModuleNode represents a module instance in the DSL with fluent API
 */
export class ModuleNode {
  // Dynamic parameter methods will be added via Proxy
  [key: string]: any;

  readonly builder: GraphBuilder;
  readonly id: string;
  readonly moduleType: string;
  readonly schema: ModuleSchema;

  constructor(
    builder: GraphBuilder,
    id: string,
    moduleType: string,
    schema: ModuleSchema
  ) {
    this.builder = builder;
    this.id = id;
    this.moduleType = moduleType;
    this.schema = schema;
    // Create a proxy to intercept parameter method calls
    return new Proxy(this, {
      get(target, prop: string) {
        // If it's an existing property, return it
        if (prop in target) {
          return (target as any)[prop];
        }

        // Check if it's a parameter name
        const paramSchema = target.schema.params.find(p => p.name === prop);
        if (paramSchema) {
          // Return a function that sets the parameter
          return (value: number | ModuleOutput) => {
            if (value instanceof ModuleOutput) {
              target.connect(prop, value);
            } else {
              target.setParam(prop, value);
            }
            return target;
          };
        }

        // Check if it's an output name
        const outputSchema = target.schema.outputs.find(o => o.name === prop);
        if (outputSchema) {
          return target.output(prop);
        }

        return undefined;
      }
    });
  }

  /**
   * Set a parameter to a constant value
   */
  setParam(paramName: string, value: number | Param): this {
    const param: Param = typeof value === 'number'
      ? { param_type: 'value', value }
      : value;

    this.builder.setParam(this.id, paramName, param);
    return this;
  }

  /**
   * Connect a parameter to another module's output
   */
  connect(paramName: string, source: ModuleOutput): this {
    const param: Param = {
      param_type: 'cable',
      module: source.moduleId,
      port: source.portName,
    };
    this.builder.setParam(this.id, paramName, param);
    return this;
  }

  /**
   * Get an output port of this module
   */
  output(portName: string = 'output'): ModuleOutput {
    // Verify output exists
    const outputSchema = this.schema.outputs.find(o => o.name === portName);
    if (!outputSchema) {
      throw new Error(`Module ${this.moduleType} does not have output: ${portName}`);
    }
    return new ModuleOutput(this.builder, this.id, portName);
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
  scale(factor: number): ModuleOutput {
    const scaleNode = this.builder.addModule('scale-and-shift');
    scaleNode.connect('input', this);
    scaleNode.setParam('scale', factor);
    return scaleNode.output();
  }

  /**
   * Shift this output by an offset
   */
  shift(offset: number): ModuleOutput {
    const shiftNode = this.builder.addModule('scale-and-shift');
    shiftNode.connect('input', this);
    shiftNode.setParam('shift', offset);
    return shiftNode.output();
  }
}

