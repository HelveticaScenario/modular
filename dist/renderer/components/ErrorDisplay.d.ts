import { ValidationError } from '@modular/core';
interface ErrorDisplayProps {
    error: string | null;
    errors?: ValidationError[] | null;
    onDismiss: () => void;
}
export declare function ErrorDisplay({ error, errors, onDismiss }: ErrorDisplayProps): import("react/jsx-runtime").JSX.Element | null;
export {};
