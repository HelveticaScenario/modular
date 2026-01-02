import test from 'ava';
import { parsePattern } from '../parser';
import { hz, note } from '../factories';
import { ASTNode } from '@modular/core';
// test('should parse numeric literals', (t) => {
//     const result = parsePattern('test', '1.5 2.0 -3.5');
//     t.is(result.id, 'test');
//     t.is(result.elements.length, 3);
//     t.deepEqual<ASTNode, ASTNode>(result.elements[0], { NumericLiteral: { literal: { value: 1.5 } } });
//     t.deepEqual<ASTNode, ASTNode>(result.elements[1], { NumericLiteral: { literal: { value: 2.0 } } });
//     t.deepEqual<ASTNode, ASTNode>(result.elements[2], { NumericLiteral: { literal: { value: -3.5 } } });
// });
// test('should parse rest character', (t) => {
//     const result = parsePattern('test', '~ 1.0 ~');
//     t.is(result.elements.length, 3);
//     t.is(result.elements[0].type, 'Rest');
//     t.deepEqual<ASTNode, ASTNode>(result.elements[1], { NumericLiteral: { literal: { value: 1.0 } } });
//     t.is(result.elements[2].type, 'Rest');
// });
// test('should parse note names', (t) => {
//     const result = parsePattern('test', 'c4 c#4 db5');
//     t.is(result.elements.length, 3);
//     t.deepEqual<ASTNode, ASTNode>(result.elements[0], { NumericLiteral: { literal: { value: note('c4') } } });
//     t.deepEqual<ASTNode, ASTNode>(result.elements[1], { NumericLiteral: { literal: { value: note('c#4') } } });
//     t.deepEqual<ASTNode, ASTNode>(result.elements[2], { NumericLiteral: { literal: { value: note('db5') } } });
// });
// test('should parse Hz values', (t) => {
//     const result = parsePattern('test', '440hz 880hz 1khz');
//     t.is(result.elements.length, 3);
//     t.deepEqual<ASTNode, ASTNode>(result.elements[0], { NumericLiteral: { literal: { value: hz(440) } } });
//     t.deepEqual<ASTNode, ASTNode>(result.elements[1], { NumericLiteral: { literal: { value: hz(880) } } });
//     t.deepEqual<ASTNode, ASTNode>(result.elements[2], { NumericLiteral: { literal: { value: hz(1000) } } });
// });
// test('should parse MIDI values', (t) => {
//     const result = parsePattern('test', 'm60 m69');
//     t.is(result.elements.length, 2);
//     t.deepEqual<ASTNode, ASTNode>(result.elements[0], {
//         NumericLiteral: { literal: { value: (60 - 69) / 12 } },
//     });
//     t.deepEqual<ASTNode, ASTNode>(result.elements[1], {
//         NumericLiteral: { literal: { value: (69 - 69) / 12 } },
//     });
// });
// test('should parse fast subsequences', (t) => {
//     const result = parsePattern('test', '[1.0 2.0 3.0]');
//     t.is(result.elements.length, 1);
//     t.deepEqual<ASTNode, ASTNode>(result.elements[0], {
//         FastSubsequence: {
//             subsequence: {
//                 elements: [
//                     { NumericLiteral: { literal: { value: 1.0 } } },
//                     { NumericLiteral: { literal: { value: 2.0 } } },
//                     { NumericLiteral: { literal: { value: 3.0 } } },
//                 ],
//             }
//         },
//     });
// });
// test('should parse slow subsequences', (t) => {
//     const result = parsePattern('test', '<c4 d4 e4>');
//     t.is(result.elements.length, 1);
//     t.deepEqual<ASTNode, ASTNode>(result.elements[0], {
//         SlowSubsequence: {
//             subsequence: {
//                 elements: [
//                     { NumericLiteral: { literal: { value: note('c4') } } },
//                     { NumericLiteral: { literal: { value: note('d4') } } },
//                     { NumericLiteral: { literal: { value: note('e4') } } },
//                 ],
//             }  { NumericLiteral: { value: note('d4') } },
//                 { NumericLiteral: { value: note('e4') } },
//             ],
//         },
//     });
// });
// test('should parse nested subsequences', (t) => {
//     const resubsequence: {
//                 elements: [
//                     {
//                         FastSubsequence: {
//                             subsequence: {
//                                 elements: [
//                                     { NumericLiteral: { literal: { value: 1.0 } } },
//                                     { NumericLiteral: { literal: { value: 2.0 } } },
//                                 ],
//                             }
//                         },
//                     },
//                     {
//                         FastSubsequence: {
//                             subsequence: {
//                                 elements: [
//                                     { NumericLiteral: { literal: { value: 3.0 } } },
//                                     { NumericLiteral: { literal: { value: 4.0 } } },
//                                 ],
//                             }
//                         },
//                     },
//                 ],
//             }              { NumericLiteral: { value: 4.0 } },
//                         ],
//                     },
//                 },: {
//                 choices: [
//                     { NumericLiteral: { literal: { value: 1.0 } } },
//                     { NumericLiteral: { literal: { value: 2.0 } } },
//                     { NumericLiteral: { literal: { value: 3.0 } } },
//                 ],
//             }
// test('should parse random choices', (t) => {
//     const result = parsePattern('test', '1.0 | 2.0 | 3.0');
//     t.is(result.elements.length, 1);
//     t.deepEqual(result.elements[0], {
//         RandomChoice: {
//             choices: [
//                 { : {
//                 choices: [
//                     { NumericLiteral: { literal: { value: note('c4') } } },
//                     { NumericLiteral: { literal: { value: note('e4') } } },
//                     { NumericLiteral: { literal: { value: note('g4') } } },
//                 ],
//             }
//     });
// });
// test('should parse random choices with notes', (t) => {
//     const result = parsePattern('test', 'c4 | e4 | g4');
//     t.is(result.elements.length, 1);
//     t.deepEqual(result.elements[0], {
//         RandomChoice: {
//             choices: [
//                 { : {
//                 choices: [{ NumericLiteral: { literal: { value: note('c4') } } }, 'Rest'],
//             }
//                 { NumericLiteral: { value: note('e4') } },
//                 { NumericLiteral: { value: note('g4') } },
//             ],
//         },
//     });
// });
// test('shouldsubsequence: {
//                 elements: [
//                     { NumericLiteral: { literal: { value: hz(440) } } },
//                     { NumericLiteral: { literal: { value: hz(880) } } },
//                     { NumericLiteral: { literal: { value: hz(1760) } } },
//                 ],
//             }Choice: {
//             choices: [{ NumericLiteral: { value: note('c4') } }, 'Rest'],
//         },
//     });
// });
// test('should parse Hz values in subsequences', (t) => {
//     const result = parsePattern('test', '[440hz 880hz 1.76khz]');
//     t.is(result.elements.length, 1);
//     t.deepEqual(result.elements[0], {
//         FastSubsequence: {
//             elements: [
//                 { NumericLiteral: { value: hz(440) } },
//                 { NumericLiteral: { value: hz(880) } },
//                 { NumericLiteral: { value: hz(1760) } },
//             ],
//         },
//     });
// });
// test('should throw on invalid syntax', (t) => {
//     t.throws(() => parsePattern('test', '['));
//     t.throws(() => parsePattern('test', '<'));
//     t.throws(() => parsePattern('test', '|'));
// });
