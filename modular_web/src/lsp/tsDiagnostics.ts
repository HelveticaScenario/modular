import { linter, type Diagnostic } from "@codemirror/lint";
import type { Extension } from "@codemirror/state";
import { tsLsRequest } from "./tsClient";

interface WorkerDiagnostic {
  start: number;
  length: number;
  message: string;
  category: "error" | "warning" | "info" | "hint";
}

interface DiagnosticsResponse {
  type: "diagnostics";
  diags: WorkerDiagnostic[];
}

function mapSeverity(category: WorkerDiagnostic["category"]): "error" | "warning" | "info" {
  if (category === "error") return "error";
  if (category === "warning") return "warning";
  return "info";
}

export const tsLinter: Extension = linter(async (view) => {
  const text = view.state.doc.toString();
  await tsLsRequest("update", { text });
  const res = await tsLsRequest<DiagnosticsResponse>("diagnostics", {});

  return res.diags.map<Diagnostic>((d) => ({
    from: d.start,
    to: d.start + d.length,
    message: d.message,
    severity: mapSeverity(d.category),
  }));
});

