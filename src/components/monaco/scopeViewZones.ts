import type { ScopeView } from '../../types/editor';
import type { editor } from 'monaco-editor';
import type { Monaco } from '../../hooks/useCustomMonaco';

type ScopeViewZoneParams = {
    editor: editor.IStandaloneCodeEditor;
    monaco: Monaco;
    views: ScopeView[];
    onRegisterScopeCanvas?: (key: string, canvas: HTMLCanvasElement) => void;
    onUnregisterScopeCanvas?: (key: string) => void;
};

export function createScopeViewZones({
    editor,
    monaco,
    views,
    onRegisterScopeCanvas,
    onUnregisterScopeCanvas,
}: ScopeViewZoneParams) {
    const viewZoneIds: string[] = [];
    const scopeCanvasMap = new Map<string, HTMLCanvasElement>();
    let layoutListener: ReturnType<editor.IStandaloneCodeEditor['onDidLayoutChange']> | null =
        null;

    const disposeViewZones = () => {
        if (viewZoneIds.length > 0) {
            editor.changeViewZones((accessor) => {
                for (const id of viewZoneIds) {
                    accessor.removeZone(id);
                }
            });
            viewZoneIds.length = 0;
        }

        scopeCanvasMap.forEach((_canvas, key) => {
            onUnregisterScopeCanvas?.(key);
        });
        scopeCanvasMap.clear();

        if (layoutListener) {
            layoutListener.dispose();
            layoutListener = null;
        }
    };

    disposeViewZones();

    if (views.length === 0) {
        return disposeViewZones;
    }

    const dpr = typeof window !== 'undefined' ? window.devicePixelRatio || 1 : 1;
    const layoutInfo = editor.getLayoutInfo();

    const zones = views.map((view) => {
        const container = document.createElement('div');
        container.className = 'scope-view-zone';
        container.style.height = `60px`;
        container.style.width = '100%';
        container.style.display = 'flex';

        const canvas = document.createElement('canvas');
        canvas.style.width = '100%';
        canvas.style.height = '60px';
        canvas.dataset.scopeKey = view.key;

        const pixelWidth = Math.max(1, Math.floor(layoutInfo.contentWidth * dpr));
        const pixelHeight = Math.floor(60 * dpr);
        canvas.width = pixelWidth;
        canvas.height = pixelHeight;

        container.appendChild(canvas);

        scopeCanvasMap.set(view.key, canvas);
        onRegisterScopeCanvas?.(view.key, canvas);

        return { view, container };
    });

    editor.changeViewZones((accessor) => {
        zones.forEach(({ view, container }) => {
            viewZoneIds.push(
                accessor.addZone({
                    afterLineNumber: Math.max(1, view.lineNumber),
                    heightInPx: 60,
                    domNode: container,
                    marginDomNode: undefined,
                }),
            );
        });
    });

    const resizeCanvases = () => {
        const info = editor.getLayoutInfo();
        const nextDpr =
            typeof window !== 'undefined' ? window.devicePixelRatio || 1 : 1;
        scopeCanvasMap.forEach((canvas) => {
            canvas.width = Math.max(1, Math.floor(info.contentWidth * nextDpr));
            canvas.height = Math.floor(60 * nextDpr);
        });
    };

    layoutListener = editor.onDidLayoutChange(resizeCanvases);

    return disposeViewZones;
}