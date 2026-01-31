import React, { useState, useEffect, useCallback, useMemo } from 'react';
import electronAPI from '../electronAPI';
import type { 
    AudioDeviceInfo, 
    MidiInputInfo,
    HostInfo,
    DeviceCacheSnapshot,
    CurrentAudioState,
} from '../ipcTypes';
import './AudioSettings.css';

interface AudioSettingsProps {
    isOpen: boolean;
    onClose: () => void;
}

/**
 * Compute intersection of sample rates supported by given devices.
 * If only output device is provided, returns its supported rates.
 */
function computeCommonSampleRates(
    outputDevice: AudioDeviceInfo | null,
    inputDevice: AudioDeviceInfo | null
): number[] {
    if (!outputDevice) return [];
    
    const outputRates = outputDevice.supportedSampleRates || [];
    if (!inputDevice) return [...outputRates].sort((a, b) => a - b);
    
    const inputRates = new Set(inputDevice.supportedSampleRates || []);
    return outputRates.filter(r => inputRates.has(r)).sort((a, b) => a - b);
}

/**
 * Compute buffer sizes available for given devices.
 * Returns power-of-2 values (64, 128, 256, 512, 1024, 2048) within the supported range.
 */
function computeBufferSizes(
    outputDevice: AudioDeviceInfo | null,
    inputDevice: AudioDeviceInfo | null
): number[] {
    const powerOf2Sizes = [64, 128, 256, 512, 1024, 2048, 4096];
    
    if (!outputDevice) return [];
    
    // Find the common range
    let minSize = outputDevice.bufferSizeRange?.min ?? 64;
    let maxSize = outputDevice.bufferSizeRange?.max ?? 4096;
    
    if (inputDevice?.bufferSizeRange) {
        minSize = Math.max(minSize, inputDevice.bufferSizeRange.min);
        maxSize = Math.min(maxSize, inputDevice.bufferSizeRange.max);
    }
    
    return powerOf2Sizes.filter(size => size >= minSize && size <= maxSize);
}

export function AudioSettings({ isOpen, onClose }: AudioSettingsProps) {
    // Device cache from Rust
    const [deviceCache, setDeviceCache] = useState<DeviceCacheSnapshot | null>(null);
    const [currentState, setCurrentState] = useState<CurrentAudioState | null>(null);
    
    // Current selections (pending changes, not yet applied)
    const [selectedHostId, setSelectedHostId] = useState<string | null>(null);
    const [selectedOutputDeviceId, setSelectedOutputDeviceId] = useState<string | null>(null);
    const [selectedInputDeviceId, setSelectedInputDeviceId] = useState<string | null>(null);
    const [selectedSampleRate, setSelectedSampleRate] = useState<number | null>(null);
    const [selectedBufferSize, setSelectedBufferSize] = useState<number | null>(null);
    
    // MIDI
    const [midiInputs, setMidiInputs] = useState<MidiInputInfo[]>([]);
    const [copiedMidiDevice, setCopiedMidiDevice] = useState<string | null>(null);
    
    // UI state
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);
    const [warning, setWarning] = useState<string | null>(null);
    const [applying, setApplying] = useState(false);

    // Filter devices by selected host
    const outputDevicesForHost = useMemo(() => {
        if (!deviceCache || !selectedHostId) return [];
        const hostData = deviceCache.hosts.find(h => h.hostId === selectedHostId);
        return hostData?.outputDevices ?? [];
    }, [deviceCache, selectedHostId]);
    
    const inputDevicesForHost = useMemo(() => {
        if (!deviceCache || !selectedHostId) return [];
        const hostData = deviceCache.hosts.find(h => h.hostId === selectedHostId);
        return hostData?.inputDevices ?? [];
    }, [deviceCache, selectedHostId]);
    
    // Get selected device objects
    const selectedOutputDevice = useMemo(() => 
        outputDevicesForHost.find(d => d.id === selectedOutputDeviceId) ?? null,
    [outputDevicesForHost, selectedOutputDeviceId]);
    
    const selectedInputDevice = useMemo(() =>
        inputDevicesForHost.find(d => d.id === selectedInputDeviceId) ?? null,
    [inputDevicesForHost, selectedInputDeviceId]);
    
    // Compute available sample rates and buffer sizes (JS-side logic)
    const availableSampleRates = useMemo(() => 
        computeCommonSampleRates(selectedOutputDevice, selectedInputDevice),
    [selectedOutputDevice, selectedInputDevice]);
    
    const availableBufferSizes = useMemo(() =>
        computeBufferSizes(selectedOutputDevice, selectedInputDevice),
    [selectedOutputDevice, selectedInputDevice]);

    // Load data from device cache
    const loadData = useCallback(async () => {
        try {
            setLoading(true);
            setError(null);
            
            const [cache, state, midi] = await Promise.all([
                electronAPI.audio.getDeviceCache(),
                electronAPI.audio.getCurrentState(),
                electronAPI.midi.listInputs(),
            ]);
            
            setDeviceCache(cache);
            setCurrentState(state);
            setMidiInputs(midi);
            
            // Initialize selections from current state
            setSelectedHostId(state.hostId);
            setSelectedOutputDeviceId(state.outputDeviceId ?? null);
            setSelectedInputDeviceId(state.inputDeviceId ?? null);
            setSelectedSampleRate(state.sampleRate);
            setSelectedBufferSize(state.bufferSize ?? null);
        } catch (err) {
            setError(err instanceof Error ? err.message : 'Failed to load settings');
        } finally {
            setLoading(false);
        }
    }, []);

    // When host changes, reset device selections to defaults for that host
    useEffect(() => {
        if (!deviceCache || !selectedHostId) return;
        
        const hostData = deviceCache.hosts.find(h => h.hostId === selectedHostId);
        if (!hostData) return;
        
        // If current output device isn't in this host, select default
        const outputInHost = hostData.outputDevices.find(d => d.id === selectedOutputDeviceId);
        if (!outputInHost) {
            const defaultOutput = hostData.outputDevices.find(d => d.isDefault) ?? hostData.outputDevices[0];
            setSelectedOutputDeviceId(defaultOutput?.id ?? null);
        }
        
        // If current input device isn't in this host, select none or default
        const inputInHost = hostData.inputDevices.find(d => d.id === selectedInputDeviceId);
        if (!inputInHost && selectedInputDeviceId !== null) {
            setSelectedInputDeviceId(null);
        }
    }, [deviceCache, selectedHostId]);

    // Update sample rate and buffer size when device selection changes
    useEffect(() => {
        // Auto-select highest sample rate if not in available list
        if (selectedSampleRate === null || !availableSampleRates.includes(selectedSampleRate)) {
            const highest = availableSampleRates.length > 0 ? Math.max(...availableSampleRates) : null;
            setSelectedSampleRate(highest);
        }

        // Auto-select lowest buffer size if not in available list
        if (selectedBufferSize === null || !availableBufferSizes.includes(selectedBufferSize)) {
            const lowest = availableBufferSizes.length > 0 ? Math.min(...availableBufferSizes) : null;
            setSelectedBufferSize(lowest);
        }
    }, [availableSampleRates, availableBufferSizes]);

    // Refresh devices from hardware
    const refreshDevices = useCallback(async () => {
        try {
            setLoading(true);
            await electronAPI.audio.refreshDeviceCache();
            await loadData();
        } catch (err) {
            setError(err instanceof Error ? err.message : 'Failed to refresh devices');
            setLoading(false);
        }
    }, [loadData]);

    // Load on open
    useEffect(() => {
        if (isOpen) {
            loadData();
        }
    }, [isOpen, loadData]);
    
    // Listen for fallback warnings
    useEffect(() => {
        const unsubscribe = electronAPI.audio.onFallbackWarning((msg: string) => {
            setWarning(msg);
        });
        return unsubscribe;
    }, []);

    // Apply configuration using recreateStreams
    const applyConfig = async () => {
        if (!selectedOutputDeviceId) {
            setError('Please select an output device');
            return;
        }
        
        if (!selectedSampleRate) {
            setError('Please select a sample rate');
            return;
        }

        try {
            setApplying(true);
            setError(null);
            setWarning(null);

            // Call the unified recreateStreams API
            // Signature: (outputDeviceId, sampleRate, bufferSize?, inputDeviceId?)
            await electronAPI.audio.recreateStreams(
                selectedOutputDeviceId,
                selectedSampleRate,
                selectedBufferSize ?? undefined,
                selectedInputDeviceId ?? undefined
            );

            // Reload state to get actual applied values
            const newState = await electronAPI.audio.getCurrentState();
            setCurrentState(newState);
            
            // Update selections to match what was actually applied
            setSelectedHostId(newState.hostId);
            setSelectedOutputDeviceId(newState.outputDeviceId ?? null);
            setSelectedInputDeviceId(newState.inputDeviceId ?? null);
            setSelectedSampleRate(newState.sampleRate);
            setSelectedBufferSize(newState.bufferSize ?? null);

        } catch (err) {
            setError(err instanceof Error ? err.message : 'Failed to apply configuration');
        } finally {
            setApplying(false);
        }
    };

    // Copy MIDI device name to clipboard
    const copyMidiDeviceName = async (deviceName: string) => {
        try {
            await navigator.clipboard.writeText(deviceName);
            setCopiedMidiDevice(deviceName);
            setTimeout(() => setCopiedMidiDevice(null), 2000);
        } catch (err) {
            setError('Failed to copy to clipboard');
        }
    };

    if (!isOpen) {
        return null;
    }

    // Only show hosts that have at least one available device
    const hosts = deviceCache?.hosts
        .filter(h => h.outputDevices.length > 0 || h.inputDevices.length > 0)
        .map(h => ({ id: h.hostId, name: h.hostName })) ?? [];

    return (
        <div className="audio-settings-overlay" onClick={onClose}>
            <div className="audio-settings-panel" onClick={(e) => e.stopPropagation()}>
                <div className="audio-settings-header">
                    <h2>Audio & MIDI Settings</h2>
                    <button className="close-btn" onClick={onClose}>×</button>
                </div>
                
                {error && (
                    <div className="audio-settings-error">{error}</div>
                )}
                
                {warning && (
                    <div className="audio-settings-warning">{warning}</div>
                )}
                
                {loading ? (
                    <div className="audio-settings-loading">Loading devices...</div>
                ) : (
                    <div className="audio-settings-content">
                        {/* Audio Host */}
                        <div className="settings-section">
                            <h3>Audio Host</h3>
                            <select 
                                className="device-select"
                                value={selectedHostId || ''}
                                onChange={(e) => setSelectedHostId(e.target.value || null)}
                            >
                                {hosts.map(h => (
                                    <option key={h.id} value={h.id}>
                                        {h.name}
                                    </option>
                                ))}
                            </select>
                        </div>

                        {/* Output Device */}
                        <div className="settings-section">
                            <h3>Output Device</h3>
                            <select 
                                className="device-select"
                                value={selectedOutputDeviceId || ''}
                                onChange={(e) => setSelectedOutputDeviceId(e.target.value || null)}
                            >
                                <option value="">-- Select Output Device --</option>
                                {outputDevicesForHost.map(d => (
                                    <option key={d.id} value={d.id}>
                                        {d.name} ({d.outputChannels} ch){d.isDefault ? ' (Default)' : ''}
                                    </option>
                                ))}
                            </select>
                            {selectedOutputDevice && (
                                <div className="device-info">
                                    {selectedOutputDevice.sampleRate} Hz, {selectedOutputDevice.outputChannels} channels
                                </div>
                            )}
                        </div>

                        {/* Input Device */}
                        <div className="settings-section">
                            <h3>Input Device</h3>
                            <select 
                                className="device-select"
                                value={selectedInputDeviceId || ''}
                                onChange={(e) => setSelectedInputDeviceId(e.target.value || null)}
                            >
                                <option value="">None (No Input)</option>
                                {inputDevicesForHost.map(d => (
                                    <option key={d.id} value={d.id}>
                                        {d.name} ({d.inputChannels} ch){d.isDefault ? ' (Default)' : ''}
                                    </option>
                                ))}
                            </select>
                            {selectedInputDevice && (
                                <div className="device-info">
                                    {selectedInputDevice.sampleRate} Hz, {selectedInputDevice.inputChannels} channels
                                </div>
                            )}
                        </div>

                        {/* Sample Rate */}
                        <div className="settings-section">
                            <h3>Sample Rate</h3>
                            <select 
                                className="device-select"
                                value={selectedSampleRate || ''}
                                onChange={(e) => setSelectedSampleRate(Number(e.target.value) || null)}
                                disabled={availableSampleRates.length === 0}
                            >
                                {availableSampleRates.length === 0 ? (
                                    <option value="">Select devices first</option>
                                ) : (
                                    availableSampleRates.map(rate => (
                                        <option key={rate} value={rate}>
                                            {rate} Hz
                                        </option>
                                    ))
                                )}
                            </select>
                        </div>

                        {/* Buffer Size */}
                        <div className="settings-section">
                            <h3>Buffer Size</h3>
                            <select 
                                className="device-select"
                                value={selectedBufferSize || ''}
                                onChange={(e) => setSelectedBufferSize(Number(e.target.value) || null)}
                                disabled={availableBufferSizes.length === 0}
                            >
                                {availableBufferSizes.length === 0 ? (
                                    <option value="">Select devices first</option>
                                ) : (
                                    availableBufferSizes.map(size => (
                                        <option key={size} value={size}>
                                            {size} samples (~{((size / (selectedSampleRate || 48000)) * 1000).toFixed(1)} ms)
                                        </option>
                                    ))
                                )}
                            </select>
                        </div>
                        
                        {/* MIDI Devices */}
                        <div className="settings-section midi-section">
                            <h3>MIDI Devices</h3>
                            <p className="midi-hint">Copy a device name to use in MIDI modules</p>
                            {midiInputs.length === 0 ? (
                                <div className="midi-no-devices">No MIDI devices found</div>
                            ) : (
                                <div className="midi-device-list">
                                    {midiInputs.map((port) => (
                                        <div key={port.name} className="midi-device-row">
                                            <span className="midi-device-name">{port.name}</span>
                                            <button 
                                                className="btn btn-copy"
                                                onClick={() => copyMidiDeviceName(port.name)}
                                            >
                                                {copiedMidiDevice === port.name ? '✓ Copied' : 'Copy'}
                                            </button>
                                        </div>
                                    ))}
                                </div>
                            )}
                        </div>
                        
                        {/* Actions */}
                        <div className="settings-actions">
                            <button 
                                className="btn btn-secondary"
                                onClick={refreshDevices}
                                disabled={applying}
                            >
                                ↻ Refresh Devices
                            </button>
                            <div className="action-spacer" />
                            <button 
                                className="btn btn-primary"
                                onClick={applyConfig}
                                disabled={applying || !selectedOutputDeviceId}
                            >
                                {applying ? 'Applying...' : 'Apply'}
                            </button>
                        </div>
                    </div>
                )}
            </div>
        </div>
    );
}

export default AudioSettings;
