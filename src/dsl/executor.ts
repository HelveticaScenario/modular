
import { ModuleSchema, PatchGraph } from '@modular/core';
import { DSLContext, hz, note, bpm } from './factories';


/**
 * Execute a DSL script and return the resulting PatchGraph
 */
export function executePatchScript(
  source: string,
  schemas: ModuleSchema[]
): PatchGraph {
  // Create DSL context
  console.log('Executing DSL script with schemas:', schemas);
  const context = new DSLContext(schemas);
  const out = context.factories.signal(undefined, { id: 'root' });

  // Create default clock module that runs at 120 BPM
  const rootClock = context.factories.clock(bpm(120), { id: 'root_clock' });
  console.log('Created clock module:', rootClock);


  // Create the execution environment with all DSL functions
  const dslGlobals = {
    ...context.factories,
    scope: context.scope.bind(context),
    // Helper functions
    hz,
    note,
    bpm,

    // Built-in modules
    out,
    rootClock,
  };

  console.log(dslGlobals)

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
    const patch = context.getBuilder().toPatch()
    console.log(patch);
    return patch;
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

