/**
 * Global type declarations for Electron IPC API
 */

import type { ElectronAPI } from './preload';

declare global {
  interface Window {
    electronAPI: ElectronAPI;
  }
}

export {};
