import * as ts from "typescript";
import { buildLibSource } from "../dsl/typescriptLibGen";
import type { ModuleSchema } from "../types/generated/ModuleSchema";

console.log(buildLibSource)

const DEFAULT_FILE_NAME = "file:///modular/dsl.js";
const LIB_FILE_NAME = "lib.dsl.d.ts";

let fileName = DEFAULT_FILE_NAME;
let fileText = "";
let fileVersion = 0;

// Dynamically generated declarations based on ModuleSchema
let schemaLibSource = "";
let libVersion = 1;


const compilerOptions: ts.CompilerOptions = {
  allowJs: true,
  checkJs: true,
  target: ts.ScriptTarget.ES2020,
  module: ts.ModuleKind.ESNext,
  lib: ["ES2020"],
  noEmit: true,
};

const host: ts.LanguageServiceHost = {
  getCompilationSettings: () => compilerOptions,
  getScriptFileNames: () => [fileName, LIB_FILE_NAME],
  getScriptVersion: (name) => {
    if (name === fileName) return String(fileVersion);
    if (name === LIB_FILE_NAME) return String(libVersion);
    return "1";
  },
  getScriptSnapshot: (name) => {
    if (name === fileName) {
      return ts.ScriptSnapshot.fromString(fileText);
    }
    if (name === LIB_FILE_NAME) {
      return ts.ScriptSnapshot.fromString(schemaLibSource ?? '');
    }
    return undefined;
  },
  getCurrentDirectory: () => "/",
  getDefaultLibFileName: () => LIB_FILE_NAME,
  fileExists: (name) => name === fileName || name === LIB_FILE_NAME,
  readFile: () => undefined,
};

const ls = ts.createLanguageService(host);

function flattenMessageText(message: string | ts.DiagnosticMessageChain): string {
  if (typeof message === "string") return message;
  let result = String(message.messageText);
  let chain = message.next;
  while (chain && chain.length) {
    for (const part of chain) {
      result += " " + String(part.messageText);
    }
    chain = chain[0].next;
  }
  return result;
}

function categoryToString(category: ts.DiagnosticCategory): "error" | "warning" | "info" | "hint" {
  switch (category) {
    case ts.DiagnosticCategory.Error:
      return "error";
    case ts.DiagnosticCategory.Warning:
      return "warning";
    case ts.DiagnosticCategory.Message:
      return "info";
    case ts.DiagnosticCategory.Suggestion:
      return "hint";
    default:
      return "info";
  }
}

function updateSchemaLib(schemas: ModuleSchema[]): void {
  schemaLibSource = buildLibSource(schemas);
  libVersion++;
  ls.cleanupSemanticCache();
}

self.onmessage = (event: MessageEvent) => {
  const { id, type, payload } = event.data as { id: number; type: string; payload: any };
  try {
    if (type === "init") {
      fileName = payload.fileName || DEFAULT_FILE_NAME;
      fileText = payload.text || "";
      fileVersion++;
      if (Array.isArray(payload.schemas)) {
        updateSchemaLib(payload.schemas as ModuleSchema[]);
      }
      (self as any).postMessage({ id, type: "init", ok: true });
      return;
    }

    if (type === "update") {
      fileText = payload.text || "";
      fileVersion++;
      (self as any).postMessage({ id, type: "update", ok: true });
      return;
    }

    if (type === "updateSchemas") {
      const schemas = (payload.schemas ?? []) as ModuleSchema[];
      updateSchemaLib(schemas);
      (self as any).postMessage({ id, type: "updateSchemas", ok: true });
      return;
    }

    if (type === "completion") {
      const offset: number = payload.offset ?? 0;
      const info = ls.getCompletionsAtPosition(fileName, offset, {
        includeCompletionsWithInsertText: true,
        includeInsertTextCompletions: true,
      });
      const entries = info?.entries ?? [];
      const items = entries.map((entry) => {
        const details = ls.getCompletionEntryDetails(
          fileName,
          offset,
          entry.name,
          /* formatOptions */ undefined,
          entry.source,
          /* preferences */ undefined,
          entry.data,
        );
        const detailText = details?.displayParts?.map((p) => p.text).join("") ?? "";
        const docText = details?.documentation?.map((p) => p.text).join("") ?? "";
        return {
          name: entry.name,
          kind: entry.kind,
          kindModifiers: entry.kindModifiers,
          sortText: entry.sortText,
          insertText: entry.insertText,
          detail: detailText || undefined,
          documentation: docText || undefined,
        };
      });
      (self as any).postMessage({ id, type: "completion", items });
      return;
    }

    if (type === "hover") {
      const offset: number = payload.offset ?? 0;
      const info = ls.getQuickInfoAtPosition(fileName, offset);
      if (!info) {
        (self as any).postMessage({ id, type: "hover", hover: null });
        return;
      }
      const display = info.displayParts?.map((p) => p.text).join("") ?? "";
      const doc = info.documentation?.map((p) => p.text).join("") ?? "";
      (self as any).postMessage({
        id,
        type: "hover",
        hover: {
          start: info.textSpan.start,
          length: info.textSpan.length,
          text: display,
          documentation: doc || undefined,
        },
      });
      return;
    }

    if (type === "diagnostics") {
      const syntactic = ls.getSyntacticDiagnostics(fileName);
      const semantic = ls.getSemanticDiagnostics(fileName);
      const all = [...syntactic, ...semantic];
      const diags = all.map((d) => ({
        start: d.start ?? 0,
        length: d.length ?? 0,
        message: flattenMessageText(d.messageText),
        category: categoryToString(d.category),
      }));
      (self as any).postMessage({ id, type: "diagnostics", diags });
      return;
    }

    (self as any).postMessage({ id, type: "error", error: `Unknown request type: ${type}` });
  } catch (err) {
    (self as any).postMessage({ id, type: "error", error: String(err) });
  }
};

