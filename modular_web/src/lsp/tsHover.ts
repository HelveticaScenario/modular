import { hoverTooltip, type Tooltip } from "@codemirror/view";
import type { Extension } from "@codemirror/state";
import { tsLsRequest } from "./tsClient";

interface HoverInfo {
  start: number;
  length: number;
  text: string;
  documentation?: string;
}

interface HoverResponse {
  type: "hover";
  hover: HoverInfo | null;
}

export const tsHover: Extension = hoverTooltip(async (_view, pos) => {
  const res = await tsLsRequest<HoverResponse>("hover", { offset: pos });
  const info = res.hover;
  if (!info || !info.text) return null;

  const from = info.start ?? pos;
  const to = info.length ? from + info.length : from;

  return {
    pos: from,
    end: to,
    create() {
      const dom = document.createElement("div");
      dom.className = "cm-ts-hover";
      dom.textContent = info.text + (info.documentation ? "\n\n" + info.documentation : "");
      return { dom };
    },
  } as Tooltip;
});

