
import { ModuleSchema } from '@modular/core';
import { GraphBuilder, ModuleNode, ModuleOutput } from './GraphBuilder';

type FactoryFunction = (id?: string) => ModuleNode;

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
 * DSL Context holds the builder and provides factory functions
 */
export class DSLContext {
  factories: Record<string, FactoryFunction> = {};
  private builder: GraphBuilder;

  constructor(schemas: ModuleSchema[]) {
    this.builder = new GraphBuilder(schemas);
    for (const schema of schemas) {
      const factoryName = sanitizeIdentifier(schema.name);
      this.factories[factoryName] = this.createFactory(schema);
    }
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
 * V/oct = log2(Hz / 27.5)
 */
export function hz(frequency: number): number {
  if (frequency <= 0) {
    throw new Error('Frequency must be positive');
  }
  return Math.log2(frequency / 27.5);
}

/**
 * Note name to V/oct conversion
 * Supports notes like "c4", "c#4", "db4", etc.
 */
export function note(noteName: string): number {
  const noteRegex = /^([a-g])([#b]?)(-?\d+)$/i;
  const match = noteName.toLowerCase().match(noteRegex);

  if (!match) {
    throw new Error(`Invalid note name: ${noteName}`);
  }

  const [, noteLetter, accidental, octaveStr] = match;
  const octave = parseInt(octaveStr, 10);

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

  // Calculate frequency: A4 = 440 Hz, A4 is 9 semitones above C4
  // C4 is octave 4, semitone 0
  const semitonesFromC4 = (octave - 4) * 12 + semitone;
  const frequency = 440 * Math.pow(2, (semitonesFromC4 - 9) / 12);

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


