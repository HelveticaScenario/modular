import { loader } from "@monaco-editor/react/dist";
import type * as monaco_editor from "monaco-editor";
import { useState, useEffect } from "react";

export function useCustomMonaco() {
    const [monaco, setMonaco] = useState(loader.__getMonacoInstance() as typeof monaco_editor | null);

    useEffect(() => {
        let cancelable: ReturnType<typeof loader.init>;

        if (!monaco) {
            cancelable = loader.init();

            cancelable.then((monaco) => {
                setMonaco(monaco as typeof monaco_editor);
            }).catch((err) => {
                if (err.type !== 'cancelation') {
                    console.error('Monaco initialization error:', err);
                    console.error('If you are running in dev with StrictMode, this can be safely ignored.');
                }
            })
        }

        return () => cancelable?.cancel();
    }, []);

    return monaco;
}

export type Monaco = NonNullable<ReturnType<typeof useCustomMonaco>>;

export default useCustomMonaco;
