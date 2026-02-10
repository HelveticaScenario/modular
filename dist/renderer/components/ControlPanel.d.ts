import type { SliderDefinition } from '../../shared/dsl/sliderTypes';
import './ControlPanel.css';
interface ControlPanelProps {
    sliders: SliderDefinition[];
    onSliderChange: (label: string, newValue: number) => void;
}
export declare function ControlPanel({ sliders, onSliderChange }: ControlPanelProps): import("react/jsx-runtime").JSX.Element;
export {};
