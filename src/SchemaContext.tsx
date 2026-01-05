import { ModuleSchema } from '@modular/core';
import { createContext, useContext, useEffect, useState, ReactNode } from 'react';
import electronAPI from './electronAPI';

export interface SchemasContextType {
    schemas: Record<string, ModuleSchema>;
    loading: boolean;
}

export const SchemasContext = createContext<SchemasContextType>({
    schemas: {},
    loading: true,
});

export function useSchemas() {
    return useContext(SchemasContext);
}

export const SchemasProvider = ({ children }: { children: ReactNode }) => {
    const [schemas, setSchemas] = useState<Record<string, ModuleSchema>>({});
    const [loading, setLoading] = useState(true);

    useEffect(() => {
        electronAPI.getSchemas().then((schemaList) => {
            const schemaMap: Record<string, ModuleSchema> = {};
            for (const s of schemaList) {
                schemaMap[s.name] = s;
            }
            setSchemas(schemaMap);
            setLoading(false);
        });
    }, []);

    return (
        <SchemasContext.Provider value={{ schemas, loading }}>
            {children}
        </SchemasContext.Provider>
    );
};
