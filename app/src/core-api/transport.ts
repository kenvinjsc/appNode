// Transport: Tauri IPC in the desktop app, HTTP to the dev server in a browser.
import type { Response } from './types';

type Invoke = (cmd: string, args: Record<string, unknown>) => Promise<string>;

let tauriInvoke: Invoke | null | undefined;

async function getTauri(): Promise<Invoke | null> {
  if (tauriInvoke !== undefined) return tauriInvoke;
  const w = window as unknown as { __TAURI_INTERNALS__?: unknown };
  if (w.__TAURI_INTERNALS__) {
    const mod = await import('@tauri-apps/api/core');
    tauriInvoke = mod.invoke as Invoke;
  } else {
    tauriInvoke = null;
  }
  return tauriInvoke;
}

export class TransportError extends Error {}

/** Send one request to the core. Requests are serialised: the core is single-threaded. */
let queue: Promise<unknown> = Promise.resolve();

export function send<T>(request: Record<string, unknown>, signal?: AbortSignal): Promise<Response<T>> {
  const run = async (): Promise<Response<T>> => {
    if (signal?.aborted) throw new DOMException('Aborted', 'AbortError');
    const invoke = await getTauri();
    const body = JSON.stringify(request);
    let text: string;
    if (invoke) {
      text = await invoke('dispatch', { request: body });
    } else {
      const r = await fetch('/api', { method: 'POST', headers: { 'content-type': 'application/json' }, body, signal });
      if (!r.ok) throw new TransportError(`HTTP ${r.status}`);
      text = await r.text();
    }
    return JSON.parse(text) as Response<T>;
  };
  const p = queue.then(run, run);
  queue = p.catch(() => undefined);
  return p;
}
