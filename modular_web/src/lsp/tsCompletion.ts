import { autocompletion, type Completion, type CompletionContext, type CompletionResult } from "@codemirror/autocomplete";
import type { Extension } from "@codemirror/state";
import { tsLsRequest } from "./tsClient";

interface WorkerCompletionItem {
  name: string;
  kind: string;
  kindModifiers?: string;
  sortText?: string;
  insertText?: string;
  detail?: string;
  documentation?: string;
}

interface CompletionResponse {
  type: "completion";
  items: WorkerCompletionItem[];
}

async function completionSource(ctx: CompletionContext): Promise<CompletionResult | null> {
  const word = ctx.matchBefore(/[a-zA-Z0-9_$]+/);
  if (!word && !ctx.explicit) return null;

  const from = word ? word.from : ctx.pos;
  const offset = ctx.pos;

  const res = await tsLsRequest<CompletionResponse>("completion", { offset });
  const options: Completion[] = res.items.map((item) => ({
    label: item.name,
    type: item.kind as any,
    detail: item.detail,
    info: item.documentation,
  }));

  if (!options.length) return null;
  return { from, options };
}

export const tsCompletion: Extension = autocompletion({
  override: [completionSource],
});

