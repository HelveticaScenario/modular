import React, { useState, useMemo, useEffect } from 'react';
import { ModuleSchema } from '@modular/core';
import electronAPI from '../electronAPI';
import './HelpWindow.css';

type Page = 'hotkeys' | 'syntax' | 'math' | 'signals' | 'output' | 'clock' | 'reference';

export const HelpWindow: React.FC = () => {
    const [activePage, setActivePage] = useState<Page>('hotkeys');
    const [searchQuery, setSearchQuery] = useState('');
    const [schemas, setSchemas] = useState<Record<string, ModuleSchema>>({});
    
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

    const filteredModules = useMemo(() => {
        if (!schemas) return [];
        return Object.values(schemas).filter(schema => 
            schema.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
            schema.description.toLowerCase().includes(searchQuery.toLowerCase())
        );
    }, [schemas, searchQuery]);

    const getParamNames = (module: any) => {
        const schema = module.paramsSchema?.schema;
        if (!schema) return [];
        // Handle RootSchema (has .schema property) or SchemaObject (has .properties)
        const props = schema.properties || schema.schema?.properties || {};
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
            case 'signals':
                return (
                    <div>
                        <h2>Signal Types</h2>
                        <p>All signals are audio-rate floating point numbers.</p>
                    </div>
                );
            case 'output':
                return (
                    <div>
                        <h2>Sound Output</h2>
                        <p>Use `signal.out()` to send audio to the speakers. Multiple signals can be sent to output and will be summed together.</p>
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
                            style={{ width: '100%', padding: '8px', marginBottom: '16px', background: '#333', color: '#fff', border: '1px solid #444' }}
                        />
                        {filteredModules.map(module => (
                            <div key={module.name} className="module-card">
                                <h3>{module.name}</h3>
                                <p style={{ whiteSpace: 'pre-wrap' }}>{module.description}</p>
                                <h4>Inputs</h4>
                                <ul>
                                    {getParamNames(module).map(param => (
                                        <li key={param}>{param}</li>
                                    ))}
                                </ul>
                                <h4>Outputs</h4>
                                <ul>
                                    {module.outputs.map(out => (
                                        <li key={out.name}>{out.name}: {out.description}</li>
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
                <button onClick={() => setActivePage('hotkeys')}>Hotkeys</button>
                <button onClick={() => setActivePage('syntax')}>Sequence Syntax</button>
                <button onClick={() => setActivePage('math')}>Math Module</button>
                <button onClick={() => setActivePage('signals')}>Signal Types</button>
                <button onClick={() => setActivePage('output')}>Sound Output</button>
                <button onClick={() => setActivePage('clock')}>Root Clock</button>
                <button onClick={() => setActivePage('reference')}>Reference</button>
            </div>
            <div className="content">
                {renderContent()}
            </div>
        </div>
    );
};
