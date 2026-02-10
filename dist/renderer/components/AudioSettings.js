"use strict";
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.AudioSettingsTab = void 0;
const jsx_runtime_1 = require("react/jsx-runtime");
const react_1 = require("react");
const electronAPI_1 = __importDefault(require("../electronAPI"));
/**
 * Compute intersection of sample rates supported by given devices.
 * If only output device is provided, returns its supported rates.
 */
function computeCommonSampleRates(outputDevice, inputDevice) {
    if (!outputDevice)
        return [];
    const outputRates = outputDevice.supportedSampleRates || [];
    if (!inputDevice)
        return [...outputRates].sort((a, b) => a - b);
    const inputRates = new Set(inputDevice.supportedSampleRates || []);
    return outputRates.filter(r => inputRates.has(r)).sort((a, b) => a - b);
}
/**
 * Compute buffer sizes available for given devices.
 * Returns power-of-2 values (64, 128, 256, 512, 1024, 2048) within the supported range.
 */
function computeBufferSizes(outputDevice, inputDevice) {
    const powerOf2Sizes = [64, 128, 256, 512, 1024, 2048, 4096];
    if (!outputDevice)
        return [];
    // Find the common range
    let minSize = outputDevice.bufferSizeRange?.min ?? 64;
    let maxSize = outputDevice.bufferSizeRange?.max ?? 4096;
    if (inputDevice?.bufferSizeRange) {
        minSize = Math.max(minSize, inputDevice.bufferSizeRange.min);
        maxSize = Math.min(maxSize, inputDevice.bufferSizeRange.max);
    }
    return powerOf2Sizes.filter(size => size >= minSize && size <= maxSize);
}
exports.AudioSettingsTab = (0, react_1.forwardRef)(function AudioSettingsTab({ isActive }, ref) {
    // Device cache from Rust
    const [deviceCache, setDeviceCache] = (0, react_1.useState)(null);
    const [currentState, setCurrentState] = (0, react_1.useState)(null);
    // Current selections (pending changes, not yet applied)
    const [selectedHostId, setSelectedHostId] = (0, react_1.useState)(null);
    const [selectedOutputDeviceId, setSelectedOutputDeviceId] = (0, react_1.useState)(null);
    const [selectedInputDeviceId, setSelectedInputDeviceId] = (0, react_1.useState)(null);
    const [selectedSampleRate, setSelectedSampleRate] = (0, react_1.useState)(null);
    const [selectedBufferSize, setSelectedBufferSize] = (0, react_1.useState)(null);
    // UI state
    const [loading, setLoading] = (0, react_1.useState)(true);
    const [error, setError] = (0, react_1.useState)(null);
    const [warning, setWarning] = (0, react_1.useState)(null);
    const [applying, setApplying] = (0, react_1.useState)(false);
    // Filter devices by selected host
    const outputDevicesForHost = (0, react_1.useMemo)(() => {
        if (!deviceCache || !selectedHostId)
            return [];
        const hostData = deviceCache.hosts.find(h => h.hostId === selectedHostId);
        return hostData?.outputDevices ?? [];
    }, [deviceCache, selectedHostId]);
    const inputDevicesForHost = (0, react_1.useMemo)(() => {
        if (!deviceCache || !selectedHostId)
            return [];
        const hostData = deviceCache.hosts.find(h => h.hostId === selectedHostId);
        return hostData?.inputDevices ?? [];
    }, [deviceCache, selectedHostId]);
    // Get selected device objects
    const selectedOutputDevice = (0, react_1.useMemo)(() => outputDevicesForHost.find(d => d.id === selectedOutputDeviceId) ?? null, [outputDevicesForHost, selectedOutputDeviceId]);
    const selectedInputDevice = (0, react_1.useMemo)(() => inputDevicesForHost.find(d => d.id === selectedInputDeviceId) ?? null, [inputDevicesForHost, selectedInputDeviceId]);
    // Compute available sample rates and buffer sizes (JS-side logic)
    const availableSampleRates = (0, react_1.useMemo)(() => computeCommonSampleRates(selectedOutputDevice, selectedInputDevice), [selectedOutputDevice, selectedInputDevice]);
    const availableBufferSizes = (0, react_1.useMemo)(() => computeBufferSizes(selectedOutputDevice, selectedInputDevice), [selectedOutputDevice, selectedInputDevice]);
    // Load data from device cache
    const loadData = (0, react_1.useCallback)(async () => {
        try {
            setLoading(true);
            setError(null);
            const [cache, state] = await Promise.all([
                electronAPI_1.default.audio.getDeviceCache(),
                electronAPI_1.default.audio.getCurrentState(),
            ]);
            setDeviceCache(cache);
            setCurrentState(state);
            // Initialize selections from current state
            setSelectedHostId(state.hostId);
            setSelectedOutputDeviceId(state.outputDeviceId ?? null);
            setSelectedInputDeviceId(state.inputDeviceId ?? null);
            setSelectedSampleRate(state.sampleRate);
            setSelectedBufferSize(state.bufferSize ?? null);
        }
        catch (err) {
            setError(err instanceof Error ? err.message : 'Failed to load settings');
        }
        finally {
            setLoading(false);
        }
    }, []);
    // When host changes, reset device selections to defaults for that host
    (0, react_1.useEffect)(() => {
        if (!deviceCache || !selectedHostId)
            return;
        const hostData = deviceCache.hosts.find(h => h.hostId === selectedHostId);
        if (!hostData)
            return;
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
    (0, react_1.useEffect)(() => {
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
    const refreshDevices = (0, react_1.useCallback)(async () => {
        try {
            setLoading(true);
            await electronAPI_1.default.audio.refreshDeviceCache();
            await loadData();
        }
        catch (err) {
            setError(err instanceof Error ? err.message : 'Failed to refresh devices');
            setLoading(false);
        }
    }, [loadData]);
    // Load on active
    (0, react_1.useEffect)(() => {
        if (isActive) {
            loadData();
        }
    }, [isActive, loadData]);
    // Listen for fallback warnings
    (0, react_1.useEffect)(() => {
        const unsubscribe = electronAPI_1.default.audio.onFallbackWarning((msg) => {
            setWarning(msg);
        });
        return unsubscribe;
    }, []);
    // Apply configuration using recreateStreams
    const applyConfig = (0, react_1.useCallback)(async () => {
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
            await electronAPI_1.default.audio.recreateStreams(selectedOutputDeviceId, selectedSampleRate, selectedBufferSize ?? undefined, selectedInputDeviceId ?? undefined);
            // Reload state to get actual applied values
            const newState = await electronAPI_1.default.audio.getCurrentState();
            setCurrentState(newState);
            // Update selections to match what was actually applied
            setSelectedHostId(newState.hostId);
            setSelectedOutputDeviceId(newState.outputDeviceId ?? null);
            setSelectedInputDeviceId(newState.inputDeviceId ?? null);
            setSelectedSampleRate(newState.sampleRate);
            setSelectedBufferSize(newState.bufferSize ?? null);
        }
        catch (err) {
            setError(err instanceof Error ? err.message : 'Failed to apply configuration');
        }
        finally {
            setApplying(false);
        }
    }, [selectedOutputDeviceId, selectedSampleRate, selectedBufferSize, selectedInputDeviceId]);
    // Check if selections differ from current running state
    const checkIsDirty = (0, react_1.useCallback)(() => {
        if (!currentState)
            return false;
        return (selectedHostId !== currentState.hostId ||
            selectedOutputDeviceId !== (currentState.outputDeviceId ?? null) ||
            selectedInputDeviceId !== (currentState.inputDeviceId ?? null) ||
            selectedSampleRate !== currentState.sampleRate ||
            selectedBufferSize !== (currentState.bufferSize ?? null));
    }, [currentState, selectedHostId, selectedOutputDeviceId, selectedInputDeviceId, selectedSampleRate, selectedBufferSize]);
    // Expose apply and isDirty to parent via ref
    (0, react_1.useImperativeHandle)(ref, () => ({
        apply: applyConfig,
        isDirty: checkIsDirty,
    }), [applyConfig, checkIsDirty]);
    // Only show hosts that have at least one available device
    const hosts = deviceCache?.hosts
        .filter(h => h.outputDevices.length > 0 || h.inputDevices.length > 0)
        .map(h => ({ id: h.hostId, name: h.hostName })) ?? [];
    return ((0, jsx_runtime_1.jsxs)("div", { className: "settings-tab-content", children: [error && ((0, jsx_runtime_1.jsx)("div", { className: "settings-tab-error", children: error })), warning && ((0, jsx_runtime_1.jsx)("div", { className: "settings-tab-warning", children: warning })), loading ? ((0, jsx_runtime_1.jsx)("div", { className: "settings-tab-loading", children: "Loading devices..." })) : ((0, jsx_runtime_1.jsxs)(jsx_runtime_1.Fragment, { children: [(0, jsx_runtime_1.jsxs)("div", { className: "settings-section", children: [(0, jsx_runtime_1.jsx)("h3", { children: "Audio Host" }), (0, jsx_runtime_1.jsx)("select", { className: "device-select", value: selectedHostId || '', onChange: (e) => setSelectedHostId(e.target.value || null), children: hosts.map(h => ((0, jsx_runtime_1.jsx)("option", { value: h.id, children: h.name }, h.id))) })] }), (0, jsx_runtime_1.jsxs)("div", { className: "settings-section", children: [(0, jsx_runtime_1.jsx)("h3", { children: "Output Device" }), (0, jsx_runtime_1.jsxs)("select", { className: "device-select", value: selectedOutputDeviceId || '', onChange: (e) => setSelectedOutputDeviceId(e.target.value || null), children: [(0, jsx_runtime_1.jsx)("option", { value: "", children: "-- Select Output Device --" }), outputDevicesForHost.map(d => ((0, jsx_runtime_1.jsxs)("option", { value: d.id, children: [d.name, " (", d.outputChannels, " ch)", d.isDefault ? ' (Default)' : ''] }, d.id)))] }), selectedOutputDevice && ((0, jsx_runtime_1.jsxs)("div", { className: "device-info", children: [selectedOutputDevice.sampleRate, " Hz, ", selectedOutputDevice.outputChannels, " channels"] }))] }), (0, jsx_runtime_1.jsxs)("div", { className: "settings-section", children: [(0, jsx_runtime_1.jsx)("h3", { children: "Input Device" }), (0, jsx_runtime_1.jsxs)("select", { className: "device-select", value: selectedInputDeviceId || '', onChange: (e) => setSelectedInputDeviceId(e.target.value || null), children: [(0, jsx_runtime_1.jsx)("option", { value: "", children: "None (No Input)" }), inputDevicesForHost.map(d => ((0, jsx_runtime_1.jsxs)("option", { value: d.id, children: [d.name, " (", d.inputChannels, " ch)", d.isDefault ? ' (Default)' : ''] }, d.id)))] }), selectedInputDevice && ((0, jsx_runtime_1.jsxs)("div", { className: "device-info", children: [selectedInputDevice.sampleRate, " Hz, ", selectedInputDevice.inputChannels, " channels"] }))] }), (0, jsx_runtime_1.jsxs)("div", { className: "settings-section", children: [(0, jsx_runtime_1.jsx)("h3", { children: "Sample Rate" }), (0, jsx_runtime_1.jsx)("select", { className: "device-select", value: selectedSampleRate || '', onChange: (e) => setSelectedSampleRate(Number(e.target.value) || null), disabled: availableSampleRates.length === 0, children: availableSampleRates.length === 0 ? ((0, jsx_runtime_1.jsx)("option", { value: "", children: "Select devices first" })) : (availableSampleRates.map(rate => ((0, jsx_runtime_1.jsxs)("option", { value: rate, children: [rate, " Hz"] }, rate)))) })] }), (0, jsx_runtime_1.jsxs)("div", { className: "settings-section", children: [(0, jsx_runtime_1.jsx)("h3", { children: "Buffer Size" }), (0, jsx_runtime_1.jsx)("select", { className: "device-select", value: selectedBufferSize || '', onChange: (e) => setSelectedBufferSize(Number(e.target.value) || null), disabled: availableBufferSizes.length === 0, children: availableBufferSizes.length === 0 ? ((0, jsx_runtime_1.jsx)("option", { value: "", children: "Select devices first" })) : (availableBufferSizes.map(size => ((0, jsx_runtime_1.jsxs)("option", { value: size, children: [size, " samples (~", ((size / (selectedSampleRate || 48000)) * 1000).toFixed(1), " ms)"] }, size)))) })] }), (0, jsx_runtime_1.jsx)("div", { className: "settings-section", children: (0, jsx_runtime_1.jsx)("button", { className: "btn btn-secondary", onClick: refreshDevices, disabled: applying, children: "\u21BB Refresh Devices" }) })] }))] }));
});
//# sourceMappingURL=AudioSettings.js.map