// import { describe, it, expect } from 'ava';
import { executePatchScript } from '../executor';
import { hz, note } from '../factories';
import { ModuleSchema } from '@modular/core';

const SIGNAL_SCHEMA = {
  oneOf: [
    {
      type: 'object',
      properties: { type: { const: 'volts' }, value: { type: 'number' } },
      required: ['type', 'value'],
    },
    {
      type: 'object',
      properties: {
        type: { const: 'cable' },
        module: { type: 'string' },
        port: { type: 'string' },
      },
      required: ['type', 'module', 'port'],
    },
    {
      type: 'object',
      properties: { type: { const: 'track' }, track: { type: 'string' } },
      required: ['type', 'track'],
    },
    {
      type: 'object',
      properties: { type: { const: 'disconnected' } },
      required: ['type'],
    },
  ],
};

const testSchemas: ModuleSchema[] = [
  {
    name: 'sine',
    description: 'Sine oscillator',
    paramsSchema: {
      type: 'object',
      properties: {
        freq: SIGNAL_SCHEMA,
        phase: SIGNAL_SCHEMA,
      },
    },
    outputs: [{ name: 'output', description: 'Audio output', default: true }],
  },
  {
    name: 'signal',
    description: 'Signal passthrough',
    paramsSchema: {
      type: 'object',
      properties: {
        source: SIGNAL_SCHEMA,
      },
    },
    outputs: [{ name: 'output', description: 'Output signal', default: true }],
  },
  {
    name: 'scaleAndShift',
    description: 'Scale and shift',
    paramsSchema: {
      type: 'object',
      properties: {
        input: SIGNAL_SCHEMA,
        scale: SIGNAL_SCHEMA,
        shift: SIGNAL_SCHEMA,
      },
    },
    outputs: [{ name: 'output', description: 'Output', default: true }],
  },
  {
    name: 'clock',
    description: 'Clock',
    paramsSchema: {
      type: 'object',
      properties: {
        freq: SIGNAL_SCHEMA,
        run: SIGNAL_SCHEMA,
      },
    },
    outputs: [{ name: 'output', description: 'Clock output', default: true }],
  },
];

// describe('DSL Executor', () => {
//   it('should execute a simple sine oscillator patch', () => {
//     const script = `
//       const osc = sine('osc1').freq(hz(440));
//       out.source(osc);
//     `;

//     const patch = executePatchScript(script, testSchemas);
    
//     expect(patch.modules).toHaveLength(3); // osc + root + root_clock
//     expect(patch.modules.find(m => m.id === 'osc1')).toBeDefined();
//     expect(patch.modules.find(m => m.id === 'root')).toBeDefined();
//     expect(patch.scopes).toEqual([]);
//   });

//   it('should handle note helper', () => {
//     const script = `
//       const osc = sine().freq(note('a4'));
//       out.source(osc);
//     `;

//     const patch = executePatchScript(script, testSchemas);
//     const sineModule = patch.modules.find(m => m.moduleType === 'sine');

//     expect(sineModule).toBeDefined();
//     expect((sineModule?.params as any).freq).toEqual({
//       type: 'volts',
//       value: expect.any(Number),
//     });
//   });

//   it('should allow setting data params', () => {
//     const schemasWithDataParams: ModuleSchema[] = [
//       {
//         name: 'sine',
//         description: 'Sine oscillator',
//         paramsSchema: {
//           type: 'object',
//           properties: {
//             freq: SIGNAL_SCHEMA,
//             phase: SIGNAL_SCHEMA,
//             label: { type: 'string', description: 'UI label' },
//             enabled: { type: 'boolean', description: 'Enabled flag' },
//             gain: { type: 'number', description: 'Static gain' },
//           },
//         },
//         outputs: [{ name: 'output', description: 'Audio output', default: true }],
//       },
//       {
//         name: 'signal',
//         description: 'Signal passthrough',
//         paramsSchema: {
//           type: 'object',
//           properties: {
//             source: SIGNAL_SCHEMA,
//           },
//         },
//         outputs: [{ name: 'output', description: 'Output signal', default: true }],
//       },
//       {
//         name: 'clock',
//         description: 'Clock',
//         paramsSchema: {
//           type: 'object',
//           properties: {
//             freq: SIGNAL_SCHEMA,
//             run: SIGNAL_SCHEMA,
//           },
//         },
//         outputs: [{ name: 'output', description: 'Clock output', default: true }],
//       },
//     ];

//     const script = `
//       const osc = sine('osc1').label('hello').enabled(true).gain(0.5).freq(hz(440));
//       out.source(osc);
//     `;

//     const patch = executePatchScript(script, schemasWithDataParams);
//     const sineModule = patch.modules.find(m => m.id === 'osc1');

//     expect((sineModule?.params as any).label).toEqual('hello');
//     expect((sineModule?.params as any).enabled).toEqual(true);
//     expect((sineModule?.params as any).gain).toEqual(0.5);
//   });

//   it('should handle scale and shift', () => {
//     const script = `
//       const osc = sine().freq(hz(440));
//       const scaled = osc.output.scale(0.5).shift(1);
//       out.source(scaled);
//     `;

//     const patch = executePatchScript(script, testSchemas);

//     // Should have sine + scale-and-shift + root
//     expect(patch.modules.length).toBeGreaterThanOrEqual(3);
//     expect(patch.modules.find(m => m.moduleType === 'scaleAndShift')).toBeDefined();
//   });

//   it('allows declaring explicit scopes', () => {
//     const script = `
//       const osc = sine('osc1').freq(hz(440));
//       scope(osc.output);
//       out.source(osc);
//     `;

//     const patch = executePatchScript(script, testSchemas);

//     expect(patch.scopes).toEqual([
//       {
//         type: 'moduleOutput',
//         moduleId: 'osc1',
//         portName: 'output',
//       },
//     ]);
//   });

//   it('should throw error for unknown module type', () => {
//     const script = `
//       const osc = unknownModule();
//     `;

//     expect(() => executePatchScript(script, testSchemas)).toThrow();
//   });
// });

// describe('Helper functions', () => {
//   it('should convert Hz to V/oct correctly', () => {
//     // A4 = 440 Hz should be around 4.75 V/oct
//     const result = hz(440);
//     expect(result).toBeCloseTo(4.0, 1);
//   });

//   it('should convert note names to V/oct', () => {
//     const a4 = note('a4');
//     const c4 = note('c4');
    
//     expect(a4).toBeCloseTo(hz(440), 2);
//     expect(c4).toBeCloseTo(hz(261.63), 2);
//   });

//   it('should handle sharps and flats', () => {
//     const cSharp4 = note('c#4');
//     const dFlat4 = note('db4');
    
//     // C# and Db should be the same
//     expect(cSharp4).toBeCloseTo(dFlat4, 2);
//   });

//   it('should throw error for invalid note names', () => {
//     expect(() => note('invalid')).toThrow();
//     expect(() => note('h4')).toThrow();
//   });
// });

