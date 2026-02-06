/**
 * Source Span Analyzer using ts-morph
 * 
 * Parses DSL source code and extracts absolute character offsets for literal
 * arguments in module factory calls. The registry is keyed by call-site
 * (line:column) for lookup from factory functions at runtime.
 */

import { Project, SyntaxKind, Node, CallExpression, ts } from 'ts-morph';
import type { ModuleSchema } from '@modular/core';

/**
 * Span representing a character range in source code
 */
export interface SourceSpan {
    /** Absolute start offset (0-based) */
    start: number;
    /** Absolute end offset (exclusive) */
    end: number;
}

/**
 * Registry entry for a single call expression's argument spans
 */
export interface CallSiteSpans {
    /** Spans for each positional argument, keyed by argument name */
    args: Map<string, SourceSpan>;
    /** The module type being called (e.g., "seq", "sine") */
    moduleType: string;
}

/**
 * Call site key using line:column format (1-based line, 0-based column)
 * This matches the format produced by Error.captureStackTrace
 */
export type CallSiteKey = `${number}:${number}`;

/**
 * Registry mapping call sites to their argument spans
 */
export type SpanRegistry = Map<CallSiteKey, CallSiteSpans>;

/**
 * Build a set of factory function names from module schemas.
 * Only includes modules that have positional arguments defined.
 */
function buildFactoryNames(schemas: ModuleSchema[]): Set<string> {
    const names = new Set<string>();
    
    for (const schema of schemas) {
        // Only track calls to modules with positional args
        if (schema.positionalArgs && schema.positionalArgs.length > 0) {
            // Extract the final identifier from dotted names (e.g., "seq.iCycle" -> "iCycle")
            // Also include the full path for property access calls
            const parts = schema.name.split('.');
            const finalName = parts[parts.length - 1];
            names.add(finalName);
            
            // Also add the sanitized version (for direct calls like `seqICycle`)
            const sanitized = schema.name.replace(/[^a-zA-Z0-9]+(.)?/g, 
                (_match, chr: string | undefined) => (chr ? chr.toUpperCase() : '')
            );
            names.add(sanitized);
        }
    }
    
    return names;
}

/**
 * Build a map from factory names to their schemas for quick lookup
 */
function buildSchemaMap(schemas: ModuleSchema[]): Map<string, ModuleSchema> {
    const map = new Map<string, ModuleSchema>();
    
    for (const schema of schemas) {
        if (schema.positionalArgs && schema.positionalArgs.length > 0) {
            const parts = schema.name.split('.');
            const finalName = parts[parts.length - 1];
            map.set(finalName, schema);
            
            const sanitized = schema.name.replace(/[^a-zA-Z0-9]+(.)?/g, 
                (_match, chr: string | undefined) => (chr ? chr.toUpperCase() : '')
            );
            map.set(sanitized, schema);
            map.set(schema.name, schema);
        }
    }
    
    return map;
}

/**
 * Extract the function name being called from a CallExpression.
 * Handles both simple calls (foo()) and property access calls (seq.iCycle()).
 */
function getCalledFunctionName(call: CallExpression): string | null {
    const expression = call.getExpression();
    
    // Simple identifier call: foo()
    if (Node.isIdentifier(expression)) {
        return expression.getText();
    }
    
    // Property access call: obj.method()
    if (Node.isPropertyAccessExpression(expression)) {
        // Return the method name for matching against schema finals
        return expression.getName();
    }
    
    return null;
}

/**
 * Get the full dotted path for a property access call.
 * e.g., "seq.iCycle" for seq.iCycle()
 */
function getFullCallPath(call: CallExpression): string | null {
    const expression = call.getExpression();
    
    if (Node.isIdentifier(expression)) {
        return expression.getText();
    }
    
    if (Node.isPropertyAccessExpression(expression)) {
        return expression.getText();
    }
    
    return null;
}

/**
 * Check if a node is a literal that we should track spans for.
 * Includes: string literals, numeric literals, template literals,
 * array literals, and object literals.
 */
function isTrackableLiteral(node: Node): boolean {
    return (
        Node.isStringLiteral(node) ||
        Node.isNumericLiteral(node) ||
        Node.isNoSubstitutionTemplateLiteral(node) ||
        Node.isTemplateExpression(node) ||
        Node.isArrayLiteralExpression(node) ||
        Node.isObjectLiteralExpression(node) ||
        Node.isPrefixUnaryExpression(node) // for negative numbers like -5
    );
}

/**
 * Analyze DSL source code and build a span registry for argument locations.
 * 
 * @param source - The DSL source code to analyze
 * @param schemas - Module schemas to determine which calls to track
 * @param lineOffset - Line offset to add (for wrapped code in new Function)
 * @returns Registry mapping call sites to argument spans
 */
export function analyzeSourceSpans(
    source: string,
    schemas: ModuleSchema[],
    lineOffset: number = 0,
    firstLineColumnOffset: number = 0,
): SpanRegistry {
    const registry: SpanRegistry = new Map();
    const factoryNames = buildFactoryNames(schemas);
    const schemaMap = buildSchemaMap(schemas);
    
    // Create an in-memory TypeScript project
    const project = new Project({
        useInMemoryFileSystem: true,
        compilerOptions: {
            target: ts.ScriptTarget.ESNext,
            allowJs: true,
            checkJs: false,
            noEmit: true,
        },
    });
    
    // Add source as a virtual file
    const sourceFile = project.createSourceFile('dsl.ts', source);
    
    // Walk all call expressions
    sourceFile.forEachDescendant((node: Node) => {
        if (!Node.isCallExpression(node)) return;
        
        const call = node as CallExpression;
        const funcName = getCalledFunctionName(call);
        
        // Skip if not a tracked factory
        if (!funcName || !factoryNames.has(funcName)) return;
        
        // Get the schema for this call
        const fullPath = getFullCallPath(call);
        const schema = schemaMap.get(funcName) || (fullPath ? schemaMap.get(fullPath) : null);
        if (!schema || !schema.positionalArgs) return;
        
        const args = call.getArguments();
        const argsMap = new Map<string, SourceSpan>();
        
        // Map each argument to its positional arg name
        for (let i = 0; i < schema.positionalArgs.length && i < args.length; i++) {
            const arg = args[i];
            const argDef = schema.positionalArgs[i];
            
            // Only track literals (not variable references, function calls, etc.)
            if (isTrackableLiteral(arg)) {
                argsMap.set(argDef.name, {
                    start: arg.getStart(),
                    end: arg.getEnd(),
                });
            }
        }
        
        // Skip if no trackable arguments
        if (argsMap.size === 0) return;
        
        // Get the call site position
        // For property access calls like `seq.iCycle()`, V8 stack traces point to the
        // opening parenthesis, not the start of the expression. So we need to find
        // the position of the `(` in the call.
        const callExpression = call.getExpression();
        let callStartPos: number;
        
        if (Node.isPropertyAccessExpression(callExpression)) {
            // For `seq.iCycle()`, get the position of `iCycle`
            // The opening paren follows immediately after the method name
            callStartPos = callExpression.getNameNode().getStart();
        } else {
            // For simple calls like `foo()`, use the identifier start
            callStartPos = call.getStart();
        }
        
        // ts-morph gives 0-based line numbers, but stack traces are 1-based
        // Add the lineOffset to account for wrapper code in new Function()
        const { line, column } = sourceFile.getLineAndColumnAtPos(callStartPos);
        // line is already 1-based from ts-morph, column is 1-based too
        // Stack traces show "line:column" where line is 1-based and column is 0-based
        // For line 1 of source, we need to add the firstLineColumnOffset because
        // the function body template indents the first line with spaces
        const columnOffset = line === 1 ? firstLineColumnOffset : 0;
        const key: CallSiteKey = `${line + lineOffset}:${column - 1 + columnOffset}`;
        
        registry.set(key, {
            args: argsMap,
            moduleType: schema.name,
        });
    });
    
    return registry;
}

/**
 * Create an empty span registry (for when analysis is not needed)
 */
export function emptySpanRegistry(): SpanRegistry {
    return new Map();
}

/**
 * Debug helper: print registry contents
 */
export function debugPrintRegistry(registry: SpanRegistry): void {
    console.log('=== Span Registry ===');
    for (const [key, value] of registry) {
        console.log(`${key} (${value.moduleType}):`);
        for (const [argName, span] of value.args) {
            console.log(`  ${argName}: [${span.start}, ${span.end})`);
        }
    }
}
