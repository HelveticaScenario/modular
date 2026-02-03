import React, { useState, useMemo, useEffect, useCallback, ReactNode } from 'react';
import { ModuleSchema } from '@modular/core';
import electronAPI from '../electronAPI';
import { TYPE_DOCS, DSL_TYPE_NAMES, DslTypeName, TypeDocumentation, isDslType } from '../dsl/typeDocs';
import './HelpWindow.css';

type Page = 'hotkeys' | 'syntax' | 'math' | 'types' | 'output' | 'clock' | 'reference';

/**
 * Regex pattern matching all DSL type names for linkification.
 * Uses word boundaries to avoid matching partial words.
 */
const TYPE_PATTERN = new RegExp(
    `\\b(${DSL_TYPE_NAMES.join('|')})\\b`,
    'g'
);

interface TypeLinkProps {
    typeName: DslTypeName;
    onTypeClick: (typeName: DslTypeName) => void;
}

/**
 * A clickable link that navigates to a type's documentation.
 */
const TypeLink: React.FC<TypeLinkProps> = ({ typeName, onTypeClick }) => (
    <button
        className="type-link"
        onClick={() => onTypeClick(typeName)}
        title={`View ${typeName} documentation`}
    >
        {typeName}
    </button>
);

interface LinkifyTypesProps {
    text: string;
    onTypeClick: (typeName: DslTypeName) => void;
}

/**
 * Component that renders text with DSL type names as clickable links.
 */
const LinkifyTypes: React.FC<LinkifyTypesProps> = ({ text, onTypeClick }) => {
    const parts: ReactNode[] = [];
    let lastIndex = 0;
    let match: RegExpExecArray | null;
    let key = 0;

    // Reset regex state
    TYPE_PATTERN.lastIndex = 0;

    while ((match = TYPE_PATTERN.exec(text)) !== null) {
        // Add text before the match
        if (match.index > lastIndex) {
            parts.push(text.slice(lastIndex, match.index));
        }
        // Add the type link
        const typeName = match[1] as DslTypeName;
        parts.push(
            <TypeLink
                key={key++}
                typeName={typeName}
                onTypeClick={onTypeClick}
            />
        );
        lastIndex = match.index + match[0].length;
    }

    // Add remaining text
    if (lastIndex < text.length) {
        parts.push(text.slice(lastIndex));
    }

    return <>{parts}</>;
};

interface TypeCardProps {
    typeDoc: TypeDocumentation;
    isExpanded: boolean;
    onToggle: () => void;
    onTypeClick: (typeName: DslTypeName) => void;
}

/**
 * Card displaying documentation for a single DSL type.
 */
const TypeCard: React.FC<TypeCardProps> = ({ typeDoc, isExpanded, onToggle, onTypeClick }) => (
    <div className={`type-card ${isExpanded ? 'expanded' : ''}`}>
        <div className="type-card-header" onClick={onToggle}>
            <h3>{typeDoc.name}</h3>
            <span className="expand-icon">{isExpanded ? '▼' : '▶'}</span>
        </div>
        
        <p className="type-description">
            <LinkifyTypes text={typeDoc.description} onTypeClick={onTypeClick} />
        </p>
        
        {isExpanded && (
            <div className="type-details">
                {typeDoc.definition && (
                    <div className="type-definition">
                        <h4>Definition</h4>
                        <code>{typeDoc.definition}</code>
                    </div>
                )}
                
                {typeDoc.examples.length > 0 && (
                    <div className="type-examples">
                        <h4>Examples</h4>
                        <pre>{typeDoc.examples.join('\n')}</pre>
                    </div>
                )}
                
                {typeDoc.methods && typeDoc.methods.length > 0 && (
                    <div className="type-methods">
                        <h4>Methods</h4>
                        {typeDoc.methods.map(method => (
                            <div key={method.name} className="method-card">
                                <code className="method-signature">
                                    <LinkifyTypes text={method.signature} onTypeClick={onTypeClick} />
                                </code>
                                <p>
                                    <LinkifyTypes text={method.description} onTypeClick={onTypeClick} />
                                </p>
                                {method.example && (
                                    <pre className="method-example">{method.example}</pre>
                                )}
                            </div>
                        ))}
                    </div>
                )}
                
                {typeDoc.seeAlso.length > 0 && (
                    <div className="type-see-also">
                        <h4>See Also</h4>
                        <div className="see-also-links">
                            {typeDoc.seeAlso.map(typeName => (
                                isDslType(typeName) ? (
                                    <TypeLink
                                        key={typeName}
                                        typeName={typeName}
                                        onTypeClick={onTypeClick}
                                    />
                                ) : (
                                    <span key={typeName}>{typeName}</span>
                                )
                            ))}
                        </div>
                    </div>
                )}
            </div>
        )}
    </div>
);

export const HelpWindow: React.FC = () => {
    const [activePage, setActivePage] = useState<Page>('hotkeys');
    const [searchQuery, setSearchQuery] = useState('');
    const [schemas, setSchemas] = useState<Record<string, ModuleSchema>>({});
    const [selectedType, setSelectedType] = useState<DslTypeName | null>(null);
    const [expandedTypes, setExpandedTypes] = useState<Set<DslTypeName>>(new Set());
    
    // Fetch schemas once at mount
    useEffect(() => {
        electronAPI.getSchemas().then((schemaList) => {
            const schemaMap: Record<string, ModuleSchema> = {};
            for (const s of schemaList) {
                schemaMap[s.name] = s;
            }
            setSchemas(schemaMap);
        }).catch(console.error);
    }, []);

    // Listen for navigation events from main process (definition provider)
    useEffect(() => {
        const unsubscribe = electronAPI.onNavigateToSymbol?.((data) => {
            const { symbolType, symbolName } = data;
            
            if (symbolType === 'type' && isDslType(symbolName)) {
                // Navigate to types page and expand the type
                setActivePage('types');
                setSelectedType(symbolName);
                setExpandedTypes(prev => new Set(prev).add(symbolName));
            } else if (symbolType === 'module' || symbolType === 'namespace') {
                // Navigate to reference page and search for the module
                setActivePage('reference');
                setSearchQuery(symbolName);
            }
        });
        
        return () => unsubscribe?.();
    }, []);

    // Handle type link clicks - navigate to types page and expand the type
    const handleTypeClick = useCallback((typeName: DslTypeName) => {
        setActivePage('types');
        setSelectedType(typeName);
        setExpandedTypes(prev => new Set(prev).add(typeName));
    }, []);

    // Toggle type card expansion
    const toggleTypeExpanded = useCallback((typeName: DslTypeName) => {
        setExpandedTypes(prev => {
            const next = new Set(prev);
            if (next.has(typeName)) {
                next.delete(typeName);
            } else {
                next.add(typeName);
            }
            return next;
        });
    }, []);

    // Scroll to selected type when it changes
    useEffect(() => {
        if (selectedType && activePage === 'types') {
            const element = document.getElementById(`type-${selectedType}`);
            if (element) {
                element.scrollIntoView({ behavior: 'smooth', block: 'start' });
            }
        }
    }, [selectedType, activePage]);

    const filteredModules = useMemo(() => {
        if (!schemas) return [];
        return Object.values(schemas).filter(schema => 
            schema.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
            schema.description.toLowerCase().includes(searchQuery.toLowerCase())
        );
    }, [schemas, searchQuery]);

    const getParamNames = (module: ModuleSchema) => {
        const paramsSchema = module.paramsSchema?.schema;
        if (!paramsSchema) return [];
        // Handle RootSchema (has .schema property) or SchemaObject (has .properties)
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        const schemaObj = paramsSchema as any;
        const props = schemaObj.properties || schemaObj.schema?.properties || {};
        return Object.keys(props);
    };

    const renderContent = () => {
        switch (activePage) {
            case 'hotkeys':
                return (
                    <div>
                        <h2>Hotkeys</h2>
                        <ul>
                            <li><b>Ctrl + Enter</b>: Update Patch</li>
                            <li><b>Ctrl + .</b>: Stop Sound</li>
                            <li><b>Cmd/Ctrl + S</b>: Save</li>
                            <li><b>Cmd/Ctrl + O</b>: Open Workspace</li>
                        </ul>
                    </div>
                );
            case 'syntax':
                return (
                    <div>
                        <h2>Sequence Syntax</h2>
                        <p>The sequencer uses a DSL to define patterns.</p>
                        <pre>
                            "C4 D4 E4 F4" // Notes{'\n'}
                            "C4 - - -"    // Holds{'\n'}
                            "C4 . . ."    // Rests
                        </pre>
                    </div>
                );
            case 'math':
                return (
                    <div>
                        <h2>Math Module</h2>
                        <p>Evaluates mathematical expressions.</p>
                        <p>Inputs: x, y, z</p>
                        <p>Variable: t (time)</p>
                    </div>
                );
            case 'types':
                return (
                    <div>
                        <h2>Type Reference</h2>
                        <p className="types-intro">
                            These are the core types used in the modular DSL. Click on any type name 
                            throughout the documentation to navigate to its definition.
                        </p>
                        
                        <div className="types-grid">
                            {DSL_TYPE_NAMES.map(typeName => {
                                const typeDoc = TYPE_DOCS[typeName];
                                return (
                                    <div key={typeName} id={`type-${typeName}`}>
                                        <TypeCard
                                            typeDoc={typeDoc}
                                            isExpanded={expandedTypes.has(typeName)}
                                            onToggle={() => toggleTypeExpanded(typeName)}
                                            onTypeClick={handleTypeClick}
                                        />
                                    </div>
                                );
                            })}
                        </div>
                    </div>
                );
            case 'output':
                return (
                    <div>
                        <h2>Sound Output</h2>
                        <p>
                            Use <TypeLink typeName="ModuleOutput" onTypeClick={handleTypeClick} />'s 
                            {' '}<code>.out()</code> method to send audio to the speakers. 
                            Multiple signals can be sent to output and will be summed together.
                        </p>
                        <p>
                            For stereo output, use <code>.out(channel, options)</code> where options 
                            is a <TypeLink typeName="StereoOutOptions" onTypeClick={handleTypeClick} /> object.
                        </p>
                    </div>
                );
            case 'clock':
                return (
                    <div>
                        <h2>Root Clock</h2>
                        <p>The system has a global clock that drives sequencers.</p>
                    </div>
                );
            case 'reference':
                return (
                    <div>
                        <h2>Module Reference</h2>
                        <input 
                            type="text" 
                            placeholder="Search modules..." 
                            value={searchQuery}
                            onChange={e => setSearchQuery(e.target.value)}
                            className="search-input"
                        />
                        {filteredModules.map(module => (
                            <div key={module.name} className="module-card">
                                <h3>{module.name}</h3>
                                <p style={{ whiteSpace: 'pre-wrap' }}>
                                    <LinkifyTypes text={module.description} onTypeClick={handleTypeClick} />
                                </p>
                                <h4>Inputs</h4>
                                <ul>
                                    {getParamNames(module).map(param => (
                                        <li key={param}>{param}</li>
                                    ))}
                                </ul>
                                <h4>Outputs</h4>
                                <ul>
                                    {module.outputs.map(out => (
                                        <li key={out.name}>
                                            <strong>{out.name}</strong>: {' '}
                                            <LinkifyTypes text={out.description} onTypeClick={handleTypeClick} />
                                        </li>
                                    ))}
                                </ul>
                            </div>
                        ))}
                    </div>
                );
        }
    };

    return (
        <div className="help-window">
            <div className="sidebar">
                <button 
                    className={activePage === 'hotkeys' ? 'active' : ''} 
                    onClick={() => setActivePage('hotkeys')}
                >
                    Hotkeys
                </button>
                <button 
                    className={activePage === 'syntax' ? 'active' : ''} 
                    onClick={() => setActivePage('syntax')}
                >
                    Sequence Syntax
                </button>
                <button 
                    className={activePage === 'math' ? 'active' : ''} 
                    onClick={() => setActivePage('math')}
                >
                    Math Module
                </button>
                <button 
                    className={activePage === 'types' ? 'active' : ''} 
                    onClick={() => setActivePage('types')}
                >
                    Types
                </button>
                <button 
                    className={activePage === 'output' ? 'active' : ''} 
                    onClick={() => setActivePage('output')}
                >
                    Sound Output
                </button>
                <button 
                    className={activePage === 'clock' ? 'active' : ''} 
                    onClick={() => setActivePage('clock')}
                >
                    Root Clock
                </button>
                <button 
                    className={activePage === 'reference' ? 'active' : ''} 
                    onClick={() => setActivePage('reference')}
                >
                    Reference
                </button>
            </div>
            <div className="content">
                {renderContent()}
            </div>
        </div>
    );
};
