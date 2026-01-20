
import { ModuleSchema } from '@modular/core';
import { GraphBuilder, ModuleNode, ModuleOutput } from './GraphBuilder';

type FactoryFunction = (...args: any[]) => ModuleNode;

type NamespaceTree = {
  [key: string]: NamespaceTree | FactoryFunction;
};

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

/**
 * Build a nested namespace tree from module schemas
 * Mirrors the logic in typescriptLibGen.ts buildTreeFromSchemas()
 */
function buildNamespaceTree(
  schemas: ModuleSchema[],
  factoryMap: Record<string, FactoryFunction>
): NamespaceTree {
  const tree: NamespaceTree = {};

  for (const schema of schemas) {
    const fullName = schema.name.trim();
    const parts = fullName.split('.').filter(p => p.length > 0);

    const factoryName = sanitizeIdentifier(fullName);
    const factory = factoryMap[factoryName];

    if (parts.length === 1) {
      // No namespace, add to root
      tree[parts[0]] = factory;
    } else {
      // Navigate/create namespace hierarchy
      const className = parts[parts.length - 1];
      const namespacePath = parts.slice(0, -1);

      let current: NamespaceTree = tree;
      for (const ns of namespacePath) {
        if (!current[ns]) {
          current[ns] = {};
        } else if (typeof current[ns] === 'function') {
          throw new Error(
            `Namespace collision: ${ns} is both a module and a namespace`
          );
        }
        current = current[ns] as NamespaceTree;
      }

      if (current[className] && typeof current[className] !== 'function') {
        throw new Error(
          `Module name collision: ${className} already exists as a namespace`
        );
      }
      current[className] = factory;
    }
  }

  return tree;
}

/**
 * DSL Context holds the builder and provides factory functions
 */
export class DSLContext {
  factories: Record<string, FactoryFunction> = {};
  namespaceTree: NamespaceTree = {};
  private builder: GraphBuilder;

  constructor(schemas: ModuleSchema[]) {
    this.builder = new GraphBuilder(schemas);

    // Build flat factory map (internal use for tree building)
    for (const schema of schemas) {
      const factoryName = sanitizeIdentifier(schema.name);
      this.factories[factoryName] = this.createFactory(schema);
    }

    // Build namespace tree (only way to access modules)
    this.namespaceTree = buildNamespaceTree(schemas, this.factories);
  }

  /**
   * Create a module factory function
   */
  private createFactory(schema: ModuleSchema) {
    return (...args: any[]): ModuleNode => {
      // @ts-ignore
      const positionalArgs = schema.positionalArgs || [];
      const params: Record<string, any> = {};
      let config: any = {};
      let id: string | undefined;

      // Extract positional args
      for (let i = 0; i < positionalArgs.length; i++) {
        if (i < args.length) {
            params[positionalArgs[i].name] = args[i];
        }
      }
      
      // The remaining arg (if any) is config.
      if (args.length > positionalArgs.length) {
          config = args[positionalArgs.length];
      }
      
      if (config) {
          if (typeof config === 'string') {
              id = config;
          } else {
              id = config.id;
              // Merge other config params
              for (const key in config) {
                  if (key !== 'id') {
                      params[key] = config[key];
                  }
              }
          }
      }
      
      const node = this.builder.addModule(schema.name, id);
      
      // Set params
      for (const [key, value] of Object.entries(params)) {
          if (value !== undefined) {
              node._setParam(key, value);
          }
      }
      
      return node;
    };
  }

  /**
   * Get the builder instance
   */
  getBuilder(): GraphBuilder {
    return this.builder;
  }

  scope(target: ModuleNode | ModuleOutput, msPerFrame: number = 500, triggerThreshold?: number) {
    this.builder.addScope(target, msPerFrame, triggerThreshold);
    return target
  }
}

/**
 * Helper function to convert Hz to V/oct
 * V/oct = log2(Hz / 55)
 */
export function hz(frequency: number): number {
  if (frequency <= 0) {
    throw new Error('Frequency must be positive');
  }
  return Math.log2(frequency / 55);
}

/**
 * Note name to V/oct conversion
 * Supports notes like "c4", "c#4", "db4", etc.
 */
export function note(noteName: string): number {
  const noteRegex = /^([a-g])([#b]?)(-?\d+)?$/i;
  const match = noteName.toLowerCase().match(noteRegex);

  if (!match) {
    throw new Error(`Invalid note name: ${noteName}`);
  }

  const [, noteLetter, accidental, octaveStr] = match;
  const octave = octaveStr ? parseInt(octaveStr, 10) : 3;

  // Map note letters to semitones (C = 0)
  const noteMap: Record<string, number> = {
    'c': 0, 'd': 2, 'e': 4, 'f': 5, 'g': 7, 'a': 9, 'b': 11
  };

  let semitone = noteMap[noteLetter];

  // Apply accidentals
  if (accidental === '#') {
    semitone += 1;
  } else if (accidental === 'b') {
    semitone -= 1;
  }

  // Calculate frequency: A0 = 55 Hz
  const semitonesFromA0 = octave * 12 + semitone - 9;
  const frequency = 55 * Math.pow(2, semitonesFromA0 / 12);

  return hz(frequency);
}

/**
 * Convert BPM (beats per minute) to V/oct frequency
 * BPM is tempo, where 1 beat = 1 quarter note
 * At 120 BPM, that's 2 beats per second = 2 Hz
 */
export function bpm(beatsPerMinute: number): number {
  if (beatsPerMinute <= 0) {
    throw new Error('BPM must be positive');
  }
  // Convert BPM to Hz: Hz = BPM / 60
  const frequency = beatsPerMinute / 60;
  return hz(frequency);
}


