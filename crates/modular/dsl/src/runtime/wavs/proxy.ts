import type { DSLExecutionOptions, WavsFolderNode } from '../executor/types';

/**
 * `$wavs()` — return a proxy tree matching the wavs/ folder structure.
 * Leaf nodes trigger `loadWav()` and return `{ type: 'wav_ref', ... }` objects.
 */
export function create$wavs(options: DSLExecutionOptions) {
    return (): unknown => {
        const tree = options.wavsFolderTree;
        if (!tree) {
            return new Proxy(
                {},
                {
                    get(_target, prop) {
                        throw new Error(
                            `$wavs().${String(prop)}: no wavs/ folder found in workspace`,
                        );
                    },
                },
            );
        }

        function makeProxy(node: WavsFolderNode, pathParts: string[]): unknown {
            return new Proxy(
                {},
                {
                    get(_target, prop) {
                        if (typeof prop !== 'string') return undefined;

                        const child = node[prop];
                        if (child === undefined) {
                            const fullPath = [...pathParts, prop].join('/');
                            throw new Error(
                                `$wavs(): "${fullPath}" not found. Available: ${Object.keys(node).join(', ') || '(empty)'}`,
                            );
                        }

                        if (child === 'file') {
                            const relPath = [...pathParts, prop].join('/');
                            if (!options.loadWav) {
                                throw new Error(
                                    '$wavs(): loadWav function not provided',
                                );
                            }
                            const info = options.loadWav(relPath);
                            return {
                                type: 'wav_ref' as const,
                                path: relPath,
                                channels: info.channels,
                                sampleRate: info.sampleRate,
                                frameCount: info.frameCount,
                                duration: info.duration,
                                bitDepth: info.bitDepth,
                                mtime: info.mtime,
                                ...(info.pitch != null && {
                                    pitch: info.pitch,
                                }),
                                ...(info.playback != null && {
                                    playback: info.playback,
                                }),
                                ...(info.bpm != null && { bpm: info.bpm }),
                                ...(info.beats != null && {
                                    beats: info.beats,
                                }),
                                ...(info.timeSignature != null && {
                                    timeSignature: {
                                        num: info.timeSignature.num,
                                        den: info.timeSignature.den,
                                    },
                                }),
                                loops: info.loops.map(
                                    (l: {
                                        loopType: string;
                                        start: number;
                                        end: number;
                                    }) => ({
                                        type: l.loopType as
                                            | 'forward'
                                            | 'pingpong'
                                            | 'backward',
                                        start: l.start,
                                        end: l.end,
                                    }),
                                ),
                                cuePoints: info.cuePoints.map(
                                    (c: {
                                        position: number;
                                        label: string;
                                    }) => ({
                                        position: c.position,
                                        label: c.label,
                                    }),
                                ),
                            };
                        }

                        return makeProxy(child, [...pathParts, prop]);
                    },
                    ownKeys() {
                        return Object.keys(node);
                    },
                    getOwnPropertyDescriptor(_target, prop) {
                        if (typeof prop === 'string' && prop in node) {
                            return {
                                configurable: true,
                                enumerable: true,
                                writable: false,
                            };
                        }
                        return undefined;
                    },
                },
            );
        }

        return makeProxy(tree, []);
    };
}
