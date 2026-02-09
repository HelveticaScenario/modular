/**
 * Definition of a slider control created by the `slider()` DSL function.
 */
export interface SliderDefinition {
    /** Backing signal module ID */
    moduleId: string;
    /** Display label for the slider */
    label: string;
    /** Current value */
    value: number;
    /** Minimum value */
    min: number;
    /** Maximum value */
    max: number;
}
