"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.ControlPanel = ControlPanel;
const jsx_runtime_1 = require("react/jsx-runtime");
const react_1 = require("react");
require("./ControlPanel.css");
function ControlPanel({ sliders, onSliderChange }) {
    if (sliders.length === 0) {
        return ((0, jsx_runtime_1.jsx)("div", { className: "control-panel control-panel-empty", children: (0, jsx_runtime_1.jsxs)("div", { className: "control-panel-placeholder", children: [(0, jsx_runtime_1.jsx)("p", { children: "No sliders defined." }), (0, jsx_runtime_1.jsxs)("p", { className: "control-panel-hint", children: ["Use ", (0, jsx_runtime_1.jsx)("code", { children: "slider(label, value, min, max)" }), " in your patch."] })] }) }));
    }
    return ((0, jsx_runtime_1.jsx)("div", { className: "control-panel", children: (0, jsx_runtime_1.jsx)("div", { className: "control-panel-sliders", children: sliders.map((s) => ((0, jsx_runtime_1.jsx)(SliderControl, { slider: s, onChange: onSliderChange }, s.label))) }) }));
}
function SliderControl({ slider, onChange }) {
    const [localValue, setLocalValue] = (0, react_1.useState)(slider.value);
    // Sync local state when slider definition changes (e.g., re-execution)
    const [prevValue, setPrevValue] = (0, react_1.useState)(slider.value);
    if (slider.value !== prevValue) {
        setLocalValue(slider.value);
        setPrevValue(slider.value);
    }
    const step = (slider.max - slider.min) / 1000;
    const handleInput = (0, react_1.useCallback)((e) => {
        const newValue = parseFloat(e.target.value);
        setLocalValue(newValue);
        onChange(slider.label, newValue);
    }, [slider.label, onChange]);
    const formatValue = (v) => {
        // Show up to 4 significant digits, removing trailing zeros
        return Number(v.toPrecision(4)).toString();
    };
    return ((0, jsx_runtime_1.jsxs)("div", { className: "slider-control", children: [(0, jsx_runtime_1.jsxs)("div", { className: "slider-header", children: [(0, jsx_runtime_1.jsx)("span", { className: "slider-label", children: slider.label }), (0, jsx_runtime_1.jsx)("span", { className: "slider-value", children: formatValue(localValue) })] }), (0, jsx_runtime_1.jsx)("input", { type: "range", className: "slider-input", min: slider.min, max: slider.max, step: step, value: localValue, onInput: handleInput }), (0, jsx_runtime_1.jsxs)("div", { className: "slider-range", children: [(0, jsx_runtime_1.jsx)("span", { children: formatValue(slider.min) }), (0, jsx_runtime_1.jsx)("span", { children: formatValue(slider.max) })] })] }));
}
//# sourceMappingURL=ControlPanel.js.map