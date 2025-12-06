import type { ModuleSchema } from '../types';
import { GraphBuilder, ModuleNode } from './GraphBuilder';

/**
 * DSL Context holds the builder and provides factory functions
 */
export class DSLContext {
  private builder: GraphBuilder;
  private rootModule: ModuleNode | null = null;

  constructor(schemas: ModuleSchema[]) {
    this.builder = new GraphBuilder(schemas);
  }

  /**
   * Create a module factory function
   */
  private createFactory(moduleType: string) {
    return (id?: string): ModuleNode => {
      return this.builder.addModule(moduleType, id);
    };
  }

  /**
   * Get the builder instance
   */
  getBuilder(): GraphBuilder {
    return this.builder;
  }

  /**
   * Set the root output
   */
  setRoot(source: ModuleNode | { output: () => any }): void {
    if (!this.rootModule) {
      this.rootModule = this.builder.addModule('signal', 'root');
    }
    
    const output = 'output' in source ? source.output() : source;
    this.rootModule.connect('source', output);
  }

  // Oscillator factories
  sine = this.createFactory('sine-osc');
  saw = this.createFactory('saw-osc');
  pulse = this.createFactory('pulse-osc');

  // Core utilities
  signal = this.createFactory('signal');
  scaleAndShift = this.createFactory('scale-and-shift');
  sum = this.createFactory('sum');
  mix = this.createFactory('mix');

  // Filter factories
  lowpass = this.createFactory('lowpass-filter');
  highpass = this.createFactory('highpass-filter');
  bandpass = this.createFactory('bandpass-filter');
  notch = this.createFactory('notch-filter');
  allpass = this.createFactory('allpass-filter');
  stateVariable = this.createFactory('state-variable-filter');
  moogLadder = this.createFactory('moog-ladder-filter');
  tb303 = this.createFactory('tb303-filter');
  sem = this.createFactory('sem-filter');
  ms20 = this.createFactory('ms20-filter');
  formant = this.createFactory('formant-filter');
  sallenKey = this.createFactory('sallen-key-filter');
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
 * Convert volts to V/oct (identity function, for clarity)
 */
export function volts(value: number): number {
  return value;
}

/**
 * Output object for setting the root module
 */
export function createOutputHelper(context: DSLContext) {
  return {
    source(node: ModuleNode | { output: () => any }): void {
      context.setRoot(node);
    }
  };
}

