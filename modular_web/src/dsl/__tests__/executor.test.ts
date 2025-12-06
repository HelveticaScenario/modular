import { describe, it, expect } from 'vitest';
import { executePatchScript } from '../executor';
import { hz, note } from '../factories';
import type { ModuleSchema } from '../../types';

const testSchemas: ModuleSchema[] = [
  {
    name: 'sine-osc',
    description: 'Sine oscillator',
    params: [
      { name: 'freq', description: 'Frequency in V/oct' },
      { name: 'phase', description: 'Phase' },
    ],
    outputs: [{ name: 'output', description: 'Audio output' }],
  },
  {
    name: 'signal',
    description: 'Signal passthrough',
    params: [{ name: 'source', description: 'Input signal' }],
    outputs: [{ name: 'output', description: 'Output signal' }],
  },
  {
    name: 'scale-and-shift',
    description: 'Scale and shift',
    params: [
      { name: 'input', description: 'Input' },
      { name: 'scale', description: 'Scale factor' },
      { name: 'shift', description: 'Shift amount' },
    ],
    outputs: [{ name: 'output', description: 'Output' }],
  },
];

describe('DSL Executor', () => {
  it('should execute a simple sine oscillator patch', () => {
    const script = `
      const osc = sine('osc1').freq(hz(440));
      out.source(osc);
    `;

    const patch = executePatchScript(script, testSchemas);
    
    expect(patch.modules).toHaveLength(2); // osc + root
    expect(patch.modules.find(m => m.id === 'osc1')).toBeDefined();
    expect(patch.modules.find(m => m.id === 'root')).toBeDefined();
  });

  it('should handle note helper', () => {
    const script = `
      const osc = sine().freq(note('a4'));
      out.source(osc);
    `;

    const patch = executePatchScript(script, testSchemas);
    const sineModule = patch.modules.find(m => m.module_type === 'sine-osc');

    expect(sineModule).toBeDefined();
    expect(sineModule?.params.freq).toEqual({
      param_type: 'value',
      value: expect.any(Number),
    });
  });

  it('should handle scale and shift', () => {
    const script = `
      const osc = sine().freq(hz(440));
      const scaled = osc.output.scale(0.5).shift(1);
      out.source(scaled);
    `;

    const patch = executePatchScript(script, testSchemas);

    // Should have sine + scale-and-shift + root
    expect(patch.modules.length).toBeGreaterThanOrEqual(3);
    expect(patch.modules.find(m => m.module_type === 'scale-and-shift')).toBeDefined();
  });

  it('should throw error for unknown module type', () => {
    const script = `
      const osc = unknownModule();
    `;

    expect(() => executePatchScript(script, testSchemas)).toThrow();
  });
});

describe('Helper functions', () => {
  it('should convert Hz to V/oct correctly', () => {
    // A4 = 440 Hz should be around 4.75 V/oct
    const result = hz(440);
    expect(result).toBeCloseTo(4.0, 1);
  });

  it('should convert note names to V/oct', () => {
    const a4 = note('a4');
    const c4 = note('c4');
    
    expect(a4).toBeCloseTo(hz(440), 2);
    expect(c4).toBeCloseTo(hz(261.63), 2);
  });

  it('should handle sharps and flats', () => {
    const cSharp4 = note('c#4');
    const dFlat4 = note('db4');
    
    // C# and Db should be the same
    expect(cSharp4).toBeCloseTo(dFlat4, 2);
  });

  it('should throw error for invalid note names', () => {
    expect(() => note('invalid')).toThrow();
    expect(() => note('h4')).toThrow();
  });
});

