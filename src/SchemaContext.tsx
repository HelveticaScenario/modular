import { ModuleSchema } from '@modular/core';
import { createContext, useContext } from 'react';

// Global context exposing the latest ModuleSchema[] from the backend.
// Monaco can consume this for schema-driven IntelliSense.
export const SchemasContext = createContext<ModuleSchema[]>([]);

export function useSchemas(): ModuleSchema[] {
    return useContext(SchemasContext);
}
