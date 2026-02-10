import type { Monaco } from '../../hooks/useCustomMonaco';
import type { ModuleSchema } from '@modular/core';
export interface MonacoSetupOptions {
    /** Module schemas for building symbol sets */
    schemas?: ModuleSchema[];
}
export declare function setupMonacoJavascript(monaco: Monaco, libSource: string, options?: MonacoSetupOptions): () => void;
