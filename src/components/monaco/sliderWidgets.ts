import { editor } from 'monaco-editor';
import type { Monaco } from '../../hooks/useCustomMonaco';

type SliderCall = {
    fullMatch: string;
    value: number;
    min: number;
    max: number;
    startIndex: number;
    endIndex: number;
    openParenIndex: number;
    valueStartIndex: number;
    valueEndIndex: number;
};

/**
 * Find all slider() calls in the code
 */
export function findSliderCalls(code: string) {
    const regex =
        /slider\s*\(\s*(-?\d+(?:\.\d+)?)\s*,\s*(-?\d+(?:\.\d+)?)\s*,\s*(-?\d+(?:\.\d+)?)\s*\)/g;
    const matches: SliderCall[] = [];
    let match;

    while ((match = regex.exec(code)) !== null) {
        const startIndex = match.index;
        const endIndex = startIndex + match[0].length;
        const openParenIndex = code.indexOf('(', startIndex);
        const firstArgMatch = match[1];
        const firstArgStart = match[0].indexOf(firstArgMatch);

        matches.push({
            fullMatch: match[0],
            value: parseFloat(match[1]),
            min: parseFloat(match[2]),
            max: parseFloat(match[3]),
            startIndex,
            endIndex,
            openParenIndex,
            valueStartIndex: startIndex + firstArgStart,
            valueEndIndex: startIndex + firstArgStart + firstArgMatch.length,
        });
    }

    return matches;
}

export function createSliderWidgets(
    editorInstance: editor.IStandaloneCodeEditor,
    model: editor.ITextModel,
    monaco: Monaco,
    code: string,
) {
    if (!monaco) return [];
    let sliderCalls = findSliderCalls(code);
    const sliderWidgets: editor.IContentWidget[] = [];
    for (const [index, call] of sliderCalls.entries()) {
        console.log('Slider call:', call);
        const position = model.getPositionAt(call.openParenIndex + 1);

        // Create slider widget DOM
        const widgetId = `slider-widget-${index}-${Date.now()}`;

        const slider = document.createElement('input');
        slider.style.width = `${
            editorInstance.getOption(monaco.editor.EditorOption.fontInfo)
                .typicalHalfwidthCharacterWidth * 10
        }px`;
        slider.style.height = `${editorInstance.getOption(
            monaco.editor.EditorOption.lineHeight,
        )}px`;
        slider.style.pointerEvents = 'auto';

        // Map call.value between call.min and call.max
        const mappedValue = (call.value - call.min) / (call.max - call.min);

        slider.type = 'range';
        slider.min = '0';
        slider.max = '1';
        slider.value = mappedValue.toString(10);
        // Set appropriate step size
        slider.step = '0.01';

        // Update code when slider changes
        slider.addEventListener('input', (e: Event) => {
            console.log('Slider changed:', e);
            sliderCalls = findSliderCalls(editorInstance.getValue());
            const updatedCall = sliderCalls[index];
            if (!updatedCall) return;
            const target = e.target as HTMLInputElement | null;
            const newValue = parseFloat(target?.value ?? '0');

            const valuePos = model.getPositionAt(updatedCall.valueStartIndex);
            const valueEndPos = model.getPositionAt(updatedCall.valueEndIndex);

            const formattedValue = newValue.toFixed(2);

            editorInstance.executeEdits('slider-update', [
                {
                    range: new monaco.Range(
                        valuePos.lineNumber,
                        valuePos.column,
                        valueEndPos.lineNumber,
                        valueEndPos.column,
                    ),
                    text: formattedValue,
                },
            ]);
        });

        const domNode = document.createElement('div');
        domNode.className = 'slider-widget';
        domNode.appendChild(slider);

        // Create and add content widget
        const contentWidget: editor.IContentWidget = {
            getId: () => widgetId,
            getDomNode: () => domNode,
            getPosition: () => ({
                position: {
                    lineNumber: position.lineNumber,
                    column: position.column,
                },
                preference: [
                    monaco.editor.ContentWidgetPositionPreference.EXACT,
                ],
            }),
        };

        editorInstance.addContentWidget(contentWidget);
        sliderWidgets.push(contentWidget);
    }
    return sliderWidgets;
}