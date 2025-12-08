import { createContext, useContext } from 'react';
import type { ModuleSchema } from './types/generated/ModuleSchema';

// Global context exposing the latest ModuleSchema[] from the backend.
// Editors (CodeMirror, Monaco) can consume this for schema-driven IntelliSense.
export const SchemasContext = createContext<ModuleSchema[]>([]);

export function useSchemas(): ModuleSchema[] {
    return useContext(SchemasContext);
}
