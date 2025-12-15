import type { ModuleSchema } from "../types/generated/ModuleSchema";
import type { ModuleState } from "../types/generated/ModuleState";
import type { Param } from "../types/generated/Param";
import type { PatchGraph } from "../types/generated/PatchGraph";
import type { InterpolationType } from "../types/generated/InterpolationType";
import type { Track } from "../types/generated/Track";
import type { TrackKeyframe } from "../types/generated/TrackKeyframe";
import type { ScopeItem } from "../types/generated/ScopeItem";

const NUM_CHANNELS = 16;
type ParamArray = Param[];
type ParamInput = Value | Param | ParamArray;

const isParam = (value: unknown): value is Param =>
  typeof value === 'object' && value !== null && 'type' in (value as Record<string, unknown>);

const cloneParam = (param: Param): Param => ({ ...param }) as Param;

const fillParamArray = (param: Param): ParamArray =>
  Array.from({ length: NUM_CHANNELS }, () => cloneParam(param));

const disconnectedParams = (): ParamArray =>
  fillParamArray({ type: 'disconnected' });

const normalizeParamInput = (value: ParamInput): ParamArray => {
  // Allow callers to pass an explicit array (must be correct length or empty)
  if (Array.isArray(value)) {
    if (value.length === 0) return disconnectedParams();
    if (value.length !== NUM_CHANNELS) {
      throw new Error(`Param arrays must contain exactly ${NUM_CHANNELS} entries`);
    }
    return value;
  }

  let paramLike: Value | Param = value;

  if (value instanceof ModuleNode) {
    paramLike = value.o;
  }

  if (value instanceof ModuleOutput) {
    return fillParamArray({
      type: 'cable',
      module: value.moduleId,
      port: value.portName,
    });
  }

  if (value instanceof TrackNode) {
    return fillParamArray({
      type: 'track',
      track: value.id,
    });
  }

  if (isParam(paramLike)) {
    return fillParamArray(paramLike);
  }

  return fillParamArray({
    type: 'value',
    value: paramLike as number,
  });
};

/**
 * GraphBuilder manages the construction of a PatchGraph from DSL code.
 * It tracks modules, generates deterministic IDs, and builds the final graph.
 */
export class GraphBuilder {

  private modules: Map<string, ModuleState> = new Map();
  private tracks: Map<string, Track> = new Map();
  private counters: Map<string, number> = new Map();
  private schemas: ModuleSchema[] = [];
  private scopes: ScopeItem[] = [];


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
    const schema = this.schemas.find(s => s.name === moduleType);
    if (!schema) {
      throw new Error(`Unknown module type: ${moduleType}`);
    }

    // Initialize module with disconnected params
    const params: Record<string, ParamArray> = {};
    for (const param of schema.params) {
      params[param.name] = disconnectedParams();
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
  setParam(moduleId: string, paramName: string, value: ParamArray): void {
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

  setTrackPlayheadParam(trackId: string, playhead: ParamArray) {
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
      playhead: normalizeParamInput({ type: 'value', value: 0 }),
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
    const proxy = new Proxy(this, {
      get(target, prop: string) {

        // Check if it's a parameter name
        const paramSchema = target.schema.params.find(p => p.name === prop);
        if (paramSchema) {
          // Return a function that sets the parameter
          return (value: Value) => {
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

  /**
   * Set a parameter to a constant value
   */
  _setParam(paramName: string, value: ParamInput): this {
    const paramArray = normalizeParamInput(value);
    this.builder.setParam(this.id, paramName, paramArray);

    return this;
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
  playhead(value: ParamInput) {
    const paramArray = normalizeParamInput(value);
    this.builder.setTrackPlayheadParam(this.id, paramArray);
    return this;
  }

  addKeyframe(time: number, value: ParamInput) {
    const param = normalizeParamInput(value);

    this.builder.addTrackKeyframe(this.id, {
      id: `keyframe-${this.counter++}`,
      trackId: this.id,
      time,
      param,
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