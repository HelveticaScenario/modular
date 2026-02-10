"use strict";
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.HelpWindow = void 0;
const jsx_runtime_1 = require("react/jsx-runtime");
const react_1 = require("react");
const electronAPI_1 = __importDefault(require("../electronAPI"));
const typeDocs_1 = require("../../shared/dsl/typeDocs");
require("./HelpWindow.css");
/**
 * Regex pattern matching all DSL type names for linkification.
 * Uses word boundaries to avoid matching partial words.
 */
const TYPE_PATTERN = new RegExp(`\\b(${typeDocs_1.DSL_TYPE_NAMES.join('|')})\\b`, 'g');
/**
 * A clickable link that navigates to a type's documentation.
 */
const TypeLink = ({ typeName, onTypeClick }) => ((0, jsx_runtime_1.jsx)("button", { className: "type-link", onClick: () => onTypeClick(typeName), title: `View ${typeName} documentation`, children: typeName }));
/**
 * Component that renders text with DSL type names as clickable links.
 */
const LinkifyTypes = ({ text, onTypeClick }) => {
    const parts = [];
    let lastIndex = 0;
    let match;
    let key = 0;
    // Reset regex state
    TYPE_PATTERN.lastIndex = 0;
    while ((match = TYPE_PATTERN.exec(text)) !== null) {
        // Add text before the match
        if (match.index > lastIndex) {
            parts.push(text.slice(lastIndex, match.index));
        }
        // Add the type link
        const typeName = match[1];
        parts.push((0, jsx_runtime_1.jsx)(TypeLink, { typeName: typeName, onTypeClick: onTypeClick }, key++));
        lastIndex = match.index + match[0].length;
    }
    // Add remaining text
    if (lastIndex < text.length) {
        parts.push(text.slice(lastIndex));
    }
    return (0, jsx_runtime_1.jsx)(jsx_runtime_1.Fragment, { children: parts });
};
/**
 * Card displaying documentation for a single DSL type.
 */
const TypeCard = ({ typeDoc, isExpanded, onToggle, onTypeClick }) => ((0, jsx_runtime_1.jsxs)("div", { className: `type-card ${isExpanded ? 'expanded' : ''}`, children: [(0, jsx_runtime_1.jsxs)("div", { className: "type-card-header", onClick: onToggle, children: [(0, jsx_runtime_1.jsx)("h3", { children: typeDoc.name }), (0, jsx_runtime_1.jsx)("span", { className: "expand-icon", children: isExpanded ? '▼' : '▶' })] }), (0, jsx_runtime_1.jsx)("p", { className: "type-description", children: (0, jsx_runtime_1.jsx)(LinkifyTypes, { text: typeDoc.description, onTypeClick: onTypeClick }) }), isExpanded && ((0, jsx_runtime_1.jsxs)("div", { className: "type-details", children: [typeDoc.definition && ((0, jsx_runtime_1.jsxs)("div", { className: "type-definition", children: [(0, jsx_runtime_1.jsx)("h4", { children: "Definition" }), (0, jsx_runtime_1.jsx)("code", { children: typeDoc.definition })] })), typeDoc.examples.length > 0 && ((0, jsx_runtime_1.jsxs)("div", { className: "type-examples", children: [(0, jsx_runtime_1.jsx)("h4", { children: "Examples" }), (0, jsx_runtime_1.jsx)("pre", { children: typeDoc.examples.join('\n') })] })), typeDoc.methods && typeDoc.methods.length > 0 && ((0, jsx_runtime_1.jsxs)("div", { className: "type-methods", children: [(0, jsx_runtime_1.jsx)("h4", { children: "Methods" }), typeDoc.methods.map(method => ((0, jsx_runtime_1.jsxs)("div", { className: "method-card", children: [(0, jsx_runtime_1.jsx)("code", { className: "method-signature", children: (0, jsx_runtime_1.jsx)(LinkifyTypes, { text: method.signature, onTypeClick: onTypeClick }) }), (0, jsx_runtime_1.jsx)("p", { children: (0, jsx_runtime_1.jsx)(LinkifyTypes, { text: method.description, onTypeClick: onTypeClick }) }), method.example && ((0, jsx_runtime_1.jsx)("pre", { className: "method-example", children: method.example }))] }, method.name)))] })), typeDoc.seeAlso.length > 0 && ((0, jsx_runtime_1.jsxs)("div", { className: "type-see-also", children: [(0, jsx_runtime_1.jsx)("h4", { children: "See Also" }), (0, jsx_runtime_1.jsx)("div", { className: "see-also-links", children: typeDoc.seeAlso.map(typeName => ((0, typeDocs_1.isDslType)(typeName) ? ((0, jsx_runtime_1.jsx)(TypeLink, { typeName: typeName, onTypeClick: onTypeClick }, typeName)) : ((0, jsx_runtime_1.jsx)("span", { children: typeName }, typeName)))) })] }))] }))] }));
const HelpWindow = () => {
    const [activePage, setActivePage] = (0, react_1.useState)('hotkeys');
    const [searchQuery, setSearchQuery] = (0, react_1.useState)('');
    const [schemas, setSchemas] = (0, react_1.useState)({});
    const [selectedType, setSelectedType] = (0, react_1.useState)(null);
    const [expandedTypes, setExpandedTypes] = (0, react_1.useState)(new Set());
    // Fetch schemas once at mount
    (0, react_1.useEffect)(() => {
        electronAPI_1.default.getSchemas().then((schemaList) => {
            const schemaMap = {};
            for (const s of schemaList) {
                schemaMap[s.name] = s;
            }
            setSchemas(schemaMap);
        }).catch(console.error);
    }, []);
    // Listen for navigation events from main process (definition provider)
    (0, react_1.useEffect)(() => {
        const unsubscribe = electronAPI_1.default.onNavigateToSymbol?.((data) => {
            const { symbolType, symbolName } = data;
            if (symbolType === 'type' && (0, typeDocs_1.isDslType)(symbolName)) {
                // Navigate to types page and expand the type
                setActivePage('types');
                setSelectedType(symbolName);
                setExpandedTypes(prev => new Set(prev).add(symbolName));
            }
            else if (symbolType === 'module' || symbolType === 'namespace') {
                // Navigate to reference page and search for the module
                setActivePage('reference');
                setSearchQuery(symbolName);
            }
        });
        return () => unsubscribe?.();
    }, []);
    // Handle type link clicks - navigate to types page and expand the type
    const handleTypeClick = (0, react_1.useCallback)((typeName) => {
        setActivePage('types');
        setSelectedType(typeName);
        setExpandedTypes(prev => new Set(prev).add(typeName));
    }, []);
    // Toggle type card expansion
    const toggleTypeExpanded = (0, react_1.useCallback)((typeName) => {
        setExpandedTypes(prev => {
            const next = new Set(prev);
            if (next.has(typeName)) {
                next.delete(typeName);
            }
            else {
                next.add(typeName);
            }
            return next;
        });
    }, []);
    // Scroll to selected type when it changes
    (0, react_1.useEffect)(() => {
        if (selectedType && activePage === 'types') {
            const element = document.getElementById(`type-${selectedType}`);
            if (element) {
                element.scrollIntoView({ behavior: 'smooth', block: 'start' });
            }
        }
    }, [selectedType, activePage]);
    const filteredModules = (0, react_1.useMemo)(() => {
        if (!schemas)
            return [];
        return Object.values(schemas).filter(schema => schema.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
            schema.description.toLowerCase().includes(searchQuery.toLowerCase()));
    }, [schemas, searchQuery]);
    const getParamNames = (module) => {
        const paramsSchema = module.paramsSchema?.schema;
        if (!paramsSchema)
            return [];
        // Handle RootSchema (has .schema property) or SchemaObject (has .properties)
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        const schemaObj = paramsSchema;
        const props = schemaObj.properties || schemaObj.schema?.properties || {};
        return Object.keys(props);
    };
    const renderContent = () => {
        switch (activePage) {
            case 'hotkeys':
                return ((0, jsx_runtime_1.jsxs)("div", { children: [(0, jsx_runtime_1.jsx)("h2", { children: "Hotkeys" }), (0, jsx_runtime_1.jsxs)("ul", { children: [(0, jsx_runtime_1.jsxs)("li", { children: [(0, jsx_runtime_1.jsx)("b", { children: "Ctrl + Enter" }), ": Update Patch"] }), (0, jsx_runtime_1.jsxs)("li", { children: [(0, jsx_runtime_1.jsx)("b", { children: "Ctrl + ." }), ": Stop Sound"] }), (0, jsx_runtime_1.jsxs)("li", { children: [(0, jsx_runtime_1.jsx)("b", { children: "Cmd/Ctrl + S" }), ": Save"] }), (0, jsx_runtime_1.jsxs)("li", { children: [(0, jsx_runtime_1.jsx)("b", { children: "Cmd/Ctrl + O" }), ": Open Workspace"] })] })] }));
            case 'syntax':
                return ((0, jsx_runtime_1.jsxs)("div", { children: [(0, jsx_runtime_1.jsx)("h2", { children: "Sequence Syntax" }), (0, jsx_runtime_1.jsx)("p", { children: "The sequencer uses a DSL to define patterns." }), (0, jsx_runtime_1.jsxs)("pre", { children: ["\"C4 D4 E4 F4\" // Notes", '\n', "\"C4 - - -\"    // Holds", '\n', "\"C4 . . .\"    // Rests"] })] }));
            case 'math':
                return ((0, jsx_runtime_1.jsxs)("div", { children: [(0, jsx_runtime_1.jsx)("h2", { children: "Math Module" }), (0, jsx_runtime_1.jsx)("p", { children: "Evaluates mathematical expressions." }), (0, jsx_runtime_1.jsx)("p", { children: "Inputs: x, y, z" }), (0, jsx_runtime_1.jsx)("p", { children: "Variable: t (time)" })] }));
            case 'types':
                return ((0, jsx_runtime_1.jsxs)("div", { children: [(0, jsx_runtime_1.jsx)("h2", { children: "Type Reference" }), (0, jsx_runtime_1.jsx)("p", { className: "types-intro", children: "These are the core types used in the modular DSL. Click on any type name throughout the documentation to navigate to its definition." }), (0, jsx_runtime_1.jsx)("div", { className: "types-grid", children: typeDocs_1.DSL_TYPE_NAMES.map(typeName => {
                                const typeDoc = typeDocs_1.TYPE_DOCS[typeName];
                                return ((0, jsx_runtime_1.jsx)("div", { id: `type-${typeName}`, children: (0, jsx_runtime_1.jsx)(TypeCard, { typeDoc: typeDoc, isExpanded: expandedTypes.has(typeName), onToggle: () => toggleTypeExpanded(typeName), onTypeClick: handleTypeClick }) }, typeName));
                            }) })] }));
            case 'output':
                return ((0, jsx_runtime_1.jsxs)("div", { children: [(0, jsx_runtime_1.jsx)("h2", { children: "Sound Output" }), (0, jsx_runtime_1.jsxs)("p", { children: ["Use ", (0, jsx_runtime_1.jsx)(TypeLink, { typeName: "ModuleOutput", onTypeClick: handleTypeClick }), "'s", ' ', (0, jsx_runtime_1.jsx)("code", { children: ".out()" }), " method to send audio to the speakers. Multiple signals can be sent to output and will be summed together."] }), (0, jsx_runtime_1.jsxs)("p", { children: ["For stereo output, use ", (0, jsx_runtime_1.jsx)("code", { children: ".out(channel, options)" }), " where options is a ", (0, jsx_runtime_1.jsx)(TypeLink, { typeName: "StereoOutOptions", onTypeClick: handleTypeClick }), " object."] })] }));
            case 'clock':
                return ((0, jsx_runtime_1.jsxs)("div", { children: [(0, jsx_runtime_1.jsx)("h2", { children: "Root Clock" }), (0, jsx_runtime_1.jsx)("p", { children: "The system has a global clock that drives sequencers." })] }));
            case 'reference':
                return ((0, jsx_runtime_1.jsxs)("div", { children: [(0, jsx_runtime_1.jsx)("h2", { children: "Module Reference" }), (0, jsx_runtime_1.jsx)("input", { type: "text", placeholder: "Search modules...", value: searchQuery, onChange: e => setSearchQuery(e.target.value), className: "search-input" }), filteredModules.map(module => ((0, jsx_runtime_1.jsxs)("div", { className: "module-card", children: [(0, jsx_runtime_1.jsx)("h3", { children: module.name }), (0, jsx_runtime_1.jsx)("p", { style: { whiteSpace: 'pre-wrap' }, children: (0, jsx_runtime_1.jsx)(LinkifyTypes, { text: module.description, onTypeClick: handleTypeClick }) }), (0, jsx_runtime_1.jsx)("h4", { children: "Inputs" }), (0, jsx_runtime_1.jsx)("ul", { children: getParamNames(module).map(param => ((0, jsx_runtime_1.jsx)("li", { children: param }, param))) }), (0, jsx_runtime_1.jsx)("h4", { children: "Outputs" }), (0, jsx_runtime_1.jsx)("ul", { children: module.outputs.map(out => ((0, jsx_runtime_1.jsxs)("li", { children: [(0, jsx_runtime_1.jsx)("strong", { children: out.name }), ": ", ' ', (0, jsx_runtime_1.jsx)(LinkifyTypes, { text: out.description, onTypeClick: handleTypeClick })] }, out.name))) })] }, module.name)))] }));
        }
    };
    return ((0, jsx_runtime_1.jsxs)("div", { className: "help-window", children: [(0, jsx_runtime_1.jsxs)("div", { className: "sidebar", children: [(0, jsx_runtime_1.jsx)("button", { className: activePage === 'hotkeys' ? 'active' : '', onClick: () => setActivePage('hotkeys'), children: "Hotkeys" }), (0, jsx_runtime_1.jsx)("button", { className: activePage === 'syntax' ? 'active' : '', onClick: () => setActivePage('syntax'), children: "Sequence Syntax" }), (0, jsx_runtime_1.jsx)("button", { className: activePage === 'math' ? 'active' : '', onClick: () => setActivePage('math'), children: "Math Module" }), (0, jsx_runtime_1.jsx)("button", { className: activePage === 'types' ? 'active' : '', onClick: () => setActivePage('types'), children: "Types" }), (0, jsx_runtime_1.jsx)("button", { className: activePage === 'output' ? 'active' : '', onClick: () => setActivePage('output'), children: "Sound Output" }), (0, jsx_runtime_1.jsx)("button", { className: activePage === 'clock' ? 'active' : '', onClick: () => setActivePage('clock'), children: "Root Clock" }), (0, jsx_runtime_1.jsx)("button", { className: activePage === 'reference' ? 'active' : '', onClick: () => setActivePage('reference'), children: "Reference" })] }), (0, jsx_runtime_1.jsx)("div", { className: "content", children: renderContent() })] }));
};
exports.HelpWindow = HelpWindow;
//# sourceMappingURL=HelpWindow.js.map