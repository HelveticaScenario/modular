import { CompletionContext } from '@codemirror/autocomplete';
import type { CompletionResult, Completion } from '@codemirror/autocomplete';
import type { ModuleSchema } from '../types';

/**
 * Build autocomplete completions from module schemas
 */
export function buildCompletions(schemas: ModuleSchema[]): Completion[] {
  const completions: Completion[] = [];

  // Add module factory functions
  for (const schema of schemas) {
    completions.push({
      label: schema.name,
      type: 'function',
      detail: schema.description,
      info: `Module: ${schema.description}\nParams: ${schema.params.map(p => p.name).join(', ')}`,
    });
  }

  // Add helper functions
  completions.push(
    {
      label: 'hz',
      type: 'function',
      detail: 'Convert frequency to V/oct',
      info: 'hz(frequency: number): number\nConverts Hz to V/oct voltage',
    },
    {
      label: 'note',
      type: 'function',
      detail: 'Convert note name to V/oct',
      info: 'note(name: string): number\nConverts note name (e.g., "c4", "a#3") to V/oct voltage',
    },
    {
      label: 'volts',
      type: 'function',
      detail: 'Pass through voltage value',
      info: 'volts(value: number): number\nIdentity function for clarity',
    },
    {
      label: 'out',
      type: 'variable',
      detail: 'Output helper',
      info: 'out.source(node)\nSet the root output source',
    }
  );

  return completions;
}

/**
 * Create a CodeMirror autocomplete extension
 */
export function dslAutocomplete(schemas: ModuleSchema[]) {
  const completions = buildCompletions(schemas);

  return (context: CompletionContext): CompletionResult | null => {
    const word = context.matchBefore(/\w*/);
    if (!word || (word.from === word.to && !context.explicit)) {
      return null;
    }

    return {
      from: word.from,
      options: completions,
    };
  };
}

