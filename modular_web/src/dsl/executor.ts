import type { PatchGraph, ModuleSchema } from '../types';
import { DSLContext, createOutputHelper, hz, note, volts } from './factories';

/**
 * Execute a DSL script and return the resulting PatchGraph
 */
export function executePatchScript(
  source: string,
  schemas: ModuleSchema[]
): PatchGraph {
  // Create DSL context
  const context = new DSLContext(schemas);
  const out = createOutputHelper(context);

  // Create the execution environment with all DSL functions
  const dslGlobals = {
    // Module factories
    sine: context.sine,
    saw: context.saw,
    pulse: context.pulse,
    signal: context.signal,
    scaleAndShift: context.scaleAndShift,
    sum: context.sum,
    mix: context.mix,
    lowpass: context.lowpass,
    highpass: context.highpass,
    bandpass: context.bandpass,
    notch: context.notch,
    allpass: context.allpass,
    stateVariable: context.stateVariable,
    moogLadder: context.moogLadder,
    tb303: context.tb303,
    sem: context.sem,
    ms20: context.ms20,
    formant: context.formant,
    sallenKey: context.sallenKey,

    // Helper functions
    hz,
    note,
    volts,

    // Output helper
    out,
  };

  // Build the function body
  const functionBody = `
    'use strict';
    ${source}
  `;

  // Create parameter names and values
  const paramNames = Object.keys(dslGlobals);
  const paramValues = Object.values(dslGlobals);

  try {
    // Execute the script
    const fn = new Function(...paramNames, functionBody);
    fn(...paramValues);

    // Build and return the patch
    return context.getBuilder().toPatch();
  } catch (error) {
    if (error instanceof Error) {
      throw new Error(`DSL execution error: ${error.message}`);
    }
    throw error;
  }
}

/**
 * Validate DSL script syntax without executing
 */
export function validateDSLSyntax(source: string): { valid: boolean; error?: string } {
  try {
    new Function(source);
    return { valid: true };
  } catch (error) {
    if (error instanceof Error) {
      return { valid: false, error: error.message };
    }
    return { valid: false, error: 'Unknown syntax error' };
  }
}

