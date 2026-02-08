/**
 * Source Span Analyzer using ts-morph
 * 
 * Parses DSL source code and extracts absolute character offsets for literal
 * arguments in module factory calls. The registry is keyed by call-site
 * (line:column) for lookup from factory functions at runtime.
 *
 * Additionally builds an interpolation resolution map for template literals
 * containing const variable references. When a template like `${root} e4 g4`
 * interpolates a const string, the resolution map records the const's literal
 * span so that highlights landing inside the interpolation result can be
 * redirected to the original const declaration site. This works recursively
 * for nested template const chains.
 */

import { Project, SyntaxKind, Node, CallExpression, VariableDeclarationKind, type SourceFile, ts } from 'ts-morph';
import type { ModuleSchema } from '@modular/core';

// Re-export shared types/state from spanTypes (which has no Node.js dependencies)
export type { SourceSpan, ResolvedInterpolation, InterpolationResolutionMap } from './spanTypes';
export { setActiveInterpolationResolutions, getActiveInterpolationResolutions } from './spanTypes';

import type { SourceSpan, InterpolationResolutionMap, ResolvedInterpolation } from './spanTypes';

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
        // Track all module calls — positional args, config object properties,
        // and const variable references can all contribute trackable spans
        const parts = schema.name.split('.');
        const finalName = parts[parts.length - 1];
        names.add(finalName);
        
        // Also add the sanitized version (for direct calls like `seqICycle`)
        const sanitized = schema.name.replace(/[^a-zA-Z0-9]+(.)?/g, 
            (_match, chr: string | undefined) => (chr ? chr.toUpperCase() : '')
        );
        names.add(sanitized);
    }
    
    return names;
}

/**
 * Build a map from factory names to their schemas for quick lookup
 */
function buildSchemaMap(schemas: ModuleSchema[]): Map<string, ModuleSchema> {
    const map = new Map<string, ModuleSchema>();
    
    for (const schema of schemas) {
        const parts = schema.name.split('.');
        const finalName = parts[parts.length - 1];
        map.set(finalName, schema);
        
        const sanitized = schema.name.replace(/[^a-zA-Z0-9]+(.)?/g, 
            (_match, chr: string | undefined) => (chr ? chr.toUpperCase() : '')
        );
        map.set(sanitized, schema);
        map.set(schema.name, schema);
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
 * e.g., "$.iCycle" for $.iCycle()
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
 * Pre-build a map of const-declared variable names to their literal initializer spans.
 * Only includes variables declared with `const` whose initializer is a trackable literal.
 * Scans top-level statements only (sufficient for flat DSL scripts).
 */
function buildConstLiteralMap(sourceFile: SourceFile): Map<string, SourceSpan> {
    const map = new Map<string, SourceSpan>();
    
    for (const statement of sourceFile.getStatements()) {
        if (!Node.isVariableStatement(statement)) continue;
        
        const declList = statement.getDeclarationList();
        if (declList.getDeclarationKind() !== VariableDeclarationKind.Const) continue;
        
        for (const decl of declList.getDeclarations()) {
            const initializer = decl.getInitializer();
            if (initializer && isTrackableLiteral(initializer)) {
                map.set(decl.getName(), { start: initializer.getStart(), end: initializer.getEnd() });
            }
        }
    }
    
    return map;
}

/**
 * Pre-build a map of const-declared variable names to their AST initializer nodes.
 * Used for deeper analysis of template expressions within const declarations.
 */
function buildConstNodeMap(sourceFile: SourceFile): Map<string, Node> {
    const map = new Map<string, Node>();
    
    for (const statement of sourceFile.getStatements()) {
        if (!Node.isVariableStatement(statement)) continue;
        
        const declList = statement.getDeclarationList();
        if (declList.getDeclarationKind() !== VariableDeclarationKind.Const) continue;
        
        for (const decl of declList.getDeclarations()) {
            const initializer = decl.getInitializer();
            if (initializer && isTrackableLiteral(initializer)) {
                map.set(decl.getName(), initializer);
            }
        }
    }
    
    return map;
}

/**
 * Get the string content length of a const literal, stripping quotes.
 * Returns null if the length cannot be statically determined.
 * 
 * For simple string literals: `"abc"` → 3
 * For no-substitution template literals: `` `abc` `` → 3
 * For template expressions with all-const interpolations: recursively computed
 */
function getConstStringLength(
    node: Node,
    constNodeMap: Map<string, Node>,
    visited: Set<string> = new Set(),
): number | null {
    if (Node.isStringLiteral(node)) {
        // Strip quotes and compute length
        return node.getLiteralValue().length;
    }
    
    if (Node.isNoSubstitutionTemplateLiteral(node)) {
        return node.getLiteralValue().length;
    }
    
    if (Node.isTemplateExpression(node)) {
        // Sum up: head literal + (each span's expression evaluated length + literal part)
        const head = node.getHead();
        // Head text is between ` and ${, strip the backtick
        let totalLength = head.getLiteralText().length;
        
        for (const span of node.getTemplateSpans()) {
            const expr = span.getExpression();
            const exprLength = getExpressionStringLength(expr, constNodeMap, visited);
            if (exprLength === null) return null; // Can't determine
            totalLength += exprLength;
            
            // The literal part of the span (between } and next ${ or closing `)
            const literal = span.getLiteral();
            totalLength += literal.getLiteralText().length;
        }
        
        return totalLength;
    }
    
    return null;
}

/**
 * Get the evaluated string length of an expression node.
 * Only works for const identifier references with known literal initializers.
 */
function getExpressionStringLength(
    expr: Node,
    constNodeMap: Map<string, Node>,
    visited: Set<string>,
): number | null {
    if (Node.isIdentifier(expr)) {
        const name = expr.getText();
        if (visited.has(name)) return null; // Circular reference
        const constNode = constNodeMap.get(name);
        if (!constNode) return null;
        
        visited.add(name);
        const result = getConstStringLength(constNode, constNodeMap, visited);
        visited.delete(name);
        return result;
    }
    
    // Direct literals in interpolation (rarely useful but handle it)
    if (Node.isStringLiteral(expr)) {
        return expr.getLiteralValue().length;
    }
    if (Node.isNumericLiteral(expr)) {
        return expr.getText().length;
    }
    
    return null;
}

/**
 * Recursively resolve interpolations in a template expression or const reference.
 * 
 * For a template like `${root} e4 ${fifth}`:
 * - Computes the evaluated start position and length for each interpolation
 * - If the interpolated const is itself a template with interpolations, recurses
 * 
 * @param node - The template expression or const literal node
 * @param constNodeMap - Map of const names to their AST nodes
 * @param constMap - Map of const names to their document spans
 * @returns Array of resolved interpolations, or null if resolution not possible
 */
function resolveInterpolations(
    node: Node,
    constNodeMap: Map<string, Node>,
    constMap: Map<string, SourceSpan>,
): ResolvedInterpolation[] | null {
    if (!Node.isTemplateExpression(node)) return null;
    
    const resolutions: ResolvedInterpolation[] = [];
    const head = node.getHead();
    let evalOffset = head.getLiteralText().length;
    
    for (const span of node.getTemplateSpans()) {
        const expr = span.getExpression();
        
        if (Node.isIdentifier(expr)) {
            const name = expr.getText();
            const constNode = constNodeMap.get(name);
            const constSpan = constMap.get(name);
            
            if (constNode && constSpan) {
                const evalLength = getConstStringLength(constNode, constNodeMap);
                
                if (evalLength !== null) {
                    // Recursively resolve nested interpolations within the const
                    const nested = resolveInterpolations(constNode, constNodeMap, constMap);
                    
                    resolutions.push({
                        evaluatedStart: evalOffset,
                        evaluatedLength: evalLength,
                        constLiteralSpan: constSpan,
                        nestedResolutions: nested ?? undefined,
                    });
                    
                    evalOffset += evalLength;
                } else {
                    // Can't determine length — skip this interpolation but continue
                    // (offset tracking becomes unreliable, bail on remaining interpolations)
                    return resolutions.length > 0 ? resolutions : null;
                }
            } else {
                // Not a const reference — can't resolve. Stop tracking offsets.
                // Any previous resolutions are still valid since their offsets are correct.
                return resolutions.length > 0 ? resolutions : null;
            }
        } else {
            // Non-identifier expression in interpolation — can't resolve
            const exprLength = getExpressionStringLength(expr, constNodeMap, new Set());
            if (exprLength !== null) {
                evalOffset += exprLength;
            } else {
                return resolutions.length > 0 ? resolutions : null;
            }
        }
        
        // Add the literal text following this interpolation
        const literal = span.getLiteral();
        evalOffset += literal.getLiteralText().length;
    }
    
    return resolutions.length > 0 ? resolutions : null;
}

/**
 * Get a trackable span from a node, either directly (if it's a literal)
 * or by resolving a const variable reference to its literal initializer.
 */
function getTrackableSpan(node: Node, constMap: Map<string, SourceSpan>): SourceSpan | null {
    if (isTrackableLiteral(node)) {
        return { start: node.getStart(), end: node.getEnd() };
    }
    
    // Try resolving const variable reference
    if (Node.isIdentifier(node)) {
        return constMap.get(node.getText()) ?? null;
    }
    
    return null;
}

/**
 * Get the AST node for a trackable argument, resolving const references.
 * Returns the literal node itself, or the const's initializer node.
 * Used for deeper analysis like interpolation resolution.
 */
function getTrackableNode(node: Node, constNodeMap: Map<string, Node>): Node | null {
    if (isTrackableLiteral(node)) {
        return node;
    }
    
    if (Node.isIdentifier(node)) {
        return constNodeMap.get(node.getText()) ?? null;
    }
    
    return null;
}

/**
 * Result of analyzing source spans, including both the span registry
 * (for Rust-side argument highlighting) and the interpolation resolution map
 * (for TS-side redirect of highlights into const declarations).
 */
export interface AnalysisResult {
    /** Registry mapping call sites to argument spans */
    registry: SpanRegistry;
    /** Map from argument span key to resolved interpolations within that span */
    interpolationResolutions: InterpolationResolutionMap;
}

/**
 * Analyze DSL source code and build a span registry for argument locations.
 * 
 * @param source - The DSL source code to analyze
 * @param schemas - Module schemas to determine which calls to track
 * @param lineOffset - Line offset to add (for wrapped code in new Function)
 * @returns Analysis result with span registry and interpolation resolution map
 */
export function analyzeSourceSpans(
    source: string,
    schemas: ModuleSchema[],
    lineOffset: number = 0,
    firstLineColumnOffset: number = 0,
): AnalysisResult {
    const registry: SpanRegistry = new Map();
    const interpolationResolutions: InterpolationResolutionMap = new Map();
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
    
    // Pre-build const literal map for resolving variable references
    const constMap = buildConstLiteralMap(sourceFile);
    const constNodeMap = buildConstNodeMap(sourceFile);
    
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
        if (!schema) return;
        
        const args = call.getArguments();
        const argsMap = new Map<string, SourceSpan>();
        
        // Map each positional argument to its arg name
        const positionalArgs = schema.positionalArgs || [];
        for (let i = 0; i < positionalArgs.length && i < args.length; i++) {
            const arg = args[i];
            const argDef = positionalArgs[i];
            
            // Track literals directly, or resolve const variable references
            const span = getTrackableSpan(arg, constMap);
            if (span) {
                argsMap.set(argDef.name, span);
                
                // Resolve interpolations for template expressions
                const node = getTrackableNode(arg, constNodeMap);
                if (node) {
                    const resolutions = resolveInterpolations(node, constNodeMap, constMap);
                    if (resolutions) {
                        const spanKey = `${span.start}:${span.end}`;
                        interpolationResolutions.set(spanKey, resolutions);
                    }
                }
            }
        }
        
        // Check for config object argument (after positional args)
        if (positionalArgs.length < args.length) {
            const configArg = args[positionalArgs.length];
            
            if (Node.isObjectLiteralExpression(configArg)) {
                for (const prop of configArg.getProperties()) {
                    if (Node.isPropertyAssignment(prop)) {
                        const propName = prop.getName();
                        if (propName === 'id') continue;
                        
                        const initializer = prop.getInitializerOrThrow();
                        const span = getTrackableSpan(initializer, constMap);
                        if (span) {
                            argsMap.set(propName, span);
                            
                            // Resolve interpolations for template expressions
                            const node = getTrackableNode(initializer, constNodeMap);
                            if (node) {
                                const resolutions = resolveInterpolations(node, constNodeMap, constMap);
                                if (resolutions) {
                                    const spanKey = `${span.start}:${span.end}`;
                                    interpolationResolutions.set(spanKey, resolutions);
                                }
                            }
                        }
                    } else if (Node.isShorthandPropertyAssignment(prop)) {
                        const propName = prop.getName();
                        if (propName === 'id') continue;
                        
                        // For shorthand { myVar }, resolve the variable to its const literal
                        const span = constMap.get(propName) ?? null;
                        if (span) {
                            argsMap.set(propName, span);
                            
                            // Resolve interpolations if it's a template const
                            const constNode = constNodeMap.get(propName);
                            if (constNode) {
                                const resolutions = resolveInterpolations(constNode, constNodeMap, constMap);
                                if (resolutions) {
                                    const spanKey = `${span.start}:${span.end}`;
                                    interpolationResolutions.set(spanKey, resolutions);
                                }
                            }
                        }
                    }
                }
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
    
    return { registry, interpolationResolutions };
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
