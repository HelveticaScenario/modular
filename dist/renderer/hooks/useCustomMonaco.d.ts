import type * as monaco_editor from "monaco-editor";
export declare function useCustomMonaco(): typeof monaco_editor | null;
export type Monaco = NonNullable<ReturnType<typeof useCustomMonaco>>;
export default useCustomMonaco;
