import type { Monaco } from '../../hooks/useCustomMonaco';
export declare function registerConfigSchema(monaco: Monaco, schema: object): void;
export declare function registerConfigSchemaForFile(monaco: Monaco, schema: object, currentFile: string): string;
