export function exhaustiveSwitch(value: never): never {
    throw new Error(`Exhaustive switch failed: ${value}`);
}
