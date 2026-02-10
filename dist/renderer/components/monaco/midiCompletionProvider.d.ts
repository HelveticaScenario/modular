/**
 * Monaco completion provider for MIDI device names.
 * Provides autocomplete suggestions for the `device` parameter in MIDI modules.
 */
import type { Monaco } from '../../hooks/useCustomMonaco';
/**
 * Function to fetch MIDI device list from the Electron main process.
 */
export type MidiDeviceFetcher = () => Promise<Array<{
    name: string;
    index: number;
}>>;
/**
 * Creates and registers a completion provider for MIDI device names.
 * The provider triggers when the user types a quote inside a `device:` property.
 *
 * @param monaco Monaco instance
 * @param fetchMidiDevices Function to fetch available MIDI devices
 * @returns Disposable to unregister the provider
 */
export declare function registerMidiCompletionProvider(monaco: Monaco, fetchMidiDevices: MidiDeviceFetcher): {
    dispose: () => void;
};
