import { useState, type ReactNode } from 'react';
import './Sidebar.css';

export type SidebarTab = 'explorer' | 'control';

interface SidebarProps {
    explorerContent: ReactNode;
    controlContent: ReactNode;
}

export function Sidebar({ explorerContent, controlContent }: SidebarProps) {
    const [activeTab, setActiveTab] = useState<SidebarTab>('explorer');

    return (
        <div className="app-sidebar">
            <div className="app-sidebar-tabs">
                <button
                    className={`app-sidebar-tab ${activeTab === 'explorer' ? 'active' : ''}`}
                    onClick={() => setActiveTab('explorer')}
                >
                    Explorer
                </button>
                <button
                    className={`app-sidebar-tab ${activeTab === 'control' ? 'active' : ''}`}
                    onClick={() => setActiveTab('control')}
                >
                    Control
                </button>
            </div>
            <div className="app-sidebar-content">
                {activeTab === 'explorer' ? explorerContent : controlContent}
            </div>
        </div>
    );
}
