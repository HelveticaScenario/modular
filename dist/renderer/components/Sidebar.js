"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.Sidebar = Sidebar;
const jsx_runtime_1 = require("react/jsx-runtime");
const react_1 = require("react");
require("./Sidebar.css");
function Sidebar({ explorerContent, controlContent }) {
    const [activeTab, setActiveTab] = (0, react_1.useState)('explorer');
    return ((0, jsx_runtime_1.jsxs)("div", { className: "app-sidebar", children: [(0, jsx_runtime_1.jsxs)("div", { className: "app-sidebar-tabs", children: [(0, jsx_runtime_1.jsx)("button", { className: `app-sidebar-tab ${activeTab === 'explorer' ? 'active' : ''}`, onClick: () => setActiveTab('explorer'), children: "Explorer" }), (0, jsx_runtime_1.jsx)("button", { className: `app-sidebar-tab ${activeTab === 'control' ? 'active' : ''}`, onClick: () => setActiveTab('control'), children: "Control" })] }), (0, jsx_runtime_1.jsx)("div", { className: "app-sidebar-content", children: activeTab === 'explorer' ? explorerContent : controlContent })] }));
}
//# sourceMappingURL=Sidebar.js.map