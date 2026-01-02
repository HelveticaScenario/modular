'use strict';
const { makeRecipe } = require('ohm-js');
const result = makeRecipe([
    'grammar',
    {
        source: 'MusicalDSL {\n  Program = Element*\n\n  Element = RandomChoice\n          | NonRandomElement\n\n  NonRandomElement = FastSubsequence\n                   | SlowSubsequence\n                   | Value\n\n  FastSubsequence = "[" Element* "]"\n  \n  SlowSubsequence = "<" Element* ">"\n\n  RandomChoice = NonRandomElement ("|" NonRandomElement)+\n\n  Value = Rest\n        | HzValue\n        | NumericLiteral\n        | NoteName\n        | MidiValue\n\n  Rest = "~"\n\n  HzValue = "-"? digit+ ("." digit+)? ("hz" | "khz")\n\n  NumericLiteral = "-"? digit+ ("." digit+)?\n\n  NoteName = letter accidental? digit+\n\n  MidiValue = "m" digit+\n\n  accidental = "#" | "b"\n}',
    },
    'MusicalDSL',
    null,
    'Program',
    {
        Program: [
            'define',
            { sourceInterval: [15, 33] },
            null,
            [],
            [
                'star',
                { sourceInterval: [25, 33] },
                ['app', { sourceInterval: [25, 32] }, 'Element', []],
            ],
        ],
        Element: [
            'define',
            { sourceInterval: [37, 88] },
            null,
            [],
            [
                'alt',
                { sourceInterval: [47, 88] },
                ['app', { sourceInterval: [47, 59] }, 'RandomChoice', []],
                ['app', { sourceInterval: [72, 88] }, 'NonRandomElement', []],
            ],
        ],
        NonRandomElement: [
            'define',
            { sourceInterval: [92, 190] },
            null,
            [],
            [
                'alt',
                { sourceInterval: [111, 190] },
                ['app', { sourceInterval: [111, 126] }, 'FastSubsequence', []],
                ['app', { sourceInterval: [148, 163] }, 'SlowSubsequence', []],
                ['app', { sourceInterval: [185, 190] }, 'Value', []],
            ],
        ],
        FastSubsequence: [
            'define',
            { sourceInterval: [194, 228] },
            null,
            [],
            [
                'seq',
                { sourceInterval: [212, 228] },
                ['terminal', { sourceInterval: [212, 215] }, '['],
                [
                    'star',
                    { sourceInterval: [216, 224] },
                    ['app', { sourceInterval: [216, 223] }, 'Element', []],
                ],
                ['terminal', { sourceInterval: [225, 228] }, ']'],
            ],
        ],
        SlowSubsequence: [
            'define',
            { sourceInterval: [234, 268] },
            null,
            [],
            [
                'seq',
                { sourceInterval: [252, 268] },
                ['terminal', { sourceInterval: [252, 255] }, '<'],
                [
                    'star',
                    { sourceInterval: [256, 264] },
                    ['app', { sourceInterval: [256, 263] }, 'Element', []],
                ],
                ['terminal', { sourceInterval: [265, 268] }, '>'],
            ],
        ],
        RandomChoice: [
            'define',
            { sourceInterval: [272, 327] },
            null,
            [],
            [
                'seq',
                { sourceInterval: [287, 327] },
                ['app', { sourceInterval: [287, 303] }, 'NonRandomElement', []],
                [
                    'plus',
                    { sourceInterval: [304, 327] },
                    [
                        'seq',
                        { sourceInterval: [305, 325] },
                        ['terminal', { sourceInterval: [305, 308] }, '|'],
                        [
                            'app',
                            { sourceInterval: [309, 325] },
                            'NonRandomElement',
                            [],
                        ],
                    ],
                ],
            ],
        ],
        Value: [
            'define',
            { sourceInterval: [331, 425] },
            null,
            [],
            [
                'alt',
                { sourceInterval: [339, 425] },
                ['app', { sourceInterval: [339, 343] }, 'Rest', []],
                ['app', { sourceInterval: [354, 361] }, 'HzValue', []],
                ['app', { sourceInterval: [372, 386] }, 'NumericLiteral', []],
                ['app', { sourceInterval: [397, 405] }, 'NoteName', []],
                ['app', { sourceInterval: [416, 425] }, 'MidiValue', []],
            ],
        ],
        Rest: [
            'define',
            { sourceInterval: [429, 439] },
            null,
            [],
            ['terminal', { sourceInterval: [436, 439] }, '~'],
        ],
        HzValue: [
            'define',
            { sourceInterval: [443, 493] },
            null,
            [],
            [
                'seq',
                { sourceInterval: [453, 493] },
                [
                    'opt',
                    { sourceInterval: [453, 457] },
                    ['terminal', { sourceInterval: [453, 456] }, '-'],
                ],
                [
                    'plus',
                    { sourceInterval: [458, 464] },
                    ['app', { sourceInterval: [458, 463] }, 'digit', []],
                ],
                [
                    'opt',
                    { sourceInterval: [465, 478] },
                    [
                        'seq',
                        { sourceInterval: [466, 476] },
                        ['terminal', { sourceInterval: [466, 469] }, '.'],
                        [
                            'plus',
                            { sourceInterval: [470, 476] },
                            [
                                'app',
                                { sourceInterval: [470, 475] },
                                'digit',
                                [],
                            ],
                        ],
                    ],
                ],
                [
                    'alt',
                    { sourceInterval: [480, 492] },
                    ['terminal', { sourceInterval: [480, 484] }, 'hz'],
                    ['terminal', { sourceInterval: [487, 492] }, 'khz'],
                ],
            ],
        ],
        NumericLiteral: [
            'define',
            { sourceInterval: [497, 539] },
            null,
            [],
            [
                'seq',
                { sourceInterval: [514, 539] },
                [
                    'opt',
                    { sourceInterval: [514, 518] },
                    ['terminal', { sourceInterval: [514, 517] }, '-'],
                ],
                [
                    'plus',
                    { sourceInterval: [519, 525] },
                    ['app', { sourceInterval: [519, 524] }, 'digit', []],
                ],
                [
                    'opt',
                    { sourceInterval: [526, 539] },
                    [
                        'seq',
                        { sourceInterval: [527, 537] },
                        ['terminal', { sourceInterval: [527, 530] }, '.'],
                        [
                            'plus',
                            { sourceInterval: [531, 537] },
                            [
                                'app',
                                { sourceInterval: [531, 536] },
                                'digit',
                                [],
                            ],
                        ],
                    ],
                ],
            ],
        ],
        NoteName: [
            'define',
            { sourceInterval: [543, 579] },
            null,
            [],
            [
                'seq',
                { sourceInterval: [554, 579] },
                ['app', { sourceInterval: [554, 560] }, 'letter', []],
                [
                    'opt',
                    { sourceInterval: [561, 572] },
                    ['app', { sourceInterval: [561, 571] }, 'accidental', []],
                ],
                [
                    'plus',
                    { sourceInterval: [573, 579] },
                    ['app', { sourceInterval: [573, 578] }, 'digit', []],
                ],
            ],
        ],
        MidiValue: [
            'define',
            { sourceInterval: [583, 605] },
            null,
            [],
            [
                'seq',
                { sourceInterval: [595, 605] },
                ['terminal', { sourceInterval: [595, 598] }, 'm'],
                [
                    'plus',
                    { sourceInterval: [599, 605] },
                    ['app', { sourceInterval: [599, 604] }, 'digit', []],
                ],
            ],
        ],
        accidental: [
            'define',
            { sourceInterval: [609, 631] },
            null,
            [],
            [
                'alt',
                { sourceInterval: [622, 631] },
                ['terminal', { sourceInterval: [622, 625] }, '#'],
                ['terminal', { sourceInterval: [628, 631] }, 'b'],
            ],
        ],
    },
]);
module.exports = result;
