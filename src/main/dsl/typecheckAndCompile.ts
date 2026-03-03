/**
 * TypeScript typechecking and compilation for user DSL scripts.
 *
 * Takes user TypeScript source + the generated DSL lib `.d.ts`,
 * runs lightweight typechecking via ts-morph, and returns either
 * diagnostics or compiled JS with a source map.
 *
 * The returned SourceFile on success is reused by span analysis
 * (analyzeArgumentSpans / analyzeCallSiteSpans) to avoid double-parsing.
 */

import { Project, type SourceFile, ts } from 'ts-morph';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface TypeDiagnostic {
    message: string;
    /** 1-based line in the user's TS source */
    line: number;
    /** 1-based column in the user's TS source */
    column: number;
    /** TS error code (e.g. 2304) */
    code: number;
    category: 'error' | 'warning' | 'suggestion';
}

export interface TypecheckFailure {
    diagnostics: TypeDiagnostic[];
}

export interface TypecheckSuccess {
    compiledJs: string;
    sourceMapJson: string;
    sourceFile: SourceFile;
}

export type TypecheckResult = TypecheckFailure | TypecheckSuccess;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const SCRIPT_PATH = 'script.ts';
const DSL_LIB_PATH = 'dsl-lib.d.ts';

function tsDiagnosticCategory(
    category: ts.DiagnosticCategory,
): TypeDiagnostic['category'] {
    switch (category) {
        case ts.DiagnosticCategory.Error:
            return 'error';
        case ts.DiagnosticCategory.Warning:
            return 'warning';
        default:
            return 'suggestion';
    }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

/**
 * Typecheck and compile a user's TypeScript DSL script.
 *
 * @param source     - The user's TypeScript source code
 * @param dslLibSource - The `.d.ts` declaration string for DSL globals
 * @returns Either `{ diagnostics }` on type errors, or
 *          `{ compiledJs, sourceMapJson, sourceFile }` on success.
 */
export function typecheckAndCompile(
    source: string,
    dslLibSource: string,
): TypecheckResult {
    const project = new Project({
        useInMemoryFileSystem: true,
        compilerOptions: {
            target: ts.ScriptTarget.ESNext,
            module: ts.ModuleKind.ESNext,
            strict: false,
            noImplicitAny: false,
            alwaysStrict: true,
            sourceMap: true,
            declaration: false,
            noEmit: false,
            skipLibCheck: true,
            lib: ['lib.esnext.d.ts'],
        },
    });

    // Add the DSL lib declarations to the virtual filesystem
    project.createSourceFile(DSL_LIB_PATH, dslLibSource);

    // Add the user's script
    const sourceFile = project.createSourceFile(SCRIPT_PATH, source);

    // -----------------------------------------------------------------------
    // Typecheck – only report diagnostics originating from the user's script
    // -----------------------------------------------------------------------
    const allDiagnostics = project.getPreEmitDiagnostics();
    const scriptDiagnostics = allDiagnostics.filter((d) => {
        const filePath = d.getSourceFile()?.getFilePath();
        return filePath !== undefined && filePath.endsWith(SCRIPT_PATH);
    });

    if (scriptDiagnostics.length > 0) {
        const diagnostics: TypeDiagnostic[] = scriptDiagnostics.map((d) => {
            const start = d.getStart();
            let line = 1;
            let column = 1;
            if (start !== undefined) {
                const pos = sourceFile.getLineAndColumnAtPos(start);
                line = pos.line;
                column = pos.column;
            }

            const rawMsg = d.getMessageText();
            const message =
                typeof rawMsg === 'string'
                    ? rawMsg
                    : ts.flattenDiagnosticMessageText(
                          rawMsg.compilerObject,
                          '\n',
                      );

            return {
                message,
                line,
                column,
                code: d.getCode(),
                category: tsDiagnosticCategory(d.getCategory()),
            };
        });

        return { diagnostics };
    }

    // -----------------------------------------------------------------------
    // Emit – extract compiled JS and source map from the in-memory output
    // -----------------------------------------------------------------------
    const emitOutput = sourceFile.getEmitOutput();
    const outputFiles = emitOutput.getOutputFiles();

    let compiledJs = '';
    let sourceMapJson = '';

    for (const file of outputFiles) {
        const filePath = file.getFilePath();
        if (filePath.endsWith('.js.map')) {
            sourceMapJson = file.getText();
        } else if (filePath.endsWith('.js')) {
            compiledJs = file.getText();
        }
    }

    return { compiledJs, sourceMapJson, sourceFile };
}
