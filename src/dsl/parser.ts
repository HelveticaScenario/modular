
import { ASTNode, PatternProgram } from '@modular/core';
import { hz, note } from './factories';
import grammar from './mini.ohm-bundle'





/**
 * Parse a musical DSL string into a PatternProgram
 */
export function parsePattern(id: string, input: string): Array<ASTNode> {
    const match = grammar.match(input);

    if (match.failed()) {
        throw new Error(`Parse error: ${match.message}`);
    }

    const semantics = grammar.createSemantics();

    semantics.addOperation('toAST', {
        Program(elements) {
            return elements.children.map((e: any) => e.toAST());
        },

        Element(node) {
            return node.toAST();
        },

        NonRandomElement(node) {
            return node.toAST();
        },

        FastSubsequence(_open, elements, _close): ASTNode {
            return {
                type: 'FastSubsequence',
                elements: elements.children.map((e: any) => e.toAST())
            };
        },

        SlowSubsequence(_open, elements, _close): ASTNode {
            return {
                type: 'SlowSubsequence',
                elements: elements.children.map((e: any) => e.toAST())
            };
        },

        RandomChoice(first, _pipes, rest): ASTNode {
            return {
                type: 'RandomChoice',
                choices: [first.toAST(), ...rest.children.map((e: any) => e.toAST())]
            };
        },

        Value(node) {
            return node.toAST();
        },

        Rest(_tilde): ASTNode {
            return {
                type: 'Rest'
            };
        },

        HzValue(sign, whole, _dot, frac, suffix): ASTNode {
            const signStr = sign.sourceString;
            const wholeStr = whole.sourceString;
            const fracStr = frac.sourceString;
            const suffixStr = suffix.sourceString.toLowerCase();

            let value = parseFloat(signStr + wholeStr + (fracStr ? '.' + fracStr : ''));

            // Convert khz to hz
            if (suffixStr === 'khz') {
                value *= 1000;
            }

            // Convert Hz to V/oct
            const voct = hz(value);

            return {
                type: 'NumericLiteral',
                value: voct
            };
        },

        NumericLiteral(sign, whole, _dot, frac): ASTNode {
            const signStr = sign.sourceString;
            const wholeStr = whole.sourceString;
            const fracStr = frac.sourceString;
            const value = parseFloat(signStr + wholeStr + (fracStr ? '.' + fracStr : ''));
            return {
                type: 'NumericLiteral',
                value
            };
        },

        NoteName(letter, accidental, octave): ASTNode {
            const noteName = letter.sourceString + accidental.sourceString + octave.sourceString;
            const voct = note(noteName);
            return {
                type: 'NumericLiteral',
                value: voct
            };
        },

        MidiValue(_m, digits): ASTNode {
            // MIDI note to V/oct: V/oct = (MIDI - 69) / 12
            const midiNote = parseInt(digits.sourceString, 10);
            const voct = (midiNote - 69) / 12;
            return {
                type: 'NumericLiteral',
                value: voct
            };
        }
    });

    const elements = semantics(match).toAST();


    return elements
}
