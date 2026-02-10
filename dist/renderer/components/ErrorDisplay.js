"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.ErrorDisplay = ErrorDisplay;
const jsx_runtime_1 = require("react/jsx-runtime");
function ErrorDisplay({ error, errors, onDismiss }) {
    if (!error && (!errors || errors.length === 0))
        return null;
    return ((0, jsx_runtime_1.jsx)("div", { className: "error-display", children: (0, jsx_runtime_1.jsxs)("div", { className: "error-content", children: [(0, jsx_runtime_1.jsx)("span", { className: "error-icon", children: "\u26A0\uFE0F" }), (0, jsx_runtime_1.jsxs)("div", { className: "error-messages", children: [error && (0, jsx_runtime_1.jsx)("pre", { className: "error-message", children: error }), errors && errors.length > 0 && ((0, jsx_runtime_1.jsx)("ul", { className: "validation-errors", children: errors.map((err, i) => ((0, jsx_runtime_1.jsxs)("li", { className: "validation-error", children: [(0, jsx_runtime_1.jsxs)("div", { className: "validation-error-main", children: [(0, jsx_runtime_1.jsx)("strong", { children: err.message }), err.location && ((0, jsx_runtime_1.jsxs)("span", { className: "error-location", children: [' ', "at ", err.location] }))] }), err.expectedType && ((0, jsx_runtime_1.jsxs)("div", { className: "validation-error-expected", children: ["Expected: ", err.expectedType] }))] }, i))) }))] }), (0, jsx_runtime_1.jsx)("button", { className: "error-dismiss", onClick: onDismiss, children: "\u00D7" })] }) }));
}
//# sourceMappingURL=ErrorDisplay.js.map