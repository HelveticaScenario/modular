"use strict";
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.AudioControls = AudioControls;
const jsx_runtime_1 = require("react/jsx-runtime");
const electronAPI_1 = __importDefault(require("../electronAPI"));
function AudioControls({ isRunning, isRecording, onStop, onStartRecording, onStopRecording, onUpdatePatch, }) {
    return ((0, jsx_runtime_1.jsx)("div", { className: "audio-controls", children: (0, jsx_runtime_1.jsxs)("div", { className: "control-buttons", children: [(0, jsx_runtime_1.jsx)("button", { onClick: onUpdatePatch, className: "btn btn-primary", title: "Ctrl+Enter / Cmd+Enter", children: "\u25B6 Update Patch" }), (0, jsx_runtime_1.jsx)("button", { onClick: onStop, disabled: !isRunning, className: "btn btn-danger", title: "Ctrl+. / Cmd+.", children: "\u23F9 Stop" }), isRecording ? ((0, jsx_runtime_1.jsx)("button", { onClick: onStopRecording, className: "btn btn-danger recording", children: "\u23FA Stop Recording" })) : ((0, jsx_runtime_1.jsx)("button", { onClick: onStartRecording, className: "btn btn-secondary", title: "Ctrl+R / Cmd+R", children: "\u23FA Record" })), (0, jsx_runtime_1.jsx)("button", { onClick: () => electronAPI_1.default.openHelpWindow(), className: "btn btn-secondary", children: "? Help" })] }) }));
}
//# sourceMappingURL=AudioControls.js.map