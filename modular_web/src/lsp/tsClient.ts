import type { ModuleSchema } from "../types";

let worker: Worker | null = null;
let nextId = 1;
	const pending = new Map<number, { resolve: (value: any) => void; reject: (error: any) => void }>();

function ensureWorker(): Worker {
  if (!worker) {
    worker = new Worker(new URL("./tsWorker.ts", import.meta.url), { type: "module" });

    worker.onmessage = (event: MessageEvent) => {
      const data = event.data as { id?: number; type?: string; error?: string } & Record<string, any>;
      if (typeof data.id !== "number") return;
      const entry = pending.get(data.id);
      if (!entry) return;
      pending.delete(data.id);
      if (data.type === "error") {
        entry.reject(new Error(data.error ?? "TypeScript worker error"));
      } else {
        entry.resolve(data);
      }
    };

    worker.onerror = (event) => {
      for (const [, { reject }] of pending) {
        reject(event.error ?? new Error("TypeScript worker error"));
      }
      pending.clear();
    };
  }
  return worker;
}

export function tsLsRequest<T = any>(type: string, payload: any): Promise<T> {
  const w = ensureWorker();
  const id = nextId++;
  return new Promise<T>((resolve, reject) => {
    pending.set(id, { resolve, reject });
    w.postMessage({ id, type, payload });
  });
}

	export function initTsWorker(fileName: string, text: string): Promise<any> {
	  return tsLsRequest("init", { fileName, text });
}

	export function updateTsWorkerSchemas(schemas: ModuleSchema[]): Promise<void> {
	  return tsLsRequest("updateSchemas", { schemas }).then(() => undefined);
	}

export function disposeTsWorker(): void {
  if (worker) {
    worker.terminate();
    worker = null;
  }
  pending.clear();
}

