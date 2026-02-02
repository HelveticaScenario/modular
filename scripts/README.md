# Generate Library Script

This script generates TypeScript type definitions for the DSL from the Rust module schemas.

## Usage

```bash
yarn generate-lib
```

This will:
1. Build the native Rust module (`yarn build-native`)
2. Compile the TypeScript script
3. Call `getSchemas()` from the N-API module
4. Generate TypeScript definitions using `buildLibSource()`
5. Write the output to `generated/dsl.d.ts`

## Output

The generated file contains:
- Console API definitions
- TypeScript type definitions for all DSL modules
- Factory functions with proper typing
- Module output interfaces

## Files

- `scripts/generateLib.ts` - The generation script
- `generated/dsl.d.ts` - Generated output (not committed)
