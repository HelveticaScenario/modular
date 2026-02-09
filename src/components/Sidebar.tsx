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
        <div className="sidebar">
            <div className="sidebar-tabs">
                <button
                    className={`sidebar-tab ${activeTab === 'explorer' ? 'active' : ''}`}
                    onClick={() => setActiveTab('explorer')}
                >
                    Explorer
                </button>
                <button
                    className={`sidebar-tab ${activeTab === 'control' ? 'active' : ''}`}
                    onClick={() => setActiveTab('control')}
                >
                    Control
                </button>
            </div>
            <div className="sidebar-content">
                {activeTab === 'explorer' ? explorerContent : controlContent}
            </div>
        </div>
    );
}
