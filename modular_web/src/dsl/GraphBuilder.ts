import type { ModuleSchema } from "../types/generated/ModuleSchema";
import type { ModuleState } from "../types/generated/ModuleState";
import type { Signal } from "../types/generated/Signal";
import type { PatchGraph } from "../types/generated/PatchGraph";
import type { InterpolationType } from "../types/generated/InterpolationType";
import type { Track } from "../types/generated/Track";
import type { TrackKeyframe } from "../types/generated/TrackKeyframe";
import type { ScopeItem } from "../types/generated/ScopeItem";
import type { ProcessedModuleSchema } from "./paramsSchema";
import { processSchemas } from "./paramsSchema";

/**
 * GraphBuilder manages the construction of a PatchGraph from DSL code.
 * It tracks modules, generates deterministic IDs, and builds the final graph.
 */
export class GraphBuilder {

  private modules: Map<string, ModuleState> = new Map();
  private tracks: Map<string, Track> = new Map();
  private counters: Map<string, number> = new Map();
  private schemas: ProcessedModuleSchema[] = [];
  private schemaByName: Map<string, ProcessedModuleSchema> = new Map();
  private scopes: ScopeItem[] = [];


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

    const counter = (this.counters.get(moduleType) || 0) + 1;
    this.counters.set(moduleType, counter);
    return `${moduleType}-${counter}`;
  }

  /**
   * Add or update a module in the graph
   */
  addModule(moduleType: string, explicitId?: string): ModuleNode {
    const id = this.generateId(moduleType, explicitId);

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
        params[param.name] = { type: 'disconnected' } satisfies Signal;
      } else if (param.kind === 'signalArray') {
        // Required arrays (e.g. sum.signals) should be valid by default.
        params[param.name] = [];
      }
    }

    const moduleState: ModuleState = {
      id,
      moduleType,
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



  addTrackKeyframe(trackId: string, keyframe: TrackKeyframe) {
    const track = this.tracks.get(trackId);
    if (!track) {
      throw new Error(`Track not found: ${trackId}`);
    }
    track.keyframes.push(keyframe);
  }

  setTrackInterpolation(trackId: string, interpolation: InterpolationType) {
    const track = this.tracks.get(trackId);
    if (!track) {
      throw new Error(`Track not found: ${trackId}`);
    }
    track.interpolationType = interpolation;
  }

  setTrackPlayheadParam(trackId: string, playhead: Signal) {
    const track = this.tracks.get(trackId);
    if (!track) {
      throw new Error(`Track not found: ${trackId}`);
    }
    track.playhead = playhead;
  }

  /**
   * Build the final PatchGraph
   */
  toPatch(): PatchGraph {
    return {
      modules: Array.from(this.modules.values()),
      tracks: Array.from(this.tracks.values()),
      scopes: Array.from(this.scopes),
      factories: []
    };
  }

  /**
   * Reset the builder state
   */
  reset(): void {
    this.modules.clear();
    this.tracks.clear();
    this.scopes = [];
    this.counters.clear();
  }

  addTrack(explicitId?: string) {
    const track = new TrackNode(this, this.generateId("track", explicitId));
    this.tracks.set(track.id, {
      id: track.id,
      playhead: 0,
      interpolationType: 'linear',
      keyframes: [],
    })

    return track;
  }

  addScope(value: ModuleOutput | ModuleNode | TrackNode) {
    if (value instanceof TrackNode) {
      this.scopes.push({ type: 'track', trackId: value.id });
      return;
    }

    const output = value instanceof ModuleNode ? value.o : value;
    this.scopes.push({
      type: 'moduleOutput',
      moduleId: output.moduleId,
      portName: output.portName,
    });
  }
}

type Value = number | ModuleOutput | ModuleNode | TrackNode;

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
        if (param) {
          return (value: unknown) => {
            target._setParam(prop, value);
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
          return (target as any)[prop];
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
}

export class TrackNode {
  readonly builder: GraphBuilder;
  readonly id: string;
  private counter: number = 0;

  constructor(
    builder: GraphBuilder,
    id: string,
  ) {
    this.builder = builder;
    this.id = id;
  }

  /**
   * Set the interpolation type for this track.
   */
  interpolation(interpolation: InterpolationType) {
    this.builder.setTrackInterpolation(this.id, interpolation);
    return this;
  }

  /**
   * Set the playhead parameter for this track.
   *
   * The value range [-5.0, 5.0] maps linearly to normalized time [0.0, 1.0].
   */
  playhead(value: Value) {
    this.builder.setTrackPlayheadParam(this.id, valueToSignal(value));
    return this;
  }

  addKeyframe(time: number, value: Value) {
    this.builder.addTrackKeyframe(this.id, {
      id: `keyframe-${this.counter++}`,
      trackId: this.id,
      time,
      signal: valueToSignal(value),
    });

    return this;
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
    if (value instanceof ModuleNode || value instanceof ModuleOutput || value instanceof TrackNode) {
      return valueToSignal(value);
    } else {
      return value
    }
  })
}

function valueToSignal(value: Value): Signal {
  if (value instanceof ModuleNode) {
    value = value.o;
  }
  let signal: Signal;
  if (value instanceof ModuleOutput) {
    signal = {
      type: 'cable',
      module: value.moduleId,
      port: value.portName,
    };
  } else if (value instanceof TrackNode) {
    signal = {
      type: 'track',
      track: value.id,
    };
  } else {
    signal = value
  }

  return signal
}