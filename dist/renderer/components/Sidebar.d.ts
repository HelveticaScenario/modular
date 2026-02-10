import { type ReactNode } from 'react';
import './Sidebar.css';
export type SidebarTab = 'explorer' | 'control';
interface SidebarProps {
    explorerContent: ReactNode;
    controlContent: ReactNode;
}
export declare function Sidebar({ explorerContent, controlContent }: SidebarProps): import("react/jsx-runtime").JSX.Element;
export {};
