import React, { useState, useEffect, useCallback } from 'react';
import electronAPI from '../electronAPI';
import type { 
    AudioDeviceInfo, 
    MidiInputInfo, 
    AudioIOConfig, 
    AudioSlotStates,
    AudioSlotState,
    SlotConfigError 
} from '../ipcTypes';
import './AudioSettings.css';

const NUM_SLOTS = 16;

interface AudioSettingsProps {
    isOpen: boolean;
    onClose: () => void;
}

interface SlotRowProps {
    slotIndex: number;
    type: 'input' | 'output';
    mapping: { deviceName: string; channel: number } | null;
    state: AudioSlotState;
    devices: AudioDeviceInfo[];
    onChangeDevice: (deviceName: string | null) => void;
    onChangeChannel: (channel: number) => void;
}

function SlotRow({ slotIndex, type, mapping, state, devices, onChangeDevice, onChangeChannel }: SlotRowProps) {
    const selectedDevice = mapping?.deviceName || '';
    const selectedChannel = mapping?.channel ?? 0;
    
    // Find the device to get channel count
    const device = devices.find(d => d.name === selectedDevice);
    const channelCount = type === 'input' 
        ? (device?.inputChannels || 0)
        : (device?.outputChannels || 0);
    
    const stateClass = state === 'Mapped' ? 'mapped' : state === 'Orphaned' ? 'orphaned' : 'empty';
    
    return (
        <div className={`slot-row ${stateClass}`}>
            <span className="slot-number">{slotIndex + 1}</span>
            <select 
                className="slot-device-select"
                value={selectedDevice}
                onChange={(e) => onChangeDevice(e.target.value || null)}
            >
                <option value="">-- None --</option>
                {devices.map(d => (
                    <option key={d.name} value={d.name}>
                        {d.name} ({type === 'input' ? d.inputChannels : d.outputChannels} ch)
                    </option>
                ))}
            </select>
            <select
                className="slot-channel-select"
                value={selectedChannel}
                onChange={(e) => onChangeChannel(parseInt(e.target.value, 10))}
                disabled={!selectedDevice}
            >
                {Array.from({ length: channelCount }, (_, i) => (
                    <option key={i} value={i}>Ch {i + 1}</option>
                ))}
                {channelCount === 0 && <option value={0}>--</option>}
            </select>
            <span className={`slot-state-indicator ${stateClass}`} title={state}>
                {state === 'Mapped' ? '●' : state === 'Orphaned' ? '○' : ''}
            </span>
        </div>
    );
}

export function AudioSettings({ isOpen, onClose }: AudioSettingsProps) {
    // Device lists
    const [outputDevices, setOutputDevices] = useState<AudioDeviceInfo[]>([]);
    const [inputDevices, setInputDevices] = useState<AudioDeviceInfo[]>([]);
    
    // Slot configuration (local state - not applied until Save)
    const [localConfig, setLocalConfig] = useState<AudioIOConfig | null>(null);
    const [slotStates, setSlotStates] = useState<AudioSlotStates | null>(null);
    
    // MIDI
    const [midiInputs, setMidiInputs] = useState<MidiInputInfo[]>([]);
    const [copiedMidiDevice, setCopiedMidiDevice] = useState<string | null>(null);
    
    // UI state
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);
    const [validationErrors, setValidationErrors] = useState<SlotConfigError[]>([]);
    const [hasUnsavedChanges, setHasUnsavedChanges] = useState(false);
    const [saving, setSaving] = useState(false);

    // Load all data
    const loadData = useCallback(async () => {
        try {
            setLoading(true);
            setError(null);
            
            const [
                outputs,
                inputs,
                midi,
                ioConfig,
                states,
            ] = await Promise.all([
                electronAPI.audio.listOutputDevices(),
                electronAPI.audio.listInputDevices(),
                electronAPI.midi.listInputs(),
                electronAPI.audio.getIoConfig(),
                electronAPI.audio.getSlotStates(),
            ]);
            
            setOutputDevices(outputs);
            setInputDevices(inputs);
            setMidiInputs(midi);
            setLocalConfig(ioConfig);
            setSlotStates(states);
            setHasUnsavedChanges(false);
        } catch (err) {
            setError(err instanceof Error ? err.message : 'Failed to load settings');
        } finally {
            setLoading(false);
        }
    }, []);

    // Refresh devices from hardware
    const refreshDevices = useCallback(async () => {
        try {
            setLoading(true);
            await electronAPI.audio.refreshDeviceList();
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

    // Update local input slot
    const updateInputSlot = (slotIndex: number, deviceName: string | null, channel: number) => {
        if (!localConfig) return;
        
        const newConfig = { ...localConfig };
        newConfig.inputSlots = [...localConfig.inputSlots];
        
        if (deviceName) {
            newConfig.inputSlots[slotIndex] = { deviceName: deviceName, channel };
        } else {
            newConfig.inputSlots[slotIndex] = null;
        }
        
        setLocalConfig(newConfig);
        setHasUnsavedChanges(true);
        setValidationErrors([]);
    };

    // Update local output slot
    const updateOutputSlot = (slotIndex: number, deviceName: string | null, channel: number) => {
        if (!localConfig) return;
        
        const newConfig = { ...localConfig };
        newConfig.outputSlots = [...localConfig.outputSlots];
        
        if (deviceName) {
            newConfig.outputSlots[slotIndex] = { deviceName: deviceName, channel };
        } else {
            newConfig.outputSlots[slotIndex] = null;
        }
        
        setLocalConfig(newConfig);
        setHasUnsavedChanges(true);
        setValidationErrors([]);
    };

    // Save configuration
    const saveConfig = async () => {
        if (!localConfig) return;
        
        try {
            setSaving(true);
            setError(null);
            
            // Validate first
            const errors = await electronAPI.audio.validateIoConfig(localConfig);
            if (errors.length > 0) {
                setValidationErrors(errors);
                return;
            }
            
            // Apply the configuration
            const applyErrors = await electronAPI.audio.applyIoConfig(localConfig);
            if (applyErrors.length > 0) {
                setValidationErrors(applyErrors);
                return;
            }
            
            // Refresh states
            const newStates = await electronAPI.audio.getSlotStates();
            setSlotStates(newStates);
            setHasUnsavedChanges(false);
            setValidationErrors([]);
        } catch (err) {
            setError(err instanceof Error ? err.message : 'Failed to save configuration');
        } finally {
            setSaving(false);
        }
    };

    // Discard changes
    const discardChanges = async () => {
        await loadData();
        setValidationErrors([]);
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

    // Handle close - warn about unsaved changes
    const handleClose = () => {
        if (hasUnsavedChanges) {
            if (!confirm('You have unsaved changes. Discard them?')) {
                return;
            }
        }
        onClose();
    };

    if (!isOpen) {
        return null;
    }

    return (
        <div className="audio-settings-overlay" onClick={handleClose}>
            <div className="audio-settings-panel audio-settings-panel-wide" onClick={(e) => e.stopPropagation()}>
                <div className="audio-settings-header">
                    <h2>Audio & MIDI Settings</h2>
                    <button className="close-btn" onClick={handleClose}>×</button>
                </div>
                
                {error && (
                    <div className="audio-settings-error">{error}</div>
                )}
                
                {validationErrors.length > 0 && (
                    <div className="audio-settings-validation-errors">
                        {validationErrors.map((err, i) => (
                            <div key={i} className="validation-error">{err.message}</div>
                        ))}
                    </div>
                )}
                
                {loading ? (
                    <div className="audio-settings-loading">Loading devices...</div>
                ) : (
                    <div className="audio-settings-content">
                        {/* I/O Slot Routing */}
                        <div className="io-routing-container">
                            {/* Output Slots */}
                            <div className="slots-column">
                                <h3>Output Slots</h3>
                                <div className="slots-list">
                                    {Array.from({ length: NUM_SLOTS }, (_, i) => (
                                        <SlotRow
                                            key={`out-${i}`}
                                            slotIndex={i}
                                            type="output"
                                            mapping={localConfig?.outputSlots[i] || null}
                                            state={slotStates?.output[i] || 'Empty'}
                                            devices={outputDevices}
                                            onChangeDevice={(deviceName) => {
                                                const channel = localConfig?.outputSlots[i]?.channel || 0;
                                                updateOutputSlot(i, deviceName, channel);
                                            }}
                                            onChangeChannel={(channel) => {
                                                const deviceName = localConfig?.outputSlots[i]?.deviceName || null;
                                                if (deviceName) {
                                                    updateOutputSlot(i, deviceName, channel);
                                                }
                                            }}
                                        />
                                    ))}
                                </div>
                            </div>
                            
                            {/* Input Slots */}
                            <div className="slots-column">
                                <h3>Input Slots</h3>
                                <div className="slots-list">
                                    {Array.from({ length: NUM_SLOTS }, (_, i) => (
                                        <SlotRow
                                            key={`in-${i}`}
                                            slotIndex={i}
                                            type="input"
                                            mapping={localConfig?.inputSlots[i] || null}
                                            state={slotStates?.input[i] || 'Empty'}
                                            devices={inputDevices}
                                            onChangeDevice={(deviceName) => {
                                                const channel = localConfig?.inputSlots[i]?.channel || 0;
                                                updateInputSlot(i, deviceName, channel);
                                            }}
                                            onChangeChannel={(channel) => {
                                                const deviceName = localConfig?.inputSlots[i]?.deviceName || null;
                                                if (deviceName) {
                                                    updateInputSlot(i, deviceName, channel);
                                                }
                                            }}
                                        />
                                    ))}
                                </div>
                            </div>
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
                                disabled={saving}
                            >
                                ↻ Refresh Devices
                            </button>
                            <div className="action-spacer" />
                            {hasUnsavedChanges && (
                                <button 
                                    className="btn btn-secondary"
                                    onClick={discardChanges}
                                    disabled={saving}
                                >
                                    Discard
                                </button>
                            )}
                            <button 
                                className="btn btn-primary"
                                onClick={saveConfig}
                                disabled={!hasUnsavedChanges || saving}
                            >
                                {saving ? 'Saving...' : 'Save & Apply'}
                            </button>
                        </div>
                    </div>
                )}
            </div>
        </div>
    );
}

export default AudioSettings;
