"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.useCustomMonaco = useCustomMonaco;
const dist_1 = require("@monaco-editor/react/dist");
const react_1 = require("react");
function useCustomMonaco() {
    const [monaco, setMonaco] = (0, react_1.useState)(dist_1.loader.__getMonacoInstance());
    (0, react_1.useEffect)(() => {
        let cancelable;
        if (!monaco) {
            cancelable = dist_1.loader.init();
            cancelable.then((monaco) => {
                setMonaco(monaco);
            }).catch((err) => {
                if (err.type !== 'cancelation') {
                    console.error('Monaco initialization error:', err);
                    console.error('If you are running in dev with StrictMode, this can be safely ignored.');
                }
            });
        }
        return () => cancelable?.cancel();
    }, []);
    return monaco;
}
exports.default = useCustomMonaco;
//# sourceMappingURL=useCustomMonaco.js.map