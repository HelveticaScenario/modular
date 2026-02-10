"use strict";
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
Object.defineProperty(exports, "__esModule", { value: true });
exports.getActiveInterpolationResolutions = exports.setActiveInterpolationResolutions = void 0;
exports.analyzeSourceSpans = analyzeSourceSpans;
exports.emptySpanRegistry = emptySpanRegistry;
exports.debugPrintRegistry = debugPrintRegistry;
const ts_morph_1 = require("ts-morph");
var spanTypes_1 = require("../../shared/dsl/spanTypes");
Object.defineProperty(exports, "setActiveInterpolationResolutions", { enumerable: true, get: function () { return spanTypes_1.setActiveInterpolationResolutions; } });
Object.defineProperty(exports, "getActiveInterpolationResolutions", { enumerable: true, get: function () { return spanTypes_1.getActiveInterpolationResolutions; } });
/**
 * Build a set of factory function names from module schemas.
 * Only includes modules that have positional arguments defined.
 */
function buildFactoryNames(schemas) {
    const names = new Set();
    for (const schema of schemas) {
        // Track all module calls — positional args, config object properties,
        // and const variable references can all contribute trackable spans
        const parts = schema.name.split('.');
        const finalName = parts[parts.length - 1];
        names.add(finalName);
        // Also add the sanitized version (for direct calls like `seqICycle`)
        const sanitized = schema.name.replace(/[^a-zA-Z0-9]+(.)?/g, (_match, chr) => (chr ? chr.toUpperCase() : ''));
        names.add(sanitized);
    }
    return names;
}
/**
 * Build a map from factory names to their schemas for quick lookup
 */
function buildSchemaMap(schemas) {
    const map = new Map();
    for (const schema of schemas) {
        const parts = schema.name.split('.');
        const finalName = parts[parts.length - 1];
        map.set(finalName, schema);
        const sanitized = schema.name.replace(/[^a-zA-Z0-9]+(.)?/g, (_match, chr) => (chr ? chr.toUpperCase() : ''));
        map.set(sanitized, schema);
        map.set(schema.name, schema);
    }
    return map;
}
/**
 * Extract the function name being called from a CallExpression.
 * Handles both simple calls (foo()) and property access calls (seq.iCycle()).
 */
function getCalledFunctionName(call) {
    const expression = call.getExpression();
    // Simple identifier call: foo()
    if (ts_morph_1.Node.isIdentifier(expression)) {
        return expression.getText();
    }
    // Property access call: obj.method()
    if (ts_morph_1.Node.isPropertyAccessExpression(expression)) {
        // Return the method name for matching against schema finals
        return expression.getName();
    }
    return null;
}
/**
 * Get the full dotted path for a property access call.
 * e.g., "$.iCycle" for $.iCycle()
 */
function getFullCallPath(call) {
    const expression = call.getExpression();
    if (ts_morph_1.Node.isIdentifier(expression)) {
        return expression.getText();
    }
    if (ts_morph_1.Node.isPropertyAccessExpression(expression)) {
        return expression.getText();
    }
    return null;
}
/**
 * Check if a node is a literal that we should track spans for.
 * Includes: string literals, numeric literals, template literals,
 * array literals, and object literals.
 */
function isTrackableLiteral(node) {
    return (ts_morph_1.Node.isStringLiteral(node) ||
        ts_morph_1.Node.isNumericLiteral(node) ||
        ts_morph_1.Node.isNoSubstitutionTemplateLiteral(node) ||
        ts_morph_1.Node.isTemplateExpression(node) ||
        ts_morph_1.Node.isArrayLiteralExpression(node) ||
        ts_morph_1.Node.isObjectLiteralExpression(node) ||
        ts_morph_1.Node.isPrefixUnaryExpression(node) // for negative numbers like -5
    );
}
/**
 * Pre-build a map of const-declared variable names to their literal initializer spans.
 * Only includes variables declared with `const` whose initializer is a trackable literal.
 * Scans top-level statements only (sufficient for flat DSL scripts).
 */
function buildConstLiteralMap(sourceFile) {
    const map = new Map();
    for (const statement of sourceFile.getStatements()) {
        if (!ts_morph_1.Node.isVariableStatement(statement))
            continue;
        const declList = statement.getDeclarationList();
        if (declList.getDeclarationKind() !== ts_morph_1.VariableDeclarationKind.Const)
            continue;
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
function buildConstNodeMap(sourceFile) {
    const map = new Map();
    for (const statement of sourceFile.getStatements()) {
        if (!ts_morph_1.Node.isVariableStatement(statement))
            continue;
        const declList = statement.getDeclarationList();
        if (declList.getDeclarationKind() !== ts_morph_1.VariableDeclarationKind.Const)
            continue;
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
function getConstStringLength(node, constNodeMap, visited = new Set()) {
    if (ts_morph_1.Node.isStringLiteral(node)) {
        // Strip quotes and compute length
        return node.getLiteralValue().length;
    }
    if (ts_morph_1.Node.isNoSubstitutionTemplateLiteral(node)) {
        return node.getLiteralValue().length;
    }
    if (ts_morph_1.Node.isTemplateExpression(node)) {
        // Sum up: head literal + (each span's expression evaluated length + literal part)
        const head = node.getHead();
        // Head text is between ` and ${, strip the backtick
        let totalLength = head.getLiteralText().length;
        for (const span of node.getTemplateSpans()) {
            const expr = span.getExpression();
            const exprLength = getExpressionStringLength(expr, constNodeMap, visited);
            if (exprLength === null)
                return null; // Can't determine
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
function getExpressionStringLength(expr, constNodeMap, visited) {
    if (ts_morph_1.Node.isIdentifier(expr)) {
        const name = expr.getText();
        if (visited.has(name))
            return null; // Circular reference
        const constNode = constNodeMap.get(name);
        if (!constNode)
            return null;
        visited.add(name);
        const result = getConstStringLength(constNode, constNodeMap, visited);
        visited.delete(name);
        return result;
    }
    // Direct literals in interpolation (rarely useful but handle it)
    if (ts_morph_1.Node.isStringLiteral(expr)) {
        return expr.getLiteralValue().length;
    }
    if (ts_morph_1.Node.isNumericLiteral(expr)) {
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
function resolveInterpolations(node, constNodeMap, constMap) {
    if (!ts_morph_1.Node.isTemplateExpression(node))
        return null;
    const resolutions = [];
    const head = node.getHead();
    let evalOffset = head.getLiteralText().length;
    for (const span of node.getTemplateSpans()) {
        const expr = span.getExpression();
        if (ts_morph_1.Node.isIdentifier(expr)) {
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
                }
                else {
                    // Can't determine length — skip this interpolation but continue
                    // (offset tracking becomes unreliable, bail on remaining interpolations)
                    return resolutions.length > 0 ? resolutions : null;
                }
            }
            else {
                // Not a const reference — can't resolve. Stop tracking offsets.
                // Any previous resolutions are still valid since their offsets are correct.
                return resolutions.length > 0 ? resolutions : null;
            }
        }
        else {
            // Non-identifier expression in interpolation — can't resolve
            const exprLength = getExpressionStringLength(expr, constNodeMap, new Set());
            if (exprLength !== null) {
                evalOffset += exprLength;
            }
            else {
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
function getTrackableSpan(node, constMap) {
    if (isTrackableLiteral(node)) {
        return { start: node.getStart(), end: node.getEnd() };
    }
    // Try resolving const variable reference
    if (ts_morph_1.Node.isIdentifier(node)) {
        return constMap.get(node.getText()) ?? null;
    }
    return null;
}
/**
 * Get the AST node for a trackable argument, resolving const references.
 * Returns the literal node itself, or the const's initializer node.
 * Used for deeper analysis like interpolation resolution.
 */
function getTrackableNode(node, constNodeMap) {
    if (isTrackableLiteral(node)) {
        return node;
    }
    if (ts_morph_1.Node.isIdentifier(node)) {
        return constNodeMap.get(node.getText()) ?? null;
    }
    return null;
}
/**
 * Analyze DSL source code and build a span registry for argument locations.
 *
 * @param source - The DSL source code to analyze
 * @param schemas - Module schemas to determine which calls to track
 * @param lineOffset - Line offset to add (for wrapped code in new Function)
 * @returns Analysis result with span registry and interpolation resolution map
 */
function analyzeSourceSpans(source, schemas, lineOffset = 0, firstLineColumnOffset = 0) {
    const registry = new Map();
    const interpolationResolutions = new Map();
    const factoryNames = buildFactoryNames(schemas);
    const schemaMap = buildSchemaMap(schemas);
    // Create an in-memory TypeScript project
    const project = new ts_morph_1.Project({
        useInMemoryFileSystem: true,
        compilerOptions: {
            target: ts_morph_1.ts.ScriptTarget.ESNext,
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
    sourceFile.forEachDescendant((node) => {
        if (!ts_morph_1.Node.isCallExpression(node))
            return;
        const call = node;
        const funcName = getCalledFunctionName(call);
        // Validate slider() calls: label (arg 0) and value (arg 1) must be literals
        if (funcName === 'slider') {
            const args = call.getArguments();
            if (args.length >= 1 && !ts_morph_1.Node.isStringLiteral(args[0])) {
                const { line, column } = sourceFile.getLineAndColumnAtPos(args[0].getStart());
                throw new Error(`slider() label (argument 1) must be a string literal at line ${line}, column ${column}`);
            }
            if (args.length >= 2 && !ts_morph_1.Node.isNumericLiteral(args[1]) && !ts_morph_1.Node.isPrefixUnaryExpression(args[1])) {
                const { line, column } = sourceFile.getLineAndColumnAtPos(args[1].getStart());
                throw new Error(`slider() value (argument 2) must be a numeric literal at line ${line}, column ${column}`);
            }
            return; // slider is not a module factory, skip further processing
        }
        // Skip if not a tracked factory
        if (!funcName || !factoryNames.has(funcName))
            return;
        // Get the schema for this call
        const fullPath = getFullCallPath(call);
        const schema = schemaMap.get(funcName) || (fullPath ? schemaMap.get(fullPath) : null);
        if (!schema)
            return;
        const args = call.getArguments();
        const argsMap = new Map();
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
            if (ts_morph_1.Node.isObjectLiteralExpression(configArg)) {
                for (const prop of configArg.getProperties()) {
                    if (ts_morph_1.Node.isPropertyAssignment(prop)) {
                        const propName = prop.getName();
                        if (propName === 'id')
                            continue;
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
                    }
                    else if (ts_morph_1.Node.isShorthandPropertyAssignment(prop)) {
                        const propName = prop.getName();
                        if (propName === 'id')
                            continue;
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
        if (argsMap.size === 0)
            return;
        // Get the call site position
        // For property access calls like `seq.iCycle()`, V8 stack traces point to the
        // opening parenthesis, not the start of the expression. So we need to find
        // the position of the `(` in the call.
        const callExpression = call.getExpression();
        let callStartPos;
        if (ts_morph_1.Node.isPropertyAccessExpression(callExpression)) {
            // For `seq.iCycle()`, get the position of `iCycle`
            // The opening paren follows immediately after the method name
            callStartPos = callExpression.getNameNode().getStart();
        }
        else {
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
        const key = `${line + lineOffset}:${column - 1 + columnOffset}`;
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
function emptySpanRegistry() {
    return new Map();
}
/**
 * Debug helper: print registry contents
 */
function debugPrintRegistry(registry) {
    console.log('=== Span Registry ===');
    for (const [key, value] of registry) {
        console.log(`${key} (${value.moduleType}):`);
        for (const [argName, span] of value.args) {
            console.log(`  ${argName}: [${span.start}, ${span.end})`);
        }
    }
}
//# sourceMappingURL=sourceSpanAnalyzer.js.map